//! Semantica dos EVENTs do deck (§4.3) — vale igual p/ BLE e p/ o deck virtual web.

use crate::app::{Shared, VoiceHold};
use crate::model::Engine;
use crate::focus::focus_session_async;
use crate::inject::{self, Injector, KeyAction, SETTLE};
use crate::model::Session;
use crate::protocol::{Action, DeckCode, Event, EventKind, State, CELL_ACTIVE, CELL_CMD, CELL_MENU, CELL_MODE, CELL_VOICE, SESSION_CELLS};
use anyhow::{anyhow, Context, Result};

fn session_at(st: &Shared, cell: u8) -> Result<(usize, Session)> {
    let m = st.model();
    let idx = if cell == CELL_ACTIVE {
        m.active_cell().ok_or_else(|| anyhow!("nenhuma sessao ativa no deck"))?
    } else {
        cell as usize
    };
    if idx >= SESSION_CELLS {
        anyhow::bail!("celula {idx} nao e de sessao");
    }
    let s = m.cell(idx).ok_or_else(|| anyhow!("celula {idx} vazia"))?;
    if s.state == State::Dead {
        anyhow::bail!("sessao da celula {idx} ja terminou");
    }
    Ok((idx, s.clone()))
}

async fn focus_cell(st: &Shared, cell: u8) -> Result<(usize, String)> {
    let (idx, s) = session_at(st, cell)?;
    let r = focus_session_async(s, st.cfg.focus.clone(), st.dry_run).await?;
    st.model().focus_ack(idx);
    st.notify_changed();
    Ok((idx, r))
}

#[cfg(target_os = "macos")]
fn accessibility_ok() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}
#[cfg(not(target_os = "macos"))]
fn accessibility_ok() -> bool {
    true
}

#[cfg(target_os = "macos")]
fn accessibility_prompt_throttled() {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::CFString;
    use std::sync::atomic::{AtomicU64, Ordering};
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now = crate::app::now_ms();
    if now.saturating_sub(LAST.load(Ordering::Relaxed)) < 60_000 {
        return;
    }
    LAST.store(now, Ordering::Relaxed);
    let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
    let dict = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe {
        AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef());
    }
}
#[cfg(not(target_os = "macos"))]
fn accessibility_prompt_throttled() {}

async fn inject_after_focus<F>(st: &Shared, cell: u8, what: &str, f: F) -> Result<String>
where
    F: FnOnce(&mut dyn Injector) -> Result<()> + Send + 'static,
{
    if !st.dry_run && !accessibility_ok() {
        // Re-pede ao sistema (no maximo 1x/min): cria a entrada em Ajustes > Acessibilidade
        // para ESTA assinatura — util quando a entrada existente e de um build antigo
        // (aparece "ativa" mas o TCC nega) e o usuario acabou de remove-la.
        accessibility_prompt_throttled();
        anyhow::bail!(
            "{what}: sem permissao de Acessibilidade para `clowdeck-agent` (Ajustes > Privacidade e Seguranca > Acessibilidade: remova a entrada antiga com '-' e ligue a nova) — tecla nao enviada"
        );
    }
    let (idx, focused) = focus_cell(st, cell).await?;
    let dry = st.dry_run;
    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut inj = inject::make(dry)?;
        std::thread::sleep(SETTLE);
        f(&mut *inj)
    })
    .await??;
    Ok(format!("{what} -> celula {idx} ({focused})"))
}

/// Executa um evento. Devolve um resumo legivel (vai p/ log e p/ o deck virtual).
pub async fn handle_event(st: &Shared, ev: Event, source: &str) -> Result<String> {
    tracing::debug!("evento de {source}: {ev:?}");
    let res = match ev.kind {
        EventKind::CellTap => match ev.cell {
            c if (c as usize) < SESSION_CELLS => focus_cell(st, c).await.map(|(i, r)| format!("foco celula {i}: {r}")),
            CELL_VOICE => Ok("VOZ: agora fica na pagina da sessao (segure Voz)".into()),
            CELL_MODE => action(st, CELL_ACTIVE, Action::ModeCycle).await,
            CELL_CMD => Ok("CMD: pagina de comandos e local ao deck".into()),
            CELL_MENU => Ok("menu: local ao deck".into()),
            c => Err(anyhow!("celula {c} invalida")),
        },
        EventKind::CellHold => Ok(format!("hold na celula {} (pagina local do deck)", ev.cell)),
        EventKind::CellRelease => Ok(format!("release na celula {}", ev.cell)),
        EventKind::Action => match Action::from_u8(ev.arg) {
            Some(a) => action(st, ev.cell, a).await,
            None => Err(anyhow!("acao 0x{:02X} desconhecida", ev.arg)),
        },
        EventKind::Deck => match DeckCode::from_u8(ev.cell) {
            DeckCode::Hello => {
                st.notify_changed();
                Ok(format!("deck: HELLO (estado reenviado); ultima queda: {}", hci_reason(ev.arg)))
            }
            DeckCode::Bonded => Ok("deck: pareamento concluido".into()),
            DeckCode::Page => Ok(format!("deck: pagina {}", ev.arg)),
            DeckCode::Stats => Ok(format!("deck: laco medio {} ms", ev.arg)),
            DeckCode::Other(o) => Ok(format!("deck: codigo {o} ignorado")),
        },
    };
    match &res {
        Ok(r) => tracing::info!("{source}: {r}"),
        Err(e) => tracing::warn!("{source}: {ev:?} falhou: {e:#}"),
    }
    res
}

