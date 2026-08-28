# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Clow Deck** — a physical "stream deck" for Claude Code on the Guition JC4832W535 (ESP32-S3 +
AXS15231B 480×320 QSPI touch display, the Usage Stick board). Three pieces, per
`PLANO-CLAUDE-DECK.md`:

- `firmware/clow_deck/` — arduino-cli sketch, LVGL 9.2 UI (4×3 grid) + **NimBLE GATT server**.
  Dumb peripheral: draws what it receives, notifies touches. **No secrets on the device.**
- `agent/` — Rust lib + CLI `clowdeck-agent` (tokio · btleplug · axum · enigo). Discovers
  `claude` processes, receives Claude Code hooks over localhost, keeps the session model,
  focuses windows / types keys, talks BLE to the deck, and serves a *virtual deck* web page.
  The runtime lives in `lib.rs` (`bind()`/`serve(st, listener, on_freeze)`); `main.rs` is the
  thin CLI.
- `app/` — **menu-bar app (M3, Tauri 2)** embedding the agent lib: tray menu (status, open
  virtual deck window, install hooks, start-at-login, quit). On launch it boots out the CLI
  LaunchAgent (same port) and takes over; "start at login" writes `my.autom.clowdeck.app.plist`
  (KeepAlive only on non-zero exit, so Quit sticks). Watchdog freeze → `AppHandle::restart()`.
  Build: `cd app && cargo tauri build` (needs `~/.cargo/bin` in PATH, rustc ≥ 1.88); signing +
  notarization in `docs/RELEASE-MACOS.md`.
- `protocol/PROTOCOL.md` — GATT service, framing and payloads. **Source of truth**; any byte
  change bumps `PROTO_VERSION` in both `firmware/clow_deck/config.h` and `agent/src/protocol.rs`.

`firmware/bringup/` is a bare display/touch sketch kept to validate new hardware — reference
only, not part of the deck build (it has its own `lv_conf.h`).

Code comments and on-device strings are Portuguese; UI text is bilingual via `TRS(pt, en)`.
Both READMEs (`README.md` English main, `README.pt-BR.md`) must stay in sync.

## Commands

```bash
./flash.sh [port]                       # autodetect /dev/cu.usbmodem*, compile + flash clow_deck
                                        # refuses to guess if 2+ boards are plugged in (pass the port)
firmware/clow_deck/build.sh             # compile only
firmware/clow_deck/build.sh upload <p>  # compile + flash
firmware/clow_deck/build.sh monitor <p> # serial @115200 (serial is unreliable on this board)
python3 tools/gen_icons.py              # assets/icons_vec.py (outline AA) + assets/pixel (mascote) -> icons.h

cd agent && cargo build --release && cargo test   # 14 unit tests (protocol vectors, model, hooks merge)
agent/target/release/clowdeck-agent run [--no-ble] [--dry-run]   # http://127.0.0.1:47831/
agent/target/release/clowdeck-agent hooks install|uninstall|status [--settings PATH]
agent/target/release/clowdeck-agent sessions | focus <pid> | ble scan|info | doctor
agent/target/release/clowdeck-agent service install|uninstall|status   # launchd LaunchAgent (KeepAlive, login)
agent/target/release/clowdeck-agent keybinding install|uninstall|status [--editor code|cursor|windsurf]
```

Firmware toolchain (pinned): arduino-cli 1.4.x · core `esp32:esp32` **3.3.11** · Arduino_GFX
**1.6.5** · lvgl **9.2.2** · **NimBLE-Arduino 2.5.1**. FQBN
`esp32:esp32:esp32s3:PSRAM=opi,FlashSize=16M,PartitionScheme=custom,CDCOnBoot=cdc,USBMode=hwcdc,FlashMode=qio`
(`PSRAM=opi` mandatory: the full-screen LVGL buffer lives in PSRAM). The board shows up as
`/dev/cu.usbmodem1101` on the dev Mac.

**The LVGL config reaches three compiler recipes.** `build.sh` injects
`-DLV_CONF_INCLUDE_SIMPLE -I<sketch>` into `compiler.c/cpp/S.extra_flags` — all three — because
`lv_conf_internal.h` is also pulled in while assembling lvgl's `.S` files. `lv_conf.h`'s
`#include <stdint.h>` must stay inside `#ifndef __ASSEMBLY__`.

