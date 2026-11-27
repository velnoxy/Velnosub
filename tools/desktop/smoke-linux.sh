#!/usr/bin/env bash
set -euo pipefail

image=$(realpath "${1:?AppImage required}")
source_dir=$(cd "$(dirname "$0")/../.." && pwd)
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
cp "$source_dir/tools/cross/apk-ui/seed/karaskel-dene.ass" "$scratch/velnosub-smoke.ass"

export HOME="$scratch" APPIMAGE_EXTRACT_AND_RUN=1
runner=(bash -eu -c '
    "$1" "$2" >"$HOME/editor.log" 2>&1 &
    app=$!
    trap "kill $app 2>/dev/null || true; wait $app 2>/dev/null || true" EXIT
    for ((attempt = 0; attempt < 30; attempt++)); do
        if ! kill -0 "$app" 2>/dev/null; then
            cat "$HOME/editor.log" >&2
            exit 1
        fi
        windows=$(xwininfo -root -tree)
        if [[ "$windows" == *"velnosub-smoke.ass"* ]]; then
            printf "%s\n" "$windows" | grep -F "velnosub-smoke.ass"
            exit 0
        fi
        sleep 1
    done
    cat "$HOME/editor.log" >&2
    exit 1
' smoke "$image" "$scratch/velnosub-smoke.ass")
if [[ -n "${DISPLAY:-}" ]]; then
    "${runner[@]}"
else
    xvfb-run -a "${runner[@]}"
fi
