# firmware/ — Clow Deck (ESP32-S3 + LVGL + NimBLE)

Firmware para a tela touch **Guition JC4832W535** (AXS15231B QSPI, 480×320).

- **`clow_deck/`** — **o projeto.** Sketch arduino-cli: grade 4×3 em LVGL 9 dirigida pelo
  agente via BLE (GATT server NimBLE, `protocol/PROTOCOL.md`), toques viram eventos BLE.
  Sem Wi-Fi, sem TLS, sem segredo no dispositivo. Grave com [`../flash.sh`](../flash.sh)
  (autodetecta a porta) ou [`clow_deck/build.sh`](clow_deck/build.sh).
- **`claude_stick/`** — firmware do Claude Usage Stick (mesma placa). Mantido como **base
  validada** de display/touch/LVGL e como referência de UI; não faz parte do build do deck.
- **`bringup/`** — bring-up puro de display/touch (cores certas, orientação USB-à-esquerda,
  touch alinhado). Compile com o `build.sh` **desta pasta** — usa `PartitionScheme=huge_app`
  e reaproveita o `lv_conf.h` do `claude_stick`.
- **`REFERENCIA-HARDWARE-LVGL.md`** — pinos, libs testadas, pipeline de flush (rotação 270° CW
  na mão) e armadilhas (PSRAM OPI obrigatória, etc.).

Comece por [`../README.md`](../README.md) para a visão geral e o passo a passo de build.