/// M6.b: para sessao Codex com thread conhecido, envia texto via `codex queue`
/// (sem focar janela); senao cai para o caminho de teclado.
async fn codex_send(st: &Shared, cell: u8, thread: Option<String>, text: &str, what: &str) -> Result<String> {
    if let Some(t) = thread {
        match crate::codex::queue_message(&t, text).await {
            Ok(()) => return Ok(format!("{what} via codex queue (sem focar; thread {t})")),
            Err(e) => tracing::warn!("codex queue falhou ({e:#}) — usando teclado"),
        }
    }
    let text = text.to_string();
    inject_after_focus(st, cell, what, move |i| i.submit(&text)).await
}

async fn action(st: &Shared, cell: u8, a: Action) -> Result<String> {
    let (engine, codex_thread) = session_at(st, cell)
        .map(|(_, s)| (Some(s.engine), s.codex_thread.clone()))
        .unwrap_or((None, None));
    let is_codex = engine == Some(Engine::Codex);
    let is_oc = engine == Some(Engine::Opencode);
    match a {
        Action::Focus => focus_cell(st, cell).await.map(|(i, r)| format!("foco celula {i}: {r}")),
        Action::ModeCycle => {
            if is_codex {
                anyhow::bail!("modo: indisponivel em sessao Codex (mude com /approvals no TUI)");
            } else if is_oc {
                // no opencode, Tab alterna o agente/modo (build <-> plan)
                inject_after_focus(st, cell, "Tab (modo do opencode)", |i| i.key(KeyAction::Tab)).await
            } else {
                inject_after_focus(st, cell, "Shift+Tab (modo)", |i| i.key(KeyAction::ShiftTab)).await
            }
        }
        Action::Compact => {
            if is_codex {
                codex_send(st, cell, codex_thread, "/compact", "/compact").await
            } else {
                inject_after_focus(st, cell, "/compact", |i| i.submit("/compact")).await
            }
        }
        Action::Clear => {
            if is_codex {
                // o equivalente do /clear no Codex e /new
                codex_send(st, cell, codex_thread, "/new", "/new").await
            } else if is_oc {
                inject_after_focus(st, cell, "/new", |i| i.submit("/new")).await
            } else {
                inject_after_focus(st, cell, "/clear", |i| i.submit("/clear")).await
            }
        }
        Action::Esc => inject_after_focus(st, cell, "Esc", |i| i.key(KeyAction::Esc)).await,
        Action::Enter => inject_after_focus(st, cell, "Enter", |i| i.key(KeyAction::Enter)).await,
        Action::Tab => inject_after_focus(st, cell, "Tab (aceitar sugestao)", |i| i.key(KeyAction::Tab)).await,
        Action::Approve => {
            // aprova o pedido pendente na TUI: codex 'y'; claude '1';
            // opencode: Enter confirma a opcao padrao (allow) do dialogo
            if is_oc {
                inject_after_focus(st, cell, "aprovar (Enter)", |i| i.key(KeyAction::Enter)).await
            } else {
                let key = if is_codex { "y" } else { "1" };
                inject_after_focus(st, cell, &format!("aprovar ('{key}')"), move |i| i.text(key)).await
            }
        }
        Action::Init => {
            // /init existe nos dois engines (claude: CLAUDE.md; codex: AGENTS.md)
            if is_codex {
                codex_send(st, cell, codex_thread, "/init", "/init").await
            } else {
                inject_after_focus(st, cell, "/init", |i| i.submit("/init")).await
            }
        }
        Action::Ack => {
            let (idx, _) = session_at(st, cell)?;
            st.model().ack(idx);
            st.notify_changed();
            Ok(format!("ack celula {idx} (DONE -> IDLE)"))
        }
        Action::VoiceStart => voice_start(st, cell).await,
        Action::VoiceStop => voice_stop(st, false).await,
        Action::VoiceCancel => voice_stop(st, true).await,
        Action::Custom(n) => {
            let cmd = st
                .cfg
                .commands
                .get(n as usize)
                .cloned()
                .ok_or_else(|| anyhow!("comando customizado {n} nao configurado ({})", st.config_path.display()))?;
            let text = cmd.text.clone();
            if is_codex {
                codex_send(st, cell, codex_thread, &text, &format!("custom '{}'", cmd.label)).await
            } else {
                inject_after_focus(st, cell, &format!("custom '{}'", cmd.label), move |i| i.submit(&text)).await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Voz = `/voice` do Claude Code: com o modo de voz ligado na sessao, SEGURAR a barra
// de espaco grava; soltar transcreve e deixa o texto no prompt esperando Enter.
// O deck e so o gatilho: hold no botao Voz -> VOICE_START, release -> VOICE_STOP.
// Sem whisper no agente. O idioma da ditado e do Claude Code (/config).
// ---------------------------------------------------------------------------

/// Tempo maximo com o espaco segurado: o deck pode sumir no meio (BLE) e a tecla
/// nao pode ficar presa na sessao do usuario.
const VOICE_MAX_HOLD: std::time::Duration = std::time::Duration::from_secs(60);

async fn press_key_blocking(st: &Shared, k: KeyAction) -> Result<()> {
    let dry = st.dry_run;
    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut inj = inject::make(dry)?;
        inj.key(k)
    })
    .await?
}

/// Thread que SEGURA o espaco: keydown inicial e, apos ~250 ms, keydowns a cada ~33 ms
/// (o auto-repeat que o SO faria com uma tecla fisica). Num TTY nao existe key-up: o
/// Claude Code so sabe que o espaco continua segurado pelos espacos repetidos — uma
/// tecla sintetica sem repeat era vista como um toque e a gravacao parava na hora.
/// A mesma thread solta a tecla quando `active` vira false.
fn spawn_space_hold(dry: bool, active: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<()> {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    std::thread::Builder::new()
        .name("voice-space".into())
        .spawn(move || {
            let mut inj = match inject::make(dry) {
                Ok(i) => i,
                Err(e) => {
                    tracing::error!("voz: injector: {e:#}");
                    active.store(false, Ordering::SeqCst);
                    return;
                }
            };
            if let Err(e) = inj.key(KeyAction::SpaceDown) {
                tracing::error!("voz: espaco: {e:#}");
                active.store(false, Ordering::SeqCst);
                return;
            }
            let mut next = Instant::now() + Duration::from_millis(250);
            let mut repeats: u32 = 0;
            while active.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(10));
                if Instant::now() >= next {
                    if !dry || repeats % 30 == 0 {
                        if let Err(e) = inj.key(KeyAction::SpaceDown) {
                            tracing::warn!("voz: repeat: {e:#}");
                        }
                    }
                    repeats += 1;
                    next += Duration::from_millis(33);
                }
            }
            let _ = inj.key(KeyAction::SpaceUp);
            tracing::debug!("voz: thread do espaco terminou apos {repeats} repeats");
        })
        .context("thread voice-space")?;
    Ok(())
}

/// Nome do processo do app de terminal como o LaunchServices reporta (p/ o caminho rapido)
fn terminal_front_name(app: Option<crate::model::TerminalApp>) -> &'static str {
    use crate::model::TerminalApp::*;
    match app {
        Some(VSCode) => "Code",
        Some(Cursor) => "Cursor",
        Some(Windsurf) => "Windsurf",
        Some(Terminal) => "Terminal",
        Some(ITerm2) => "iTerm2",
        Some(Warp) => "Warp",
        Some(Ghostty) => "Ghostty",
        Some(Alacritty) => "Alacritty",
        Some(Kitty) => "kitty",
        Some(WezTerm) => "WezTerm",
        _ => "",
    }
}

