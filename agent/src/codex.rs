//! Integracao com o Codex CLI (M6.a, leitura): embute `codex app-server` como
//! processo filho (JSON-RPC NDJSON via stdio), lista threads periodicamente e
//! traduz status/notificacoes para o estado das sessoes Codex descobertas por
//! processo (casamento por cwd). Nao executa nenhuma acao (M6.b).

use crate::app::Shared;
use crate::protocol::State;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// Mapeia o `ThreadStatus` do protocolo do app-server para o nosso estado.
fn map_status(status: &Value) -> Option<State> {
    let t = status.get("type")?.as_str()?;
    Some(match t {
        "active" => {
            let flags = status.get("activeFlags").and_then(|f| f.as_array());
            let waiting = flags
                .map(|a| a.iter().any(|v| v.as_str() == Some("waitingOnApproval") || v.as_str() == Some("waitingOnUserInput")))
                .unwrap_or(false);
            if waiting {
                State::Attention
            } else {
                State::Working
            }
        }
        "idle" => State::Idle,
        "systemError" => State::Error,
        "notLoaded" => return None,
        _ => return None,
    })
}

fn apply_thread(st: &Shared, threads: &mut HashMap<String, String>, seen: &mut HashMap<String, i64>, item: &Value) {
    let Some(id) = item.get("id").and_then(|v| v.as_str()) else { return };
    let cwd = item
        .get("cwd")
        .or_else(|| item.get("workingDirectory"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if !cwd.is_empty() {
        threads.insert(id.to_string(), cwd.clone());
        st.model().set_codex_thread(&cwd, id);
    }
    let mut state = item.get("status").and_then(map_status);
    // threads carregados em OUTRO processo aparecem como notLoaded aqui; o avanco
    // de updatedAt entre polls e o sinal de que ha um turno em andamento
    if let Some(upd) = item.get("updatedAt").and_then(|v| v.as_i64()) {
        let prev = seen.insert(id.to_string(), upd);
        if state.is_none() {
            if let Some(prev) = prev {
                if upd > prev {
                    state = Some(State::Working);
                }
            }
        }
    }
    let Some(state) = state else { return };
    let cwd = if cwd.is_empty() {
        match threads.get(id) {
            Some(c) => c.clone(),
            None => return,
        }
    } else {
        cwd
    };
    if st.model().apply_engine_state(crate::model::Engine::Codex, &cwd, state).is_some() {
        st.notify_changed();
    }
}

fn handle_notification(st: &Shared, threads: &mut HashMap<String, String>, method: &str, params: &Value) {
    let thread_cwd = |threads: &HashMap<String, String>, params: &Value| -> Option<String> {
        let id = params.get("threadId").and_then(|v| v.as_str())?;
        threads.get(id).cloned()
    };
    match method {
        "thread/status/changed" => {
            if let (Some(cwd), Some(status)) = (thread_cwd(threads, params), params.get("status")) {
                if let Some(state) = map_status(status) {
                    if st.model().apply_engine_state(crate::model::Engine::Codex, &cwd, state).is_some() {
                        st.notify_changed();
                    }
                }
            }
        }
        "turn/started" => {
            if let Some(cwd) = thread_cwd(threads, params) {
                if st.model().apply_engine_state(crate::model::Engine::Codex, &cwd, State::Working).is_some() {
                    st.notify_changed();
                }
            }
        }
        "turn/completed" => {
            if let Some(cwd) = thread_cwd(threads, params) {
                if st.model().apply_engine_state(crate::model::Engine::Codex, &cwd, State::Done).is_some() {
                    st.notify_changed();
                }
            }
        }
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" | "item/permissions/requestApproval" => {
            if let Some(cwd) = thread_cwd(threads, params) {
                if st.model().apply_engine_state(crate::model::Engine::Codex, &cwd, State::Attention).is_some() {
                    st.notify_changed();
                }
            }
        }
        "thread/closed" | "thread/archived" => {
            if let Some(id) = params.get("threadId").and_then(|v| v.as_str()) {
                threads.remove(id);
            }
        }
        _ => {}
    }
}

async fn run_once(st: &Shared) -> Result<()> {
    let mut child = Command::new("codex")
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("codex app-server (o codex esta no PATH?)")?;
    let mut stdin = child.stdin.take().context("stdin do app-server")?;
    let stdout = child.stdout.take().context("stdout do app-server")?;
    let mut lines = BufReader::new(stdout).lines();

    let send = |obj: Value| {
        let mut s = obj.to_string();
        s.push('\n');
        s
    };
    stdin
        .write_all(
            send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "clientInfo": {"name": "clowdeck-agent", "version": env!("CARGO_PKG_VERSION")},
                "capabilities": {"experimentalApi": true}}}))
            .as_bytes(),
        )
        .await?;

    let mut threads: HashMap<String, String> = HashMap::new();
    let mut seen: HashMap<String, i64> = HashMap::new();
    let mut req_id: u64 = 10;
    let poll = Duration::from_secs(st.cfg.codex.poll_s.max(2));
    let mut tick = tokio::time::interval(poll);
    tracing::info!("codex: app-server embutido conectado");
    loop {
        tokio::select! {
            _ = tick.tick() => {
                req_id += 1;
                stdin.write_all(send(json!({"jsonrpc":"2.0","id":req_id,"method":"thread/list",
                    "params":{"limit":32,"sortKey":"recency_at","sortDirection":"desc","sourceKinds":["cli","vscode","exec","appServer","unknown"]}})).as_bytes()).await?;
            }
            line = lines.next_line() => {
                let Some(line) = line? else { anyhow::bail!("app-server encerrou o stdout") };
                let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                    let params = msg.get("params").cloned().unwrap_or(Value::Null);
                    handle_notification(st, &mut threads, method, &params);
                } else if let Some(result) = msg.get("result") {
                    if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
                        for item in data {
                            apply_thread(st, &mut threads, &mut seen, item);
                        }
                    }
                }
            }
        }
    }
}

