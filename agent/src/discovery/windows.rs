//! Windows (M4): descoberta de sessoes via `sysinfo` — nome do executavel + cwd +
//! cadeia de pais para achar o app de terminal. Espelha a heuristica do macOS.

use crate::model::{DiscoveredProcess, Engine, TerminalApp};
use anyhow::Result;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Classifica um processo pelo nome do executavel + linha de comando.
fn engine_of(name: &str, cmd: &str) -> Option<Engine> {
    let base = name.trim_end_matches(".exe").to_ascii_lowercase();
    let lc = cmd.to_ascii_lowercase();
    if lc.contains("clowdeck") || lc.contains("codeium") {
        return None;
    }
    if base == "claude" || lc.contains("claude-code\\cli.js") || lc.contains("@anthropic-ai\\claude-code") {
        return Some(Engine::Claude);
    }
    if base == "codex" {
        // helpers de servico do pacote nao sao sessoes
        if ["app-server", "mcp-server", "exec-server", "remote-control", " proxy"]
            .iter()
            .any(|s| lc.contains(s))
        {
            return None;
        }
        return Some(Engine::Codex);
    }
    if base == "opencode" {
        const NOT_TUI: &[&str] = &["serve", "acp", "run", "db", "export", "import", "session",
            "plugin", "plug", "mcp", "providers", "auth", "agent", "upgrade", "uninstall",
            "github", "pr ", "web", "models", "stats", "completion", "debug", "attach"];
        // o subcomando e o 2o token da linha de comando
        let second = cmd.split_whitespace().nth(1).unwrap_or("").to_ascii_lowercase();
        if NOT_TUI.iter().any(|s| second == s.trim()) {
            return None;
        }
        return Some(Engine::Opencode);
    }
    None
}

pub fn discover_sync() -> Result<Vec<DiscoveredProcess>> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    let mut out = Vec::new();
    for (pid, proc_) in sys.processes() {
        let name = proc_.name().to_string_lossy().to_string();
        let cmd: String = proc_
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let Some(engine) = engine_of(&name, &cmd) else { continue };

        // cadeia de pais: primeiro ancestral que for um app de terminal conhecido
        let mut terminal_app = None;
        let mut terminal_pid = None;
        let mut cur = proc_.parent();
        let mut hops = 0;
        while let Some(ppid) = cur {
            if hops > 16 {
                break;
            }
            let Some(parent) = sys.process(ppid) else { break };
            let pname = parent.name().to_string_lossy().to_string();
            if terminal_app.is_none() {
                terminal_app = TerminalApp::from_comm(&pname);
                if terminal_app.is_some() {
                    terminal_pid = Some(ppid.as_u32());
                }
            }
            cur = parent.parent();
            hops += 1;
        }
        if terminal_app.is_none() {
            terminal_app = Some(TerminalApp::Other);
            terminal_pid = proc_.parent().map(|p| p.as_u32());
        }

        out.push(DiscoveredProcess {
            engine,
            pid: pid.as_u32(),
            ppid: proc_.parent().map(|p| p.as_u32()).unwrap_or(0),
            tty: None, // Windows nao tem tty; o casamento e por pid/cwd
            cwd: proc_.cwd().map(|p| p.to_string_lossy().to_string()),
            args: cmd,
            terminal_app,
            terminal_pid,
        });
    }
    out.sort_by_key(|p| p.pid);
    Ok(out)
}
