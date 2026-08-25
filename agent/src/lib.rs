//! clowdeck-agent como biblioteca: o CLI (`main.rs`) e o app de bandeja (`app/`, M3)
//! compartilham o mesmo runtime — descoberta, hooks, modelo, BLE e deck virtual.

pub mod app;
pub mod ble;
pub mod codex;
pub mod config;
pub mod discovery;
pub mod dispatch;
pub mod focus;
pub mod hooks;
pub mod inject;
pub mod keybinding;
pub mod model;
pub mod opencode;
pub mod protocol;
pub mod service;
pub mod web;

use anyhow::{Context, Result};
use app::{AppState, Shared};
use axum::routing::{get, post};
use axum::Router;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// O que fazer quando o watchdog detecta o runtime congelado (alem de encerrar):
/// o CLI usa `None` (exit 70 → launchd KeepAlive reinicia); o app chama `restart()`.
pub type FreezeAction = Box<dyn Fn() + Send + Sync + 'static>;

pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,btleplug=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();
}

/// Log em arquivo (o `.app` nao tem stderr util).
pub fn init_tracing_file(path: &Path) -> Result<()> {
    use tracing_subscriber::EnvFilter;
    let f = std::fs::OpenOptions::new().create(true).append(true).open(path)
        .with_context(|| format!("abrindo log {}", path.display()))?;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,btleplug=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).with_ansi(false)
        .with_writer(move || f.try_clone().expect("clone do arquivo de log")).compact().init();
    Ok(())
}

pub fn router(st: Shared) -> Router {
    Router::new()
        .route("/", get(web::index))
        .route("/health", get(web::health))
        .route("/state", get(web::get_state))
        .route("/event", post(web::post_event))
        .route("/hook/:event", post(hooks::hook_handler))
        .with_state(st)
}

/// Cria o estado e abre a porta local (falha cedo se outro agente estiver rodando).
pub async fn bind(cfg: config::Config, cfg_path: PathBuf, dry_run: bool, ble_enabled: bool)
    -> Result<(Shared, tokio::net::TcpListener)> {
    let ble_enabled = ble_enabled && cfg.ble.enabled;
    let port = cfg.port;
    let st = AppState::new(cfg, cfg_path.clone(), dry_run, ble_enabled);
    tracing::info!("clowdeck-agent {} — config {}", env!("CARGO_PKG_VERSION"), cfg_path.display());
    if dry_run {
        tracing::warn!("DRY-RUN: nenhuma tecla sera enviada");
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("porta {port} em uso? (outro agente rodando)"))?;
    tracing::info!("deck virtual + hooks em http://127.0.0.1:{port}/");
    Ok((st, listener))
}

/// Roda o agente ate Ctrl+C/erro: watchdog, descoberta, tick, BLE e servidor HTTP.
pub async fn serve(st: Shared, listener: tokio::net::TcpListener, on_freeze: Option<FreezeAction>) -> Result<()> {
    let port = st.cfg.port;
    match hooks::status(&hooks::default_settings_path()) {
        Ok(s) if s.installed_events.is_empty() => {
            tracing::warn!("hooks nao instalados — rode `clowdeck-agent hooks install` p/ ver estados ao vivo")
        }
        Ok(s) if !s.missing_events.is_empty() => tracing::warn!("hooks parciais; faltam {:?}", s.missing_events),
        Ok(s) if s.port != Some(port) => tracing::warn!("hooks apontam p/ porta {:?}, agente na {port}", s.port),
        Ok(_) => {}
        Err(e) => tracing::warn!("settings do Claude: {e:#}"),
    }

    // Watchdog: uma task tokio pulsa a cada 1 s; uma thread std (fora do runtime) checa a
    // cada 5 s e age se o pulso parar por 20 s. Nunca pega mutex.
    {
        let st2 = st.clone();
        tokio::spawn(async move {
            let mut iv = tokio::time::interval(Duration::from_secs(1));
            loop {
                iv.tick().await;
                st2.liveness_ms.store(app::now_ms(), std::sync::atomic::Ordering::Relaxed);
            }
        });
        let st3 = st.clone();
        std::thread::Builder::new()
            .name("watchdog".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(5));
                let age = app::now_ms().saturating_sub(st3.liveness_ms.load(std::sync::atomic::Ordering::Relaxed));
                if age > 20_000 {
                    tracing::error!(
                        "WATCHDOG: runtime sem pulso ha {} s (fase BLE: {}) — reiniciando",
                        age / 1000,
                        st3.phase()
                    );
                    if let Some(f) = &on_freeze {
                        f();
                    }
                    std::process::exit(70);
                }
            })
            .context("thread do watchdog")?;
    }
    // Pedido de Acessibilidade SO depois que o BLE subiu: o tccd serializa os dialogos
    // de um processo e um pedido pendente segura a autorizacao de Bluetooth.
    if !st.dry_run {
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(25)).await;
            if !tokio::task::spawn_blocking(accessibility_prompt).await.unwrap_or(true) {
                tracing::warn!(
                    "Acessibilidade: ative o Clow Deck em Ajustes > Privacidade e Seguranca > Acessibilidade (o macOS acabou de pedir); sem isso as teclas nao funcionam"
                );
            }
        });
    }
    tokio::spawn(discovery::run_loop(st.clone()));
    tokio::spawn(codex::run_codex(st.clone()));
    tokio::spawn(opencode::run_opencode(st.clone()));
    {
        let st = st.clone();
        tokio::spawn(async move {
            let mut iv = tokio::time::interval(Duration::from_secs(1));
            loop {
                iv.tick().await;
                if st.model().tick() {
                    st.notify_changed();
                }
            }
        });
    }
    if st.ble_status().enabled {
        tokio::spawn(ble::run_ble(st.clone()));
    } else {
        tracing::info!("BLE desligado — so deck virtual");
    }

    let app = router(st);
    let server = axum::serve(listener, app);
    tokio::select! {
        r = server => { r.context("servidor HTTP")?; }
        _ = tokio::signal::ctrl_c() => { tracing::info!("encerrando"); }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn accessibility_trusted() -> Option<bool> {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    Some(unsafe { AXIsProcessTrusted() })
}

/// Pede a permissao de Acessibilidade com o dialogo do sistema (cria a entrada em
/// Ajustes > Privacidade e Seguranca para o processo atual — CLI ou app).
#[cfg(target_os = "macos")]
pub fn accessibility_prompt() -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::CFString;
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }
    let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
    let dict = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) }
}
#[cfg(not(target_os = "macos"))]
pub fn accessibility_prompt() -> bool {
    true
}
#[cfg(not(target_os = "macos"))]
pub fn accessibility_trusted() -> Option<bool> {
    None
}
