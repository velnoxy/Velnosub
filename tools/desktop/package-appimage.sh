#!/usr/bin/env bash
set -euo pipefail

build_dir=${1:?Meson build directory required}
output_dir=${2:?Output directory required}
: "${LINUXDEPLOY:?Set LINUXDEPLOY to the linuxdeploy executable}"
: "${APPIMAGETOOL:?Set APPIMAGETOOL to the appimagetool executable}"

source_dir=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$output_dir"
output_dir=$(cd "$output_dir" && pwd)
appdir=$(mktemp -d)
trap 'rm -rf "$appdir"' EXIT
export APPIMAGE_EXTRACT_AND_RUN=1

meson install -C "$build_dir" --no-rebuild --destdir "$appdir"
test -x "$appdir/usr/bin/aegisub"
desktop_id=com.velnoxy.velnosub.desktop
test -e "$appdir/$desktop_id"
sed -i 's/^Name=Aegisub$/Name=Velnosub/' "$appdir/usr/share/applications/$desktop_id"
install -Dm644 "$source_dir/LICENCE" "$appdir/usr/share/doc/velnosub/LICENCE"
"$LINUXDEPLOY" --appdir "$appdir" --desktop-file "$appdir/$desktop_id" \
  --icon-file "$appdir/usr/share/icons/hicolor/64x64/apps/aegisub.png"
ARCH=x86_64 "$APPIMAGETOOL" "$appdir" "$output_dir/Velnosub-linux-x86_64.AppImage"
chmod +x "$output_dir/Velnosub-linux-x86_64.AppImage"
