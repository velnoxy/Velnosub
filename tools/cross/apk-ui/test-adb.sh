#!/usr/bin/env bash
set -euo pipefail

TEST_BINARY=${1:?Android Rust test ikilisinin yolu gerekli}
LIBRARIES=${2:?Android native kitaplık dizini gerekli}
REPO=$(cd "$(dirname "$0")/../../.." && pwd)

adb wait-for-device
device_dir=$(adb shell mktemp -d /data/local/tmp/velnosub-tests.XXXXXX | tr -d '\r')
test -n "$device_dir"
trap 'adb shell rm -rf "$device_dir" >/dev/null 2>&1 || true' EXIT

adb shell mkdir -p "$device_dir/src/libresrc" "$device_dir/tools/cross/apk-ui/seed" \
    "$device_dir/lib" "$device_dir/cache"
adb push "$TEST_BINARY" "$device_dir/tests"
adb push "$REPO/src/libresrc/default_config.json" "$device_dir/src/libresrc/"
adb push "$REPO/tools/cross/apk-ui/seed/karaskel-dene.ass" "$device_dir/tools/cross/apk-ui/seed/"
adb push "$LIBRARIES/." "$device_dir/lib/"
adb shell chmod 755 "$device_dir/tests"
adb shell "cd $device_dir && HOME=$device_dir TMPDIR=$device_dir/cache \
    VELNOSUB_CACHE=$device_dir/cache VELNOSUB_USER_DIR=$device_dir \
    VELNOSUB_DATA_DIR=$device_dir VELNOSUB_TEST_ROOT=$device_dir \
    LD_LIBRARY_PATH=$device_dir/lib ./tests --test-threads=1"
