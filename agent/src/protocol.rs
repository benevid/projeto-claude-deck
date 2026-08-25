//! Protocolo BLE do Clow Deck — espelho byte a byte de `protocol/PROTOCOL.md` (v1).
//!
//! Este modulo nao depende de nada alem de `uuid`: e o unico lugar do agente que
//! conhece o layout dos payloads. Firmware e agente precisam concordar aqui.

use uuid::{uuid, Uuid};

pub const PROTO_VERSION: u8 = 1;

pub const SERVICE_UUID: Uuid = uuid!("c1a0de00-0dec-4a11-8000-c10dec000001");
pub const CHAR_INFO: Uuid = uuid!("c1a0de01-0dec-4a11-8000-c10dec000001");
pub const CHAR_SESSIONS: Uuid = uuid!("c1a0de02-0dec-4a11-8000-c10dec000001");
pub const CHAR_EVENT: Uuid = uuid!("c1a0de03-0dec-4a11-8000-c10dec000001");
pub const CHAR_USAGE: Uuid = uuid!("c1a0de04-0dec-4a11-8000-c10dec000001");
pub const CHAR_CONFIG: Uuid = uuid!("c1a0de05-0dec-4a11-8000-c10dec000001");

pub const DECK_NAME: &str = "Clow Deck";

/// Celulas de sessao (linhas 1 e 2 da grade 4x3).
pub const SESSION_CELLS: usize = 8;
/// Celulas totais (inclui a fileira de acoes 8..11).
#[allow(dead_code)]
pub const TOTAL_CELLS: usize = 12;
pub const LABEL_LEN: usize = 12;
pub const ENTRY_LEN: usize = 18;
pub const SESSIONS_LEN: usize = 4 + SESSION_CELLS * ENTRY_LEN; // 148
/// Maior frame que o agente assume sem negociar MTU (MTU 185 - 3).
#[allow(dead_code)]
pub const MAX_FRAME: usize = 182;
pub const MAX_CUSTOM_COMMANDS: usize = 16;

pub const CELL_VOICE: u8 = 8;
pub const CELL_MODE: u8 = 9;
pub const CELL_CMD: u8 = 10;
pub const CELL_MENU: u8 = 11;
pub const CELL_ACTIVE: u8 = 0xFF;

