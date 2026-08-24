//! macOS: AppleScript via `osascript`.
//!
//! - Terminal.app e iTerm2 expoem o tty de cada aba → foco **exato** da aba.
//! - VS Code/Cursor/Windsurf: o Electron nao expoe janelas ao System Events, entao
//!   lemos os titulos pelo CGWindowList, achamos a pasta da janela (cwd ou ancestral)
//!   e chamamos `code <pasta>` — ja aberta, so traz a janela pra frente. Opcional:
//!   Ctrl+` p/ focar o painel de terminal. Nao da p/ escolher a ABA de terminal de
//!   fora: com varios terminais na mesma janela, a aba certa fica com o usuario.
//! - Outros: `frontmost` do processo-raiz do app de terminal.

use crate::config::FocusCfg;
use crate::model::{Session, TerminalApp};
use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub fn osascript(script: &str) -> Result<String> {
    let mut child = Command::new("osascript")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("osascript")?;
    child.stdin.take().unwrap().write_all(script.as_bytes())?;
    let out = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(anyhow!("osascript: {}", if err.is_empty() { stdout.clone() } else { err }));
    }
    Ok(stdout)
}

fn as_lit(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn basename(cwd: &str) -> String {
    let t = cwd.trim_end_matches('/');
    t.rsplit('/').next().unwrap_or(t).to_string()
}

/// Janela on-screen (CGWindowList): dono, pid e titulo. Nao depende de AX —
/// o Electron (VS Code/Cursor/Windsurf) nao expoe janelas ao System Events.
#[derive(Debug, Clone)]
pub struct WinInfo {
    pub owner: String,
    pub owner_pid: i32,
    pub name: String,
    pub layer: i32,
}

pub fn window_list() -> Vec<WinInfo> {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly};

    let mut out = Vec::new();
    let Some(arr) = copy_window_info(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID) else {
        return out;
    };
    for item in arr.iter() {
        let dict: CFDictionary<CFString, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(*item as core_foundation::dictionary::CFDictionaryRef) };
        let get_str = |k: &str| -> String {
            dict.find(CFString::new(k))
                .and_then(|v| v.downcast::<CFString>())
                .map(|s| s.to_string())
                .unwrap_or_default()
        };
        let get_i32 = |k: &str| -> i32 {
            dict.find(CFString::new(k))
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i64())
                .unwrap_or(-1) as i32
        };
        out.push(WinInfo {
            owner: get_str("kCGWindowOwnerName"),
            owner_pid: get_i32("kCGWindowOwnerPID"),
            name: get_str("kCGWindowName"),
            layer: get_i32("kCGWindowLayer"),
        });
    }
    out
}

/// Titulo do VS Code = "arquivo — pasta — ..." (separador " — "). Casa se algum segmento e a pasta.
fn title_matches(title: &str, base: &str) -> bool {
    if title == base {
        return true;
    }
    title.split(" — ").any(|seg| seg.trim() == base) || title.split(" - ").any(|seg| seg.trim() == base)
}

struct Editor {
    process: &'static str,
    app: &'static str,
    /// pasta em ~/Library/Application Support/ (storage.json com as janelas abertas)
    support_dir: &'static str,
    cli: &'static str,
    cli_fallback: &'static str,
}

