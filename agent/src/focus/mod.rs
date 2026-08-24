//! Foco de janela da sessao (por SO).

use crate::config::FocusCfg;
use crate::model::Session;
use anyhow::Result;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(not(target_os = "macos"))]
pub mod windows;
#[cfg(not(target_os = "macos"))]
use windows as platform;

/// Traz a janela da sessao pra frente. Devolve uma descricao do que foi feito.
/// `dry_run` evita qualquer tecla sintetica (o foco em si e permitido).
/// Nome do app em primeiro plano (macOS; None em outros SOs / falha).
pub fn frontmost_app_name() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        macos::frontmost_app_name()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn focus_session(s: &Session, cfg: &FocusCfg, dry_run: bool) -> Result<String> {
    platform::focus_session(s, cfg, dry_run)
}

pub async fn focus_session_async(s: Session, cfg: FocusCfg, dry_run: bool) -> Result<String> {
    let fut = tokio::task::spawn_blocking(move || focus_session(&s, &cfg, dry_run));
    match tokio::time::timeout(std::time::Duration::from_secs(8), fut).await {
        Ok(r) => r?,
        Err(_) => anyhow::bail!("foco: osascript nao respondeu em 8 s (permissao de Automacao pendente?)"),
    }
}
