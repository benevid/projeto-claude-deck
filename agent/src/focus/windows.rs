//! Windows (M4): traz a janela da sessao pra frente.
//!
//! O processo `claude`/`codex`/`opencode` e um app de console: a JANELA pertence ao
//! host do terminal (Windows Terminal, VS Code, conhost...). Por isso procuramos a
//! janela do proprio pid e, se nao houver, a do `terminal_pid` / ancestrais.
//! `SetForegroundWindow` so obedece em algumas condicoes; o truque padrao e
//! `AllowSetForegroundWindow` + restaurar a janela antes.

use crate::config::FocusCfg;
use crate::model::Session;
use anyhow::{anyhow, Result};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, EnumWindows, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow,
    ShowWindow, IsIconic, SW_RESTORE,
};

struct Search {
    want: u32,
    found: Option<HWND>,
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let s = &mut *(lparam.0 as *mut Search);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == s.want && IsWindowVisible(hwnd).as_bool() {
        s.found = Some(hwnd);
        return BOOL(0); // para a enumeracao
    }
    TRUE
}

fn window_of_pid(pid: u32) -> Option<HWND> {
    let mut s = Search { want: pid, found: None };
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(&mut s as *mut _ as isize));
    }
    s.found
}

fn raise(hwnd: HWND) -> Result<()> {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = AllowSetForegroundWindow(u32::MAX); // ASFW_ANY
        SetForegroundWindow(hwnd)
            .ok()
            .map_err(|e| anyhow!("SetForegroundWindow: {e}"))
    }
}

pub fn focus_session(s: &Session, _cfg: &FocusCfg, _dry_run: bool) -> Result<String> {
    // 1) janela do proprio processo  2) janela do host do terminal
    let candidates: Vec<u32> = [s.pid, s.terminal_pid].into_iter().flatten().collect();
    if candidates.is_empty() {
        anyhow::bail!("foco: sessao sem pid conhecido");
    }
    for pid in &candidates {
        if let Some(hwnd) = window_of_pid(*pid) {
            raise(hwnd)?;
            let via = if Some(*pid) == s.pid { "janela do processo" } else { "janela do terminal" };
            return Ok(format!("janela trazida pra frente ({via}, pid {pid})"));
        }
    }
    anyhow::bail!("foco: nenhuma janela visivel para os pids {candidates:?}")
}
