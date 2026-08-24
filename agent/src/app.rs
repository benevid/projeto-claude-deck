//! Estado compartilhado entre servidor HTTP, BLE, descoberta e dispatch.

use crate::config::Config;
use crate::model::SessionTable;
use crate::protocol::Info;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use tokio::sync::watch;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct BleStatus {
    pub enabled: bool,
    pub connected: bool,
    pub device: Option<String>,
    pub info: Option<Info>,
    pub last_error: Option<String>,
    pub writes: u64,
    pub events: u64,
    pub hint: Option<String>,
}

/// Push-to-talk em curso: a barra de espaco esta pressionada na sessao `cell`
/// desde `since` (o `/voice` do Claude Code grava enquanto o espaco estiver segurado).
#[derive(Debug, Clone)]
pub struct VoiceHold {
    pub cell: usize,
    /// instante em que o EVENT VOICE_START chegou (antes de focar) — mede o hold real do usuario
    pub event_at: Instant,
    /// instante em que o espaco foi de fato pressionado
    pub since: Instant,
    /// `false` manda a thread de repeat soltar o espaco e terminar
    pub active: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub struct AppState {
    pub cfg: Config,
    pub config_path: std::path::PathBuf,
    pub dry_run: bool,
    pub model: Mutex<SessionTable>,
    pub changed: watch::Sender<u64>,
    pub ble: Mutex<BleStatus>,
    pub hooks_received: AtomicU64,
    pub last_hook: Mutex<Option<serde_json::Value>>,
    /// ultimos 20 hooks recebidos (debug no deck virtual / `GET /state`)
    pub recent_hooks: Mutex<std::collections::VecDeque<serde_json::Value>>,
    pub started: Instant,
    /// ultimo pulso do runtime tokio em ms (epoch); o watchdog (thread std) encerra o
    /// processo se isso parar de avancar — visto na bancada: processo inteiro congelado
    /// 15 min sem nenhum timer disparar.
    pub liveness_ms: AtomicU64,
    /// indice em PHASES: onde a task BLE esta (p/ o watchdog dizer onde travou)
    pub ble_phase: std::sync::atomic::AtomicUsize,
    /// barra de espaco segurada (voz); `None` = solta
    pub voice_hold: Mutex<Option<VoiceHold>>,
    /// serializa VOICE_START/STOP: os EVENTs do deck sao despachados em tasks paralelas e o
    /// STOP chegava antes de o START terminar de focar a janela (hold perdido/preso)
    pub voice_lock: tokio::sync::Mutex<()>,
}

pub const PHASES: &[&str] = &[
    "inicio", "adaptador", "scan", "connect", "discover", "info", "config(pareamento)",
    "subscribe", "sessions", "loop", "idle",
];

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub fn new(cfg: Config, config_path: std::path::PathBuf, dry_run: bool, ble_enabled: bool) -> Shared {
        let (tx, _rx) = watch::channel(0u64);
        Arc::new(AppState {
            cfg,
            config_path,
            dry_run,
            model: Mutex::new(SessionTable::new()),
            changed: tx,
            ble: Mutex::new(BleStatus { enabled: ble_enabled, ..Default::default() }),
            hooks_received: AtomicU64::new(0),
            last_hook: Mutex::new(None),
            recent_hooks: Mutex::new(std::collections::VecDeque::with_capacity(20)),
            started: Instant::now(),
            liveness_ms: AtomicU64::new(now_ms()),
            ble_phase: std::sync::atomic::AtomicUsize::new(0),
            voice_hold: Mutex::new(None),
            voice_lock: tokio::sync::Mutex::new(()),
        })
    }

    pub fn model(&self) -> MutexGuard<'_, SessionTable> {
        self.model.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn ble_status(&self) -> MutexGuard<'_, BleStatus> {
        self.ble.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Acorda quem re-escreve SESSIONS (BLE) — chamar depois de mudar o modelo.
    pub fn notify_changed(&self) {
        let v = self.model().version;
        self.changed.send_replace(v);
    }

    pub fn set_phase(&self, name: &str) {
        let idx = PHASES.iter().position(|p| *p == name).unwrap_or(0);
        self.ble_phase.store(idx, Ordering::Relaxed);
    }
    pub fn phase(&self) -> &'static str {
        PHASES[self.ble_phase.load(Ordering::Relaxed).min(PHASES.len() - 1)]
    }

    pub fn voice_hold(&self) -> MutexGuard<'_, Option<VoiceHold>> {
        self.voice_hold.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn hooks_count(&self) -> u64 {
        self.hooks_received.load(Ordering::Relaxed)
    }
}
