//! BLE central (btleplug): acha o deck, valida INFO, assina EVENT, escreve
//! CONFIG/SESSIONS e reconecta sozinho. Uma conexao por vez.

use crate::app::Shared;
use crate::config::BleCfg;
use crate::dispatch;
use crate::protocol::{self, decode_event, decode_info, encode_config, encode_sessions, frame, DeckCode, EventKind, Info, Reassembler, CHAR_CONFIG, CHAR_EVENT, CHAR_INFO, CHAR_SESSIONS, DECK_NAME, PROTO_VERSION, SERVICE_UUID};
use anyhow::{anyhow, Context, Result};
use btleplug::api::{Central, CentralEvent, CharPropFlags, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::time::Duration;

pub const PERMISSION_HINT: &str = "no macOS o pedido de permissao de Bluetooth aparece para o app que lancou o agente (Terminal/iTerm/VS Code): clique em Permitir, ou libere em Ajustes > Privacidade e Seguranca > Bluetooth";

/// Obtem o adaptador. NUNCA descartar o future do `Manager::new()` por timeout:
/// no macOS ele cria uma thread CoreBluetooth que sobrevive ao drop e passa a
/// logar "Error dispatching event" a cada evento, para sempre (uma por tentativa).
/// Enquanto a permissao de Bluetooth nao e concedida, o mesmo future fica
/// esperando; so o aviso tem prazo (15 s).
pub async fn adapter(st: Option<&Shared>) -> Result<Adapter> {
    let mut task = tokio::spawn(async {
        let manager = Manager::new().await.context("CoreBluetooth/WinRT indisponivel")?;
        let mut adapters = manager.adapters().await?;
        if adapters.is_empty() {
            anyhow::bail!("nenhum adaptador Bluetooth (Bluetooth desligado ou sem permissao)");
        }
        Ok::<Adapter, anyhow::Error>(adapters.remove(0))
    });
    match tokio::time::timeout(Duration::from_secs(15), &mut task).await {
        Ok(r) => r.context("task do adaptador")?,
        Err(_) => {
            tracing::warn!("Bluetooth ainda nao respondeu (15 s) — {PERMISSION_HINT}");
            if let Some(st) = st {
                st.ble_status().hint = Some(PERMISSION_HINT.to_string());
            }
            let r = task.await.context("task do adaptador")?;
            if let Some(st) = st {
                st.ble_status().hint = None;
            }
            r
        }
    }
}

/// Toda chamada ao btleplug passa por aqui: se o periferico sumiu, o future do
/// CoreBluetooth pode nunca resolver (visto na pratica: write preso por horas).
async fn with_timeout<T, F>(secs: u64, what: &str, f: F) -> Result<T>
where
    F: std::future::Future<Output = btleplug::Result<T>>,
{
    match tokio::time::timeout(Duration::from_secs(secs), f).await {
        Ok(r) => r.with_context(|| what.to_string()),
        Err(_) => anyhow::bail!("{what}: sem resposta em {secs} s (deck sumiu?)"),
    }
}

pub struct Found {
    pub peripheral: Peripheral,
    pub name: String,
}

async fn is_deck(p: &Peripheral) -> Option<String> {
    let props = with_timeout(5, "properties", p.properties()).await.ok().flatten()?;
    let name = props.local_name.clone().unwrap_or_default();
    if props.services.contains(&SERVICE_UUID) || name == DECK_NAME {
        Some(if name.is_empty() { DECK_NAME.to_string() } else { name })
    } else {
        None
    }
}

/// Escaneia ate `timeout` procurando um deck (UUID do servico ou nome).
pub async fn find_deck(adapter: &Adapter, timeout: Duration) -> Result<Option<Found>> {
    with_timeout(10, "start_scan", adapter.start_scan(ScanFilter { services: vec![SERVICE_UUID] })).await?;
    let deadline = tokio::time::Instant::now() + timeout;
    let mut found = None;
    while tokio::time::Instant::now() < deadline {
        for p in with_timeout(10, "peripherals", adapter.peripherals()).await.unwrap_or_default() {
            if let Some(name) = is_deck(&p).await {
                found = Some(Found { peripheral: p, name });
                break;
            }
        }
        if found.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    let _ = with_timeout(5, "stop_scan", adapter.stop_scan()).await;
    Ok(found)
}

pub struct Chars {
    pub info: Characteristic,
    pub sessions: Characteristic,
    pub event: Characteristic,
    pub config: Option<Characteristic>,
    pub usage: Option<Characteristic>,
}

pub fn chars_of(p: &Peripheral) -> Result<Chars> {
    let all = p.characteristics();
    let find = |u| all.iter().find(|c| c.uuid == u).cloned();
    Ok(Chars {
        info: find(CHAR_INFO).ok_or_else(|| anyhow!("deck sem characteristic INFO"))?,
        sessions: find(CHAR_SESSIONS).ok_or_else(|| anyhow!("deck sem characteristic SESSIONS"))?,
        event: find(CHAR_EVENT).ok_or_else(|| anyhow!("deck sem characteristic EVENT"))?,
        config: find(CHAR_CONFIG),
        usage: find(protocol::CHAR_USAGE),
    })
}

/// `with_response=false` so vale p/ trafego de regime: um write-without-response num link
/// ainda nao cifrado e descartado pelo deck SEM erro, e o macOS nunca fica sabendo que
/// precisa cifrar (visto na bancada: 231 "escritas" e o deck em "aguardando sessoes").
pub async fn write_msg(p: &Peripheral, ch: &Characteristic, msg: &[u8], cfg: &BleCfg, with_response: bool, timeout_s: u64) -> Result<()> {
    for f in frame(msg, cfg.max_frame.clamp(8, 244)) {
        let wt = if !with_response && ch.properties.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE) {
            WriteType::WithoutResponse
        } else {
            WriteType::WithResponse
        };
        with_timeout(timeout_s, &format!("write {} ({} bytes)", short(ch.uuid), f.len()), p.write(ch, &f, wt)).await?;
    }
    Ok(())
}

fn short(u: uuid::Uuid) -> String {
    let s = u.to_string();
    s[..8].to_string()
}

fn looks_like_auth_error(e: &anyhow::Error) -> bool {
    let s = format!("{e:#}").to_ascii_lowercase();
    s.contains("authentic") || s.contains("encrypt") || s.contains("insufficient") || s.contains("pair")
}

pub async fn read_info(p: &Peripheral, ch: &Characteristic) -> Result<Info> {
    let raw = with_timeout(10, "read INFO", p.read(ch)).await?;
    let info = decode_info(&raw).map_err(|e| anyhow!("INFO invalida: {e}"))?;
    if info.proto != PROTO_VERSION {
        anyhow::bail!("deck fala protocolo v{} e o agente v{PROTO_VERSION} — atualize o firmware ou o agente", info.proto);
    }
    Ok(info)
}

async fn push_sessions(st: &Shared, p: &Peripheral, ch: &Characteristic) -> Result<()> {
    let view = st.model().view(false, false);
    write_msg(p, ch, &encode_sessions(&view), &st.cfg.ble, st.cfg.ble.write_with_response, 5).await?;
    st.ble_status().writes += 1;
    Ok(())
}

async fn push_config(st: &Shared, p: &Peripheral, chars: &Chars) -> Result<()> {
    if let Some(ch) = &chars.config {
        // sempre COM resposta: e o erro ATT "insufficient authentication" desta escrita
        // que faz o CoreBluetooth parear/cifrar; sem resposta nao ha erro nem cifra.
        // 75 s: se for a primeira vez, o macOS abre o dialogo do passkey e segura esta
        // escrita ate o usuario digitar; abandonar cedo (e reconectar) cancela o pareamento.
        write_msg(p, ch, &encode_config(&st.cfg.deck_config()), &st.cfg.ble, true, 75).await?;
        st.ble_status().writes += 1;
    }
    Ok(())
}

/// Uma conexao completa, do scan ate a queda. Erro = reconectar.
async fn session(st: &Shared, adapter: &Adapter) -> Result<()> {
    st.set_phase("scan");
    let Some(found) = find_deck(adapter, Duration::from_secs(15)).await? else {
        anyhow::bail!("deck nao encontrado (anunciando? ligado?)");
    };
    let p = found.peripheral;
    tracing::info!("deck encontrado: {} — conectando", found.name);
    st.set_phase("connect");
    with_timeout(20, "connect", p.connect()).await?;
    let result = match tokio::time::timeout(Duration::from_secs(150), setup(st, adapter, &p, &found.name)).await {
        Err(_) => Err(anyhow!("setup da conexao nao terminou em 150 s (fase {})", st.phase())),
        Ok(Err(e)) => Err(e),
        Ok(Ok(ctx)) => run_loop(st, &p, ctx).await,
    };
    st.set_phase("idle");
    let _ = with_timeout(5, "disconnect", p.disconnect()).await;
    {
        let mut b = st.ble_status();
        b.connected = false;
        b.device = None;
    }
    // hold de voz nao pode sobreviver a queda do deck (o release nunca chegaria)
    let _ = dispatch::release_voice(st, "deck desconectou").await;
    result
}

struct LoopCtx {
    chars: Chars,
    notifs: std::pin::Pin<Box<dyn futures::Stream<Item = btleplug::api::ValueNotification> + Send>>,
    events: std::pin::Pin<Box<dyn futures::Stream<Item = CentralEvent> + Send>>,
    my_id: btleplug::platform::PeripheralId,
}

/// Fase de setup: discover -> INFO -> CONFIG (pareamento) -> subscribe -> SESSIONS.
async fn setup(st: &Shared, adapter: &Adapter, p: &Peripheral, name: &str) -> Result<LoopCtx> {
    st.set_phase("discover");
    with_timeout(15, "discover_services", p.discover_services()).await?;
    let chars = chars_of(p)?;
    st.set_phase("info");
    let info = read_info(p, &chars.info).await?;
    tracing::info!(
        "deck {name}: fw {}.{} grade {}x{} ({} sessoes, rotulo {}) seguro={}",
        info.fw_major,
        info.fw_minor,
        info.cols,
        info.rows,
        info.session_cells,
        info.label_len,
        info.secure()
    );
    // o deck manda quantas celulas de sessao tem (6 no deck vertical 3x4): o modelo
    // so ocupa essas; o payload SESSIONS continua com 8 entradas (PROTO_VERSION 1)
    st.model().set_capacity(info.session_cells as usize);
    st.notify_changed();

    // A primeira operacao AUTENTICADA e uma escrita (CONFIG): e ela que dispara o
    // pareamento/cifra no macOS. So depois assinamos EVENT — se o subscribe chegasse
    // antes da cifra, o NimBLEServer mandaria um Security Request que o macOS ignora e
    // o timer SM de 30 s derrubaria a conexao (queda 0x16). Ver ble_link.cpp.
    st.set_phase("config(pareamento)");
    let mut attempt = 0;
    loop {
        match push_config(st, p, &chars).await {
            Ok(()) => break,
            Err(e) => {
                attempt += 1;
                let hint = if looks_like_auth_error(&e) || info.secure() {
                    "PAREAMENTO: digite no Mac o passkey de 6 digitos mostrado na tela do deck (dialogo do macOS)"
                } else {
                    "falha ao escrever CONFIG"
                };
                st.ble_status().hint = Some(hint.to_string());
                tracing::warn!("CONFIG/pareamento ({attempt}/3): {e:#} — {hint}");
                if attempt >= 3 {
                    return Err(e.context("nao foi possivel escrever CONFIG (pareamento?)"));
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
                if !with_timeout(5, "is_connected", p.is_connected()).await.unwrap_or(false) {
                    anyhow::bail!("desconectou durante o pareamento");
                }
            }
        }
    }
    {
        let mut b = st.ble_status();
        b.connected = true;
        b.device = Some(name.to_string());
        b.info = Some(info);
        b.last_error = None;
        b.hint = None;
    }
    tracing::info!("deck {name}: conectado e autenticado");

    let notifs = with_timeout(10, "notifications", p.notifications()).await?;
    let events = with_timeout(10, "adapter events", adapter.events()).await?;
    let my_id = p.id();

    st.set_phase("subscribe");
    let mut notifs = notifs;
    // Assina e espera o DECK/HELLO. Num reconecte de central pareada o macOS pode
    // restaurar o CCCD do bond sem escrever nada: o deck nao ve "subscribe" e nunca
    // notifica (visto na bancada: 0 eventos em 200 s com a conexao viva). Se o HELLO nao
    // vier em 4 s, desassina e assina de novo — isso forca a escrita do CCCD.
    let mut got_hello = false;
    for round in 0..3 {
        if round > 0 {
            tracing::warn!("sem HELLO apos assinar EVENT — re-assinando ({round}/2)");
            let _ = with_timeout(10, "unsubscribe EVENT", p.unsubscribe(&chars.event)).await;
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
        with_timeout(10, "subscribe EVENT", p.subscribe(&chars.event)).await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout_at(deadline, notifs.next()).await {
                Ok(Some(n)) if n.uuid == CHAR_EVENT => {
                    let st2 = st.clone();
                    let mut reasm = Reassembler::new();
                    if let Some(msg) = reasm.push(&n.value) {
                        if let Ok(ev) = decode_event(&msg) {
                            st.ble_status().events += 1;
                            if ev.kind == EventKind::Deck && DeckCode::from_u8(ev.cell) == DeckCode::Hello {
                                got_hello = true;
                            }
                            tokio::spawn(async move { let _ = dispatch::handle_event(&st2, ev, "deck").await; });
                        }
                    }
                    if got_hello { break; }
                }
                Ok(Some(_)) => {}
                Ok(None) => anyhow::bail!("stream de notificacoes terminou durante o subscribe"),
                Err(_) => break,
            }
        }
        if got_hello { break; }
    }
    if !got_hello {
        tracing::warn!("deck nao mandou HELLO — seguindo mesmo assim (toques podem nao chegar)");
    }
    st.set_phase("sessions");
    push_sessions(st, p, &chars.sessions).await?;
    st.set_phase("loop");
    Ok(LoopCtx { chars, notifs, events, my_id })
}

/// Regime: notificacoes, mudancas do modelo, heartbeat, eventos do adaptador.
async fn run_loop(st: &Shared, p: &Peripheral, ctx: LoopCtx) -> Result<()> {
    let LoopCtx { chars, mut notifs, mut events, my_id } = ctx;

    let mut rx = st.changed.subscribe();
    rx.mark_unchanged();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(st.cfg.ble.heartbeat_s.max(1)));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    heartbeat.tick().await; // o primeiro dispara na hora
    let mut reasm = Reassembler::new();

    loop {
        tokio::select! {
            n = notifs.next() => {
                let Some(n) = n else { anyhow::bail!("stream de notificacoes terminou (desconectado)") };
                if n.uuid != CHAR_EVENT { continue; }
                let Some(msg) = reasm.push(&n.value) else { continue };
                match decode_event(&msg) {
                    Ok(ev) => {
                        st.ble_status().events += 1;
                        if ev.kind == EventKind::Deck && DeckCode::from_u8(ev.cell) == DeckCode::Hello {
                            push_config(st, p, &chars).await?;
                            push_sessions(st, p, &chars.sessions).await?;
                        }
                        let st2 = st.clone();
                        tokio::spawn(async move { let _ = dispatch::handle_event(&st2, ev, "deck").await; });
                    }
                    Err(e) => tracing::warn!("EVENT invalido {:02X?}: {e}", msg),
                }
            }
            r = rx.changed() => {
                if r.is_err() { anyhow::bail!("canal de mudancas fechado"); }
                push_sessions(st, p, &chars.sessions).await?;
            }
            _ = heartbeat.tick() => {
                if !with_timeout(5, "is_connected", p.is_connected()).await.unwrap_or(false) {
                    anyhow::bail!("deck desconectou (is_connected=false)");
                }
                push_sessions(st, p, &chars.sessions).await?;
            }
            e = events.next() => {
                let Some(e) = e else { anyhow::bail!("stream de eventos do adaptador terminou") };
                if let CentralEvent::DeviceDisconnected(id) = e {
                    if id == my_id { anyhow::bail!("deck desconectou"); }
                }
            }
        }
    }
}

/// Loop principal do BLE: reconecta com backoff 1 s → 5 s.
pub async fn run_ble(st: Shared) {
    st.set_phase("adaptador");
    let adapter = loop {
        match adapter(Some(&st)).await {
            Ok(a) => break a,
            Err(e) => {
                tracing::error!("BLE: {e:#} — tentando de novo em 10 s");
                st.ble_status().hint = Some(PERMISSION_HINT.to_string());
                st.ble_status().last_error = Some(format!("{e:#}"));
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    };
    let mut backoff = 1u64;
    loop {
        match session(&st, &adapter).await {
            Ok(()) => backoff = 1,
            Err(e) => {
                tracing::warn!("BLE: {e:#} — nova tentativa em {backoff} s");
                st.ble_status().last_error = Some(format!("{e:#}"));
            }
        }
        tokio::time::sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(5);
    }
}

/// `ble scan`: lista tudo que anuncia por alguns segundos, marcando decks.
pub async fn scan(secs: u64) -> Result<()> {
    let adapter = adapter(None).await?;
    println!("adaptador: {}", adapter.adapter_info().await.unwrap_or_else(|_| "?".into()));
    adapter.start_scan(ScanFilter::default()).await.context("start_scan")?;
    tokio::time::sleep(Duration::from_secs(secs)).await;
    let mut decks = 0;
    for p in adapter.peripherals().await? {
        let Some(props) = p.properties().await? else { continue };
        let name = props.local_name.clone().unwrap_or_default();
        let deck = props.services.contains(&SERVICE_UUID) || name == DECK_NAME;
        if deck {
            decks += 1;
        }
        if deck || !name.is_empty() {
            println!(
                "{} {:<24} rssi={:>4} servicos={} {}",
                if deck { "★" } else { " " },
                if name.is_empty() { "(sem nome)".to_string() } else { name },
                props.rssi.map(|r| r.to_string()).unwrap_or_else(|| "?".into()),
                props.services.len(),
                props.address
            );
        }
    }
    let _ = adapter.stop_scan().await;
    println!("{decks} deck(s) Clow encontrado(s)");
    Ok(())
}

/// `ble info`: conecta, le INFO e desconecta.
pub async fn info() -> Result<()> {
    let adapter = adapter(None).await?;
    let Some(found) = find_deck(&adapter, Duration::from_secs(10)).await? else {
        anyhow::bail!("nenhum deck anunciando");
    };
    let p = found.peripheral;
    with_timeout(20, "connect", p.connect()).await?;
    let r: Result<()> = async {
        with_timeout(15, "discover_services", p.discover_services()).await?;
        let chars = chars_of(&p)?;
        let info = read_info(&p, &chars.info).await?;
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "name": found.name, "info": info, "secure": info.secure(),
            "has_config": chars.config.is_some(), "has_usage": chars.usage.is_some(),
        }))?);
        Ok(())
    }
    .await;
    let _ = p.disconnect().await;
    r
}
