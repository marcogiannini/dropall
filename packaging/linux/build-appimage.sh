#!/usr/bin/env bash
# Costruisce target/appimage/dropall-linux-<arch>.AppImage
# Uso:  packaging/linux/build-appimage.sh
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."
ARCH="${ARCH:-$(uname -m)}"            # x86_64 | aarch64
OUT="target/appimage"
APPDIR="$OUT/Dropall.AppDir"

cargo build --release

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/512x512/apps"

cp target/release/dropall                "$APPDIR/usr/bin/dropall"
cp packaging/linux/dropall.desktop       "$APPDIR/dropall.desktop"
cp packaging/linux/dropall.desktop       "$APPDIR/usr/share/applications/dropall.desktop"
mkdir -p "$APPDIR/usr/share/doc/dropall"
cp LICENSE THIRD_PARTY_LICENSES.txt      "$APPDIR/usr/share/doc/dropall/"
cp assets/icon.png                       "$APPDIR/dropall.png"
cp assets/icon.png                       "$APPDIR/usr/share/icons/hicolor/512x512/apps/dropall.png"
ln -sf usr/bin/dropall                   "$APPDIR/AppRun"

TOOL="$OUT/appimagetool-$ARCH.AppImage"
if [ ! -x "$TOOL" ]; then
  mkdir -p "$OUT"
  curl -fsSL -o "$TOOL" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$ARCH.AppImage"
  chmod +x "$TOOL"
fi

# --appimage-extract-and-run: funziona anche dove FUSE non è disponibile (CI, container)
ARCH="$ARCH" "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT/dropall-linux-$ARCH.AppImage"
echo "Creato: $OUT/dropall-linux-$ARCH.AppImage"
