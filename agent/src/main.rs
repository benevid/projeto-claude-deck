//! clowdeck-agent — CLI do agente do Clow Deck. O runtime vive em `lib.rs`
//! (compartilhado com o app de bandeja `app/`).

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use clowdeck_agent::{accessibility_trusted, ble, config, discovery, focus, hooks, keybinding, model, protocol, service};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "clowdeck-agent", version, about = "Agente do Clow Deck (sessoes do Claude Code → deck BLE)")]
struct Cli {
    /// Nunca dispara teclas reais (so loga). O foco de janela continua valendo.
    #[arg(long, global = true)]
    dry_run: bool,
    /// Porta do servidor local (hooks + deck virtual)
    #[arg(long, global = true)]
    port: Option<u16>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Roda o agente (descoberta + hooks + deck virtual + BLE)
    Run {
        /// Nao usa Bluetooth (so o deck virtual)
        #[arg(long)]
        no_ble: bool,
    },
    /// Instala/remove/consulta os hooks em ~/.claude/settings.json
    Hooks {
        #[command(subcommand)]
        cmd: HooksCmd,
    },
    /// Lista as sessoes do Claude Code vistas agora
    Sessions,
    /// Traz a janela da sessao (pid) pra frente
    Focus { pid: u32 },
    /// Utilitarios BLE
    Ble {
        #[command(subcommand)]
        cmd: BleCmd,
    },
    /// Checa permissoes e dependencias
    Doctor,
    /// Binding nao-toggle `workbench.action.terminal.focus` no keybindings.json do editor
    Keybinding {
        #[command(subcommand)]
        cmd: KbCmd,
    },
    /// Instala/remove/consulta o agente como servico de login (launchd, macOS)
    Service {
        #[command(subcommand)]
        cmd: ServiceCmd,
    },
}

#[derive(Subcommand)]
enum KbCmd {
    Install {
        /// code | cursor | windsurf
        #[arg(long, default_value = "code")]
        editor: String,
    },
    Uninstall {
        #[arg(long, default_value = "code")]
        editor: String,
    },
    Status {
        #[arg(long, default_value = "code")]
        editor: String,
    },
}

#[derive(Subcommand)]
enum ServiceCmd {
    /// Cria ~/Library/LaunchAgents/my.autom.clowdeck.plist (KeepAlive) e sobe o servico
    Install,
    /// Para o servico e remove o plist
    Uninstall,
    /// Mostra se o servico esta carregado/rodando
    Status,
}

