//! Windows: descoberta de processos fica para o M4 (WMI/`tasklist` + ponte WSL).

use crate::model::DiscoveredProcess;
use anyhow::Result;
use std::sync::Once;

static WARN: Once = Once::new();

pub fn discover_sync() -> Result<Vec<DiscoveredProcess>> {
    WARN.call_once(|| tracing::warn!("descoberta Windows: M4 — so hooks alimentam o modelo por enquanto"));
    Ok(Vec::new())
}
