//! Hooks do Claude Code: receptor HTTP (`POST /hook/{event}?pid=$PPID`) e
//! instalacao/remocao idempotente em `~/.claude/settings.json`.

use crate::app::Shared;
use crate::model::HookEvent;
use anyhow::{Context, Result};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Map, Value};
use std::path::{Path as FsPath, PathBuf};
use std::sync::atomic::Ordering;

pub const EVENTS: [&str; 9] = [
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "Notification",
    "Stop",
    "PreCompact",
];

/// Marcador que identifica os nossos hooks (vai na URL: funciona em sh e cmd).
pub const MARKER: &str = "src=clowdeck";

#[derive(serde::Deserialize, Default)]
pub struct HookQuery {
    pub pid: Option<u32>,
    #[allow(dead_code)]
    pub src: Option<String>,
}

fn str_field(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string())
}

pub fn parse_hook(event: &str, pid: Option<u32>, body: &str) -> (HookEvent, Value) {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let ev = HookEvent {
        event: str_field(&v, "hook_event_name").unwrap_or_else(|| event.to_string()),
        pid: pid.filter(|p| *p > 1),
        session_id: str_field(&v, "session_id"),
        cwd: str_field(&v, "cwd"),
        permission_mode: str_field(&v, "permission_mode"),
        notification_type: str_field(&v, "notification_type"),
        source: str_field(&v, "source"),
    };
    (ev, v)
}

/// `POST /hook/{event}?pid=…` — sempre 200 e rapido; o Claude nao pode esperar.
pub async fn hook_handler(
    State(st): State<Shared>,
    Path(event): Path<String>,
    Query(q): Query<HookQuery>,
    body: String,
) -> Json<Value> {
    let (ev, raw) = parse_hook(&event, q.pid, &body);
    st.hooks_received.fetch_add(1, Ordering::Relaxed);
    let cell = {
        let mut m = st.model();
        m.apply_hook(&ev)
    };
    tracing::info!(
        "hook {} pid={:?} cwd={:?} mode={:?} -> celula {:?}",
        ev.event,
        ev.pid,
        ev.cwd.as_deref().map(crate::model::label_for),
        ev.permission_mode,
        cell
    );
    let rec = json!({
        "event": event, "pid": q.pid, "cell": cell,
        "at": chrono::Local::now().format("%H:%M:%S%.3f").to_string(), "body": raw
    });
    *st.last_hook.lock().unwrap_or_else(|e| e.into_inner()) = Some(rec.clone());
    {
        let mut r = st.recent_hooks.lock().unwrap_or_else(|e| e.into_inner());
        if r.len() >= 20 {
            r.pop_front();
        }
        r.push_back(rec);
    }
    st.notify_changed();
    Json(json!({}))
}

pub fn default_settings_path() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().join(".claude").join("settings.json"))
        .unwrap_or_else(|| PathBuf::from("settings.json"))
}

/// Comando de hook (um por evento). `$PPID` = processo `claude` (pai do shell do hook).
pub fn hook_command(event: &str, port: u16) -> String {
    if cfg!(windows) {
        format!(
            "curl.exe -s -m 1 -X POST -H \"Content-Type: application/json\" --data-binary @- \"http://127.0.0.1:{port}/hook/{event}?{MARKER}\" >NUL 2>&1"
        )
    } else {
        format!(
            "curl -s -m 1 -X POST -H 'Content-Type: application/json' --data-binary @- \"http://127.0.0.1:{port}/hook/{event}?pid=$PPID&{MARKER}\" >/dev/null 2>&1 || true"
        )
    }
}

pub fn is_ours(cmd: &str) -> bool {
    cmd.contains("/hook/") && cmd.contains(MARKER)
}

#[derive(Debug, Default, serde::Serialize)]
pub struct Report {
    pub path: PathBuf,
    pub backup: Option<PathBuf>,
    pub installed: Vec<String>,
    pub removed: usize,
    pub foreign_kept: usize,
}

