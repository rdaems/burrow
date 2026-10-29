#!/bin/sh
# Build burrow.app: the window around the room (src/main.rs), an Info.plist, an icon drawn from one of the room's
# robots (macos/icon-dots.txt), and a copy of web/ for a Mac without this checkout (built here, the app serves this
# checkout's web/ itself, so a change shows on a reload).
#   macos/bundle.sh            -> target/release/burrow.app
#   cp -R target/release/burrow.app /Applications/
set -e
cd "$(dirname "$0")/.."
cargo build --release
app=target/release/burrow.app
work=target/release/icon
rm -rf "$app" "$work"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$work/burrow.iconset"
cp target/release/burrow "$app/Contents/MacOS/burrow"
rsync -a --exclude __pycache__ web/ "$app/Contents/Resources/web/"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

python3 -I - macos/icon-dots.txt "$work/icon.png" <<'PY'
# a dark rounded square with a rug and the robot on it, 1024 px, written as a PNG by hand
import sys, zlib, struct
dots = [l.split() for l in open(sys.argv[1]) if l.strip()]
N, K = 1024, 26                     # canvas, dots per robot pixel
INK, FLOOR, RUG = (0x14, 0x0f, 0x1e), (0x13, 0x0f, 0x1b), (0x3a, 0x22, 0x1c)
img = [[None] * N for _ in range(N)]
M, R = 100, 180                     # Apple's grid: 824 px of icon, rounded corners
for y in range(M, N - M):
    for x in range(M, N - M):
        cx = min(max(x, M + R), N - M - R - 1)
        cy = min(max(y, M + R), N - M - R - 1)
        if (x - cx) ** 2 + (y - cy) ** 2 <= R * R:
            img[y][x] = INK if y < 420 else FLOOR
for y in range(560, 820):           # the rug
    for x in range(200, 824):
        img[y][x] = RUG
xs = [int(d[0]) for d in dots]; ys = [int(d[1]) for d in dots]
ox = (N - (max(xs) - min(xs) + 1) * K) // 2 - min(xs) * K
oy = 760 - (max(ys) + 1) * K
for x, y, c in dots:
    rgb = tuple(int(c[i:i + 2], 16) for i in (0, 2, 4))
    for yy in range(oy + int(y) * K, oy + (int(y) + 1) * K):
        for xx in range(ox + int(x) * K, ox + (int(x) + 1) * K):
            img[yy][xx] = rgb
raw = b"".join(b"\0" + b"".join(bytes(p + (255,)) if p else b"\0\0\0\0" for p in row) for row in img)
chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)
open(sys.argv[2], "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0))
                              + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
PY
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
  <key>CFBundleName</key><string>burrow</string>
  <key>CFBundleDisplayName</key><string>burrow</string>
  <key>CFBundleIdentifier</key><string>me.daems.burrow</string>
  <key>CFBundleExecutable</key><string>burrow</string>
  <key>CFBundleIconFile</key><string>burrow</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSAppTransportSecurity</key><dict><key>NSAllowsLocalNetworking</key><true/></dict>
</dict>
</plist>
PLIST
codesign --force --sign - "$app" 2>/dev/null || true
echo "$app"
