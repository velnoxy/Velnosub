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
#   freetype → (png16, z);  png16 → (z)  — P4.8.1 v7: png16 paketlenir!
#   [2026-10-21 crash-buffer kanıtı: UnsatisfiedLinkError "libpng16.so not
#    found needed by libfreetype.so" — JDK tarafı çökme, tombstone'suz;
#    shizuku/rish ile okundu]
#   sistem küpü (c, dl, m, log, z) platformdadır — paketlenmez; c++_shared PAKETLENİR.
# Çıkış: tools/cross/apk-ui/velnosub-ui-0.1.0-alpha.apk (imzalı, hizalı)
set -e
cd "$(dirname "$0")"
BT=${BT:-/root/velsub/android-sdk/bt/android-14}
PF=${PF:-/root/velsub/android-sdk/pf/android-34}
NDKBIN=${NDKBIN:-$(echo /root/velsub/ndk/r29/toolchains/llvm/prebuilt/*/bin | head -1)}
STRIP="$NDKBIN/llvm-strip"
RE="$NDKBIN/llvm-readelf"
AAPT="${AAPT:-$(command -v aapt || echo "$BT/aapt")}"
ZIPALIGN="${ZIPALIGN:-$(command -v zipalign || echo "$BT/zipalign")}"
APKSIGNER="${APKSIGNER:-$(command -v apksigner || echo "$BT/apksigner")}"
PKGSRC="${PKGSRC:-/root/velsub/device-pkg}"
ABISO="${ABISO:-/root/velsub/aegisub-upstream/build-android-arm64/tools/cross/libvelnosub-abi.so}"
UISO="${UISO:-/root/velsub/aegisub-upstream/velnosub-ui/target/aarch64-linux-android/release/libvelnosub_ui.so}"

echo "---- 0) giriş denetimi ----"
# REL=1 → release modu: versionCode 13 / versionName 0.1.0 / debuggable
# YOK / çıktı velnosub-ui-0.1.0.apk (alpha zincirinin ÜSTÜNE kurulabilir:
# aynı keystore). Res/mipmap-* release ikondan basılıdır (PIL kelimesi).
if [ "${REL:-0}" = "1" ]; then
    # DİKKAT: bu aapt porte'si -M dosyasının adının TAM "AndroidManifest.xml"
    # olmasını istiyor (byte-aynı başka ad = "No AndroidManifest.xml found");
    # o yüzden rel manifest ayrı DİZİN'de aynı adla basılır
    mkdir -p rel-mod
    sed -e 's/android:versionCode="[0-9]*"/android:versionCode="13"/' \
        -e 's/android:versionName="[^"]*"/android:versionName="0.1.0"/' \
        -e 's/ *android:debuggable="true"//' AndroidManifest.xml > rel-mod/AndroidManifest.xml
    MANIFEST=rel-mod/AndroidManifest.xml
    OUT=velnosub-ui-0.1.0.apk
    # ikon kuralı (kullanıcı ruling): alpha çağında res/ NIGHTLY ikon;
    # düz sürümde (REL=1) res_rel/ RENKLİ ikon basılır (ikon-release.png)
    rm -rf res_rel
    cp -r res res_rel
    python3 - <<'PYEOF'
from PIL import Image
src = "/root/velsub/aegisub-upstream/tools/cross/apk-ui/ikon-release.png"
res = "/root/velsub/aegisub-upstream/tools/cross/apk-ui/res_rel"
base = Image.open(src).convert("RGBA")
for d, px in [("mdpi",48),("hdpi",72),("xhdpi",96),("xxhdpi",144),("xxxhdpi",192)]:
    base.resize((px,px), Image.LANCZOS).save(
        f"{res}/mipmap-{d}/ic_launcher.png", optimize=True)
print("ikon: release (renkli) 5 yoğunluk")
PYEOF
    RES=res_rel
    echo "MOD: RELEASE (code 13, 0.1.0, debuggable yok, renkli ikon)"
else
    MANIFEST=AndroidManifest.xml
    RES=res
    OUT=velnosub-ui-0.1.1-alpha12.apk
    echo "MOD: ALPHA (code 16, 0.1.1-ui-alpha12, debuggable true, nightly ikon)"
fi
for f in "$UISO" "$ABISO"; do
    [ -f "$f" ] || { echo "EKSİK: $f"; exit 1; }
done
echo "ui NEEDED:"; "$RE" -d "$UISO" | grep NEEDED

echo "---- 1) assets (font + örnek ass + GÖMÜLÜ otomasyon zinciri) ----"
# her şey gömülü kuralı (kullanıcı ruling): Lua vendor+autos, config, shim,
# örnek ass'ler — UI uygulaması pompaj alpha'sından bağımsız kendi içinde
# tam otomasyon malzemesini taşır (in-app pump ayrı dilim)
rm -rf assets res_tmp
mkdir -p assets/velnosub/share
cp -f "$PKGSRC"/default_config.json "$PKGSRC"/DejaVuSans.ttf \
      "$PKGSRC"/ass-format-tests.ass "$PKGSRC"/karaskel-dene.ass \
      "$PKGSRC"/res-yarim.ass assets/velnosub/
cp -rf "$PKGSRC"/share/lua assets/velnosub/share/
mkdir -p assets/velnosub/tools
cp -f /root/velsub/aegisub-upstream/tools/cross/velnosub-aegisub.lua assets/velnosub/tools/
echo "assets: $(find assets -type f | wc -l) dosya"

echo "---- 2) lib/arm64-v8a (kapanış kopyası + strip) ----"
rm -rf lib
mkdir -p lib/arm64-v8a
for ad in libvelnosub-core.so libfreetype.so libfribidi.so libharfbuzz.so libuchardet.so libpng16.so libc++_shared.so; do
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
"$AAPT" package -f \
      --min-sdk-version 24 --target-sdk-version 28 \
      -M "$MANIFEST" \
      -S "$RES" \
      -I "$PF/android.jar" \
      -A assets \
      -F velnosub-ui-unsigned.apk

echo "---- 4) lib ekleme (dex yok: hasCode=false) ----"
zip -q -rg velnosub-ui-unsigned.apk lib

echo "---- 5) zipalign (Debian arm64) ----"
"$ZIPALIGN" -f 4 velnosub-ui-unsigned.apk velnosub-ui-aligned.apk

echo "---- 6) apksigner (alpha keystore ile aynı kimlik ailesi) ----"
"$APKSIGNER" sign --ks ../apk-alpha/velnosub-debug.keystore \
      --ks-key-alias velnosubdebug --ks-pass pass:velnosub \
      --out "$OUT" velnosub-ui-aligned.apk

"$APKSIGNER" verify "$OUT" && echo "APK İMZA GEÇERLİ"

echo "---- 7) DT_NEEDED kapanış denetimi (PAKET İÇİ çözüm) ----"
# Her lib'in NEEDED'ı paket içinde veya platform küpünde çözümlenmeli;
# eksik kalırsa build BAŞARISIZ bitmeli (clns UnsatisfiedLinkError'ı ölmez)
P_SISTEM="libc.so libdl.so libm.so liblog.so libz.so libandroid.so"
HATA=0
for f in lib/arm64-v8a/*.so; do
    for n in $("$RE" -d "$f" | grep NEEDED | sed 's/.*\[//;s/\].*//'); do
        if [ ! -f "lib/arm64-v8a/$n" ]; then
            case " $P_SISTEM " in
                *" $n "*) ;;
                *) echo "KAPANIŞ HATASI: $f → $n YOK"; HATA=1 ;;
            esac
        fi
    done