Testing while developing: **never send real keystrokes with live sessions open** — use
`--dry-run`. Test hooks without touching `~/.claude/settings.json`: put the hooks in a scratch
project's `.claude/settings.json` and run `env -u CLAUDECODE claude -p "…" --model haiku` from
there (`CLAUDECODE` must be unset to start a nested session).

## Architecture

### Agent (`agent/src/`)
- `protocol.rs` — constants/enums exactly as PROTOCOL.md, `encode_sessions` (148 B),
  `encode_usage`, `encode_config` (TLV), `decode_event`/`decode_info`, `frame()`/`Reassembler`.
- `model.rs` — `SessionTable`: 8 **positional** cells; a session keeps its cell until it dies,
  new ones take the first free cell; `sid` 1..255, never 0. Hook → state: SessionStart→IDLE,
  UserPromptSubmit/PreToolUse/PostToolUse/PreCompact→WORKING, PermissionRequest→ATTENTION,
  Notification→ATTENTION (`idle_prompt` doesn't demote DONE/IDLE), Stop→DONE, SessionEnd→DEAD
  (freed after 5 s). Tap/FOCUS/ACK on DONE → IDLE.
- `discovery/` — macOS: `ps` + one `lsof -d cwd` call + parent chain → `terminal_app` + tty,
  every 2 s. **Windows (M4)**: `sysinfo` (name/cmdline → engine, `cwd`, parent chain →
  `terminal_app`); no tty, so hooks match by cwd. Labels split on `/` AND `\`.
  Windows install, BLE pairing rules and the **adapter requirement** (central role + LE
  Secure Connections; UB500 yes, RTL8821CU no) are in `docs/WINDOWS.md`.
- `hooks.rs` — axum `POST /hook/{event}?pid=$PPID&src=clowdeck`. `$PPID` inside the hook shell
  **is the `claude` PID** (hooks run as `sh -c` children of the session) — primary match key,
  `cwd` is the fallback. `install/uninstall/status` merge only the `hooks` key, recognise ours
  by the `src=clowdeck` marker, keep foreign hooks, write a `settings.json.bak-clowdeck-<ts>`.
- `focus/` — Terminal.app / iTerm2 by exact TTY (AppleScript). VS Code/Cursor/Windsurf: the
  folder of the window containing the cwd comes from the editor's own
  `~/Library/Application Support/<Code>/User/globalStorage/storage.json`
  (`windowsState.openedWindows[].folder`) — window titles via `CGWindowListCopyWindowInfo`
  are empty without the Screen Recording permission, and System Events sees 0 windows for
  Electron. The window is chosen with `code <folder>` and the app activated with **`open -a`**
  (LaunchServices, no TCC) — the CLI alone only bounces the Dock icon when another app is in
  front. Keys (`enigo`) need Accessibility granted to the process that runs the agent.
  Terminal-panel focus sends the chord from `focus.vscode_terminal_keys` (default
  `ctrl+alt+cmd+t`) **only if** `keybinding.rs` finds it bound to `workbench.action.terminal.focus`
  in the editor's `keybindings.json` (`clowdeck-agent keybinding install`). Never send Ctrl+`:
  it is *toggle* and hid the panel on every second tap. Cannot pick a terminal tab (documented).
- `inject.rs` — `Injector` trait: `DryRun` (logs) | `Enigo` (CGEvent/SendInput). Enigo is not
  `Send` on macOS → created inside `spawn_blocking` per action.
- `ble.rs` — btleplug central: scan by service UUID → read INFO (reject other PROTO_VERSION) →
  **write CONFIG with response** (the ATT "insufficient authentication" error is what makes
  CoreBluetooth pair/encrypt; a write-without-response on an unencrypted link is silently
  dropped by the deck and the Mac never learns it must pair — hence `write_with_response`
  defaults to true and the pairing write waits 75 s for the passkey) → subscribe EVENT →
  SESSIONS, heartbeat 3 s, reconnect backoff 1→5 s. Each BLE step stores a phase
  (`AppState::set_phase`) that the watchdog prints if the runtime freezes. **Every btleplug call goes through
  `with_timeout()`**: on CoreBluetooth a call against a peripheral that vanished never resolves
  (seen in practice: a write stuck for 19 h while the deck advertised). The heartbeat also
  checks `is_connected()`. `adapter()` never drops the `Manager::new()` future on timeout — a
  dropped one leaves an orphan CoreBluetooth thread that logs `Error dispatching event` forever;
  on macOS the Bluetooth TCC prompt belongs to the app that launched the agent, so it just waits.
- `codex.rs` — M6.a: embeds `codex app-server` (NDJSON stdio child), polls `thread/list`
  (MUST pass explicit `sourceKinds`), maps thread status/updatedAt to session states for
  processes the discovery classified as `Engine::Codex` (flags bit2 → deck chip "CDX").
  Threads loaded in another process read as `notLoaded` — study in `docs/CODEX-INTEGRATION.md`.
- `opencode.rs` — M7: live states for opencode sessions by polling its sqlite event log
  (`~/.local/share/opencode/opencode.db`, system `sqlite3 -json`, rowid offset); engine
  flag bit3 → chip "OC". Actions per engine in dispatch (mode=Tab, /new, /init, approve=Enter).
- `dispatch.rs` — EVENT semantics (§4.3): tap on an occupied cell = focus + active + DONE→IDLE;
  actions; DECK/HELLO → immediate push. **Voice is Claude Code's `/voice`**: `VOICE_START(cell)`
  = focus + press-and-hold the space bar (`KeyAction::SpaceDown`), `VOICE_STOP` = release
  (`SpaceUp`), `VOICE_CANCEL` = release + Esc. The hold lives in `AppState::voice_hold`; it is
  released automatically after 60 s and whenever the BLE session ends (`ble.rs` → 
  `dispatch::release_voice`). No speech model in the agent; dictation language is Claude's.
- `model.rs` capacity: `SessionTable::set_capacity(info.session_cells)` is called after
  reading INFO — the vertical 3×4 deck has 6 session cells, so cells 6–7 are never assigned
  (sessions wait in overflow); the SESSIONS payload still carries 8 entries (PROTO_VERSION 1).
- `web.rs` — `GET /` virtual deck, `GET /state`, `POST /event`, `GET /health`. `config.rs` —
  `~/Library/Application Support/clowdeck/config.toml` (port, ble, deck, `[[commands]]`,
  `auto_enter`). **`auto_enter`** (default true) decides whether typed commands end with
  Enter; `submit()` waits 280 ms before Enter (80 ms was swallowed by the macOS TUI while
  Windows sent it — the platforms behaved differently). Custom commands that are
  Claude-Code-only (`/voice`, `/config`, `/permissions`…) are REFUSED on codex/opencode:
  their TUIs fuzzy-match to another command (bench: `/voice` ran `/review` on opencode).
- TCC when running as a service: permissions (Bluetooth, Accessibility, Automation) belong to
  `clowdeck-agent` itself, not to the terminal that built it. `run` calls
  `AXIsProcessTrustedWithOptions(prompt)` once so the Accessibility entry appears; the VS Code
  Ctrl+` keystroke is non-fatal (focus still counts without it). **TCC keys grants on the code
  signature**: the linker's ad-hoc signature has a new cdhash (and identifier) per build, so every
  rebuild silently voided Bluetooth/Accessibility. `service install` therefore runs `codesign
  --identifier my.autom.clowdeck-agent` with the user's Apple Development / Developer ID identity
  (`sign_identity` in config.toml overrides auto-detect) — grants then survive rebuilds.
  An Accessibility entry created for an earlier (ad-hoc) build shows as enabled but is denied:
  remove it with "−" and let the agent re-prompt. `AXIsProcessTrusted` is cached per process —
  after granting, restart the agent (`service install`) or keys keep failing.
- `service.rs` — macOS LaunchAgent `my.autom.clowdeck` (RunAtLoad + KeepAlive, log in
  `~/Library/Logs/clowdeck/agent.log`). `main.rs` runs a **watchdog**: a tokio task pulses
  `liveness_ms` every 1 s and a plain std thread exits the process (code 70) if the pulse
  stops for 20 s — seen once: the whole runtime frozen for 15 min with no timer firing.
  launchd restarts it in 5 s. Re-run `service install` after rebuilding (plist points at the
  binary path).

### Firmware (`firmware/clow_deck/`)
- **Portrait 320×480, grid 3×4** (`DECK_COLS 3`, `DECK_ROWS 4`, `DECK_SESSION_CELLS 6`): rows 0–1
  = 6 session cells, row 2 = 3 utility/button cells, row 3 = free strip (no dividers) —
  **the same geometry on every screen** so the 3D divider fits (`design/THEME.md` §4;
  `cell_x/cell_y`, `STRIP_*`). The SESSIONS payload still carries 8 entries
  (`DECK_PROTO_SESSIONS`); cells 6–7 are parsed and ignored.
- **`DECK_ORIENTATION`** (config.h): `0` native portrait (USB top), `2` inverted portrait
  (USB bottom, default), `3` landscape (the Usage Stick pipeline, kept as reference — the UI is
  portrait-only). `put_px()` in the flush and `TOUCH_ROTATION` both derive from it; never
  change one without the other.
- **Theme "Deep Space Glass"** = `design/THEME.md` (tokens `C_*`, radii, state colours). `glass()`
  builds every surface (translucent bg over the screen gradient, 1 px edge, 2 px top shine);
  `over_glass()` mixes a state colour into the glass as a solid colour (cheap). Brand accent is
  the Anthropic coral `C_ACCENT` (#D97757); the palette follows the Claude brand.
- **Mascot is A8 pixel art; button icons are A8 outline with AA** (`icons.h`, GENERATED
  by `tools/gen_icons.py` from `assets/pixel/*.txt` + `assets/icons_vec.py`, 40/16 px;
  catalogue in `design/ICONS.md`). Colour at runtime with
  `lv_obj_set_style_image_recolor` (+`recolor_opa 255`), fade with `image_opa` (`icon()` helper).
  The mascot `clow_a/clow_b` frames animate on SEARCH; in session cells it sits faded behind
  the text, tinted by the state colour (`grid_anim`). `logo_assets.h` (Clawd) was removed.
- `clow_deck.ino` — display/touch pipeline, `request_state()/render_state()` (tears down,
  **zeroes every cached `lv_obj_t*`**, rebuilds), screens SEARCH / GRID / SESSION / CMD / SETTINGS /
  ABOUT, overlays (passkey, confirm) in `lv_layer_top()`, all animation procedural from `loop()`.
  SESSIONS updates the grid **in place**; only a screen change rebuilds. Session page = 9 cell
  buttons (back, focus, voice hold|approve, mode|deny, esc, enter, tab, /compact, `>` to the CMD page — which holds ONLY what page 1 doesn't: /clear|/new, /init (0x18) and the custom commands, paginated `<`/`>`); voice push-to-talk
  sends `VOICE_START/STOP` with the session's cell as target.
- **Render is `LV_DISPLAY_RENDER_MODE_PARTIAL`** (`DECK_RENDER_PARTIAL 1`): a 320×40 strip in
  internal RAM; the flush copies only the dirty area into the PSRAM canvas and calls
  `gfx->flush()` once per frame. FULL mode saturated `loop()` and dropped touch sampling.
- **Touch driver (`touch.h`) re-reads the controller while pressed** and uses the point count
  (`buf[1]`): INT-edge-only polling reported "released" for a still finger and holds never fired.
- `loop()` sends `DECK/STATS` (code 4, `arg` = avg ms per iteration) every 10 s — the agent logs
  "laco medio N ms"; measured 6 ms at rest after the redesign.
- `ble_link.{h,cpp}` — NimBLE server. **Callbacks run on the NimBLE host task**: they only copy
  raw frames into a ring under `portMUX` and set flags; `loop()` reassembles (§3) and touches
  LVGL; notifications are sent from `loop()` and are gated only on "connected" (the CCCD may be
  restored from the bond without an `onSubscribe`). Security: bonding + MITM + SC, IO cap
  DisplayOnly, random passkey per boot, `WRITE_ENC|WRITE_AUTHEN` on `SESSIONS/USAGE/CONFIG` when
  `DECK_BLE_SECURE` (default 1); `INFO` open; **`EVENT` deliberately has no ENC/AUTHEN flags**
  (NimBLEServer would send a Slave Security Request that macOS ignores → 30 s SM timeout →
  `0x16`). **Do not call `NimBLEDevice::setSecurityPasskey()`**: the library only invokes
  `onPassKeyDisplay()` when the static passkey is the default. The deck reports the last
  disconnect reason in the `arg` of `DECK/HELLO`.
- `deck_types.h` — protocol enums/structs and `enum State` in a header (the `.ino` auto-prototype
  bug below). Confirm/overlay buttons only set a request flag; `loop()` executes and deletes.
- NVS namespace `clowdeck`: `bri`, `lang` (range-validated on load). No LittleFS use.

## Hardware invariants (breaking these costs hours)

- **Canvas `rotation = 0` + manual pixel mapping in the flush** (`put_px()` per
  `DECK_ORIENTATION`: native copy, 180° or the Usage Stick's 270° CW). Rotating via
  `Arduino_Canvas` produces wrong colors; the canvas stays native 320×480.
- **Display and touch orientation move together**: `TOUCH_ROTATION` is defined as
  `DECK_ORIENTATION`. Change the define, not the individual pieces, or touch is mirrored.
- Pin map, tested lib versions, known-good bring-up: `firmware/REFERENCIA-HARDWARE-LVGL.md`,
  `firmware/bringup/`.
- Grid geometry (`DECK_COLS 3`/`DECK_ROWS 4`, 4 px gaps, rows 0–2 cells, row 3 free strip) is what
  the 3D divider (`case-3d/`) is modelled on — identical on every screen.

## LVGL 9.2 gotchas in this codebase

- **Embedded Montserrat fonts carry ASCII + `°` + `•` only.** No accents. Write UI strings
  unaccented; labels from the agent are transliterated to ASCII (12 chars).
- **`.ino` auto-generated prototypes break on custom types**: a function returning `MyStruct*`
  gets a prototype above the struct definition. Return an index, or keep types in a header
  (`deck_types.h`).
- **Overlays live in `lv_layer_top()`** and are cleaned explicitly in `render_state()`.
- **Animate procedurally from `loop()`, not with `lv_anim`**, for anything dismissible.
- Gestures: `LV_EVENT_SHORT_CLICKED` = tap, `LV_EVENT_LONG_PRESSED` (400 ms) = hold; a hold on a
  session cell rebuilds the screen, so no `CELL_RELEASE` follows for session cells.
- Small touch targets get `lv_obj_set_ext_click_area()`.
- `lv_image_dsc_t` uses positional init; ARGB8888 wants bytes ordered **B,G,R,A** (the old
  `logo_assets.h` did that and is gone). Everything in `icons.h` today is **A8** — generated,
  do not hand-edit; colour it at runtime with `lv_obj_set_style_image_recolor()`.

## Where to change what

- Wire format → `protocol/PROTOCOL.md` first, then `agent/src/protocol.rs` (+ tests) and
  `firmware/clow_deck/{deck_types.h,ble_link.cpp}`; bump `PROTO_VERSION` on both sides.
- Pins, BLE name/UUIDs, `DECK_BLE_SECURE`, grid constants, `FW_VERSION`, `DEV_NAME`/`DEV_EMAIL`
  (author credit on the ABOUT screen) → `firmware/clow_deck/config.h`.
- Palette (`C_*`), cell visuals per state, screen builders → top/middle of `clow_deck.ino`.
- Hook → state mapping → `agent/src/model.rs`; hook command string → `agent/src/hooks.rs`.
- Terminal support (new app) → `agent/src/discovery/macos.rs` (ancestor names) + `agent/src/focus/macos.rs`.
- Default commands for the CMD page → `agent/src/config.rs`.

## Repo conventions

- `.env`, `.mcp.json`, `.claude/` are not for git; `agent/target/` is gitignored.
- READMEs are bilingual and must not drift; a stale translation is worse than none.
- `case-3d/` holds the STL of the divider grid (M5); `PLANO-CLAUDE-DECK.md` is the roadmap.
