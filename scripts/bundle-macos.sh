#!/bin/sh
# Build and ad-hoc sign a local macOS app, then archive the verified bundle.
set -eu
if [ "$(uname -s)" != Darwin ]; then
  echo "macOS is required to bundle gallery" >&2
  exit 1
fi
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist/gallery.app"
BIN="$ROOT/target/release/gallery"
ICON="$ROOT/assets/icon/gallery.icns"
ZIP="$ROOT/dist/gallery.app.zip"

cd "$ROOT"
python3 "$ROOT/scripts/gen_icon.py"
cargo build --release --locked --bin gallery
VERSION="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; data = json.load(sys.stdin); print(next(p["version"] for p in data["packages"] if p["name"] == "gallery"))')"

rm -rf "$DIST"
mkdir -p "$DIST/Contents/MacOS" "$DIST/Contents/Resources"
cp "$BIN" "$DIST/Contents/MacOS/gallery"
cp "$ICON" "$DIST/Contents/Resources/gallery.icns"

python3 - "$DIST/Contents/Info.plist" "$VERSION" <<'PYINFO'
import pathlib
import plistlib
import sys

info = {
    "CFBundleName": "gallery",
    "CFBundleDisplayName": "gallery",
    "CFBundleIdentifier": "com.yanicells.gallery",
    "CFBundleVersion": sys.argv[2],
    "CFBundleShortVersionString": sys.argv[2],
    "CFBundleExecutable": "gallery",
    "CFBundleIconFile": "gallery",
    "CFBundlePackageType": "APPL",
    "LSMinimumSystemVersion": "13.0",
    "LSUIElement": True,
    "NSHighResolutionCapable": True,
}
pathlib.Path(sys.argv[1]).write_bytes(plistlib.dumps(info))
PYINFO

plutil -lint "$DIST/Contents/Info.plist"
codesign --force --sign - "$DIST"
codesign --verify --deep --strict --verbose=2 "$DIST"
rm -f "$ZIP"
ditto -c -k --sequesterRsrc --keepParent "$DIST" "$ZIP"

echo "wrote $DIST and $ZIP"
echo "install locally: ditto '$DIST' /Applications/gallery.app"
echo "open it with: open /Applications/gallery.app"
