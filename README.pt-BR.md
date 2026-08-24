[🇺🇸 English](README.md) · [🇧🇷 Português](README.pt-BR.md)

# Clow Deck — um stream deck físico para o Claude Code

Transforma a tela touch de 3,5" do Usage Stick (Guition JC4832W535, ESP32-S3) num
**painel de mesa para as suas sessões do Claude Code**: cada sessão aberta vira um botão com
estado ao vivo (trabalhando / **precisa de você** / terminou / ociosa), um toque traz a janela
certa pra frente e uma fileira de ações manda comandos para a sessão escolhida. A tela é um
periférico "burro" por **Bluetooth LE**; o cérebro é um **agente** pequeno rodando no computador.

> Status: **fatia vertical M1 no macOS** (firmware + agente + protocolo), conforme
> [`PLANO-CLAUDE-DECK.md`](PLANO-CLAUDE-DECK.md). A voz usa o `/voice` do próprio Claude Code
> (sem modelo de fala no agente); strip de uso (M2), app de configuração (M3), Windows (M4)
> e grade 3D (M5) estão esboçados, não prontos — ver *Limitações*.

## Como funciona

```
 sessão Claude Code ──hooks (curl → localhost:47831)──▶ ┌──────────────┐   BLE (GATT)   ┌──────────┐
 sessão Claude Code ──hooks────────────────────────────▶ │ clowdeck-    │ ─SESSIONS────▶ │ Clow     │
 processos `claude` ◀─ps + lsof (a cada 2 s)──────────── │ agent (Rust) │ ◀─EVENT (toque)│ Deck     │
 janela do terminal ◀─foco + teclas sintéticas────────── │              │                │ (ESP32)  │
 http://127.0.0.1:47831/  (deck virtual, mesma grade) ◀─ └──────────────┘                └──────────┘
```