fn read_settings(path: &FsPath) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    let txt = std::fs::read_to_string(path).with_context(|| format!("lendo {}", path.display()))?;
    if txt.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let v: Value = serde_json::from_str(&txt).with_context(|| format!("{} nao e JSON valido", path.display()))?;
    anyhow::ensure!(v.is_object(), "{} nao e um objeto JSON", path.display());
    Ok(v)
}

fn write_settings(path: &FsPath, v: &Value) -> Result<Option<PathBuf>> {
    let mut backup = None;
    if path.exists() {
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let b = path.with_file_name(format!(
            "{}.bak-clowdeck-{ts}",
            path.file_name().and_then(|f| f.to_str()).unwrap_or("settings.json")
        ));
        std::fs::copy(path, &b).with_context(|| format!("backup em {}", b.display()))?;
        backup = Some(b);
    } else if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let txt = serde_json::to_string_pretty(v)?;
    std::fs::write(path, format!("{txt}\n")).with_context(|| format!("escrevendo {}", path.display()))?;
    Ok(backup)
}

/// Remove os nossos hooks de `hooks` (mantendo os alheios). Devolve (removidos, alheios mantidos).
fn strip_ours(hooks: &mut Map<String, Value>) -> (usize, usize) {
    let mut removed = 0;
    let mut kept = 0;
    let keys: Vec<String> = hooks.keys().cloned().collect();
    for k in keys {
        let Some(Value::Array(groups)) = hooks.get_mut(&k) else { continue };
        for g in groups.iter_mut() {
            if let Some(Value::Array(list)) = g.get_mut("hooks") {
                let before = list.len();
                list.retain(|h| !h.get("command").and_then(|c| c.as_str()).map(is_ours).unwrap_or(false));
                removed += before - list.len();
                kept += list.len();
            }
        }
        groups.retain(|g| g.get("hooks").and_then(|h| h.as_array()).map(|a| !a.is_empty()).unwrap_or(true));
        if groups.is_empty() {
            hooks.remove(&k);
        }
    }
    (removed, kept)
}

pub fn install(path: &FsPath, port: u16) -> Result<Report> {
    let mut v = read_settings(path)?;
    let obj = v.as_object_mut().unwrap();
    let hooks = obj.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    if !hooks.is_object() {
        *hooks = Value::Object(Map::new());
    }
    let hooks = hooks.as_object_mut().unwrap();
    let (removed, foreign_kept) = strip_ours(hooks);
    let mut installed = Vec::new();
    for ev in EVENTS {
        let group = json!({
            "hooks": [ { "type": "command", "command": hook_command(ev, port), "timeout": 5 } ]
        });
        let entry = hooks.entry(ev).or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
        entry.as_array_mut().unwrap().push(group);
        installed.push(ev.to_string());
    }
    let backup = write_settings(path, &v)?;
    Ok(Report { path: path.to_path_buf(), backup, installed, removed, foreign_kept })
}

pub fn uninstall(path: &FsPath) -> Result<Report> {
    let mut v = read_settings(path)?;
    let obj = v.as_object_mut().unwrap();
    let mut removed = 0;
    let mut foreign_kept = 0;
    if let Some(Value::Object(hooks)) = obj.get_mut("hooks") {
        let r = strip_ours(hooks);
        removed = r.0;
        foreign_kept = r.1;
        if hooks.is_empty() {
            obj.remove("hooks");
        }
    }
    let backup = if removed > 0 { write_settings(path, &v)? } else { None };
    Ok(Report { path: path.to_path_buf(), backup, installed: Vec::new(), removed, foreign_kept })
}

#[derive(Debug, serde::Serialize)]
pub struct Status {
    pub path: PathBuf,
    pub exists: bool,
    pub installed_events: Vec<String>,
    pub missing_events: Vec<String>,
    pub port: Option<u16>,
}

