#!/usr/bin/env bash
# velnosub alpha APK pakacı — aapt2 + javac/d8 + zipalign + apksigner
# bileşenleri: android-sdk/bt (build-tools r34) + pf (platform android-34)
# Girişler:
#   PKGDEST  = /root/velsub/device-pkg          (paket gövdesi; lib/bin hariç assets'e)
#   PKGLIB   = /root/velsub/device-pkg/lib      (arm64 .so'lar → lib/arm64-v8a, strip'li)
#   AUTOM    = /root/velsub/aegisub-upstream/build-android-arm64/tools/cross/velnosub-automation
#   LANE     = /root/velsub/aegisub-upstream/build-android-arm64/tools/cross (libvelnosub-abi.so burada)
# Çıkış: tools/cross/apk-alpha/velnosub-0.1.0-alpha.apk (imzalı, hizalı)
set -e
cd "$(dirname "$0")"
BT=/root/velsub/android-sdk/bt/android-14
PF=/root/velsub/android-sdk/pf/android-34
NDKBIN=$(echo /root/velsub/ndk/r29/toolchains/llvm/prebuilt/*/bin | head -1)
STRIP="$NDKBIN/llvm-strip"
LD=/root/velsub/ndk/r29/toolchains/llvm/prebuilt/*/bin/ld.lld
PKGSRC="${PKGSRC:-/root/velsub/device-pkg}"
AUTOM="${AUTOM:-/root/velsub/aegisub-upstream/build-android-arm64/tools/cross/velnosub-automation}"
ABISO="${ABISO:-/root/velsub/aegisub-upstream/build-android-arm64/tools/cross/libvelnosub-abi.so}"

echo "---- 1) assets (pkg gövdesi; lib/bin hariç) ----"
rm -rf assets/velnosub
mkdir -p assets/velnosub
cp -f "$PKGSRC"/default_config.json "$PKGSRC"/DejaVuSans.ttf \
      "$PKGSRC"/ass-format-tests.ass "$PKGSRC"/karaskel-dene.ass \
      "$PKGSRC"/res-yarim.ass assets/velnosub/
mkdir -p assets/velnosub/share/lua/inc assets/velnosub/share/lua/autos
cp -f "$PKGSRC"/share/lua/*.lua assets/velnosub/share/lua/
cp -f "$PKGSRC"/share/lua/inc/*.lua assets/velnosub/share/lua/inc/
cp -f "$PKGSRC"/share/lua/autos/*.lua assets/velnosub/share/lua/autos/ 2>/dev/null || true
echo "assets: $(find assets -type f | wc -l) dosya"

echo "---- 2) lib/arm64-v8a (strip'li) ----"
rm -rf lib/arm64-v8a
mkdir -p lib/arm64-v8a
for s in "$PKGSRC"/lib/lib*.so; do
    cp -f "$s" lib/arm64-v8a/
done
cp -f "$ABISO" lib/arm64-v8a/libvelnosub-abi.so
chmod +x "$AUTOM"
cp -f "$AUTOM" lib/arm64-v8a/libvelnosub_automation.so
for f in lib/arm64-v8a/*; do
    "$STRIP" --strip-unneeded -o lib/arm64-v8a/$(basename "$f").stripped "$f" && mv "$f.stripped" "$f"
done
find lib/arm64-v8a -exec chmod 755 {} +
ls -l lib/arm64-v8a | awk '{print $5, $9}'

echo "---- 3) javac + d8 ----"
rm -rf classes classes.dex
mkdir classes
javac -encoding UTF-8 -source 8 -target 8 -bootclasspath "$PF/android.jar" \
      -d classes java/com/velnoxy/velnosub/MainActivity.java
"$BT/d8" --release --lib "$PF/android.jar" --min-api 24 \
      --output . classes/com/velnoxy/velnosub/*.class

echo "---- 4) aapt (Debian arm64 ikilisi — Google build-tools x86_64 bu hatta koşmaz) ----"
rm -f velnosub-unsigned.apk
aapt package -f \
      --min-sdk-version 24 --target-sdk-version 28 \
      -M AndroidManifest.xml \
      -S res \
      -I "$PF/android.jar" \
      -A assets \
      -F velnosub-unsigned.apk

echo "---- 5) lib + dex ekleme ----"
zip -q -rg velnosub-unsigned.apk lib
zip -q -rg velnosub-unsigned.apk classes.dex

echo "---- 6) zipalign (Debian arm64) ----"
zipalign -f 4 velnosub-unsigned.apk velnosub-aligned.apk

echo "---- 7) keystore + apksigner ----"
if [ ! -f velnosub-debug.keystore ]; then
    keytool -genkeypair -keystore velnosub-debug.keystore \
        -alias velnosubdebug -keyalg RSA -keysize 2048 \
        -validity 10000 -storepass velnosub \
        -dname "CN=velnosub debug, O=velnoxy, C=TR" 2>/dev/null
fi
"$BT/apksigner" sign --ks velnosub-debug.keystore \
      --ks-key-alias velnosubdebug --ks-pass pass:velnosub \
      --out velnosub-0.1.0-alpha.apk velnosub-aligned.apk

"$BT/apksigner" verify velnosub-0.1.0-alpha.apk && echo "APK İMZA GEÇERLİ"
ls -l velnosub-0.1.0-alpha.apk
