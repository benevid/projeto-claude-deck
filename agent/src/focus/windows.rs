//! Windows: foco por janela Win32 / UIA (Windows Terminal) fica para o M4.

use crate::config::FocusCfg;
use crate::model::Session;
use anyhow::Result;

pub fn focus_session(s: &Session, _cfg: &FocusCfg, _dry_run: bool) -> Result<String> {
    anyhow::bail!("foco no Windows: M4 (sessao pid {:?})", s.pid)
}