pub fn status(path: &FsPath) -> Result<Status> {
    let v = read_settings(path)?;
    let mut installed = Vec::new();
    let mut port = None;
    if let Some(Value::Object(hooks)) = v.get("hooks") {
        for ev in EVENTS {
            let Some(Value::Array(groups)) = hooks.get(ev) else { continue };
            let ours = groups.iter().any(|g| {
                g.get("hooks")
                    .and_then(|h| h.as_array())
                    .map(|a| {
                        a.iter().any(|h| {
                            let c = h.get("command").and_then(|c| c.as_str()).unwrap_or("");
                            if is_ours(c) {
                                if port.is_none() {
                                    port = c
                                        .split("127.0.0.1:")
                                        .nth(1)
                                        .and_then(|r| r.split('/').next())
                                        .and_then(|p| p.parse().ok());
                                }
                                true
                            } else {
                                false
                            }
                        })
                    })
                    .unwrap_or(false)
            });
            if ours {
                installed.push(ev.to_string());
            }
        }
    }
    let missing = EVENTS.iter().filter(|e| !installed.iter().any(|i| i == *e)).map(|e| e.to_string()).collect();
    Ok(Status { path: path.to_path_buf(), exists: path.exists(), installed_events: installed, missing_events: missing, port })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("clowdeck-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn install_is_idempotent_and_preserves_foreign() {
        let p = tmp("settings.json");
        std::fs::write(
            &p,
            r#"{ "model": "x", "hooks": { "Stop": [ { "matcher": "", "hooks": [ { "type": "command", "command": "echo foreign" } ] } ] }, "permissions": {"allow": ["Bash(ls)"]} }"#,
        )
        .unwrap();
        let r1 = install(&p, 47831).unwrap();
        assert_eq!(r1.installed.len(), EVENTS.len());
        assert!(r1.backup.is_some());
        let r2 = install(&p, 47831).unwrap();
        assert_eq!(r2.removed, EVENTS.len(), "segunda instalacao substitui a primeira");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(v["model"], "x");
        assert_eq!(v["permissions"]["allow"][0], "Bash(ls)");
        let stop = v["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2, "alheio + nosso");
        assert_eq!(stop[0]["hooks"][0]["command"], "echo foreign");
        assert!(is_ours(stop[1]["hooks"][0]["command"].as_str().unwrap()));
        let cmd = stop[1]["hooks"][0]["command"].as_str().unwrap();
        // o comando difere por SO: POSIX casa por $PPID, Windows por cwd (sem PPID no shell)
        if cfg!(windows) {
            assert!(cmd.contains("/hook/Stop?") && cmd.contains(MARKER), "cmd: {cmd}");
        } else {
            assert!(cmd.contains("/hook/Stop?pid=$PPID"), "cmd: {cmd}");
        }
        let st = status(&p).unwrap();
        assert!(st.missing_events.is_empty());
        assert_eq!(st.port, Some(47831));
        let r3 = uninstall(&p).unwrap();
        assert_eq!(r3.removed, EVENTS.len());
        assert_eq!(r3.foreign_kept, 1);
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(v["hooks"]["Stop"].as_array().unwrap().len(), 1);
        assert!(v["hooks"].get("PreToolUse").is_none());
        let st = status(&p).unwrap();
        assert!(st.installed_events.is_empty());
    }

    #[test]
    fn install_into_missing_file() {
        let p = tmp("new-settings.json");
        let _ = std::fs::remove_file(&p);
        let r = install(&p, 5000).unwrap();
        assert!(r.backup.is_none());
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert!(v["hooks"]["SessionStart"][0]["hooks"][0]["command"].as_str().unwrap().contains(":5000/hook/SessionStart"));
    }

    #[test]
    fn parse_hook_fields() {
        let (ev, _) = parse_hook(
            "Stop",
            Some(123),
            r#"{"session_id":"abc","cwd":"/tmp/x","permission_mode":"plan","hook_event_name":"Stop","stop_hook_active":false}"#,
        );
        assert_eq!(ev.event, "Stop");
        assert_eq!(ev.pid, Some(123));
        assert_eq!(ev.session_id.as_deref(), Some("abc"));
        assert_eq!(ev.permission_mode.as_deref(), Some("plan"));
        let (ev, _) = parse_hook("Stop", Some(0), "not json");
        assert_eq!(ev.pid, None);
        assert_eq!(ev.event, "Stop");
    }
}
