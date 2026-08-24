//! `keybinding install|uninstall|status`: binding nao-toggle no keybindings.json do
//! editor (VS Code/Cursor/Windsurf) para `workbench.action.terminal.focus`. O agente so
//! envia o atalho depois do foco se este binding existir; Ctrl+` (padrao) e
//! "View: Toggle Terminal" e ESCONDIA o painel quando ja estava focado.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

pub const COMMAND: &str = "workbench.action.terminal.focus";
const HEADER: &str = "// Place your key bindings in this file to override the defaults\n";

/// pasta em ~/Library/Application Support para o nome curto do editor
pub fn support_dir(editor: &str) -> &'static str {
    match editor.to_ascii_lowercase().as_str() {
        "cursor" => "Cursor",
        "windsurf" => "Windsurf",
        _ => "Code",
    }
}

pub fn path(support_dir: &str) -> Result<PathBuf> {
    let home = directories::BaseDirs::new().context("sem HOME")?.home_dir().to_path_buf();
    #[cfg(target_os = "macos")]
    let base = home.join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(std::env::var("APPDATA").unwrap_or_default());
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = home.join(".config");
    Ok(base.join(support_dir).join("User/keybindings.json"))
}

/// Le o keybindings.json (JSONC: tolera linhas de comentario `//` e virgula final).
fn load(p: &PathBuf) -> Result<Vec<Value>> {
    if !p.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(p).with_context(|| format!("lendo {}", p.display()))?;
    let stripped: String = raw
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let stripped = stripped.replace(",\n]", "\n]").replace(",]", "]");
    let v: Value = serde_json::from_str(stripped.trim()).map_err(|e| {
        anyhow!("{} nao e JSON que eu saiba ler ({e}); adicione a mao: {{ \"key\": \"<atalho>\", \"command\": \"{COMMAND}\" }}", p.display())
    })?;
    v.as_array().cloned().ok_or_else(|| anyhow!("{} nao contem uma lista", p.display()))
}

fn save(p: &PathBuf, entries: &[Value]) -> Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let body = serde_json::to_string_pretty(&Value::Array(entries.to_vec()))?;
    std::fs::write(p, format!("{HEADER}{body}\n")).with_context(|| format!("escrevendo {}", p.display()))
}

fn is_ours(e: &Value, keys: &str) -> bool {
    e.get("command").and_then(|c| c.as_str()) == Some(COMMAND) && e.get("key").and_then(|k| k.as_str()) == Some(keys)
}

/// true se o binding (tecla + comando) esta no arquivo — e o gate do envio da tecla
pub fn installed(support_dir: &str, keys: &str) -> bool {
    path(support_dir).ok().and_then(|p| load(&p).ok()).map(|v| v.iter().any(|e| is_ours(e, keys))).unwrap_or(false)
}

pub fn install(support_dir: &str, keys: &str) -> Result<()> {
    let p = path(support_dir)?;
    let mut entries = load(&p)?;
    entries.retain(|e| e.get("command").and_then(|c| c.as_str()) != Some(COMMAND));
    entries.push(json!({ "key": keys, "command": COMMAND }));
    save(&p, &entries)?;
    println!("binding `{keys}` -> {COMMAND} instalado em {}", p.display());
    println!("(o editor recarrega o arquivo sozinho; nao precisa reiniciar)");
    Ok(())
}

pub fn uninstall(support_dir: &str, keys: &str) -> Result<()> {
    let p = path(support_dir)?;
    let mut entries = load(&p)?;
    let before = entries.len();
    entries.retain(|e| !is_ours(e, keys));
    if entries.len() == before {
        println!("nada a remover em {}", p.display());
        return Ok(());
    }
    save(&p, &entries)?;
    println!("binding removido de {}", p.display());
    Ok(())
}

pub fn status(support_dir: &str, keys: &str) -> Result<()> {
    let p = path(support_dir)?;
    println!("{}: {}", p.display(), if installed(support_dir, keys) { format!("binding `{keys}` -> {COMMAND} presente") } else { "binding ausente (rode `keybinding install`)".into() });
    Ok(())
}
