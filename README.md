[🇺🇸 English](README.md) · [🇧🇷 Português](README.pt-BR.md)

# Clow Deck — a physical stream deck for Claude Code

Turns the 3.5" touch screen of the Usage Stick (Guition JC4832W535, ESP32-S3) into a
**desk panel for your Claude Code sessions**: every open session is a button with a live
state (working / **needs you** / done / idle), a tap brings the right window to the front,
and a row of actions sends commands to the chosen session. The screen is a dumb peripheral
over **Bluetooth LE**; the brain is a small **agent** running on your computer.

> Status: **M1 vertical slice on macOS** (firmware + agent + protocol), per
> [`PLANO-CLAUDE-DECK.md`](PLANO-CLAUDE-DECK.md). Voice works through Claude Code's own
> `/voice` (no speech model in the agent); usage strip (M2), config app (M3), Windows (M4)
> and the 3D grid (M5) are scaffolded but not done — see *Limitations*.

## How it works

```
 Claude Code session ──hooks (curl → localhost:47831)──▶ ┌──────────────┐   BLE (GATT)   ┌──────────┐
 Claude Code session ──hooks────────────────────────────▶ │ clowdeck-    │ ─SESSIONS────▶ │ Clow     │
 `claude` processes  ◀─ps + lsof (every 2 s)───────────── │ agent (Rust) │ ◀─EVENT (tap)─ │ Deck     │
 terminal window     ◀─focus + synthetic keys──────────── │              │                │ (ESP32)  │
 http://127.0.0.1:47831/  (virtual deck, same grid)  ◀──  └──────────────┘                └──────────┘
```