async fn voice_start(st: &Shared, cell: u8) -> Result<String> {
    let event_at = std::time::Instant::now(); // antes do lock: mede o hold real do usuario
    let _serial = st.voice_lock.lock().await;
    if !st.dry_run && !accessibility_ok() {
        accessibility_prompt_throttled();
        anyhow::bail!("voz: sem permissao de Acessibilidade para `clowdeck-agent` — espaco nao pressionado");
    }
    let prev: Option<VoiceHold> = st.voice_hold().clone(); // clone: o guard nao pode atravessar o await
    if let Some(h) = prev {
        // dois holds sem release (evento perdido?): solta o anterior antes
        tracing::warn!("voz: hold anterior na celula {} ainda ativo — soltando", h.cell);
        h.active.store(false, std::sync::atomic::Ordering::SeqCst);
        *st.voice_hold() = None;
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }
    // Caminho rapido: a sessao ja e a ativa e o app dela esta na frente -> nao refoca
    // (o foco completo custa ~1 s, que era descontado do tempo de gravacao).
    let (idx0, s0) = session_at(st, cell)?;
    if s0.engine != Engine::Claude {
        anyhow::bail!("voz: o /voice e do Claude Code — indisponivel em sessao {:?}", s0.engine);
    }
    let already_front = st.model().active_cell() == Some(idx0)
        && crate::focus::frontmost_app_name().as_deref() == Some(terminal_front_name(s0.terminal_app))
        && !terminal_front_name(s0.terminal_app).is_empty();
    let (idx, focused) = if already_front {
        (idx0, "foco rapido (app ja na frente)".to_string())
    } else {
        let r = focus_cell(st, cell).await?;
        tokio::time::sleep(SETTLE).await;
        r
    };
    let active = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    spawn_space_hold(st.dry_run, active.clone())?;
    let since = std::time::Instant::now();
    *st.voice_hold() = Some(VoiceHold { cell: idx, event_at, since, active: active.clone() });
    // rede de seguranca: solta sozinho depois de VOICE_MAX_HOLD se ninguem soltou
    let st2 = st.clone();
    tokio::spawn(async move {
        tokio::time::sleep(VOICE_MAX_HOLD).await;
        let _serial = st2.voice_lock.lock().await;
        let still = st2.voice_hold().as_ref().map(|h| h.since == since).unwrap_or(false);
        if still {
            tracing::warn!("voz: {} s segurado sem release — soltando espaco", VOICE_MAX_HOLD.as_secs());
            active.store(false, std::sync::atomic::Ordering::SeqCst);
            *st2.voice_hold() = None;
        }
    });
    Ok(format!("voz: gravando (espaco segurado c/ repeat) -> celula {idx} ({focused})"))
}

