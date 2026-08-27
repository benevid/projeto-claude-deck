//! Integracao opencode (M7): estados ao vivo pelo event-sourcing em sqlite.
//!
//! O opencode grava um log de eventos em `~/.local/share/opencode/opencode.db`
//! (tabela `event`: seq/type/data-JSON com sessionID; tabela `session` tem o
//! `directory` = cwd). Leitura read-only no banco vivo funciona sem conflito (WAL).
//! Poll por rowid com `rusqlite` (SQLite embutido — cross-platform, o Windows nao
//! tem `sqlite3` no PATH). Offset comeca no fim (so eventos novos). Acoes continuam
//! por teclado (dispatch por engine).

use crate::app::Shared;
use crate::model::Engine;
use crate::protocol::State;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

fn db_path() -> Option<std::path::PathBuf> {
    directories::BaseDirs::new().map(|b| b.home_dir().join(".local/share/opencode/opencode.db"))
}

/// Roda um SELECT read-only e devolve as linhas como objetos JSON (colunas -> valor).
/// Bloqueante: sempre chamada dentro de `spawn_blocking`.
fn query_blocking(db: &std::path::Path, sql: &str) -> Option<Vec<Value>> {
    use rusqlite::{Connection, OpenFlags};
    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    let mut stmt = conn.prepare(sql).ok()?;
    let cols: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let rows = stmt
        .query_map([], |row| {
            let mut obj = serde_json::Map::new();
            for (i, name) in cols.iter().enumerate() {
                let v = match row.get_ref(i) {
                    Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                    Ok(rusqlite::types::ValueRef::Integer(n)) => Value::from(n),
                    Ok(rusqlite::types::ValueRef::Real(f)) => Value::from(f),
                    Ok(rusqlite::types::ValueRef::Text(t)) => Value::from(String::from_utf8_lossy(t).to_string()),
                    Ok(rusqlite::types::ValueRef::Blob(_)) | Err(_) => Value::Null,
                };
                obj.insert(name.clone(), v);
            }
            Ok(Value::Object(obj))
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect();
    Some(rows)
}

async fn query_json(db: &std::path::Path, sql: &str) -> Option<Vec<Value>> {
    let db = db.to_path_buf();
    let sql = sql.to_string();
    tokio::task::spawn_blocking(move || query_blocking(&db, &sql)).await.ok()?
}

async fn max_rowid(db: &std::path::Path, table: &str) -> i64 {
    query_json(db, &format!("select coalesce(max(rowid),0) as m from {table}"))
        .await
        .and_then(|r| r.first().and_then(|v| v.get("m")).and_then(|v| v.as_i64()))
        .unwrap_or(0)
}

pub async fn run_opencode(st: Shared) {
    if !st.cfg.opencode.enabled {
        tracing::info!("opencode: integracao desligada na config");
        return;
    }
    let Some(db) = db_path() else { return };
    if !db.exists() {
        tracing::info!("opencode: banco nao encontrado — integracao inativa");
        return;
    }
    let mut last_ev = max_rowid(&db, "event").await;
    let mut dirs: HashMap<String, String> = HashMap::new();
    tracing::info!("opencode: acompanhando eventos de {} (a partir do rowid {last_ev})", db.display());
    let mut iv = tokio::time::interval(Duration::from_millis(1500));
    loop {
        iv.tick().await;
        // mapa sessao -> diretorio (barato; limita as recentes)
        if let Some(rows) = query_json(&db, "select id, directory from session order by time_updated desc limit 60").await {
            for r in rows {
                if let (Some(id), Some(d)) = (
                    r.get("id").and_then(|v| v.as_str()),
                    r.get("directory").and_then(|v| v.as_str()),
                ) {
                    dirs.insert(id.to_string(), d.to_string());
                }
            }
        }
        let sql = format!("select rowid, type, substr(data,1,2000) as data from event where rowid > {last_ev} order by rowid limit 300");
        let Some(rows) = query_json(&db, &sql).await else { continue };
        let mut per_dir: HashMap<String, State> = HashMap::new();
        for r in &rows {
            if let Some(rid) = r.get("rowid").and_then(|v| v.as_i64()) {
                last_ev = last_ev.max(rid);
            }
            let t = r.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let data: Value = r
                .get("data")
                .and_then(|v| v.as_str())
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(Value::Null);
            if t.starts_with("session.") {
                if let (Some(id), Some(d)) = (
                    data.get("info").and_then(|i| i.get("id")).and_then(|v| v.as_str()),
                    data.get("info").and_then(|i| i.get("directory")).and_then(|v| v.as_str()),
                ) {
                    dirs.insert(id.to_string(), d.to_string());
                }
                continue;
            }
            let sid = data
                .get("sessionID")
                .and_then(|v| v.as_str())
                .or_else(|| data.get("info").and_then(|i| i.get("sessionID")).and_then(|v| v.as_str()));
            let Some(sid) = sid else { continue };
            let Some(dir) = dirs.get(sid) else { continue };
            let state = if t.starts_with("message.part.updated") {
                // tool parts carregam state.status: um pedido de Allow/Reject fica
                // parado em "pending" (auto-aprovados viram "running" no mesmo lote,
                // e o ultimo estado do lote vence)
                let part = data.get("part");
                let is_tool = part.and_then(|pt| pt.get("type")).and_then(|v| v.as_str()) == Some("tool");
                let status = part
                    .and_then(|pt| pt.get("state"))
                    .and_then(|st| st.get("status"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if is_tool && status == "pending" {
                    State::Attention
                } else {
                    State::Working
                }
            } else if t.starts_with("message.updated") {
                let role = data.get("info").and_then(|i| i.get("role")).and_then(|v| v.as_str()).unwrap_or("");
                let done = data
                    .get("info")
                    .and_then(|i| i.get("time"))
                    .and_then(|tm| tm.get("completed"))
                    .is_some();
                if role == "assistant" && done {
                    State::Done
                } else {
                    State::Working
                }
            } else if t.contains("permission") {
                State::Attention
            } else {
                continue;
            };
            per_dir.insert(dir.clone(), state);
        }
        let mut changed = false;
        for (dir, state) in per_dir {
            if st.model().apply_engine_state(Engine::Opencode, &dir, state).is_some() {
                changed = true;
            }
        }
        if changed {
            st.notify_changed();
        }
    }
}
