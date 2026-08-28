//! Modelo de sessoes: 8 celulas posicionais, atribuicao estavel, transicoes
//! dirigidas pelos hooks do Claude Code e pela descoberta de processos.

use crate::protocol::{ascii_label, CellEntry, Mode, SessionsView, State, LABEL_LEN, SESSION_CELLS};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TerminalApp {
    VSCode,
    Cursor,
    Windsurf,
    Terminal,
    ITerm2,
    Warp,
    Ghostty,
    Alacritty,
    Kitty,
    WezTerm,
    /// Windows Terminal / conhost / PowerShell / cmd (M4)
    WindowsTerminal,
    Other,
}

impl TerminalApp {
    pub fn name(self) -> &'static str {
        match self {
            TerminalApp::VSCode => "VS Code",
            TerminalApp::Cursor => "Cursor",
            TerminalApp::Windsurf => "Windsurf",
            TerminalApp::Terminal => "Terminal",
            TerminalApp::ITerm2 => "iTerm2",
            TerminalApp::Warp => "Warp",
            TerminalApp::Ghostty => "Ghostty",
            TerminalApp::Alacritty => "Alacritty",
            TerminalApp::Kitty => "kitty",
            TerminalApp::WezTerm => "WezTerm",
            TerminalApp::WindowsTerminal => "Windows Terminal",
            TerminalApp::Other => "outro",
        }
    }
    /// Identifica o app de terminal pelo caminho/nome do executavel de um ancestral.
    pub fn from_comm(comm: &str) -> Option<TerminalApp> {
        let base = comm.rsplit('/').next().unwrap_or(comm);
        let lc = comm.to_ascii_lowercase();
        if lc.contains("visual studio code") || base == "Code" || base == "Code Helper" || base.starts_with("Code Helper") {
            return Some(TerminalApp::VSCode);
        }
        if lc.contains("cursor.app") || base == "Cursor" {
            return Some(TerminalApp::Cursor);
        }
        if lc.contains("windsurf") {
            return Some(TerminalApp::Windsurf);
        }
        if lc.contains("iterm") {
            return Some(TerminalApp::ITerm2);
        }
        if lc.contains("terminal.app") || base == "Terminal" {
            return Some(TerminalApp::Terminal);
        }
        if lc.contains("warp.app") || base == "Warp" || base == "stable" && lc.contains("warp") {
            return Some(TerminalApp::Warp);
        }
        if lc.contains("ghostty") {
            return Some(TerminalApp::Ghostty);
        }
        if lc.contains("alacritty") {
            return Some(TerminalApp::Alacritty);
        }
        if base == "kitty" || lc.contains("kitty.app") {
            return Some(TerminalApp::Kitty);
        }
        if lc.contains("wezterm") {
            return Some(TerminalApp::WezTerm);
        }
        // Windows (M4)
        if lc.contains("windowsterminal") || base == "conhost.exe" || base == "powershell.exe"
            || base == "pwsh.exe" || base == "cmd.exe"
        {
            return Some(TerminalApp::WindowsTerminal);
        }
        if base == "code.exe" {
            return Some(TerminalApp::VSCode);
        }
        None
    }
}

/// Engine da sessao: Claude Code (hooks) ou Codex CLI (app-server, M6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Engine {
    Claude,
    Codex,
    Opencode,
}

/// Processo de sessao (claude/codex) visto pela descoberta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredProcess {
    pub engine: Engine,
    pub pid: u32,
    pub ppid: u32,
    pub tty: Option<String>,
    pub cwd: Option<String>,
    pub args: String,
    pub terminal_app: Option<TerminalApp>,
    /// pid do processo-raiz do app de terminal (ppid == 1), p/ foco generico.
    pub terminal_pid: Option<u32>,
}

