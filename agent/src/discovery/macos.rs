//! macOS: `ps` p/ a arvore de processos + `lsof` (uma chamada) p/ o cwd.

use crate::model::{DiscoveredProcess, Engine, TerminalApp};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone)]
struct Proc {
    pid: u32,
    ppid: u32,
    tty: Option<String>,
    comm: String,
    args: String,
}

fn run(cmd: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(cmd).args(args).output().with_context(|| format!("executando {cmd}"))?;
    // lsof devolve 1 quando algum pid nao tem cwd legivel — ainda assim ha saida util
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn ps_table() -> Result<HashMap<u32, Proc>> {
    let mut map = HashMap::new();
    // comm pode ter espacos ("Code Helper"): pid/ppid/tty sao os 3 primeiros tokens, o resto e comm
    for line in run("ps", &["-axo", "pid=,ppid=,tty=,comm="])?.lines() {
        let mut it = line.split_whitespace();
        let (Some(pid), Some(ppid), Some(tty)) = (it.next(), it.next(), it.next()) else { continue };
        let comm: String = it.collect::<Vec<_>>().join(" ");
        let (Ok(pid), Ok(ppid)) = (pid.parse::<u32>(), ppid.parse::<u32>()) else { continue };
        let tty = if tty == "??" || tty == "-" { None } else { Some(tty.to_string()) };
        map.insert(pid, Proc { pid, ppid, tty, comm, args: String::new() });
    }
    for line in run("ps", &["-axo", "pid=,args="])?.lines() {
        let line = line.trim_start();
        let Some((pid, args)) = line.split_once(' ') else { continue };
        if let Ok(pid) = pid.parse::<u32>() {
            if let Some(p) = map.get_mut(&pid) {
                p.args = args.trim().to_string();
            }
        }
    }
    Ok(map)
}

fn engine_of(p: &Proc) -> Option<Engine> {
    let lc_args = p.args.to_ascii_lowercase();
    if lc_args.contains("chrome-native-host")
        || lc_args.contains("codeium")
        || lc_args.contains("grep")
        || lc_args.contains("clowdeck")
    {
        return None;
    }
    let comm_base = p.comm.rsplit('/').next().unwrap_or(&p.comm);
    let first = p.args.split_whitespace().next().unwrap_or("");
    let first_base = first.rsplit('/').next().unwrap_or(first);
    if comm_base == "claude" || first_base == "claude"
        || lc_args.contains("claude-code/cli.js") || lc_args.contains("@anthropic-ai/claude-code")
    {
        return Some(Engine::Claude);
    }
    // Codex CLI (TUI/exec); ignora os servicos auxiliares (inclusive o filho
    // `codex app-server` que o proprio agente embute — M6)
    // basename EXATO "codex": o pacote vendoriza helpers como codex-code-mode
    // (filho do TUI, com TTY) que nao sao sessoes
    if comm_base == "codex" || first_base == "codex" {
        if lc_args.contains("app-server") || lc_args.contains("mcp-server")
            || lc_args.contains(" proxy") || lc_args.contains("exec-server")
            || lc_args.contains("remote-control")
        {
            return None;
        }
        // sessao interativa de verdade tem TTY; filhos re-executados do app-server
        // (que podem perder o "app-server" no argv) e daemons nao tem
        if p.tty.is_none() {
            return None;
        }
        return Some(Engine::Codex);
    }
    // opencode (M7): TUI = binario `opencode` sem subcomando de servico; sessao real tem TTY
    if comm_base == "opencode" || first_base == "opencode" {
        let second = p.args.split_whitespace().nth(1).unwrap_or("");
        const NOT_TUI: &[&str] = &["serve", "acp", "run", "db", "export", "import", "session",
            "plugin", "plug", "mcp", "providers", "auth", "agent", "upgrade", "uninstall",
            "github", "pr", "web", "models", "stats", "completion", "debug", "attach"];
        if p.tty.is_none() || NOT_TUI.contains(&second) {
            return None;
        }
        return Some(Engine::Opencode);
    }
    None
}

fn cwds(pids: &[u32]) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    if pids.is_empty() {
        return out;
    }
    let list = pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
    let Ok(txt) = run("lsof", &["-a", "-p", &list, "-d", "cwd", "-Fpn"]) else { return out };
    let mut cur: Option<u32> = None;
    for line in txt.lines() {
        if let Some(p) = line.strip_prefix('p') {
            cur = p.trim().parse().ok();
        } else if let Some(n) = line.strip_prefix('n') {
            if let Some(pid) = cur {
                out.entry(pid).or_insert_with(|| n.trim().to_string());
            }
        }
    }
    out
}

fn terminal_of(map: &HashMap<u32, Proc>, pid: u32) -> (Option<TerminalApp>, Option<u32>) {
    let mut app = None;
    let mut root = None;
    let mut cur = map.get(&pid).map(|p| p.ppid);
    let mut hops = 0;
    while let Some(pp) = cur {
        if pp <= 1 || hops > 32 {
            break;
        }
        let Some(p) = map.get(&pp) else { break };
        if app.is_none() {
            app = TerminalApp::from_comm(&p.comm);
        }
        if p.ppid <= 1 {
            root = Some(p.pid);
        }
        cur = Some(p.ppid);
        hops += 1;
    }
    if app.is_none() && root.is_some() {
        app = Some(TerminalApp::Other);
    }
    (app, root)
}

pub fn discover_sync() -> Result<Vec<DiscoveredProcess>> {
    let map = ps_table()?;
    let mut found: Vec<(&Proc, Engine)> = map.values().filter_map(|p| engine_of(p).map(|e| (p, e))).collect();
    found.sort_by_key(|(p, _)| p.pid);
    let pids: Vec<u32> = found.iter().map(|(p, _)| p.pid).collect();
    let cwd = cwds(&pids);
    Ok(found
        .into_iter()
        .map(|(p, engine)| {
            let (terminal_app, terminal_pid) = terminal_of(&map, p.pid);
            DiscoveredProcess {
                engine,
                pid: p.pid,
                ppid: p.ppid,
                tty: p.tty.clone(),
                cwd: cwd.get(&p.pid).cloned(),
                args: p.args.clone(),
                terminal_app,
                terminal_pid,
            }
        })
        .collect())
}
