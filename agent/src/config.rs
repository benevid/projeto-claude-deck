//! Config do agente: `~/Library/Application Support/clowdeck/config.toml`
//! (Windows: `%APPDATA%\clowdeck\config.toml`). Criada com defaults no 1o `run`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_PORT: u16 = 47831;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Identidade do `codesign` usada pelo `service install` (nome ou hash de
    /// `security find-identity -v -p codesigning`). Vazio = auto: "Developer ID Application"
    /// senao "Apple Development". Sem assinatura estavel o macOS descarta Bluetooth/
    /// Acessibilidade a cada recompilacao (assinatura ad-hoc = cdhash novo).
    #[serde(default)]
    pub sign_identity: Option<String>,
    pub port: u16,
    pub dry_run: bool,
    pub ble: BleCfg,
    pub deck: DeckCfg,
    pub focus: FocusCfg,
    pub codex: CodexCfg,
    pub commands: Vec<CommandCfg>,
}

/// Integracao com o Codex CLI (M6): o agente embute `codex app-server` (stdio
/// JSON-RPC) para enriquecer o estado das sessoes Codex descobertas por processo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CodexCfg {
    pub enabled: bool,
    /// intervalo do poll de `thread/list` (segundos)
    pub poll_s: u64,
}
impl Default for CodexCfg {
    fn default() -> Self {
        CodexCfg { enabled: true, poll_s: 5 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BleCfg {
    pub enabled: bool,
    /// bytes por pacote (inclui o byte de framing). 182 = MTU 185 - 3.
    pub max_frame: usize,
    /// forca write-with-response em tudo (se o link negociar MTU pequeno)
    pub write_with_response: bool,
    /// heartbeat de SESSIONS em segundos
    pub heartbeat_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeckCfg {
    pub brightness: u8,
    /// 0 = pt, 1 = en
    pub lang: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FocusCfg {
    /// VS Code/Cursor/Windsurf: depois de levantar a janela, foca o painel de terminal
    pub vscode_focus_terminal: bool,
    /// Atalho NAO-toggle p/ `workbench.action.terminal.focus`, instalado no keybindings.json
    /// do editor por `clowdeck-agent keybinding install`. (Ctrl+` e *toggle*: escondia o
    /// painel quando ja estava focado.) Sem o binding instalado, nenhuma tecla e enviada.
    #[serde(default = "default_terminal_keys")]
    pub vscode_terminal_keys: String,
}

pub fn default_terminal_keys() -> String {
    "ctrl+alt+cmd+t".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandCfg {
    pub label: String,
    pub text: String,
    #[serde(default)]
    pub confirm: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sign_identity: None,
            port: DEFAULT_PORT,
            dry_run: false,
            ble: BleCfg::default(),
            deck: DeckCfg::default(),
            focus: FocusCfg::default(),
            codex: CodexCfg::default(),
            commands: vec![
                CommandCfg { label: "voice".into(), text: "/voice".into(), confirm: false },
                CommandCfg { label: "continue".into(), text: "continue".into(), confirm: false },
                CommandCfg { label: "testes".into(), text: "rode os testes e corrija o que falhar".into(), confirm: false },
                CommandCfg { label: "commit".into(), text: "faca commit do que esta pronto com uma mensagem curta".into(), confirm: true },
            ],
        }
    }
}
impl Default for BleCfg {
    fn default() -> Self {
        BleCfg { enabled: true, max_frame: 182, write_with_response: true, heartbeat_s: 3 }
    }
}
impl Default for DeckCfg {
    fn default() -> Self {
        DeckCfg { brightness: 200, lang: 0 }
    }
}
impl Default for FocusCfg {
    fn default() -> Self {
        FocusCfg { vscode_focus_terminal: true, vscode_terminal_keys: default_terminal_keys() }
    }
}

pub fn config_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.config_dir().join("clowdeck"))
        .unwrap_or_else(|| PathBuf::from(".clowdeck"))
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load_or_create() -> Result<(Config, PathBuf)> {
    let path = config_path();
    if path.exists() {
        let txt = std::fs::read_to_string(&path).with_context(|| format!("lendo {}", path.display()))?;
        let mut cfg: Config = toml::from_str(&txt).with_context(|| format!("parse de {}", path.display()))?;
        cfg.commands.truncate(crate::protocol::MAX_CUSTOM_COMMANDS);
        Ok((cfg, path))
    } else {
        let cfg = Config::default();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("criando {}", dir.display()))?;
        }
        let txt = toml::to_string_pretty(&cfg)?;
        std::fs::write(&path, format!("# Clow Deck — config do agente (gerada com defaults)\n{txt}"))?;
        Ok((cfg, path))
    }
}

impl Config {
    pub fn deck_config(&self) -> crate::protocol::DeckConfig {
        crate::protocol::DeckConfig {
            brightness: Some(self.deck.brightness),
            lang: Some(self.deck.lang),
            commands: self.commands.iter().map(|c| (c.label.clone(), c.confirm)).collect(),
        }
    }
}
