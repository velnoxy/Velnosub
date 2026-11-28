#!/usr/bin/env bash
set -euo pipefail

build_dir=${1:?Meson build directory required}
output_dir=${2:?Output directory required}
source_dir=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$output_dir"
output_dir=$(cd "$output_dir" && pwd)
build_dir=$(cd "$build_dir" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

# Meson's macOS bundle option installs the application into the build directory.
meson install -C "$build_dir" --no-rebuild
test -x "$build_dir/Aegisub.app/Contents/MacOS/aegisub"
cp -R "$build_dir/Aegisub.app" "$stage/Velnosub.app"
app="$stage/Velnosub.app"
editor="$app/Contents/MacOS/aegisub"
version=$(meson introspect "$build_dir" --projectinfo | python3 -c 'import json, sys; print(json.load(sys.stdin)["version"])')
arch=$(uname -m)
mkdir -p "$app/Contents/Resources/en.lproj" "$app/Contents/SharedSupport"
cp "$source_dir/packages/osx_bundle/Contents/Resources/"*.icns "$app/Contents/Resources/"
cp "$source_dir/LICENCE" "$app/Contents/SharedSupport/LICENCE"
python3 - "$source_dir/packages/osx_bundle/Contents/Info.plist" "$app/Contents" "$version" "$arch" <<'PY'
import plistlib
import sys
from pathlib import Path

template, contents, version, arch = sys.argv[1:]
contents = Path(contents)
data = plistlib.loads(Path(template).read_bytes().replace(b"@LOCALIZATIONS@", b""))
data.update({
    "CFBundleName": "Velnosub",
    "CFBundleDisplayName": "Velnosub",
    "CFBundleIdentifier": "com.velnoxy.velnosub",
    "CFBundleShortVersionString": version,
    "CFBundleLongVersionString": f"Velnosub {version}",
    "CFBundleVersion": version,
    "CFBundleGetInfoString": f"Velnosub {version}",
    "LSArchitecturePriority": [arch],
    "CFBundleLocalizations": sorted(path.stem for path in (contents / "Resources").glob("*.lproj")),
})
(contents / "Info.plist").write_bytes(plistlib.dumps(data))
PY

dylibbundler -od -b -x "$editor" -d "$app/Contents/Frameworks" -p '@executable_path/../Frameworks/'
find "$app/Contents/Frameworks" -type f -name '*.dylib' -exec codesign --force --sign - {} \;
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
plutil -lint "$app/Contents/Info.plist"
lipo -verify_arch "$arch" "$editor"

ln -s /Applications "$stage/Applications"
image="$output_dir/Velnosub-macos-$arch.dmg"
hdiutil create -ov -srcfolder "$stage" -volname Velnosub -format UDZO "$image"
hdiutil verify "$image"

mount_dir=$(mktemp -d)
cleanup_mount() {
    hdiutil detach "$mount_dir" >/dev/null 2>&1 || true
    rmdir "$mount_dir"
    rm -rf "$stage"
}
trap cleanup_mount EXIT
hdiutil attach -readonly -nobrowse -mountpoint "$mount_dir" "$image"
codesign --verify --deep --strict "$mount_dir/Velnosub.app"
test "$(readlink "$mount_dir/Applications")" = /Applications
