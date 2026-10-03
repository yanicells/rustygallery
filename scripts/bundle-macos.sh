#!/bin/sh
# Build a signed-enough .app for local use (ad-hoc). Icon + LSUIElement included.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist/gallery.app"
BIN="$ROOT/target/release/gallery"
ICON="$ROOT/assets/icon/gallery.icns"

cd "$ROOT"
python3 "$ROOT/scripts/gen_icon.py"
cargo build --release --bin gallery

rm -rf "$DIST"
mkdir -p "$DIST/Contents/MacOS" "$DIST/Contents/Resources"
cp "$BIN" "$DIST/Contents/MacOS/gallery"
cp "$ICON" "$DIST/Contents/Resources/gallery.icns"

cat > "$DIST/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>gallery</string>
  <key>CFBundleDisplayName</key><string>gallery</string>
  <key>CFBundleIdentifier</key><string>com.yanicells.gallery</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleExecutable</key><string>gallery</string>
  <key>CFBundleIconFile</key><string>gallery</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

echo "wrote $DIST"
echo "open it with: open $DIST"
echo
echo "Homebrew (once a release zip exists):"
echo "  brew install --cask $ROOT/scripts/gallery.rb"
