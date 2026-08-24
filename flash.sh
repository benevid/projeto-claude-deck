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
    port="$(ls /dev/cu.usbmodem* 2>/dev/null | head -n1 || true)"
    [ -n "$port" ] && break
    echo "aguardando a placa em /dev/cu.usbmodem*..."
    sleep 1
  done
fi
if [ -z "$port" ]; then
  echo "nenhuma placa encontrada (/dev/cu.usbmodem*)" >&2
  exit 1
fi
exec "$ROOT/firmware/clow_deck/build.sh" upload "$port"
