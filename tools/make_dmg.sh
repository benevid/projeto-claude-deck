#!/bin/bash
# Gera o DMG do Clow Deck a partir do .app ja compilado/assinado pelo `cargo tauri build`.
# Usa hdiutil puro (sem o AppleScript de Finder do bundler, que precisa de permissao de
# Automacao e estoura "AppleEvent timed out (-1712)" em shells nao interativos).
# Inclui o icone do volume (.VolumeIcon.icns + bit de icone custom) e o icone do
# proprio arquivo .dmg (resource fork via Rez — nao altera o data fork).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/app/target/release/bundle/macos/Clow Deck.app"
ICON="$ROOT/app/icons/icon.icns"
VERSION=$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$APP/Contents/Info.plist")
ARCH=$(uname -m); [ "$ARCH" = "arm64" ] && ARCH=aarch64
OUT="$ROOT/app/target/release/bundle/dmg/Clow Deck_${VERSION}_${ARCH}.dmg"
[ -d "$APP" ] || { echo "compile antes: cd app && cargo tauri build --bundles app"; exit 1; }
STAGE=$(mktemp -d); RW=$(mktemp -u).dmg
trap 'rm -rf "$STAGE" "$RW"' EXIT
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"

# 1) imagem UDRW p/ poder marcar o icone do volume
hdiutil create -volname "Clow Deck" -srcfolder "$STAGE" -ov -format UDRW -quiet "$RW"
MNT=$(hdiutil attach -readwrite -noverify -noautoopen "$RW" | grep -o "/Volumes/.*" | tail -1)
cp "$ICON" "$MNT/.VolumeIcon.icns"
xcrun SetFile -a C "$MNT" 2>/dev/null || SetFile -a C "$MNT" 2>/dev/null || true
hdiutil detach "$MNT" -quiet

# 2) converte p/ UDZO comprimido
hdiutil convert "$RW" -format UDZO -quiet -o "$OUT"

# 3) assinatura (antes do icone do arquivo: o resource fork fica fora do data fork)
if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  codesign --force --sign "$APPLE_SIGNING_IDENTITY" "$OUT"
fi

# 4) icone do ARQUIVO .dmg (resource fork + bit custom; assinatura/staple continuam validos)
if xcrun --find Rez >/dev/null 2>&1; then
  TMPI=$(mktemp -d); cp "$ICON" "$TMPI/i.icns"
  sips -i "$TMPI/i.icns" >/dev/null
  xcrun DeRez -only icns "$TMPI/i.icns" > "$TMPI/i.rsrc"
  xcrun Rez -append "$TMPI/i.rsrc" -o "$OUT"
  xcrun SetFile -a C "$OUT" 2>/dev/null || true
  rm -rf "$TMPI"
fi
echo "$OUT"