/// Evento de hook ja normalizado (vem do servidor HTTP).
#[derive(Debug, Clone, Default)]
pub struct HookEvent {
    pub event: String,
    pub pid: Option<u32>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub permission_mode: Option<String>,
    pub notification_type: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub sid: u8,
    pub engine: Engine,
    pub pid: Option<u32>,
    pub session_id: Option<String>,
    pub cwd: String,
    pub label: String,
    pub tty: Option<String>,
    pub terminal_app: Option<TerminalApp>,
    pub terminal_pid: Option<u32>,
    pub state: State,
    pub mode: Mode,
    pub state_since: Instant,
    pub created_at: Instant,
    pub has_hooks: bool,
    /// id do thread no app-server do Codex (M6.b: acoes via `codex queue`)
    pub codex_thread: Option<String>,
    pub seen_by_discovery: bool,
    pub last_event: String,
}

impl Session {
    fn new(sid: u8, cwd: &str) -> Session {
        let now = Instant::now();
        Session {
            sid,
            engine: Engine::Claude,
            pid: None,
            session_id: None,
            cwd: cwd.to_string(),
            label: label_for(cwd),
            tty: None,
            terminal_app: None,
            terminal_pid: None,
            state: State::Unknown,
            mode: Mode::Unknown,
            state_since: now,
            created_at: now,
            has_hooks: false,
            codex_thread: None,
            seen_by_discovery: false,
            last_event: String::new(),
        }
    }
    pub fn age(&self) -> Duration {
        self.state_since.elapsed()
    }
    pub fn age_s(&self) -> u16 {
        self.age().as_secs().min(u16::MAX as u64) as u16
    }
    fn set_state(&mut self, s: State) {
        if self.state != s {
            self.state = s;
            self.state_since = Instant::now();
        }
    }
}

pub fn label_for(cwd: &str) -> String {
    if cwd.is_empty() {
        return "?".into();
    }
    // aceita separador POSIX e Windows (`C:\dev\proj\` -> "proj")
    let trimmed = cwd.trim_end_matches(['/', '\\']);
    let base = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    if base.is_empty() {
        return "/".into();
    }
    // raiz de volume no Windows ("C:") vira o nome do drive
    if base.ends_with(':') {
        return base.trim_end_matches(':').to_ascii_uppercase();
    }
    let l = ascii_label(base, LABEL_LEN);
    if l.is_empty() {
        "?".into()
    } else {
        l
    }
}

pub const DEAD_LINGER: Duration = Duration::from_secs(5);
/// Sessao vinda de hook cujo pid nunca apareceu no `ps`: so e dada como morta
/// depois deste prazo (a descoberta roda a cada 2 s e pode atrasar).
const UNSEEN_GRACE: Duration = Duration::from_secs(6);

#[derive(Debug)]
pub struct SessionTable {
    cells: Vec<Option<Session>>,
    overflow: Vec<Session>,
    active: Option<usize>,
    next_sid: u8,
    pub version: u64,
    /// celulas de sessao que o deck conectado tem (INFO `session_cells`); sessoes so
    /// ocupam 0..capacity-1, o resto espera em `overflow`. O payload SESSIONS continua
    /// com 8 entradas (PROTO_VERSION 1) — o deck ignora as que nao tem.
    capacity: usize,
}