/// Acao M6.b: enfileira uma mensagem num thread existente do Codex sem focar
/// janela (`codex queue` grava na fila compartilhada que o TUI consome).
pub async fn queue_message(thread_id: &str, text: &str) -> Result<()> {
    let out = Command::new("codex")
        .args(["queue", "--thread", thread_id, "--message", text])
        .output()
        .await
        .context("codex queue")?;
    if !out.status.success() {
        anyhow::bail!("codex queue: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

/// M6.c: estados AO VIVO das sessoes Codex dirigidas pelo TUI, sem conflito —
/// tail dos rollouts (`~/.codex/sessions/AAAA/MM/DD/rollout-*.jsonl`), que o TUI
/// escreve conforme o turno avanca: task_started -> WORKING, task_complete -> DONE,
/// eventos de aprovacao -> ATTENTION. So eventos NOVOS (offset começa no fim).
async fn run_rollout_tail(st: Shared) {
    use std::collections::HashMap;
    use std::io::{Read as _, Seek as _};
    let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) else { return };
    let root = home.join(".codex/sessions");
    // path -> (cwd, offset)
    let mut files: HashMap<std::path::PathBuf, (String, u64)> = HashMap::new();
    let mut iv = tokio::time::interval(Duration::from_millis(1500));
    loop {
        iv.tick().await;
        let root = root.clone();
        let known: Vec<std::path::PathBuf> = files.keys().cloned().collect();
        // varre so os diretorios de hoje/ontem (arquivos novos)
        let found = tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            let today = chrono::Local::now();
            for d in [today, today - chrono::Duration::days(1)] {
                let dir = root.join(format!("{}", d.format("%Y/%m/%d")));
                if let Ok(rd) = std::fs::read_dir(&dir) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.extension().map(|x| x == "jsonl").unwrap_or(false) && !known.contains(&p) {
                            out.push(p);
                        }
                    }
                }
            }
            out
        })
        .await
        .unwrap_or_default();
        for p in found {
            // primeira linha = session_meta com o cwd; offset comeca no fim (so futuro)
            let meta = std::fs::File::open(&p).ok().and_then(|f| {
                use std::io::BufRead as _;
                // a 1a linha (session_meta) embute as base_instructions — pode passar de 4 KB
                let mut rd = std::io::BufReader::new(f);
                let mut first = String::new();
                rd.read_line(&mut first).ok()?;
                let v: Value = serde_json::from_str(first.trim()).ok()?;
                let cwd = v.get("payload")?.get("cwd")?.as_str()?.to_string();
                let len = rd.into_inner().seek(std::io::SeekFrom::End(0)).ok()?;
                Some((cwd, len))
            });
            if let Some((cwd, len)) = meta {
                tracing::info!("codex: acompanhando rollout de {cwd} ({})", p.display());
                files.insert(p, (cwd, len));
            }
        }
        for (p, (cwd, offset)) in files.iter_mut() {
            let Ok(md) = std::fs::metadata(p) else { continue };
            let len = md.len();
            if len <= *offset {
                continue;
            }
            let Ok(mut f) = std::fs::File::open(p) else { continue };
            if f.seek(std::io::SeekFrom::Start(*offset)).is_err() {
                continue;
            }
            let mut buf = String::new();
            if f.read_to_string(&mut buf).is_err() {
                continue;
            }
            *offset = len;
            let mut state: Option<State> = None;
            for line in buf.lines() {
                let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
                if v.get("type").and_then(|t| t.as_str()) != Some("event_msg") {
                    continue;
                }
                let pt = v.get("payload").and_then(|pl| pl.get("type")).and_then(|t| t.as_str()).unwrap_or("");
                state = Some(match pt {
                    "task_started" | "item_completed" | "token_count" => State::Working,
                    "task_complete" => State::Done,
                    "turn_aborted" => State::Idle,
                    t if t.contains("approval") || t.contains("request_user") => State::Attention,
                    _ => continue,
                });
            }
            if let Some(state) = state {
                if st.model().apply_engine_state(crate::model::Engine::Codex, cwd, state).is_some() {
                    st.notify_changed();
                }
            }
        }
    }
}

/// Task de integracao: reinicia o filho com backoff se cair; silenciosa se o
/// binario `codex` nao existir.
pub async fn run_codex(st: Shared) {
    if !st.cfg.codex.enabled {
        tracing::info!("codex: integracao desligada na config");
        return;
    }
    if std::process::Command::new("which").arg("codex").output().map(|o| !o.status.success()).unwrap_or(true) {
        tracing::info!("codex: binario nao encontrado — integracao inativa");
        return;
    }
    tokio::spawn(run_rollout_tail(st.clone()));
    let mut backoff = 2u64;
    loop {
        match run_once(&st).await {
            Ok(()) => backoff = 2,
            Err(e) => {
                tracing::warn!("codex: {e:#} — nova tentativa em {backoff}s");
            }
        }
        tokio::time::sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(60);
    }
}