#[derive(Subcommand)]
enum HooksCmd {
    Install {
        #[arg(long)]
        settings: Option<PathBuf>,
    },
    Uninstall {
        #[arg(long)]
        settings: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        settings: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum BleCmd {
    /// Escaneia por alguns segundos e lista o que anuncia
    Scan {
        #[arg(long, default_value_t = 5)]
        secs: u64,
    },
    /// Conecta no deck, le INFO e desconecta
    Info,
}

#[tokio::main]
async fn main() -> Result<()> {
    clowdeck_agent::init_tracing();
    let cli = Cli::parse();
    let (mut cfg, cfg_path) = config::load_or_create()?;
    if let Some(p) = cli.port {
        cfg.port = p;
    }
    let dry_run = cli.dry_run || cfg.dry_run;

    match cli.cmd {
        Cmd::Run { no_ble } => {
            let (st, listener) = clowdeck_agent::bind(cfg, cfg_path, dry_run, !no_ble).await?;
            clowdeck_agent::serve(st, listener, None).await
        }
        Cmd::Hooks { cmd } => hooks_cmd(cmd, cfg.port),
        Cmd::Sessions => sessions().await,
        Cmd::Focus { pid } => focus_pid(pid, &cfg, dry_run).await,
        Cmd::Ble { cmd } => match cmd {
            BleCmd::Scan { secs } => ble::scan(secs).await,
            BleCmd::Info => ble::info().await,
        },
        Cmd::Doctor => doctor(&cfg).await,
        Cmd::Keybinding { cmd } => {
            let keys = cfg.focus.vscode_terminal_keys.clone();
            match cmd {
                KbCmd::Install { editor } => keybinding::install(keybinding::support_dir(&editor), &keys),
                KbCmd::Uninstall { editor } => keybinding::uninstall(keybinding::support_dir(&editor), &keys),
                KbCmd::Status { editor } => keybinding::status(keybinding::support_dir(&editor), &keys),
            }
        }
        Cmd::Service { cmd } => match cmd {
            ServiceCmd::Install => service::install(cfg.port, cfg.sign_identity.as_deref()),
            ServiceCmd::Uninstall => service::uninstall(),
            ServiceCmd::Status => service::status(),
        },
    }
}

fn hooks_cmd(cmd: HooksCmd, port: u16) -> Result<()> {
    let path = |p: Option<PathBuf>| p.unwrap_or_else(hooks::default_settings_path);
    match cmd {
        HooksCmd::Install { settings } => {
            let r = hooks::install(&path(settings), port)?;
            println!("hooks instalados em {} ({} eventos, porta {port})", r.path.display(), r.installed.len());
            if let Some(b) = r.backup {
                println!("backup: {}", b.display());
            }
            if r.removed > 0 {
                println!("substituidos {} hooks antigos do Clow Deck", r.removed);
            }
            println!("hooks de terceiros preservados: {}", r.foreign_kept);
        }
        HooksCmd::Uninstall { settings } => {
            let r = hooks::uninstall(&path(settings))?;
            println!("removidos {} hooks do Clow Deck de {}", r.removed, r.path.display());
            if let Some(b) = r.backup {
                println!("backup: {}", b.display());
            }
        }
        HooksCmd::Status { settings } => {
            let s = hooks::status(&path(settings))?;
            println!("{}", serde_json::to_string_pretty(&s)?);
        }
    }
    Ok(())
}

async fn sessions() -> Result<()> {
    let procs = discovery::discover().await?;
    let mut t = model::SessionTable::new();
    t.apply_discovery(&procs);
    println!("{:<4} {:<7} {:<9} {:<10} {:<12} {}", "cel", "pid", "tty", "terminal", "rotulo", "cwd");
    for i in 0..protocol::SESSION_CELLS {
        if let Some(s) = t.cell(i) {
            println!(
                "{:<4} {:<7} {:<9} {:<10} {:<12} {}",
                i,
                s.pid.map(|p| p.to_string()).unwrap_or_default(),
                s.tty.clone().unwrap_or_else(|| "-".into()),
                s.terminal_app.map(|a| a.name()).unwrap_or("?"),
                s.label,
                s.cwd
            );
        }
    }
    if t.overflow_len() > 0 {
        println!("+{} sessao(oes) esperando celula", t.overflow_len());
    }
    println!("{} sessao(oes)", procs.len());
    Ok(())
}

async fn focus_pid(pid: u32, cfg: &config::Config, dry_run: bool) -> Result<()> {
    let procs = discovery::discover().await?;
    let mut t = model::SessionTable::new();
    t.apply_discovery(&procs);
    let cell = t.cell_of_pid(pid).with_context(|| format!("pid {pid} nao e uma sessao do Claude Code"))?;
    let s = t.cell(cell).unwrap().clone();
    let r = focus::focus_session_async(s, cfg.focus.clone(), dry_run).await?;
    println!("{r}");
    Ok(())
}

async fn doctor(cfg: &config::Config) -> Result<()> {
    let has = |bin: &str| {
        std::process::Command::new("which")
            .arg(bin)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };
    let mut rows: Vec<(String, bool, String)> = Vec::new();
    rows.push(("curl".into(), has("curl"), "usado pelos hooks".into()));
    if cfg!(target_os = "macos") {
        rows.push(("lsof".into(), has("lsof"), "cwd das sessoes".into()));
        rows.push(("osascript".into(), has("osascript"), "foco de janela (AppleScript)".into()));
        match accessibility_trusted() {
            Some(ok) => rows.push(("Acessibilidade".into(), ok, "Ajustes > Privacidade > Acessibilidade (teclas sinteticas)".into())),
            None => {}
        }
        let bt = std::process::Command::new("system_profiler")
            .args(["SPBluetoothDataType", "-detailLevel", "mini"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("State: On"))
            .unwrap_or(false);
        rows.push(("Bluetooth".into(), bt, format!("ligado (system_profiler); {}", ble::PERMISSION_HINT)));
    }
    let hs = hooks::status(&hooks::default_settings_path());
    match &hs {
        Ok(s) => rows.push((
            "hooks".into(),
            s.missing_events.is_empty(),
            format!("{} instalados, faltam {:?} ({})", s.installed_events.len(), s.missing_events, s.path.display()),
        )),
        Err(e) => rows.push(("hooks".into(), false, format!("{e:#}"))),
    }
    let port_free = std::net::TcpListener::bind(("127.0.0.1", cfg.port)).is_ok();
    rows.push((format!("porta {}", cfg.port), port_free, if port_free { "livre".into() } else { "em uso (agente ja rodando?)".into() }));
    match discovery::discover().await {
        Ok(p) => rows.push(("descoberta".into(), true, format!("{} sessao(oes) do Claude Code agora", p.len()))),
        Err(e) => rows.push(("descoberta".into(), false, format!("{e:#}"))),
    }
    rows.push(("config".into(), true, config::config_path().display().to_string()));
    for (name, ok, note) in rows {
        println!("{} {:<16} {}", if ok { "✔" } else { "✘" }, name, note);
    }
    Ok(())
}
