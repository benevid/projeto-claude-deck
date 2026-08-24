#!/usr/bin/env bash
#
# Build / upload / monitor do Clow Deck (Guition JC4832W535, ESP32-S3).
#
# Uso:
#   ./build.sh                 # compila
#   ./build.sh upload          # compila + grava (porta padrao abaixo)
#   ./build.sh upload <porta>  # compila + grava na porta indicada
#   ./build.sh monitor <porta> # abre o serial monitor (115200)
#
# Pre-requisitos (ver firmware/REFERENCIA-HARDWARE-LVGL.md):
#   - arduino-cli 1.4.x, core esp32:esp32 3.3.11
#   - libs: GFX Library for Arduino 1.6.5, lvgl 9.2.2, NimBLE-Arduino 2.5.x
#
# O -DLV_CONF_INCLUDE_SIMPLE + -I<sketch> faz o LVGL achar o nosso lv_conf.h.
# Vai nos TRES recipes (c, cpp, S): o core monta lv_blend_helium.S /
# lv_blend_neon.S com compiler.S.extra_flags e lv_conf_internal.h puxa o
# lv_conf.h tambem nessa etapa.
set -euo pipefail

SKETCH_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FQBN="esp32:esp32:esp32s3:PSRAM=opi,FlashSize=16M,PartitionScheme=custom,CDCOnBoot=cdc,USBMode=hwcdc,FlashMode=qio"
PORT_DEFAULT="/dev/cu.usbmodem1101"

LVFLAGS="-DLV_CONF_INCLUDE_SIMPLE -DLV_LVGL_H_INCLUDE_SIMPLE -I${SKETCH_DIR}"

cmd="${1:-build}"
port="${2:-$PORT_DEFAULT}"

case "$cmd" in
  monitor)
    exec arduino-cli monitor -p "$port" -c baudrate=115200
    ;;
  build)
    echo "==> compilando ($FQBN)"
    arduino-cli compile \
      --fqbn "$FQBN" \
      --build-property "compiler.cpp.extra_flags=$LVFLAGS" \
      --build-property "compiler.c.extra_flags=$LVFLAGS" \
      --build-property "compiler.S.extra_flags=$LVFLAGS" \
      "$SKETCH_DIR"
    ;;
  upload)
    # `compile --upload` compila e grava num passo so (upload puro nao aceita --build-property)
    echo "==> compilando + gravando em $port ($FQBN)"
    arduino-cli compile \
      --fqbn "$FQBN" \
      --build-property "compiler.cpp.extra_flags=$LVFLAGS" \
      --build-property "compiler.c.extra_flags=$LVFLAGS" \
      --build-property "compiler.S.extra_flags=$LVFLAGS" \
      --upload -p "$port" \
      "$SKETCH_DIR"
    ;;
  *)
    echo "comando desconhecido: $cmd (use: build | upload | monitor)" >&2
    exit 1
    ;;
esac