// flags do cabecalho de SESSIONS
pub const SF_READY: u8 = 1 << 0;
pub const SF_VOICE: u8 = 1 << 1;
pub const SF_USAGE: u8 = 1 << 2;
// flags por entrada
pub const EF_ACTIVE: u8 = 1 << 0;
pub const EF_NO_HOOKS: u8 = 1 << 1;
/// sessao de outro engine (Codex CLI) — M6
pub const EF_CODEX: u8 = 1 << 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum State {
    Empty = 0,
    Unknown = 1,
    Working = 2,
    Attention = 3,
    Done = 4,
    Idle = 5,
    Error = 6,
    Dead = 7,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Empty => "empty",
            State::Unknown => "unknown",
            State::Working => "working",
            State::Attention => "attention",
            State::Done => "done",
            State::Idle => "idle",
            State::Error => "error",
            State::Dead => "dead",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Mode {
    Unknown = 0,
    Default = 1,
    AcceptEdits = 2,
    Plan = 3,
    Bypass = 4,
    DontAsk = 5,
}

impl Mode {
    /// `permission_mode` como chega nos hooks do Claude Code.
    pub fn from_hook(s: &str) -> Mode {
        match s {
            "default" => Mode::Default,
            "acceptEdits" => Mode::AcceptEdits,
            "plan" => Mode::Plan,
            "bypassPermissions" => Mode::Bypass,
            "dontAsk" | "auto" => Mode::DontAsk,
            _ => Mode::Unknown,
        }
    }
    /// Rotulo curto mostrado na celula MODO (tabela do protocolo).
    pub fn short(self) -> &'static str {
        match self {
            Mode::Unknown => "--",
            Mode::Default => "ask",
            Mode::AcceptEdits => "edits",
            Mode::Plan => "plan",
            Mode::Bypass => "bypass",
            Mode::DontAsk => "auto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum EventKind {
    CellTap = 1,
    CellHold = 2,
    CellRelease = 3,
    Action = 4,
    Deck = 5,
}

impl EventKind {
    pub fn from_u8(v: u8) -> Option<EventKind> {
        Some(match v {
            1 => EventKind::CellTap,
            2 => EventKind::CellHold,
            3 => EventKind::CellRelease,
            4 => EventKind::Action,
            5 => EventKind::Deck,
            _ => return None,
        })
    }
    pub fn from_name(s: &str) -> Option<EventKind> {
        Some(match s {
            "cell_tap" | "tap" => EventKind::CellTap,
            "cell_hold" | "hold" => EventKind::CellHold,
            "cell_release" | "release" => EventKind::CellRelease,
            "action" => EventKind::Action,
            "deck" => EventKind::Deck,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Focus,
    ModeCycle,
    Compact,
    Clear,
    Esc,
    Enter,
    Ack,
    Tab,
    Approve,
    Init,
    VoiceStart,
    VoiceStop,
    VoiceCancel,
    Custom(u8),
}

impl Action {
    pub const FOCUS: u8 = 0x01;
    pub const MODE_CYCLE: u8 = 0x10;
    pub const COMPACT: u8 = 0x11;
    pub const CLEAR: u8 = 0x12;
    pub const ESC: u8 = 0x13;
    pub const ENTER: u8 = 0x14;
    pub const ACK: u8 = 0x15;
    pub const TAB: u8 = 0x16;
    /// aprova o pedido pendente na sessao (codex: 'y'; claude: '1') — M6.c
    pub const APPROVE: u8 = 0x17;
    /// digita `/init` (claude: CLAUDE.md; codex: AGENTS.md)
    pub const INIT: u8 = 0x18;
    pub const VOICE_START: u8 = 0x20;
    pub const VOICE_STOP: u8 = 0x21;
    pub const VOICE_CANCEL: u8 = 0x22;
    pub const CUSTOM_BASE: u8 = 0x30;

    pub fn from_u8(v: u8) -> Option<Action> {
        Some(match v {
            Self::FOCUS => Action::Focus,
            Self::MODE_CYCLE => Action::ModeCycle,
            Self::COMPACT => Action::Compact,
            Self::CLEAR => Action::Clear,
            Self::ESC => Action::Esc,
            Self::ENTER => Action::Enter,
            Self::ACK => Action::Ack,
            Self::TAB => Action::Tab,
            Self::APPROVE => Action::Approve,
            Self::INIT => Action::Init,
            Self::VOICE_START => Action::VoiceStart,
            Self::VOICE_STOP => Action::VoiceStop,
            Self::VOICE_CANCEL => Action::VoiceCancel,
            0x30..=0x3F => Action::Custom(v - Self::CUSTOM_BASE),
            _ => return None,
        })
    }
    pub fn to_u8(self) -> u8 {
        match self {
            Action::Focus => Self::FOCUS,
            Action::ModeCycle => Self::MODE_CYCLE,
            Action::Compact => Self::COMPACT,
            Action::Clear => Self::CLEAR,
            Action::Esc => Self::ESC,
            Action::Enter => Self::ENTER,
            Action::Ack => Self::ACK,
            Action::Tab => Self::TAB,
            Action::Approve => Self::APPROVE,
            Action::Init => Self::INIT,
            Action::VoiceStart => Self::VOICE_START,
            Action::VoiceStop => Self::VOICE_STOP,
            Action::VoiceCancel => Self::VOICE_CANCEL,
            Action::Custom(n) => Self::CUSTOM_BASE + (n & 0x0F),
        }
    }
    pub fn from_name(s: &str) -> Option<Action> {
        Some(match s {
            "focus" => Action::Focus,
            "mode_cycle" | "mode" => Action::ModeCycle,
            "compact" => Action::Compact,
            "clear" => Action::Clear,
            "esc" => Action::Esc,
            "enter" => Action::Enter,
            "ack" => Action::Ack,
            "tab" => Action::Tab,
            "approve" => Action::Approve,
            "init" => Action::Init,
            "voice_start" => Action::VoiceStart,
            "voice_stop" => Action::VoiceStop,
            "voice_cancel" => Action::VoiceCancel,
            _ => {
                let n = s.strip_prefix("custom_")?.parse::<u8>().ok()?;
                if n > 0x0F {
                    return None;
                }
                Action::Custom(n)
            }
        })
    }
    #[allow(dead_code)]
    pub fn name(self) -> String {
        match self {
            Action::Focus => "focus".into(),
            Action::ModeCycle => "mode_cycle".into(),
            Action::Compact => "compact".into(),
            Action::Clear => "clear".into(),
            Action::Esc => "esc".into(),
            Action::Enter => "enter".into(),
            Action::Ack => "ack".into(),
            Action::Tab => "tab".into(),
            Action::Approve => "approve".into(),
            Action::Init => "init".into(),
            Action::VoiceStart => "voice_start".into(),
            Action::VoiceStop => "voice_stop".into(),
            Action::VoiceCancel => "voice_cancel".into(),
            Action::Custom(n) => format!("custom_{n}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckCode {
    Hello,
    Bonded,
    Page,
    /// arg = media de ms por iteracao do loop() do deck (ultimos 10 s)
    Stats,
    Other(u8),
}

impl DeckCode {
    pub fn from_u8(v: u8) -> DeckCode {
        match v {
            1 => DeckCode::Hello,
            2 => DeckCode::Bonded,
            3 => DeckCode::Page,
            4 => DeckCode::Stats,
            o => DeckCode::Other(o),
        }
    }
}

/// Evento decodificado (deck → agente, §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub cell: u8,
    pub arg: u8,
}

impl Event {
    pub fn encode(&self) -> Vec<u8> {
        vec![PROTO_VERSION, self.kind as u8, self.cell, self.arg]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Short(usize),
    Version(u8),
    Kind(u8),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Short(n) => write!(f, "payload curto ({n} bytes)"),
            DecodeError::Version(v) => write!(f, "versao de protocolo {v} (esperado {PROTO_VERSION})"),
            DecodeError::Kind(k) => write!(f, "kind desconhecido {k}"),
        }
    }
}
impl std::error::Error for DecodeError {}

pub fn decode_event(b: &[u8]) -> Result<Event, DecodeError> {
    if b.len() < 4 {
        return Err(DecodeError::Short(b.len()));
    }
    if b[0] != PROTO_VERSION {
        return Err(DecodeError::Version(b[0]));
    }
    let kind = EventKind::from_u8(b[1]).ok_or(DecodeError::Kind(b[1]))?;
    Ok(Event { kind, cell: b[2], arg: b[3] })
}

/// `INFO` (§4.1) — lida antes de parear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Info {
    pub proto: u8,
    pub fw_major: u8,
    pub fw_minor: u8,
    pub cols: u8,
    pub rows: u8,
    pub session_cells: u8,
    pub label_len: u8,
    pub caps: u8,
}

impl Info {
    pub fn secure(&self) -> bool {
        self.caps & 1 != 0
    }
}

pub fn decode_info(b: &[u8]) -> Result<Info, DecodeError> {
    if b.len() < 8 {
        return Err(DecodeError::Short(b.len()));
    }
    Ok(Info {
        proto: b[0],
        fw_major: b[1],
        fw_minor: b[2],
        cols: b[3],
        rows: b[4],
        session_cells: b[5],
        label_len: b[6],
        caps: b[7],
    })
}

/// Uma celula de sessao como o deck a ve (§4.2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellEntry {
    pub sid: u8,
    pub state: u8,
    pub mode: u8,
    pub active: bool,
    pub no_hooks: bool,
    /// engine Codex CLI (EF_CODEX)
    pub codex: bool,
    pub age_s: u16,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionsView {
    pub voice: bool,
    pub usage: bool,
    pub active: Option<u8>,
    pub cells: Vec<CellEntry>, // sempre SESSION_CELLS entradas
}

impl Default for SessionsView {
    fn default() -> Self {
        SessionsView {
            voice: false,
            usage: false,
            active: None,
            cells: vec![CellEntry::default(); SESSION_CELLS],
        }
    }
}

/// Transliteracao minima p/ a fonte do deck (ASCII puro, sem acentos).
pub fn ascii_label(s: &str, max: usize) -> String {
    let mut out = String::with_capacity(max);
    for ch in s.chars() {
        let mapped: Option<char> = match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | ' ' | '+' | '#' | '@' | '~' => Some(ch),
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => Some('a'),
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => Some('A'),
            'è' | 'é' | 'ê' | 'ë' | 'ē' => Some('e'),
            'È' | 'É' | 'Ê' | 'Ë' => Some('E'),
            'ì' | 'í' | 'î' | 'ï' => Some('i'),
            'Ì' | 'Í' | 'Î' | 'Ï' => Some('I'),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => Some('o'),
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => Some('O'),
            'ù' | 'ú' | 'û' | 'ü' => Some('u'),
            'Ù' | 'Ú' | 'Û' | 'Ü' => Some('U'),
            'ç' => Some('c'),
            'Ç' => Some('C'),
            'ñ' => Some('n'),
            'Ñ' => Some('N'),
            'ý' | 'ÿ' => Some('y'),
            'ß' => Some('s'),
            '—' | '–' => Some('-'),
            _ => None,
        };
        if let Some(c) = mapped {
            out.push(c);
            if out.len() >= max {
                break;
            }
        }
    }
    out
}

pub fn encode_sessions(v: &SessionsView) -> Vec<u8> {
    let mut b = Vec::with_capacity(SESSIONS_LEN);
    let mut flags = SF_READY;
    if v.voice {
        flags |= SF_VOICE;
    }
    if v.usage {
        flags |= SF_USAGE;
    }
    b.push(PROTO_VERSION);
    b.push(flags);
    b.push(SESSION_CELLS as u8);
    b.push(v.active.unwrap_or(CELL_ACTIVE));
    for i in 0..SESSION_CELLS {
        let e = v.cells.get(i).cloned().unwrap_or_default();
        b.push(e.sid);
        b.push(e.state);
        b.push(e.mode);
        let mut f = 0u8;
        if e.active {
            f |= EF_ACTIVE;
        }
        if e.no_hooks {
            f |= EF_NO_HOOKS;
        }
        if e.codex {
            f |= EF_CODEX;
        }
        b.push(f);
        b.extend_from_slice(&e.age_s.to_le_bytes());
        let lbl = ascii_label(&e.label, LABEL_LEN);
        let mut raw = [0u8; LABEL_LEN];
        raw[..lbl.len()].copy_from_slice(lbl.as_bytes());
        b.extend_from_slice(&raw);
    }
    debug_assert_eq!(b.len(), SESSIONS_LEN);
    b
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub pct_5h: Option<u8>,
    pub pct_7d: Option<u8>,
    pub reset_5h: u32,
    pub reset_7d: u32,
    pub now: u32,
}

/// USAGE (§4.4) — o agente ainda nao sonda a API (M2); a codificacao ja segue o contrato.
#[allow(dead_code)]
pub fn encode_usage(u: &Usage) -> Vec<u8> {
    let mut b = Vec::with_capacity(15);
    b.push(PROTO_VERSION);
    b.push(u.pct_5h.map(|p| p.min(100)).unwrap_or(255));
    b.push(u.pct_7d.map(|p| p.min(100)).unwrap_or(255));
    b.extend_from_slice(&u.reset_5h.to_le_bytes());
    b.extend_from_slice(&u.reset_7d.to_le_bytes());
    b.extend_from_slice(&u.now.to_le_bytes());
    b
}

/// Configuracao enviada ao deck (§4.5, TLV).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeckConfig {
    pub brightness: Option<u8>,
    pub lang: Option<u8>,
    /// (rotulo, exige confirmacao) — rotulo ja sem o `!`.
    pub commands: Vec<(String, bool)>,
}

pub fn encode_config(c: &DeckConfig) -> Vec<u8> {
    let mut b = vec![PROTO_VERSION];
    if let Some(v) = c.brightness {
        b.extend_from_slice(&[1, 1, v]);
    }
    if let Some(v) = c.lang {
        b.extend_from_slice(&[2, 1, v.min(1)]);
    }
    if !c.commands.is_empty() {
        let n = c.commands.len().min(MAX_CUSTOM_COMMANDS);
        b.push(3);
        b.push((1 + LABEL_LEN * n) as u8);
        b.push(n as u8);
        for (label, confirm) in c.commands.iter().take(n) {
            // o `!` ocupa 1 dos 12 bytes quando exige confirmacao
            let body = ascii_label(label, if *confirm { LABEL_LEN - 1 } else { LABEL_LEN });
            let mut raw = [0u8; LABEL_LEN];
            let mut off = 0;
            if *confirm {
                raw[0] = b'!';
                off = 1;
            }
            raw[off..off + body.len()].copy_from_slice(body.as_bytes());
            b.extend_from_slice(&raw);
        }
    }
    b
}

/// Divide uma mensagem em frames com o cabecalho da §3 (`total<<4 | index`).
/// `max_frame` = bytes por pacote BLE **incluindo** o byte de cabecalho.
pub fn frame(msg: &[u8], max_frame: usize) -> Vec<Vec<u8>> {
    let per = max_frame.saturating_sub(1).max(1);
    let total = msg.len().div_ceil(per).clamp(1, 15);
    assert!(msg.len() <= 15 * per, "mensagem maior que 15 frames");
    let mut out = Vec::with_capacity(total);
    for i in 0..total {
        let start = i * per;
        let end = (start + per).min(msg.len());
        let mut f = Vec::with_capacity(1 + end - start);
        f.push(((total as u8) << 4) | (i as u8));
        f.extend_from_slice(&msg[start..end]);
        out.push(f);
    }
    out
}

/// Remonta frames (§3). `push` devolve a mensagem completa quando o ultimo frame chega.
#[derive(Debug, Default)]
pub struct Reassembler {
    buf: Vec<u8>,
    total: u8,
    next: u8,
}

impl Reassembler {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, f: &[u8]) -> Option<Vec<u8>> {
        if f.is_empty() {
            return None;
        }
        let total = f[0] >> 4;
        let idx = f[0] & 0x0F;
        if total == 0 || idx >= total {
            self.reset();
            return None;
        }
        if idx == 0 {
            self.buf.clear();
            self.total = total;
            self.next = 0;
        } else if total != self.total || idx != self.next {
            self.reset();
            return None;
        }
        self.buf.extend_from_slice(&f[1..]);
        self.next = idx + 1;
        if self.next == self.total {
            let msg = std::mem::take(&mut self.buf);
            self.reset();
            Some(msg)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.buf.clear();
        self.total = 0;
        self.next = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_vector_from_spec() {
        let mut v = SessionsView::default();
        v.active = Some(0);
        v.cells[0] = CellEntry {
            sid: 7,
            state: State::Working as u8,
            mode: Mode::Plan as u8,
            active: true,
            no_hooks: false,
            codex: false,
            age_s: 42,
            label: "deck".into(),
        };
        let b = encode_sessions(&v);
        assert_eq!(b.len(), SESSIONS_LEN);
        assert_eq!(&b[..4], &[0x01, 0x01, 0x08, 0x00]);
        let expect0 = [
            0x07, 0x02, 0x03, 0x01, 0x2A, 0x00, b'd', b'e', b'c', b'k', 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(&b[4..22], &expect0);
        assert!(b[22..].iter().all(|&x| x == 0));
        let frames = frame(&b, MAX_FRAME);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0][0], 0x10);
        assert_eq!(&frames[0][1..], &b[..]);
    }

    #[test]
    fn event_vectors_from_spec() {
        let tap = decode_event(&[0x01, 0x01, 0x02, 0x00]).unwrap();
        assert_eq!(tap, Event { kind: EventKind::CellTap, cell: 2, arg: 0 });
        let clear = decode_event(&[0x01, 0x04, 0xFF, 0x12]).unwrap();
        assert_eq!(clear.kind, EventKind::Action);
        assert_eq!(clear.cell, CELL_ACTIVE);
        assert_eq!(Action::from_u8(clear.arg), Some(Action::Clear));
        // com framing
        let mut r = Reassembler::new();
        assert_eq!(r.push(&[0x10, 0x01, 0x04, 0xFF, 0x12]).unwrap(), vec![0x01, 0x04, 0xFF, 0x12]);
        assert!(decode_event(&[0x02, 0x01, 0x00, 0x00]).is_err());
        assert!(decode_event(&[0x01, 0x09, 0x00, 0x00]).is_err());
    }

    #[test]
    fn framing_roundtrip_small_mtu() {
        let msg: Vec<u8> = (0..200u8).collect();
        let frames = frame(&msg, 20);
        assert_eq!(frames.len(), 11); // 200/19 = 10.5
        let mut r = Reassembler::new();
        let mut got = None;
        for f in &frames {
            assert!(f.len() <= 20);
            got = r.push(f);
        }
        assert_eq!(got.unwrap(), msg);
        // frame fora de ordem descarta
        let mut r = Reassembler::new();
        assert!(r.push(&frames[0]).is_none());
        assert!(r.push(&frames[2]).is_none());
        assert!(r.push(&frames[1]).is_none());
        // recomeca do zero
        let mut got = None;
        for f in &frames {
            got = r.push(f);
        }
        assert_eq!(got.unwrap(), msg);
    }

    #[test]
    fn usage_and_config_layout() {
        let u = encode_usage(&Usage { pct_5h: Some(72), pct_7d: None, reset_5h: 1, reset_7d: 2, now: 0x01020304 });
        assert_eq!(u.len(), 15);
        assert_eq!(&u[..3], &[1, 72, 255]);
        assert_eq!(&u[11..15], &[0x04, 0x03, 0x02, 0x01]);

        let c = encode_config(&DeckConfig {
            brightness: Some(200),
            lang: Some(1),
            commands: vec![("compact".into(), false), ("limpar tudo".into(), true)],
        });
        assert_eq!(&c[..7], &[1, 1, 1, 200, 2, 1, 1]);
        assert_eq!(c[7], 3);
        assert_eq!(c[8] as usize, 1 + 12 * 2);
        assert_eq!(c[9], 2);
        assert_eq!(&c[10..17], b"compact");
        assert_eq!(c[22], b'!');
        assert_eq!(&c[23..34], b"limpar tudo");
    }

    #[test]
    fn labels_are_ascii() {
        assert_eq!(ascii_label("projeto-configuração", 12), "projeto-conf");
        assert_eq!(ascii_label("São Paulo", 12), "Sao Paulo");
        assert_eq!(ascii_label("日本語-app", 12), "-app");
    }

    #[test]
    fn mode_mapping() {
        assert_eq!(Mode::from_hook("bypassPermissions"), Mode::Bypass);
        assert_eq!(Mode::from_hook("auto"), Mode::DontAsk);
        assert_eq!(Mode::from_hook("x"), Mode::Unknown);
    }
}
