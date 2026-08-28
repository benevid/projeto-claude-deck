#!/usr/bin/env bash
#
# Grava o firmware do Clow Deck na placa: autodetecta a porta USB
# (/dev/cu.usbmodem*) e chama firmware/clow_deck/build.sh upload.
#
#   ./flash.sh            # autodetecta (espera ate 30 s pela placa)
#   ./flash.sh <porta>    # porta explicita
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

port="${1:-}"
if [ -z "$port" ]; then
  for _ in $(seq 1 30); do
    ports="$(ls /dev/cu.usbmodem* 2>/dev/null || true)"
    [ -n "$ports" ] && break
    echo "aguardando a placa em /dev/cu.usbmodem*..."
    sleep 1
  done
  n="$(printf '%s\n' "$ports" | grep -c . || true)"
  if [ "${n:-0}" -gt 1 ]; then
    # Mais de uma placa ESP32 no Mac: escolher a primeira gravaria a placa ERRADA.
    # O numero de serie USB do ESP32-S3 e o MAC base; o do deck e o BLE menos 1.
    echo "ATENCAO: ha mais de uma placa em /dev/cu.usbmodem*:" >&2
    printf '  %s\n' $ports >&2
    echo "" >&2
    echo "Identifique a do deck pelo numero de serie USB e passe a porta explicitamente:" >&2
    echo "  ioreg -p IOUSB -l -w0 | grep -B12 Espressif | grep 'USB Serial Number'" >&2
    echo "  ./flash.sh /dev/cu.usbmodemXXXX" >&2
    exit 1
  fi
  port="$ports"
fi
if [ -z "$port" ]; then
  echo "nenhuma placa encontrada (/dev/cu.usbmodem*)" >&2
  exit 1
fi
exec "$ROOT/firmware/clow_deck/build.sh" upload "$port"
