//! Teclas sinteticas na janela focada (macOS CGEvent / Windows SendInput via `enigo`).
//! `DryRun` so loga — e o unico injetor usado em testes.

use anyhow::{anyhow, Result};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    ShiftTab,
    /// Tab simples: aceita a sugestao/auto-complete do terminal
    Tab,
    Enter,
    Esc,
    /// barra de espaco PRESSIONADA (sem soltar): push-to-talk do `/voice` do Claude Code
    SpaceDown,
    /// barra de espaco solta: termina a gravacao, o Claude transcreve e espera Enter
    SpaceUp,
}

pub trait Injector {
    fn text(&mut self, s: &str) -> Result<()>;
    fn key(&mut self, k: KeyAction) -> Result<()>;
    /// Texto + Enter (comandos `/compact`, prompts prontos).
    fn submit(&mut self, s: &str) -> Result<()> {
        self.text(s)?;
        // 80 ms era pouco no macOS: o TUI ainda estava processando o texto e
        // engolia o Enter (no Windows chegava a tempo) — comportamento diferente
        // por SO, reportado na bancada.
        std::thread::sleep(Duration::from_millis(280));
        self.key(KeyAction::Enter)
    }
    /// Digita sem enviar (config `auto_enter = false`: o usuario revisa e da Enter).
    fn type_only(&mut self, s: &str) -> Result<()> {
        self.text(s)
    }
}

pub struct DryRun;

impl Injector for DryRun {
    fn text(&mut self, s: &str) -> Result<()> {
        tracing::info!("[dry-run] digitar: {s:?}");
        Ok(())
    }
    fn key(&mut self, k: KeyAction) -> Result<()> {
        tracing::info!("[dry-run] tecla: {k:?}");
        Ok(())
    }
}

pub struct Enigo {
    e: enigo::Enigo,
}

impl Enigo {
    pub fn new() -> Result<Enigo> {
        let e = enigo::Enigo::new(&enigo::Settings::default()).map_err(|e| anyhow!("enigo: {e:?}"))?;
        Ok(Enigo { e })
    }
}

impl Injector for Enigo {
    fn text(&mut self, s: &str) -> Result<()> {
        use enigo::Keyboard;
        self.e.text(s).map_err(|e| anyhow!("enigo text: {e:?}"))
    }
    fn key(&mut self, k: KeyAction) -> Result<()> {
        use enigo::{Direction, Key, Keyboard};
        let r = match k {
            KeyAction::ShiftTab => self
                .e
                .key(Key::Shift, Direction::Press)
                .and_then(|_| self.e.key(Key::Tab, Direction::Click))
                .and_then(|_| self.e.key(Key::Shift, Direction::Release)),
            KeyAction::Tab => self.e.key(Key::Tab, Direction::Click),
            KeyAction::Enter => self.e.key(Key::Return, Direction::Click),
            KeyAction::Esc => self.e.key(Key::Escape, Direction::Click),
            KeyAction::SpaceDown => self.e.key(Key::Space, Direction::Press),
            KeyAction::SpaceUp => self.e.key(Key::Space, Direction::Release),
        };
        r.map_err(|e| anyhow!("enigo key {k:?}: {e:?}"))
    }
}

/// Cria o injetor adequado. Com `dry_run` nunca toca no teclado.
pub fn make(dry_run: bool) -> Result<Box<dyn Injector>> {
    if dry_run {
        Ok(Box::new(DryRun))
    } else {
        Ok(Box::new(Enigo::new()?))
    }
}

/// Pausa entre focar a janela e digitar (o app precisa receber o foco).
pub const SETTLE: Duration = Duration::from_millis(120);
