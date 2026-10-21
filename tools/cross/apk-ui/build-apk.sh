#!/usr/bin/env bash
# velnosub UI alpha APK pakacı — NativeActivity + Rust cdylib (faz 4.8, T3)
# javac/d8 YOK: android:hasCode="false" — saf native aktivite.
# Girişler:
#   UISO   = T2 çıktısı libvelnosub_ui.so (aarch64 cdylib; ANativeActivity_onCreate + android_main)
#   ABISO  = libvelnosub-abi.so (47 ABI sembolü; build-android-arm64 lane)
#   PKGSRC = device-pkg (kapanış .so'ları: core/freetype/fribidi/harfbuzz/uchardet/c++_shared
#            + assets: font/ass örnekleri)
# Kapanış (DT_NEEDED zinciri, 2026-10-21 llvm-readelf ölçümü):
#   ui → abi → (core, freetype); core → (freetype, fribidi, harfbuzz, uchardet);
#   sistem küpü (c, dl, m, log, z) platformdadır — paketlenmez; c++_shared PAKETLENİR.
# Çıkış: tools/cross/apk-ui/velnosub-ui-0.1.0-alpha.apk (imzalı, hizalı)
set -e
cd "$(dirname "$0")"
BT=/root/velsub/android-sdk/bt/android-14
PF=/root/velsub/android-sdk/pf/android-34
NDKBIN=$(echo /root/velsub/ndk/r29/toolchains/llvm/prebuilt/*/bin | head -1)
STRIP="$NDKBIN/llvm-strip"
RE="$NDKBIN/llvm-readelf"
PKGSRC="${PKGSRC:-/root/velsub/device-pkg}"
ABISO="${ABISO:-/root/velsub/aegisub-upstream/build-android-arm64/tools/cross/libvelnosub-abi.so}"
UISO="${UISO:-/root/velsub/aegisub-upstream/velnosub-ui/target/aarch64-linux-android/release/libvelnosub_ui.so}"

echo "---- 0) giriş denetimi ----"
for f in "$UISO" "$ABISO"; do
    [ -f "$f" ] || { echo "EKSİK: $f"; exit 1; }
done
echo "ui NEEDED:"; "$RE" -d "$UISO" | grep NEEDED

echo "---- 1) assets (font + örnek ass; Lua pump UI'da YOK — ABI FFI doğrudan) ----"
rm -rf assets res_tmp
mkdir -p assets
cp -f "$PKGSRC"/DejaVuSans.ttf "$PKGSRC"/karaskel-dene.ass "$PKGSRC"/res-yarim.ass assets/
echo "assets: $(find assets -type f | wc -l) dosya"

echo "---- 2) lib/arm64-v8a (kapanış kopyası + strip) ----"
rm -rf lib
mkdir -p lib/arm64-v8a
for ad in libvelnosub-core.so libfreetype.so libfribidi.so libharfbuzz.so libuchardet.so libc++_shared.so; do
    cp -f "$PKGSRC"/lib/"$ad" lib/arm64-v8a/
done
cp -f "$ABISO" lib/arm64-v8a/libvelnosub-abi.so
cp -f "$UISO"  lib/arm64-v8a/libvelnosub_ui.so
for f in lib/arm64-v8a/*; do
    "$STRIP" --strip-unneeded -o lib/arm64-v8a/$(basename "$f").stripped "$f" && mv "$f.stripped" "$f"
done
find lib/arm64-v8a -exec chmod 755 {} +
ls -l lib/arm64-v8a | awk '{print $5, $9}'

echo "---- 3) aapt (Debian arm64 — Google build-tools x86_64 bu hatta koşmaz) ----"
rm -f velnosub-ui-unsigned.apk
aapt package -f \
      --min-sdk-version 24 --target-sdk-version 28 \
      -M AndroidManifest.xml \
      -S res \
      -I "$PF/android.jar" \
      -A assets \
      -F velnosub-ui-unsigned.apk

echo "---- 4) lib ekleme (dex yok: hasCode=false) ----"
zip -q -rg velnosub-ui-unsigned.apk lib

echo "---- 5) zipalign (Debian arm64) ----"
zipalign -f 4 velnosub-ui-unsigned.apk velnosub-ui-aligned.apk

echo "---- 6) apksigner (alpha keystore ile aynı kimlik ailesi) ----"
"$BT/apksigner" sign --ks ../apk-alpha/velnosub-debug.keystore \
      --ks-key-alias velnosubdebug --ks-pass pass:velnosub \
      --out velnosub-ui-0.1.0-alpha.apk velnosub-ui-aligned.apk

"$BT/apksigner" verify velnosub-ui-0.1.0-alpha.apk && echo "APK İMZA GEÇERLİ"
ls -l velnosub-ui-0.1.0-alpha.apk
