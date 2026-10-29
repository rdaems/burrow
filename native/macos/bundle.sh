#!/bin/sh
# Build Burrow.app from the native app: the release binary, an Info.plist, the icon (one of the room's robots).
#   native/macos/bundle.sh            -> native/target/release/Burrow.app
#   cp -R native/target/release/Burrow.app /Applications/
set -e
cd "$(dirname "$0")/.."
cargo build --release -p burrow-app
app=target/release/Burrow.app
work=target/release/icon
rm -rf "$app" "$work"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$work/burrow.iconset"
cp target/release/burrow "$app/Contents/MacOS/burrow"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
target/release/burrow --icon "$work/icon.png"
for s in 16 32 128 256 512; do
  sips -z $s $s "$work/icon.png" --out "$work/burrow.iconset/icon_${s}x${s}.png" >/dev/null
  sips -z $((s * 2)) $((s * 2)) "$work/icon.png" --out "$work/burrow.iconset/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$work/burrow.iconset" -o "$app/Contents/Resources/burrow.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Burrow</string>
  <key>CFBundleDisplayName</key><string>Burrow</string>
  <key>CFBundleIdentifier</key><string>me.daems.burrow</string>
  <key>CFBundleExecutable</key><string>burrow</string>
  <key>CFBundleIconFile</key><string>burrow</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$app" 2>/dev/null || true
echo "$app"