1. **Hooks** — `clowdeck-agent hooks install` adiciona um hook `curl` por evento do Claude Code
   (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`,
   `Notification`, `Stop`, `PreCompact`, `SessionEnd`) em `~/.claude/settings.json`. Cada hook
   envia o JSON do stdin ao agente em menos de 1 s e nunca bloqueia o Claude (`-m 1 … || true`).
   O `$PPID` do shell é o PID do `claude` — é assim que o hook casa com o processo descoberto.
2. **Descoberta** — o agente lista os processos `claude` (`ps`) com o cwd (`lsof`) e sobe a
   cadeia de pais para saber qual app de terminal é o dono (VS Code, Terminal, iTerm2…).
   Sessões sem dados de hook aparecem como *unknown* (neutro).
3. **Deck** — o agente escreve as 8 células de sessão no deck por GATT a cada mudança
   (+ heartbeat de 3 s); o deck notifica toques/holds de volta. O pareamento usa um passkey
   de 6 dígitos mostrado **no deck** (bonding + MITM). Nenhum segredo fica no dispositivo.
4. **Foco / teclas** — um toque foca a janela da sessão (AppleScript por TTY no Terminal/iTerm2;
   `code <pasta>` + título da janela no VS Code) e a marca ativa no deck; as ações digitam
   `Shift+Tab`, `/compact`, `/clear` (confirmado no deck), `Esc`, `Enter`.

## Começando (macOS)

### 1. Firmware

Toolchain: arduino-cli 1.4.x · core `esp32:esp32` 3.3.11 · Arduino_GFX 1.6.5 · lvgl 9.2.2 ·
**NimBLE-Arduino 2.5.1** (`arduino-cli lib install NimBLE-Arduino`).

```bash
./flash.sh                         # autodetecta /dev/cu.usbmodem*, compila + grava
firmware/clow_deck/build.sh        # só compila
```

O deck inicia em **"procurando host"** (mascote Clow + nome/MAC BLE + heap livre).
Segurar o mascote abre os ajustes do deck (brilho, idioma, esquecer pareamento).

### 2. Agente

```bash
cd agent && cargo build --release            # Rust 1.87+
./target/release/clowdeck-agent doctor       # curl, lsof, osascript, Acessibilidade, Bluetooth, hooks
./target/release/clowdeck-agent hooks install   # escreve ~/.claude/settings.json (com backup)
./target/release/clowdeck-agent run          # descoberta + hooks + deck virtual + BLE
```

- Abra **http://127.0.0.1:47831/** — o *deck virtual* espelha o físico (mesma grade, mesmos
  estados, clique = tap, clique longo = menu da sessão). Funciona mesmo sem a placa.
- **Voz** = o `/voice` do Claude Code: uma vez por sessão ligue o modo de voz (digite `/voice`,
  ou use a entrada `voice` da lista de comandos do deck), depois **segure** o botão Voz da
  sessão no deck — o agente foca a sessão e segura a barra de espaço; solte para parar. O
  Claude Code transcreve (demora 2–3 s para começar a gravar) e deixa o texto no prompt —
  confira e aperte Enter (no deck ou no teclado). O idioma da ditado é configuração do
  Claude Code (`/config` → Dictation language); o PT/EN do deck não muda isso.
- **Permissões do macOS**: na primeira execução o sistema pede **Bluetooth** e **Acessibilidade**
  (os pedidos nomeiam o processo que roda o agente — `clowdeck-agent` quando instalado como
  serviço, senão o app de terminal que o lançou — clique em *Permitir* / ligue a chave), e
  **Automação** na primeira vez que uma janela é focada. Sem Acessibilidade a janela ainda vem
  pra frente, mas nenhuma tecla é enviada. `doctor` mostra o que falta; um binário não assinado
  recompilado pode exigir religar a chave de Acessibilidade.
- **Pareamento**: na primeira escrita autenticada o macOS abre o diálogo de pareamento e o
  deck mostra um passkey de 6 dígitos — digite uma vez; o bond fica guardado dos dois lados
  (o agente espera até 75 s por você).
- **Subir no login**: `clowdeck-agent service install` registra um LaunchAgent do launchd
  (KeepAlive: reinicia em 5 s se o agente cair ou o watchdog disparar). Logs:
  `~/Library/Logs/clowdeck/agent.log`. Rode de novo após recompilar; `service uninstall` remove.
  O `service install` também **assina o binário** com identificador fixo
  (`my.autom.clowdeck-agent`) usando sua identidade "Developer ID Application" ou "Apple
  Development" (sobrescreva com `sign_identity` no config.toml). Sem isso, a assinatura ad-hoc
  do linker muda a cada build e o macOS descarta as permissões de Bluetooth/Acessibilidade toda
  vez. Conceda **Bluetooth** e **Acessibilidade** ao `clowdeck-agent` uma vez após o primeiro
  install assinado e rode `service install` de novo (o macOS só aplica uma permissão nova de
  Acessibilidade a um processo reiniciado). Uma entrada sobrando de um build sem assinatura
  aparece ligada mas é ignorada — remova-a.

`clowdeck-agent hooks uninstall` remove só os hooks do deck e preserva quaisquer outros.

## A tela (320×480 retrato, grade 3×4 — a divisória 3D cai nos gaps de 4 px)

```
┌─────────┬─────────┬─────────┐
│ saup  ● │ deck  ✔ │ n8n   ⚠ │  linha 0: sessões 0–2  (pasta · chip de modo · idade ·
├─────────┼─────────┼─────────┤           mascote pixel esmaecido na cor do estado)
│ ios-app │   ---   │   ---   │  linha 1: sessões 3–5
├─────────┼─────────┼─────────┤
│ 🌐 PT   │ ☀ 80%   │ ⚙      │  linha 2: idioma · brilho · ajustes
├─────────┴─────────┴─────────┤
│ 3 sessoes · 1 precisa de vc ᗧ│  linha 3: faixa livre (status em duas sub-linhas)
└─────────────────────────────┘
```

Tema **Deep Space Glass** (`design/THEME.md`): gradiente quase preto, células de vidro escuro
translúcido com borda fina e fio de luz no topo, raios de 18 px, Montserrat. O mascote é
pixel-art original no estilo Space Invaders; os ícones de botão são glifos outline com
traço suavizado, no estilo dos icon packs de Stream Deck (`design/ICONS.md`,
`assets/pixel/` + `assets/icons_vec.py`, virando bitmaps A8 por `tools/gen_icons.py`).

| Estado | Significado | Visual |
|---|---|---|
| `WORKING` | Claude trabalhando | pulso ciano, mascote a 24 % |
| `ATTENTION` | pedido de permissão / pergunta / notificação | **âmbar piscando** |
| `DONE` | Claude terminou e espera você | verde (pisca lento nos primeiros 60 s) |
| `IDLE` | esperando prompt, reconhecida | vidro, mascote verde apagado |
| `UNKNOWN` | processo visto, sem dados de hook ainda | neutro |
| `ERROR` / `DEAD` | erro do agente / processo encerrou (célula libera em 5 s) | vermelho / cinza riscado |

Todas as telas mantêm a mesma grade: linhas 0–2 são células, linha 3 é faixa livre.
- **Home**: tap numa sessão = foca a janela + marca ativa (borda coral); hold = página da
  sessão. Linha 2: tap no idioma alterna PT/EN, tap no brilho cicla 3 níveis (hold em qualquer
  um → ajustes), engrenagem = ajustes. Faixa: status do link/sessões e uso 5 h quando o agente envia.
- **Página da sessão** (9 células): `<` voltar, focar, **voz** (segure para falar — o agente
  mantém a barra de espaço pressionada no modo `/voice` da sessão e solta quando você solta),
  modo (`Shift+Tab`), esc, enter, `/compact`, `/clear` (confirmado), tab (aceita a sugestao do terminal). Faixa: rótulo,
  estado, modo, idade + botão `cmd` (página de comandos, paginada com `>`).
- **Ajustes**: brilho −/+, idioma, esquecer pareamento (confirmado), sobre. **Procurando host**:
  o mascote anima enquanto o deck anuncia; o passkey de 6 dígitos aparece na tela quando o macOS pede.

## CLI e config do agente

```
clowdeck-agent run [--no-ble] [--dry-run] [--port N]
clowdeck-agent hooks install|uninstall|status [--settings PATH]
clowdeck-agent sessions            # célula · pid · tty · terminal · cwd · estado
clowdeck-agent focus <pid>
clowdeck-agent ble scan|info
clowdeck-agent doctor
clowdeck-agent service install|uninstall|status   # launchd do macOS
```

Config: `~/Library/Application Support/clowdeck/config.toml` (criada na primeira execução) —
porta, BLE ligado/desligado e tamanho de frame, brilho/idioma do deck e `[[commands]]`
(`label`, `text`, `confirm`) para a página CMD. `--dry-run` loga as teclas em vez de enviá-las —
útil para testar com sessões reais abertas.

## Estrutura do repositório

```
protocol/PROTOCOL.md    serviço GATT, framing, payloads — fonte da verdade (PROTO_VERSION 1)
firmware/clow_deck/     sketch ESP32-S3: UI LVGL 9.2 + GATT server NimBLE (grave com ./flash.sh)
app/                    app de menu-bar (M3, Tauri 2): embute o agente, menu na bandeja, janela do deck, DMG
firmware/claude_stick/  firmware do Usage Stick, mantido como base validada de display/touch (referência)
firmware/bringup/       bring-up puro de display/touch (referência)
agent/                  agente Rust: descoberta, servidor de hooks, modelo de sessões, foco, teclas, BLE, deck virtual
case-3d/                grade divisória 3D (M5, pendente)
assets/brand/           SVGs do Clawd/Claude Code → firmware/clow_deck/logo_assets.h
```

## Limitações (hoje)

- **Terminal integrado do VS Code**: o agente levanta a janela certa do VS Code e, se você rodou
  `clowdeck-agent keybinding install` (adiciona `ctrl+alt+cmd+t → workbench.action.terminal.focus`
  ao `keybindings.json` do VS Code — atalho que não alterna; Ctrl+` esconderia o painel), foca o
  painel de terminal. Não consegue escolher a aba de terminal — essa é sua. Terminal.app e iTerm2
  são focados pelo TTY exato.
- **Windows**: módulos de descoberta/foco são stubs (M4). **Voz** exige `/voice` ligado na
  sessão (uma vez por sessão) e segue o idioma de ditado do Claude Code, não o do deck.
  **Strip de uso**: protocolo e renderização no deck existem; o agente ainda não envia (M2).
- **Permissão BLE**: no macOS o pedido de Bluetooth pertence ao app que lançou o agente; um
  binário não assinado recompilado com frequência pode ser perguntado de novo.
- Atualização de firmware só por USB (sem OTA, por decisão).

Veja [`PLANO-CLAUDE-DECK.md`](PLANO-CLAUDE-DECK.md) para o roadmap e
[`protocol/PROTOCOL.md`](protocol/PROTOCOL.md) para o formato de fio.