impl Default for SessionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionTable {
    pub fn new() -> SessionTable {
        SessionTable {
            cells: (0..SESSION_CELLS).map(|_| None).collect(),
            overflow: Vec::new(),
            active: None,
            next_sid: 1,
            version: 0,
            capacity: SESSION_CELLS,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Ajusta ao `session_cells` do deck (1..=8). Encolhendo, as sessoes das celulas
    /// removidas vao para o inicio da fila; crescendo, a fila e promovida.
    pub fn set_capacity(&mut self, n: usize) {
        let n = n.clamp(1, SESSION_CELLS);
        if n == self.capacity {
            return;
        }
        if n < self.capacity {
            let mut moved: Vec<Session> = Vec::new();
            for i in n..self.capacity {
                if let Some(s) = self.cells[i].take() {
                    moved.push(s);
                }
                if self.active == Some(i) {
                    self.active = None;
                }
            }
            moved.extend(self.overflow.drain(..));
            self.overflow = moved;
        }
        self.capacity = n;
        self.promote_overflow();
        self.bump();
    }

    fn free_cell(&self) -> Option<usize> {
        (0..self.capacity).find(|&i| self.cells[i].is_none())
    }

    fn promote_overflow(&mut self) {
        while !self.overflow.is_empty() {
            if let Some(i) = self.free_cell() {
                let s = self.overflow.remove(0);
                self.cells[i] = Some(s);
                self.bump();
            } else {
                break;
            }
        }
    }

    fn bump(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    fn alloc_sid(&mut self) -> u8 {
        loop {
            let sid = self.next_sid;
            self.next_sid = if self.next_sid == 255 { 1 } else { self.next_sid + 1 };
            let in_use = self.iter().any(|s| s.sid == sid) || self.overflow.iter().any(|s| s.sid == sid);
            if !in_use {
                return sid;
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Session> {
        self.cells.iter().filter_map(|c| c.as_ref())
    }

    pub fn cell(&self, i: usize) -> Option<&Session> {
        self.cells.get(i).and_then(|c| c.as_ref())
    }

    pub fn active_cell(&self) -> Option<usize> {
        self.active
    }

    #[allow(dead_code)]
    pub fn active_session(&self) -> Option<&Session> {
        self.active.and_then(|i| self.cell(i))
    }

    pub fn cell_of_pid(&self, pid: u32) -> Option<usize> {
        self.cells.iter().position(|c| c.as_ref().map(|s| s.pid == Some(pid)).unwrap_or(false))
    }

    fn cell_of_session_id(&self, sid: &str) -> Option<usize> {
        self.cells
            .iter()
            .position(|c| c.as_ref().map(|s| s.session_id.as_deref() == Some(sid)).unwrap_or(false))
    }

    fn cell_of_cwd(&self, cwd: &str, prefer_unhooked: bool) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (i, c) in self.cells.iter().enumerate() {
            if let Some(s) = c {
                if s.cwd == cwd && s.state != State::Dead {
                    if prefer_unhooked && s.session_id.is_none() {
                        return Some(i);
                    }
                    if best.is_none() {
                        best = Some(i);
                    }
                }
            }
        }
        best
    }

    /// Insere na primeira celula livre; se nao houver, vai p/ a fila de espera.
    fn insert(&mut self, s: Session) -> Option<usize> {
        if let Some(i) = self.free_cell() {
            self.cells[i] = Some(s);
            self.bump();
            Some(i)
        } else {
            self.overflow.push(s);
            self.bump();
            None
        }
    }

    /// Reconcilia com a lista de processos `claude` vivos.
    pub fn apply_discovery(&mut self, procs: &[DiscoveredProcess]) {
        let now = Instant::now();
        for p in procs {
            let cwd = p.cwd.clone().unwrap_or_default();
            let idx = self.cell_of_pid(p.pid).or_else(|| {
                // hook chegou antes da descoberta (sem pid valido): casa por cwd
                if cwd.is_empty() {
                    None
                } else {
                    self.cells.iter().position(|c| {
                        c.as_ref()
                            .map(|s| s.pid.is_none() && s.cwd == cwd && s.state != State::Dead)
                            .unwrap_or(false)
                    })
                }
            });
            let existing_overflow = self.overflow.iter().position(|s| s.pid == Some(p.pid));
            match idx {
                Some(i) => {
                    let s = self.cells[i].as_mut().unwrap();
                    let mut changed = false;
                    if s.pid != Some(p.pid) {
                        s.pid = Some(p.pid);
                        changed = true;
                    }
                    if !cwd.is_empty() && s.cwd != cwd {
                        s.cwd = cwd.clone();
                        s.label = label_for(&cwd);
                        changed = true;
                    }
                    if s.tty != p.tty {
                        s.tty = p.tty.clone();
                        changed = true;
                    }
                    if s.terminal_app != p.terminal_app {
                        s.terminal_app = p.terminal_app;
                        changed = true;
                    }
                    s.terminal_pid = p.terminal_pid;
                    if s.engine != p.engine {
                        s.engine = p.engine;
                        changed = true;
                    }
                    s.seen_by_discovery = true;
                    if changed {
                        self.bump();
                    }
                }
                None if existing_overflow.is_some() => {
                    let s = &mut self.overflow[existing_overflow.unwrap()];
                    s.tty = p.tty.clone();
                    s.terminal_app = p.terminal_app;
                    s.terminal_pid = p.terminal_pid;
                    s.seen_by_discovery = true;
                }
                None => {
                    let sid = self.alloc_sid();
                    let mut s = Session::new(sid, &cwd);
                    s.engine = p.engine;
                    s.pid = Some(p.pid);
                    s.tty = p.tty.clone();
                    s.terminal_app = p.terminal_app;
                    s.terminal_pid = p.terminal_pid;
                    s.seen_by_discovery = true;
                    s.state_since = now;
                    self.insert(s);
                }
            }
        }
        // processos que sumiram
        let alive: std::collections::HashSet<u32> = procs.iter().map(|p| p.pid).collect();
        for i in 0..self.cells.len() {
            let Some(s) = self.cells[i].as_mut() else { continue };
            if s.state == State::Dead {
                continue;
            }
            let Some(pid) = s.pid else { continue };
            if alive.contains(&pid) {
                continue;
            }
            if s.seen_by_discovery || s.created_at.elapsed() > UNSEEN_GRACE {
                s.set_state(State::Dead);
                s.last_event = "gone".into();
                self.bump();
            }
        }
        self.overflow.retain(|s| s.pid.map(|p| alive.contains(&p)).unwrap_or(true));
    }

    /// Aplica um evento de hook. Devolve a celula afetada.
    pub fn apply_hook(&mut self, ev: &HookEvent) -> Option<usize> {
        let cwd = ev.cwd.clone().unwrap_or_default();
        // 1) pid do query  2) session_id  3) cwd (preferindo quem ainda nao tem session_id)
        let mut idx = ev
            .pid
            .and_then(|p| self.cell_of_pid(p))
            .or_else(|| ev.session_id.as_deref().and_then(|s| self.cell_of_session_id(s)))
            .or_else(|| if cwd.is_empty() { None } else { self.cell_of_cwd(&cwd, true) });

        if idx.is_none() {
            if ev.event == "SessionEnd" {
                return None; // nunca vimos: nada a fazer
            }
            let sid = self.alloc_sid();
            let mut s = Session::new(sid, &cwd);
            s.pid = ev.pid;
            idx = self.insert(s);
            let Some(_) = idx else { return None };
        }
        let i = idx.unwrap();
        let s = self.cells[i].as_mut().unwrap();
        if let Some(p) = ev.pid {
            if s.pid.is_none() {
                s.pid = Some(p);
            }
        }
        if let Some(sidv) = &ev.session_id {
            s.session_id = Some(sidv.clone());
        }
        if !cwd.is_empty() && s.cwd != cwd {
            s.cwd = cwd.clone();
            s.label = label_for(&cwd);
        }
        if let Some(m) = &ev.permission_mode {
            let m = Mode::from_hook(m);
            if m != Mode::Unknown {
                s.mode = m;
            }
        }
        s.has_hooks = true;
        s.last_event = ev.event.clone();
        let new_state = match ev.event.as_str() {
            "SessionStart" => {
                if ev.source.as_deref() == Some("compact") {
                    Some(State::Working)
                } else {
                    Some(State::Idle)
                }
            }
            "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "PreCompact" => Some(State::Working),
            "PermissionRequest" => Some(State::Attention),
            "Notification" => match ev.notification_type.as_deref() {
                Some("idle_prompt") => {
                    if s.state == State::Working {
                        Some(State::Done)
                    } else {
                        None
                    }
                }
                _ => Some(State::Attention),
            },
            "Stop" => Some(State::Done),
            "SessionEnd" => Some(State::Dead),
            _ => None, // SubagentStop e afins
        };
        if let Some(st) = new_state {
            s.set_state(st);
        }
        self.bump();
        Some(i)
    }

    /// Vincula o thread do app-server a sessao Codex daquele cwd (M6.b).
    pub fn set_codex_thread(&mut self, cwd: &str, thread_id: &str) {
        for c in self.cells.iter_mut().flatten() {
            if c.engine == Engine::Codex && c.cwd == cwd && c.state != State::Dead {
                if c.codex_thread.as_deref() != Some(thread_id) {
                    c.codex_thread = Some(thread_id.to_string());
                }
                return;
            }
        }
    }

    /// Estado vindo de um engine externo (codex/opencode): aplica na sessao
    /// daquele engine + cwd (M6/M7).
    pub fn apply_engine_state(&mut self, engine: Engine, cwd: &str, state: State) -> Option<usize> {
        for i in 0..self.cells.len() {
            let Some(s) = self.cells[i].as_mut() else { continue };
            if s.engine == engine && s.cwd == cwd && s.state != State::Dead {
                if s.state != state {
                    s.set_state(state);
                    s.last_event = "engine".into();
                    self.bump();
                }
                return Some(i);
            }
        }
        None
    }

    /// Tap/FOCUS confirmado: marca ativa; DONE vira IDLE (o usuario esta olhando).
    pub fn focus_ack(&mut self, cell: usize) -> bool {
        if self.cell(cell).is_none() {
            return false;
        }
        self.active = Some(cell);
        if let Some(s) = self.cells[cell].as_mut() {
            if s.state == State::Done {
                s.set_state(State::Idle);
            }
        }
        self.bump();
        true
    }

    pub fn ack(&mut self, cell: usize) -> bool {
        let Some(s) = self.cells.get_mut(cell).and_then(|c| c.as_mut()) else { return false };
        if s.state == State::Done {
            s.set_state(State::Idle);
            self.bump();
        }
        true
    }

    #[allow(dead_code)]
    pub fn set_error(&mut self, cell: usize) {
        if let Some(s) = self.cells.get_mut(cell).and_then(|c| c.as_mut()) {
            s.set_state(State::Error);
            self.bump();
        }
    }

    /// Limpeza periodica: remove mortas antigas, promove a fila, re-emite versao
    /// quando `age_s` muda de forma relevante (deck pisca DONE ate 60 s).
    pub fn tick(&mut self) -> bool {
        let before = self.version;
        for i in 0..self.cells.len() {
            let remove = self.cells[i]
                .as_ref()
                .map(|s| s.state == State::Dead && s.age() > DEAD_LINGER)
                .unwrap_or(false);
            if remove {
                self.cells[i] = None;
                if self.active == Some(i) {
                    self.active = None;
                }
                self.bump();
            }
        }
        self.promote_overflow();
        self.version != before
    }

    pub fn view(&self, voice: bool, usage: bool) -> SessionsView {
        let mut v = SessionsView { voice, usage, active: self.active.map(|a| a as u8), cells: Vec::new() };
        for (i, c) in self.cells.iter().enumerate() {
            v.cells.push(match c {
                Some(s) => CellEntry {
                    sid: s.sid,
                    state: s.state as u8,
                    mode: s.mode as u8,
                    active: self.active == Some(i),
                    no_hooks: !s.has_hooks,
                    codex: s.engine == Engine::Codex,
                    opencode: s.engine == Engine::Opencode,
                    age_s: s.age_s(),
                    label: s.label.clone(),
                },
                None => CellEntry::default(),
            });
        }
        v
    }

    pub fn overflow_len(&self) -> usize {
        self.overflow.len()
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        let mut open = 0;
        let mut attention = 0;
        let mut done = 0;
        for s in self.iter() {
            if s.state == State::Dead {
                continue;
            }
            open += 1;
            match s.state {
                State::Attention => attention += 1,
                State::Done => done += 1,
                _ => {}
            }
        }
        (open, attention, done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, cwd: &str) -> DiscoveredProcess {
        DiscoveredProcess {
            engine: Engine::Claude,
            pid,
            ppid: 1,
            tty: Some(format!("ttys{pid:03}")),
            cwd: Some(cwd.into()),
            args: "claude".into(),
            terminal_app: Some(TerminalApp::VSCode),
            terminal_pid: Some(1),
        }
    }
    fn hook(event: &str, pid: Option<u32>, cwd: &str) -> HookEvent {
        HookEvent {
            event: event.into(),
            pid,
            session_id: Some(format!("sess-{}", pid.unwrap_or(0))),
            cwd: Some(cwd.into()),
            permission_mode: Some("plan".into()),
            notification_type: None,
            source: None,
        }
    }

    #[test]
    fn stable_cells_and_sids() {
        let mut t = SessionTable::new();
        t.apply_discovery(&[proc(10, "/a/alpha"), proc(20, "/b/beta")]);
        assert_eq!(t.cell(0).unwrap().pid, Some(10));
        assert_eq!(t.cell(1).unwrap().pid, Some(20));
        assert_eq!(t.cell(0).unwrap().state, State::Unknown);
        assert_eq!(t.cell(0).unwrap().label, "alpha");
        let sid0 = t.cell(0).unwrap().sid;
        // 10 morre: celula 0 fica DEAD e depois livre; 20 nao se move
        t.apply_discovery(&[proc(20, "/b/beta")]);
        assert_eq!(t.cell(0).unwrap().state, State::Dead);
        t.cells[0].as_mut().unwrap().state_since = Instant::now() - Duration::from_secs(10);
        assert!(t.tick());
        assert!(t.cell(0).is_none());
        assert_eq!(t.cell(1).unwrap().pid, Some(20));
        // nova ocupa a primeira livre com sid novo
        t.apply_discovery(&[proc(20, "/b/beta"), proc(30, "/c/gamma")]);
        assert_eq!(t.cell(0).unwrap().pid, Some(30));
        assert_ne!(t.cell(0).unwrap().sid, sid0);
        assert_ne!(t.cell(0).unwrap().sid, 0);
    }

    #[test]
    fn hook_transitions() {
        let mut t = SessionTable::new();
        t.apply_discovery(&[proc(10, "/a/alpha")]);
        assert_eq!(t.apply_hook(&hook("SessionStart", Some(10), "/a/alpha")), Some(0));
        let s = t.cell(0).unwrap();
        assert_eq!(s.state, State::Idle);
        assert!(s.has_hooks);
        assert_eq!(s.mode, Mode::Plan);
        t.apply_hook(&hook("UserPromptSubmit", Some(10), "/a/alpha"));
        assert_eq!(t.cell(0).unwrap().state, State::Working);
        t.apply_hook(&hook("PermissionRequest", Some(10), "/a/alpha"));
        assert_eq!(t.cell(0).unwrap().state, State::Attention);
        t.apply_hook(&hook("PostToolUse", Some(10), "/a/alpha"));
        assert_eq!(t.cell(0).unwrap().state, State::Working);
        t.apply_hook(&hook("Stop", Some(10), "/a/alpha"));
        assert_eq!(t.cell(0).unwrap().state, State::Done);
        // idle_prompt nao rebaixa DONE
        let mut n = hook("Notification", Some(10), "/a/alpha");
        n.notification_type = Some("idle_prompt".into());
        t.apply_hook(&n);
        assert_eq!(t.cell(0).unwrap().state, State::Done);
        // tap confirma: DONE -> IDLE + ativa
        assert!(t.focus_ack(0));
        assert_eq!(t.cell(0).unwrap().state, State::Idle);
        assert_eq!(t.active_cell(), Some(0));
        let v = t.view(false, false);
        assert_eq!(v.active, Some(0));
        assert!(v.cells[0].active);
        assert!(!v.cells[0].no_hooks);
        t.apply_hook(&hook("SessionEnd", Some(10), "/a/alpha"));
        assert_eq!(t.cell(0).unwrap().state, State::Dead);
    }

    #[test]
    fn hook_before_discovery_matches_by_pid_then_cwd() {
        let mut t = SessionTable::new();
        // hook chega primeiro (pid conhecido)
        t.apply_hook(&hook("SessionStart", Some(77), "/x/proj"));
        assert_eq!(t.cell(0).unwrap().pid, Some(77));
        // descoberta encontra o mesmo pid: nao duplica
        t.apply_discovery(&[proc(77, "/x/proj")]);
        assert!(t.cell(1).is_none());
        assert!(t.cell(0).unwrap().seen_by_discovery);
        // hook sem pid (Windows) casa por cwd
        let mut h = hook("Stop", None, "/x/proj");
        h.session_id = Some("sess-77".into());
        assert_eq!(t.apply_hook(&h), Some(0));
        assert_eq!(t.cell(0).unwrap().state, State::Done);
        // hook sem pid de cwd desconhecido cria celula nova (sem ser morta cedo)
        t.apply_hook(&hook("Stop", None, "/y/outro"));
        assert_eq!(t.cell(1).unwrap().label, "outro");
        t.apply_discovery(&[proc(77, "/x/proj")]);
        assert_ne!(t.cell(1).unwrap().state, State::Dead);
    }

    #[test]
    fn capacity_follows_deck_session_cells() {
        let mut t = SessionTable::new();
        let procs: Vec<DiscoveredProcess> = (0..8)
            .map(|i| DiscoveredProcess { engine: Engine::Claude, pid: 1000 + i, ppid: 1, tty: None, cwd: Some(format!("/tmp/p{i}")), args: "claude".into(), terminal_app: None, terminal_pid: None })
            .collect();
        t.apply_discovery(&procs);
        assert!(t.cell(7).is_some());
        t.set_capacity(6);
        assert_eq!(t.capacity(), 6);
        assert!(t.cell(6).is_none() && t.cell(7).is_none());
        assert_eq!(t.overflow_len(), 2);
        // nova sessao nao ocupa 6/7
        let view = t.view(false, false);
        assert_eq!(view.cells[6].sid, 0);
        t.set_capacity(8);
        assert_eq!(t.overflow_len(), 0);
        assert!(t.cell(6).is_some() && t.cell(7).is_some());
    }

    #[test]
    fn overflow_promotes_when_cell_frees() {
        let mut t = SessionTable::new();
        let procs: Vec<_> = (1..=9).map(|i| proc(i, &format!("/p/{i}"))).collect();
        t.apply_discovery(&procs);
        assert_eq!(t.overflow_len(), 1);
        t.apply_discovery(&procs[1..]);
        t.cells[0].as_mut().unwrap().state_since = Instant::now() - Duration::from_secs(10);
        t.tick();
        assert_eq!(t.cell(0).unwrap().pid, Some(9));
        assert_eq!(t.overflow_len(), 0);
    }

    #[test]
    fn labels() {
        assert_eq!(label_for("/Users/x/projeto-configuração"), "projeto-conf");
        assert_eq!(label_for("/"), "/");
        assert_eq!(label_for(""), "?");
        // Windows (M4): separador `\` e barra final
        assert_eq!(label_for("C:\\dev\\clowdeck"), "clowdeck");
        assert_eq!(label_for("C:\\dev\\clowdeck\\"), "clowdeck");
        assert_eq!(label_for("C:\\Users\\dev\\"), "dev");
        assert_eq!(label_for("C:\\"), "C");
    }
}