done
[ "$HATA" -eq 0 ] && echo "DT_NEEDED KAPANIŞ DENETİMİ: TAM (arm64-v8a)"

echo "---- 8) SEMBOL kapanış denetimi (UND → paket içinde tanım) ----"
# DT_NEEDED tek başına YETMEZ (P5.4 ikinci kanıt: abi.so 'velnosub_core_set_line_meta'
# istedi ama paketteki core eskiydi → dlopen UnsatisfiedLinkError proses-öldürücü).
# Kural: gerçek-proje sembol uzayları (velnosub_*, ass_*) için her UND paketteki
# BİR .so'da FUNC tanımlı olmalı; yoksa build BAŞARISIZ.
# NB: readelf kolonları: Num: Value Size Type Bind Vis Ndx Name → UND sütunu
# Bind ($) DEĞİL Ndx'tedir; llvm-li okumada $5 Bind olduğu için $5!="UND"
# hep-yanlış-geçoneliydi; doğrusu $(NF-1) (Ndx).
DEFD=$(for f in lib/arm64-v8a/*.so; do
    "$RE" --dyn-syms "$f" 2>/dev/null | awk '$4=="FUNC" && $(NF-1)!="UND" {print $NF}'
done | sed 's/@.*//' | sort -u)
for f in lib/arm64-v8a/*.so; do
    for n in $("$RE" --dyn-syms "$f" 2>/dev/null \
        | awk '$4=="FUNC" && $(NF-1)=="UND" {print $NF}' \
        | sed 's/@.*//' | sort -u); do
        case "$n" in
            velnosub_*|ass_*) : ;;
            *) continue ;;
        esac
        if ! printf '%s\n' "$DEFD" | grep -qx "$n"; then
            echo "SEMBOL KAPANIŞ HATASI: $f istenen $n pakette YOK"; HATA=1
        fi
    done
done
[ "$HATA" -eq 0 ] && echo "SEMBOL KAPANIŞ DENETİMİ: TAM (velnosub_/ass_ uzayı)"
find lib/arm64-v8a -name "*.so" -printf "%f %s\n" | sort
ls -l "$OUT"
