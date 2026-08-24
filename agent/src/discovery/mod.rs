//! Descoberta de processos `claude` (sessoes interativas) — por SO.

use crate::app::Shared;
use crate::model::DiscoveredProcess;
use anyhow::Result;
use std::time::Duration;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(not(target_os = "macos"))]
mod windows;
#[cfg(not(target_os = "macos"))]
use windows as platform;

pub const INTERVAL: Duration = Duration::from_secs(2);

pub async fn discover() -> Result<Vec<DiscoveredProcess>> {
    tokio::task::spawn_blocking(platform::discover_sync).await?
}

pub async fn run_loop(state: Shared) {
    loop {
        match discover().await {
            Ok(procs) => {
                let changed = {
                    let mut m = state.model();
                    let before = m.version;
                    m.apply_discovery(&procs);
                    m.version != before
                };
                if changed {
                    state.notify_changed();
                }
            }
            Err(e) => tracing::warn!("descoberta falhou: {e:#}"),
        }
        tokio::time::sleep(INTERVAL).await;
    }
}
