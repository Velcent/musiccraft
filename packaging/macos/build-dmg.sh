#!/usr/bin/env bash
# Builds MusicCraft.app and a compressed .dmg from a (universal) binary.
# Must run on macOS (uses sips, iconutil, codesign and hdiutil).
#
# usage: build-dmg.sh <version> <binary> <icon-1024.png> <output.dmg>
#
# Signing: the app is ad-hoc signed by default, which is enough for it to launch on
# Apple Silicon but NOT enough to avoid Gatekeeper's "unidentified developer" prompt.
# Set APPLE_SIGN_IDENTITY (e.g. "Developer ID Application: Name (TEAMID)") to sign
# with a certificate that is already in the keychain (hardened runtime plus
# entitlements.plist, which lets the app load third-party CLAP plugins and record
# audio). Notarization is not done here.
#
# ORT_DIR, when set, is the folder packaging/onnxruntime/fetch.sh filled: ONNX
# Runtime for stem splitting goes into Contents/Frameworks, signed with the app.
set -euo pipefail

if [ "$#" -ne 4 ]; then
  echo "usage: $0 <version> <binary> <icon-1024.png> <output.dmg>" >&2
  exit 2
fi
version=$1
binary=$2
icon=$3
out=$4

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

app="$work/stage/MusicCraft.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/musiccraft"
chmod 755 "$app/Contents/MacOS/musiccraft"
sed "s/@VERSION@/$version/g" "$here/Info.plist.in" > "$app/Contents/Info.plist"
if [ -n "${ORT_DIR:-}" ]; then
  mkdir -p "$app/Contents/Frameworks"
  cp "$ORT_DIR/libonnxruntime.dylib" "$ORT_DIR/ONNXRUNTIME-LICENSE.txt" "$app/Contents/Frameworks/"
fi

# Build the .icns from the 1024px master.
iconset="$work/MusicCraft.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$icon" --out "$iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" "$icon" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/MusicCraft.icns"

identity=${APPLE_SIGN_IDENTITY:--}
if [ "$identity" = "-" ]; then
  codesign --force --deep --sign - "$app"
else
  codesign --force --deep --options runtime --timestamp \
    --entitlements "$here/entitlements.plist" --sign "$identity" "$app"
fi
codesign --verify --deep --strict "$app"

ln -s /Applications "$work/stage/Applications"
mkdir -p "$(dirname "$out")"
rm -f "$out"
hdiutil create -volname "MusicCraft $version" -srcfolder "$work/stage" -ov -format UDZO "$out"
echo "wrote $out"