const VSCODE: Editor = Editor { process: "Code", app: "Visual Studio Code", support_dir: "Code", cli: "code", cli_fallback: "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" };
const CURSOR: Editor = Editor { process: "Cursor", app: "Cursor", support_dir: "Cursor", cli: "cursor", cli_fallback: "/Applications/Cursor.app/Contents/Resources/app/bin/cursor" };
const WINDSURF: Editor = Editor { process: "Windsurf", app: "Windsurf", support_dir: "Windsurf", cli: "windsurf", cli_fallback: "/Applications/Windsurf.app/Contents/Resources/app/bin/windsurf" };

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() + 0 && i + 2 <= b.len() - 1 {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Pasta da janela do editor que contem `cwd`, lida do `storage.json` do editor
/// (`windowsState.openedWindows[].folder`). Nao depende de titulos de janela —
/// `CGWindowListCopyWindowInfo` so devolve nomes para processos com permissao de
/// Gravacao de Tela, e o agente como servico nao tem (visto: "janelas: []").
fn folder_from_state(ed: &Editor, cwd: &str) -> Option<String> {
    let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
    let p = home.join("Library/Application Support").join(ed.support_dir).join("User/globalStorage/storage.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    let ws = v.get("windowsState")?;
    let mut urls: Vec<String> = Vec::new();
    if let Some(arr) = ws.get("openedWindows").and_then(|a| a.as_array()) {
        urls.extend(arr.iter().filter_map(|w| w.get("folder").and_then(|f| f.as_str()).map(String::from)));
    }
    if let Some(f) = ws.get("lastActiveWindow").and_then(|w| w.get("folder")).and_then(|f| f.as_str()) {
        urls.push(f.to_string());
    }
    let cwd_p = std::path::Path::new(cwd);
    let mut best: Option<std::path::PathBuf> = None;
    for u in urls {
        let Some(rest) = u.strip_prefix("file://") else { continue };
        let path = std::path::PathBuf::from(percent_decode(rest));
        if cwd_p == path.as_path() || cwd_p.starts_with(&path) {
            if best.as_ref().map(|b| path.as_os_str().len() > b.as_os_str().len()).unwrap_or(true) {
                best = Some(path);
            }
        }
    }
    best.map(|b| b.to_string_lossy().into_owned())
}

/// Traz um app pra frente via LaunchServices (`open -a`): nao usa Apple Events, logo
/// nao depende de permissao de Automacao/Acessibilidade — funciona ate quando o
/// agente roda como servico. E o que de fato ROUBA o foco de outro app: o CLI do
/// editor (`code <pasta>`) so escolhe a janela dentro do editor; se o editor nao e
/// o app ativo, o macOS apenas faz o icone pular no Dock.
/// Nome do app na frente (LaunchServices: `lsappinfo front` + `info -only name`), sem TCC.
pub fn frontmost_app_name() -> Option<String> {
    let asn = Command::new("lsappinfo").arg("front").output().ok()?;
    let asn = String::from_utf8_lossy(&asn.stdout).trim().to_string();
    if asn.is_empty() {
        return None;
    }
    let info = Command::new("lsappinfo").args(["info", "-only", "name", &asn]).output().ok()?;
    let txt = String::from_utf8_lossy(&info.stdout);
    // formato: "name"="Code"
    let name = txt.split('=').nth(1)?.trim().trim_matches('"').to_string();
    if name.is_empty() { None } else { Some(name) }
}

fn open_app(app: &str) -> Result<()> {
    let out = Command::new("open").arg("-a").arg(app).output().context("open -a")?;
    if !out.status.success() {
        anyhow::bail!("open -a {app}: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

fn find_cli(ed: &Editor) -> Option<String> {
    let ok = Command::new("which").arg(ed.cli).output().map(|o| o.status.success()).unwrap_or(false);
    if ok {
        return Some(ed.cli.to_string());
    }
    if std::path::Path::new(ed.cli_fallback).exists() {
        return Some(ed.cli_fallback.to_string());
    }
    None
}

fn activate_process(process: &str) -> Result<()> {
    osascript(&format!(
        "tell application \"System Events\"\n  if not (exists process {p}) then error \"{process} nao esta rodando\"\n  set frontmost of process {p} to true\nend tell",
        p = as_lit(process)
    ))?;
    Ok(())
}

/// VS Code/Cursor/Windsurf: descobre a pasta da janela (cwd ou um ancestral cujo
/// nome aparece no titulo de alguma janela do editor) e pede ao CLI do editor
/// para "abrir" essa pasta — que, ja aberta, so traz a janela pra frente.
/// "ctrl+alt+cmd+t" -> AppleScript `keystroke "t" using {control down, option down, command down}`
fn chord_script(process: &str, keys: &str) -> Option<String> {
    let parts: Vec<&str> = keys.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let (key, mods) = parts.split_last()?;
    let mut using: Vec<&str> = Vec::new();
    for m in mods {
        using.push(match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "control down",
            "alt" | "option" => "option down",
            "cmd" | "meta" | "command" => "command down",
            "shift" => "shift down",
            _ => return None,
        });
    }
    let k = key.to_ascii_lowercase();
    if k.chars().count() != 1 {
        return None;
    }
    Some(format!(
        "tell application \"System Events\" to tell process {} to keystroke \"{}\" using {{{}}}",
        as_lit(process),
        k,
        using.join(", ")
    ))
}

fn focus_editor(ed: &Editor, cwd: &str, app_pid: Option<u32>, cfg: &FocusCfg, dry_run: bool) -> Result<String> {
    let focus_terminal = cfg.vscode_focus_terminal;
    // janelas do editor: pelo pid do app quando a descoberta o conhece, senao pelo nome
    let wins: Vec<WinInfo> = window_list()
        .into_iter()
        .filter(|w| match app_pid {
            Some(p) => w.owner_pid == p as i32,
            None => w.owner == ed.process,
        })
        .filter(|w| w.layer == 0 && !w.name.is_empty())
        .collect();
    // 1) janelas abertas segundo o proprio editor (storage.json); 2) titulos (se houver)
    let mut folder: Option<String> = folder_from_state(ed, cwd);
    let mut dir = std::path::Path::new(cwd);
    for _ in 0..6 {
        if folder.is_some() { break; }
        let base = dir.file_name().and_then(|b| b.to_str()).unwrap_or("");
        if !base.is_empty() && wins.iter().any(|w| title_matches(&w.name, base)) {
            folder = Some(dir.to_string_lossy().into_owned());
            break;
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => break,
        }
    }
    let mut desc;
    match (&folder, find_cli(ed)) {
        (Some(f), Some(cli)) => {
            let out = Command::new(&cli).arg(f).output().with_context(|| format!("executando {cli}"))?;
            if !out.status.success() {
                anyhow::bail!("{cli} {f}: {}", String::from_utf8_lossy(&out.stderr).trim());
            }
            desc = format!("{}: `{} {}`", ed.process, cli, f);
            // o CLI escolheu a janela; agora ativa o app (rouba o foco de quem estiver na frente)
            std::thread::sleep(std::time::Duration::from_millis(120));
            match open_app(ed.app) {
                Ok(()) => desc.push_str(" + open -a"),
                Err(e) => desc.push_str(&format!(" (open -a falhou: {e:#})")),
            }
        }
        (Some(f), None) => {
            open_app(ed.app).or_else(|_| activate_process(ed.process))?;
            desc = format!("{}: ativado (sem CLI `{}` p/ escolher a janela de {})", ed.process, ed.cli, f);
        }
        (None, _) => {
            open_app(ed.app).or_else(|_| activate_process(ed.process))?;
            desc = format!(
                "{}: ativado, mas nenhuma janela com titulo de {} (janelas: {:?})",
                ed.process,
                basename(cwd),
                wins.iter().map(|w| w.name.clone()).collect::<Vec<_>>()
            );
        }
    }
    if focus_terminal && !dry_run {
        // Foco do painel de terminal SO com o binding nao-toggle instalado
        // (`clowdeck-agent keybinding install` -> workbench.action.terminal.focus).
        // Ctrl+` e "View: Toggle Terminal": escondia o painel quando ja estava focado.
        let keys = cfg.vscode_terminal_keys.as_str();
        if crate::keybinding::installed(ed.support_dir, keys) {
            std::thread::sleep(std::time::Duration::from_millis(250));
            match chord_script(ed.process, keys).ok_or_else(|| anyhow!("atalho invalido: {keys}")).and_then(|s| osascript(&s)) {
                Ok(_) => desc.push_str(&format!(" + {keys} (terminal.focus)")),
                Err(e) => {
                    static ONCE: std::sync::Once = std::sync::Once::new();
                    ONCE.call_once(|| tracing::warn!("atalho do terminal falhou ({e:#}) — Acessibilidade p/ `clowdeck-agent`?"));
                    desc.push_str(" (atalho do terminal falhou)");
                }
            }
        } else {
            static ONCE2: std::sync::Once = std::sync::Once::new();
            ONCE2.call_once(|| tracing::warn!("painel de terminal nao focado: rode `clowdeck-agent keybinding install` (binding nao-toggle no VS Code)"));
            desc.push_str(" (sem binding terminal.focus)");
        }
    } else if focus_terminal {
        desc.push_str(" (dry-run: sem foco do terminal)");
    }
    Ok(desc)
}

fn focus_terminal_app(tty: &str) -> Result<String> {
    let script = format!(
        r#"tell application "Terminal"
  repeat with w in windows
    repeat with t in tabs of w
      if (tty of t) is {t} then
        set selected tab of w to t
        set index of w to 1
        activate
        return "ok"
      end if
    end repeat
  end repeat
end tell
return "nomatch""#,
        t = as_lit(tty)
    );
    let r = osascript(&script)?;
    if r != "ok" {
        anyhow::bail!("Terminal.app: nenhuma aba com tty {tty}");
    }
    Ok(format!("Terminal.app: aba {tty}"))
}

fn focus_iterm(tty: &str) -> Result<String> {
    let script = format!(
        r#"tell application "iTerm2"
  repeat with w in windows
    repeat with t in tabs of w
      repeat with s in sessions of t
        if (tty of s) is {t} then
          select w
          select t
          select s
          activate
          return "ok"
        end if
      end repeat
    end repeat
  end repeat
end tell
return "nomatch""#,
        t = as_lit(tty)
    );
    let r = osascript(&script)?;
    if r != "ok" {
        anyhow::bail!("iTerm2: nenhuma sessao com tty {tty}");
    }
    Ok(format!("iTerm2: aba {tty}"))
}

fn focus_pid(pid: u32) -> Result<String> {
    osascript(&format!(
        "tell application \"System Events\" to set frontmost of (first process whose unix id is {pid}) to true"
    ))?;
    Ok(format!("frontmost pid {pid}"))
}

pub fn focus_session(s: &Session, cfg: &FocusCfg, dry_run: bool) -> Result<String> {
    let tty_dev = s.tty.as_ref().map(|t| if t.starts_with("/dev/") { t.clone() } else { format!("/dev/{t}") });
    match s.terminal_app {
        Some(TerminalApp::VSCode) => focus_editor(&VSCODE, &s.cwd, s.terminal_pid, cfg, dry_run),
        Some(TerminalApp::Cursor) => focus_editor(&CURSOR, &s.cwd, s.terminal_pid, cfg, dry_run),
        Some(TerminalApp::Windsurf) => focus_editor(&WINDSURF, &s.cwd, s.terminal_pid, cfg, dry_run),
        Some(TerminalApp::Terminal) => match &tty_dev {
            Some(t) => focus_terminal_app(t),
            None => s.terminal_pid.map(focus_pid).unwrap_or_else(|| Err(anyhow!("sem tty nem pid do Terminal"))),
        },
        Some(TerminalApp::ITerm2) => match &tty_dev {
            Some(t) => focus_iterm(t),
            None => s.terminal_pid.map(focus_pid).unwrap_or_else(|| Err(anyhow!("sem tty nem pid do iTerm2"))),
        },
        _ => match s.terminal_pid {
            Some(p) => focus_pid(p),
            None => Err(anyhow!("sessao sem app de terminal conhecido (pid {:?})", s.pid)),
        },
    }
}
