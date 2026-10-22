#!/usr/bin/env bash
# velnosub alpha10 gece-otomatik smoke kanalı — APK'yi cihaza rish ile
# kurar, açar, breadcrumb + logcat + dropbox'tan kanıt kırpar. Kullanıcı
# uyurken çalışır: tuş yok, gözlem ve (hata durumunda) rapor.
# Kullanım: bash smoke.sh <apk-yolu>
set -u
APK="${1:?apk yolu verilcek}"
RISH=/root/rish
PKG=com.velnoxy.velnosubui

echo "== 1) kurulum (rish pm install) =="
timeout -k 4 120 "$RISH/rish" -c "pm install -r $APK" < /dev/null 2>&1 | tail -2
# P5.5: kendi dosyaları — /sdcard read izni (targetSdk 28 runtime izni)
timeout -k 4 20 "$RISH/rish" -c "pm grant $PKG android.permission.READ_EXTERNAL_STORAGE" < /dev/null 2>&1 | tail -1

echo "== 2) önce güvenli kapanış (DONMA/Make it ...) =="
timeout -k 4 30 "$RISH/rish" -c "am force-stop $PKG" < /dev/null 2>&1 | tail -1
sleep 2

echo "== 3) açılış (am start) =="
timeout -k 4 30 "$RISH/rish" -c "am start -n $PKG/android.app.NativeActivity" < /dev/null 2>&1 | tail -2
sleep 12

echo "== 4) process kanıtı =="
timeout -k 4 30 "$RISH/rish" -c "pidof $PKG" < /dev/null 2>&1 | tail -1

echo "== 5) breadcrumb tail (dahili filesDir; run-as) =="
timeout -k 4 30 "$RISH/rish" -c "run-as $PKG cat files/velnosub-boot.txt 2>/dev/null | tail -6" < /dev/null 2>&1 | tail -8

echo "== 6) ext breadcrumb tail (rish okur) =="
timeout -k 4 30 "$RISH/rish" -c "cat /storage/emulated/0/Android/data/$PKG/files/velnosub-boot.txt 2>/dev/null | tail -8" < /dev/null 2>&1 | tail -10

echo "== 7) logcat 마지막 velnosub PANIC/Диагностика =="
timeout -k 4 30 "$RISH/rish" -c "logcat -d -t 300 | grep -iE 'velnosub|FATAL|beginning of crash' | tail -12" < /dev/null 2>&1 | tail -14

echo "== 8) dropbox (çökme kimliği var mı?) =="
timeout -k 4 60 "$RISH/rish" -c "dumpsys dropbox --print 2>/dev/null | grep -A 3 'AndroidRuntime.*$PKG\|native_crash.*$PKG' | head -12" < /dev/null 2>&1 | tail -14

echo "== 9) SMOKE BİTİŞ =="
