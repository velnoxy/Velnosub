#!/usr/bin/env bash
set -euo pipefail

APK=${1:?APK yolu gerekli}
PACKAGE=com.velnoxy.velnosubui
ACTIVITY="$PACKAGE/.DocumentActivity"

adb wait-for-device
adb install --no-streaming -r "$APK"
adb shell am force-stop "$PACKAGE"
adb logcat -b all -c
adb shell am start -W -n "$ACTIVITY"
for ((attempt = 0; attempt < 45; attempt++)); do
    if adb shell pidof "$PACKAGE" >/dev/null 2>&1 &&
       adb logcat -d -s velnosub:I | grep -F 'A4 editor ready' >/dev/null; then
        sleep 5
        adb shell pidof "$PACKAGE"
        if adb shell dumpsys activity activities | grep -F 'ResumedActivity' | grep -F "$ACTIVITY" >/dev/null; then
            exit 0
        fi
    fi
    sleep 1
done
adb logcat -d -s velnosub:I AndroidRuntime:E >&2
adb logcat -d -b crash >&2
exit 1