async fn voice_stop(st: &Shared, cancel: bool) -> Result<String> {
    let stop_at = std::time::Instant::now(); // antes do lock
    let _serial = st.voice_lock.lock().await;
    let cur: Option<VoiceHold> = st.voice_hold().take();
    let Some(h) = cur else {
        return Ok(format!("voz: {} sem hold ativo — ignorado", if cancel { "cancel" } else { "stop" }));
    };
    // Preserva a duracao real do hold: o usuario segurou de event_at ate stop_at, mas o
    // espaco so foi pressionado em `since` (depois do foco). Segura o que falta.
    if !cancel {
        let user_hold = stop_at.saturating_duration_since(h.event_at);
        let held = h.since.elapsed();
        if user_hold > held {
            let extra = (user_hold - held).min(std::time::Duration::from_secs(30));
            tracing::info!("voz: segurando mais {:.1} s p/ completar o hold do usuario", extra.as_secs_f32());
            tokio::time::sleep(extra).await;
        }
    }
    h.active.store(false, std::sync::atomic::Ordering::SeqCst); // a thread solta o espaco
    if cancel {
        tokio::time::sleep(SETTLE).await;
        press_key_blocking(st, KeyAction::Esc).await?;
        return Ok(format!("voz: cancelada (espaco solto + Esc) na celula {}", h.cell));
    }
    Ok(format!(
        "voz: espaco solto apos {:.1} s na celula {} — o Claude transcreve e espera Enter",
        h.since.elapsed().as_secs_f32(),
        h.cell
    ))
}

/// Solta a barra de espaco se houver hold de voz ativo (deck desconectou, timeout,
/// encerramento). Seguro chamar sem hold.
pub async fn release_voice(st: &Shared, reason: &str) -> Result<()> {
    let had: Option<VoiceHold> = st.voice_hold().take();
    if let Some(h) = had {
        tracing::warn!("voz: soltando espaco da celula {} ({reason})", h.cell);
        h.active.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    Ok(())
}

/// Motivo da ultima desconexao reportado pelo deck no arg do HELLO (PROTOCOL §4.3):
/// codigo HCI, ou 0x80|erro do host NimBLE; 0 = nenhuma desde o boot.
pub fn hci_reason(code: u8) -> String {
    let txt = match code {
        0x00 => "nenhuma desde o boot",
        0x05 => "falha de autenticacao",
        0x08 => "supervision timeout (link perdido)",
        0x13 => "terminada pelo Mac (remote user terminated)",
        0x14 => "terminada pelo Mac (low resources)",
        0x15 => "terminada pelo Mac (power off)",
        0x16 => "terminada pelo deck (local host)",
        0x1F => "erro nao especificado",
        0x22 => "LL response timeout",
        0x28 => "instant passed",
        0x3B => "parametros de conexao inaceitaveis",
        0x3D => "falha de MIC (chave de criptografia errada?)",
        0x3E => "conexao nao chegou a ser estabelecida",
        c if c & 0x80 != 0 => "erro do host NimBLE",
        _ => "desconhecido",
    };
    format!("0x{code:02X} {txt}")
}
