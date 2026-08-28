# Building from source

Everything the release binaries do, you can do from this repository. This page is for people
who cloned the repo; if you just want to use the deck, the
[README](README.md) is enough.

*This document is English-only.*

---

## Contents

- [Repository layout](#repository-layout)
- [Firmware](#firmware)
- [Agent](#agent)
- [Menu-bar app](#menu-bar-app)
- [Agent CLI](#agent-cli)
- [Configuration](#configuration)
- [Regenerating assets](#regenerating-assets)
- [Platform notes](#platform-notes)

---

## Repository layout

```
protocol/PROTOCOL.md    GATT service, framing, payloads — the source of truth (PROTO_VERSION 1)
firmware/clow_deck/     ESP32-S3 sketch: LVGL 9.2 UI + NimBLE GATT server (flash with ./flash.sh)
firmware/bringup/       bare display/touch bring-up, to validate new hardware (reference)
agent/                  Rust agent: discovery, hooks server, session model, focus, keys, BLE, web UI
app/                    menu-bar app (Tauri 2): embeds the agent, tray menu, deck window, DMG/EXE
case-3d/                3D divider grid
assets/                 icon sources: vector spec, pixel-art mascot, engine logos → icons.h
tools/                  gen_icons.py, gen_mockups.py, make_dmg.sh
docs/WINDOWS.md         installing, pairing and the Bluetooth adapter requirement on Windows
```

---

## Firmware

**Board**: Guition JC4832W535 — ESP32-S3 with an AXS15231B 480×320 QSPI touch panel, used in
**portrait**. Pin map and the LVGL flush pipeline are in
[`firmware/REFERENCIA-HARDWARE-LVGL.md`](firmware/REFERENCIA-HARDWARE-LVGL.md).

**Pinned toolchain** — other versions are not tested:

| Component | Version |
|---|---|
| arduino-cli | 1.4.x |
| core `esp32:esp32` | 3.3.11 |
| Arduino_GFX | 1.6.5 |
| lvgl | 9.2.2 |
| NimBLE-Arduino | 2.5.1 |

```bash
arduino-cli core install esp32:esp32@3.3.11
arduino-cli lib install "GFX Library for Arduino"@1.6.5 lvgl@9.2.2 NimBLE-Arduino@2.5.1
```

Build and flash:

```bash
./flash.sh                          # autodetects the port, compiles and flashes
./flash.sh /dev/cu.usbmodem2101     # explicit port
firmware/clow_deck/build.sh         # compile only
firmware/clow_deck/build.sh monitor /dev/cu.usbmodem2101   # serial @115200
```

> `flash.sh` **refuses to guess** when two or more boards are plugged in — pass the port. On
> macOS you can tell them apart by USB serial number (the ESP32-S3 reports its base MAC):
> `ioreg -p IOUSB -l -w0 | grep -B12 Espressif | grep "USB Serial Number"`.

The FQBN matters. `PSRAM=opi` is **mandatory** — the full-screen LVGL canvas lives in PSRAM:

```
esp32:esp32:esp32s3:PSRAM=opi,FlashSize=16M,PartitionScheme=custom,CDCOnBoot=cdc,USBMode=hwcdc,FlashMode=qio
```

`build.sh` injects `-DLV_CONF_INCLUDE_SIMPLE -I<sketch>` into `compiler.c/cpp/S.extra_flags` —
all three, because `lv_conf_internal.h` is also pulled in while lvgl's `.S` files are assembled.

On first boot the deck shows **“searching host”**. Holding the mascot opens Settings
(brightness, language, forget pairing) without an agent.

---

## Agent

Rust 1.87+ (the Tauri app needs 1.88+).

```bash
cd agent
cargo build --release
cargo test                          # 15 unit tests: protocol vectors, session model, hooks merge

./target/release/clowdeck-agent doctor          # environment check
./target/release/clowdeck-agent hooks install   # writes ~/.claude/settings.json (backup kept)
./target/release/clowdeck-agent run             # discovery + hooks + web UI + BLE
```

Open **http://127.0.0.1:47831/** for the virtual deck. It mirrors the physical one and works
without the board.

While developing, **never send real keystrokes with live sessions open** — use `--dry-run`,
which logs what it would type. To test hooks without touching your real
`~/.claude/settings.json`, put them in a scratch project's `.claude/settings.json` and run
`env -u CLAUDECODE claude -p "…" --model haiku` from there (`CLAUDECODE` must be unset to start
a nested session).

`hooks uninstall` removes only this project's hooks — foreign hooks are recognised and kept.

### Run at login

```bash
clowdeck-agent service install      # macOS launchd LaunchAgent / Windows HKCU Run
```

On macOS this also **code-signs the binary** with a stable identifier
(`my.autom.clowdeck-agent`). That is not cosmetic: TCC keys permission grants to the code
signature, and the linker's ad-hoc signature changes on every build — without a stable identity,
every rebuild silently voids the Bluetooth and Accessibility grants. Re-run `service install`
after each rebuild (the plist points at the binary path). Logs go to
`~/Library/Logs/clowdeck/agent.log`.

---

## Menu-bar app

```bash
cd app
cargo tauri build          # needs ~/.cargo/bin in PATH and rustc ≥ 1.88
```

Artifacts land in `app/target/release/bundle/`. For a distributable macOS build you need a
**Developer ID Application** certificate plus notarization (`xcrun notarytool` with a keychain
profile); `tools/make_dmg.sh` builds the DMG with a volume icon, because the stock Tauri DMG
bundler drives Finder through AppleScript and times out.

> The app-specific password for notarization must be generated on the **same** Apple account
> that owns the certificate — one from a different Apple ID returns a bare `401`.

The app embeds the agent as a library — on launch it boots out the CLI LaunchAgent holding the
same port and takes over.

---

## Agent CLI

```
clowdeck-agent run [--no-ble] [--dry-run] [--port N]
clowdeck-agent hooks install|uninstall|status [--settings PATH]
clowdeck-agent sessions                  # cell · pid · tty · terminal · cwd · state
clowdeck-agent focus <pid>
clowdeck-agent ble scan|info|pair [passkey]
clowdeck-agent doctor
clowdeck-agent service install|uninstall|status
clowdeck-agent keybinding install|uninstall|status [--editor code|cursor|windsurf]
```

`ble pair` is Windows-only (macOS triggers pairing through the system dialog) and must run from
a desktop session — see [`docs/WINDOWS.md`](docs/WINDOWS.md).

---

## Configuration

`~/Library/Application Support/clowdeck/config.toml` on macOS (`%APPDATA%\clowdeck\` on
Windows), created on first run:

| Key | What it does |
|---|---|
| `port` | HTTP port for hooks and the virtual deck (default `47831`) |
| `auto_enter` | whether typed commands end with Enter (default `true`) |
| `[ble]` | enable/disable, frame size, `write_with_response` |
| `[deck]` | brightness and language pushed to the device |
| `[[commands]]` | extra buttons on the deck's command page: `label`, `text`, `confirm` |
| `[focus]` | `vscode_terminal_keys` — the chord sent to focus the editor's terminal panel |
| `sign_identity` | overrides code-signing identity auto-detection (macOS) |

Commands that only exist in Claude Code (`/voice`, `/config`, `/permissions`, …) are **refused**
on Codex and opencode sessions: their TUIs fuzzy-match and would run something else.

---

## Regenerating assets

```bash
python3 tools/gen_icons.py       # assets/icons_vec.py + assets/pixel/*.txt → firmware/clow_deck/icons.h
python3 tools/gen_mockups.py     # assets/mock-*.png for the README
```

`icons.h` is **generated — do not hand-edit**. Icons are A8 masks coloured at runtime with
`lv_obj_set_style_image_recolor()`; the catalogue is in [`design/ICONS.md`](design/ICONS.md) and
the palette in [`design/THEME.md`](design/THEME.md).

Embedded Montserrat fonts carry **ASCII plus `°` and `•` only** — no accents. On-device strings
must be written unaccented, and labels coming from the agent are transliterated to ASCII.

---

## Platform notes

**macOS.** The first run asks for **Bluetooth** and **Accessibility**, and **Automation** the
first time a window is focused. The prompts name the process that runs the agent. Without
Accessibility the window is still raised but no keys are sent — `doctor` reports what is
missing. `AXIsProcessTrusted` is cached per process, so restart the agent after granting.

**Windows.** Discovery uses `sysinfo`, focus uses `EnumWindows` + `SetForegroundWindow`. There is
no tty, so hooks match sessions by working directory. BLE needs an adapter that supports the
**central role with LE Secure Connections** — full details, including how to check yours, in
[`docs/WINDOWS.md`](docs/WINDOWS.md).

**Changing the wire format.** Edit [`protocol/PROTOCOL.md`](protocol/PROTOCOL.md) first, then
`agent/src/protocol.rs` (with tests) and `firmware/clow_deck/{deck_types.h,ble_link.cpp}`, and
bump `PROTO_VERSION` on both sides. The agent refuses a deck reporting a different version.