1. **Hooks** — `clowdeck-agent hooks install` adds one `curl` hook per Claude Code event
   (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`,
   `Notification`, `Stop`, `PreCompact`, `SessionEnd`) to `~/.claude/settings.json`. Each
   hook posts its stdin JSON to the agent in under a second and never blocks Claude
   (`-m 1 … || true`). The shell's `$PPID` is the `claude` PID, which is how a hook is
   matched to the process the agent discovered.
2. **Discovery** — the agent lists `claude` processes (`ps`) with their cwd (`lsof`) and
   walks the parent chain to learn which terminal app owns them (VS Code, Terminal, iTerm2…).
   Sessions without hook data show as *unknown* (neutral).
3. **Deck** — the agent writes the 8 session cells to the deck over GATT on every change
   (+ a 3 s heartbeat); the deck notifies taps/holds back. Pairing uses a 6-digit passkey
   shown **on the deck** (bonding + MITM). Nothing secret ever lives on the device.
4. **Focus / keys** — a tap focuses the session's window (AppleScript for Terminal/iTerm2 by
   TTY; `code <folder>` + window title for VS Code) and marks it active on the deck;
   actions type `Shift+Tab`, `/compact`, `/clear` (confirmed on the deck), `Esc`, `Enter`.

## Quick start (macOS)

### 1. Firmware

Toolchain: arduino-cli 1.4.x · core `esp32:esp32` 3.3.11 · Arduino_GFX 1.6.5 · lvgl 9.2.2 ·
**NimBLE-Arduino 2.5.1** (`arduino-cli lib install NimBLE-Arduino`).

```bash
./flash.sh                         # autodetects /dev/cu.usbmodem*, compiles + flashes
firmware/clow_deck/build.sh        # compile only
```

The deck boots into **"searching host"** (Clow mascot + BLE name/MAC + free heap).
Holding the mascot opens the deck settings (brightness, language, forget pairing).

### 2. Agent

```bash
cd agent && cargo build --release            # Rust 1.87+
./target/release/clowdeck-agent doctor       # curl, lsof, osascript, Accessibility, Bluetooth, hooks
./target/release/clowdeck-agent hooks install   # writes ~/.claude/settings.json (backup kept)
./target/release/clowdeck-agent run          # discovery + hooks + virtual deck + BLE
```

- Open **http://127.0.0.1:47831/** — the *virtual deck* mirrors the physical one (same grid,
  same states, click = tap, long-press = session menu). It works even without the board.
- **Voice** = Claude Code's `/voice`: once per session turn voice mode on (type `/voice`, or
  the `voice` entry in the deck's command list), then **hold** the session's Voz button on the
  deck — the agent focuses the session and holds the space bar; release to stop. Claude Code
  transcribes (it takes 2–3 s to start recording) and leaves the text in the prompt — check it
  and press Enter (on the deck or the keyboard). Dictation language is Claude Code's own
  setting (`/config` → Dictation language); the deck's PT/EN does not change it.
- **macOS permissions**: the first run asks for **Bluetooth** and **Accessibility** (the
  prompts name the process that runs the agent — `clowdeck-agent` when installed as a service,
  otherwise the terminal app that launched it — click *Allow* / enable the toggle), and
  **Automation** the first time a window is focused. Without Accessibility the window is still
  raised but no keys are sent. `doctor` reports what is missing; an unsigned binary that is
  rebuilt may need the Accessibility toggle flipped again.
- **Pairing**: on the first authenticated write macOS opens a pairing dialog and the deck shows
  a 6-digit passkey — type it once; the bond is remembered on both sides (the agent waits up
  to 75 s for you).
- **Run at login**: `clowdeck-agent service install` registers a launchd LaunchAgent
  (KeepAlive: restarts in 5 s if the agent dies or its watchdog trips). Logs:
  `~/Library/Logs/clowdeck/agent.log`. Re-run it after rebuilding; `service uninstall` removes it.
  `service install` also **code-signs the binary** with a stable identifier
  (`my.autom.clowdeck-agent`) using your "Developer ID Application" or "Apple Development"
  identity (override with `sign_identity` in config.toml). Without that, the linker's ad-hoc
  signature changes on every build and macOS drops the Bluetooth/Accessibility grants each time.
  Grant **Bluetooth** and **Accessibility** to `clowdeck-agent` once after the first signed install,
  then run `service install` again (macOS only applies a new Accessibility grant to a restarted
  process). An entry left over from an unsigned build shows as enabled but is ignored — remove it.

`clowdeck-agent hooks uninstall` removes only the deck's hooks and leaves any other hooks intact.

## The screen (320×480 portrait, 3×4 grid — the 3D divider sits on the 4 px gaps)

```
┌─────────┬─────────┬─────────┐
│ saup  ● │ deck  ✔ │ n8n   ⚠ │  row 0: sessions 0–2  (folder · mode chip · age ·
├─────────┼─────────┼─────────┤           faded pixel mascot tinted by state)
│ ios-app │   ---   │   ---   │  row 1: sessions 3–5
├─────────┼─────────┼─────────┤
│ 🌐 PT   │ ☀ 80%   │ ⚙      │  row 2: language · brightness · settings
├─────────┴─────────┴─────────┤
│ 3 sessions · 1 needs you  ᗧ │  row 3: free strip (status, two text lines)
└─────────────────────────────┘
```

Theme **Deep Space Glass** (`design/THEME.md`): near-black gradient, translucent dark-glass
cells with a hairline edge and a top shine, 18 px radii, Montserrat. The mascot is
original Space-Invaders-style pixel art; button icons are outline glyphs with
anti-aliased strokes, Stream-Deck-icon-pack style (`design/ICONS.md`,
`assets/pixel/` + `assets/icons_vec.py`, rendered as A8 bitmaps by
`tools/gen_icons.py`).

| State | Meaning | Look |
|---|---|---|
| `WORKING` | Claude is working | cyan pulse, mascot 24 % |
| `ATTENTION` | permission prompt / question / notification | **amber, blinking** |
| `DONE` | Claude finished and is waiting for you | green (slow blink for the first 60 s) |
| `IDLE` | waiting for a prompt, acknowledged | glass, dim green mascot |
| `UNKNOWN` | process seen, no hook data yet | neutral |
| `ERROR` / `DEAD` | agent error / process gone (cell frees after 5 s) | red / grey, struck through |

Every screen keeps the same grid: rows 0–2 are cells, row 3 is a free strip.
- **Home**: tap a session = focus its window + mark active (coral border); hold = session
  page. Row 2: tap language toggles PT/EN, tap brightness cycles 3 levels (hold either →
  settings), gear = settings. Strip: link/session status and 5 h usage when the agent sends it.
- **Session page** (9 cells): `<` back, focus, **voice** (hold to talk — the agent holds
  Space in the session's `/voice` mode and releases it when you let go), mode (`Shift+Tab`),
  esc, enter, `/compact`, `/clear` (confirmed), tab (accept the terminal's suggestion). Strip: label, state, mode, age + a `cmd`
  button (commands page, paginated with `>`).
- **Settings**: brightness −/+, language, forget pairing (confirmed), about. **Searching host**:
  the mascot animates while the deck advertises; the 6-digit passkey shows on screen when
  macOS asks for it.

## Agent CLI and config

```
clowdeck-agent run [--no-ble] [--dry-run] [--port N]
clowdeck-agent hooks install|uninstall|status [--settings PATH]
clowdeck-agent sessions            # cell · pid · tty · terminal · cwd · state
clowdeck-agent focus <pid>
clowdeck-agent ble scan|info
clowdeck-agent doctor
clowdeck-agent service install|uninstall|status   # macOS launchd
```

Config: `~/Library/Application Support/clowdeck/config.toml` (created on first run) —
port, BLE on/off and frame size, deck brightness/language, and `[[commands]]`
(`label`, `text`, `confirm`) for the CMD page. `--dry-run` logs keystrokes instead of
sending them — useful while testing with real sessions open.

## Repository layout

```
protocol/PROTOCOL.md    GATT service, framing, payloads — the source of truth (PROTO_VERSION 1)
firmware/clow_deck/     ESP32-S3 sketch: LVGL 9.2 UI + NimBLE GATT server (flash with ./flash.sh)
app/                    menu-bar app (M3, Tauri 2): embeds the agent, tray menu, deck window, DMG
firmware/claude_stick/  Usage Stick firmware, kept as the validated display/touch base (reference)
firmware/bringup/       bare display/touch bring-up (reference)
agent/                  Rust agent: discovery, hooks server, session model, focus, keys, BLE, virtual deck
case-3d/                3D divider grid (M5, pending)
assets/brand/           Clawd/Claude Code SVGs → firmware/clow_deck/logo_assets.h
```

## Limitations (today)

- **VS Code integrated terminal**: the agent raises the right VS Code window and, if you ran
  `clowdeck-agent keybinding install` (adds `ctrl+alt+cmd+t → workbench.action.terminal.focus`
  to VS Code's `keybindings.json` — a non-toggling shortcut; Ctrl+` would hide the panel), focuses
  the terminal panel. It cannot pick a specific terminal tab — that is yours. Terminal.app and
  iTerm2 are focused by exact TTY.
- **Windows**: discovery/focus modules are stubs (M4). **Voice** needs `/voice` enabled in the
  session (once per session) and follows Claude Code's dictation language, not the deck's.
  **Usage strip**: protocol and deck rendering exist, the agent does not send it yet (M2).
- **BLE permission**: on macOS the Bluetooth prompt belongs to the app that launched the agent;
  an unsigned binary rebuilt often may be asked again.
- Firmware updates are over USB only (no OTA by design).

See [`PLANO-CLAUDE-DECK.md`](PLANO-CLAUDE-DECK.md) for the roadmap and
[`protocol/PROTOCOL.md`](protocol/PROTOCOL.md) for the wire format.
