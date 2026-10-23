#!/usr/bin/env bash
# velnosub — GitHub Actions'ta tam APK derleme zinciri (root-layout replika).
# Cihaz oturumundaki dizin hiyerarşisi (/root/velsub/…) birebir kurulur:
#   /root/velsub/aegisub-upstream → repo kökü symlink (meson cross-file +
#   build-apk.sh yol beyanları bu isimlere kilitli)
#   /root/velsub/ndk/r29, /root/velsub/ndk-deps, /root/velsub/device-pkg
# PHASE'bölme: deps (ICU+Boost+ffmpeg+luajit+oboe) → doğrudan artifact/ cache;
# native (meson + ninja); rust (cdylib); apk (device-pkg + build-apk.sh).
#   PHASE=all ./tools/cross/apk-ui/ci-setup.sh
set -euo pipefail

PHASE=${PHASE:-all}
REPO=${REPO:-$(cd "$(dirname "$0")/../../.." && pwd)}
ROOT=${ROOT:-/root/velsub}
NDK_ROOT=${NDK_ROOT:-$ROOT/ndk/r29}
export NDK_ROOT
DEPS_ROOT=${DEPS_ROOT:-$ROOT/ndk-deps}
API=29
BT=${BT:-/usr/local/lib/android/sdk/build-tools/34.0.0}
PF=${PF:-/usr/local/lib/android/sdk/platforms/android-34}
JOBS=${JOBS:-$(nproc)}
TOOLBIN=$(echo "$NDK_ROOT"/toolchains/llvm/prebuilt/*/bin | head -1)
export PATH=$TOOLBIN:$PATH
export PKG_CONFIG_PATH=$DEPS_ROOT/arm64-v8a/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}

# Geçmiş deps cache'i oboe.pc'de "-log" (tek l) taşıyor: ld bunu libog.so
# sanıp "unable to find library -log" veriyor. Doğrusu -llog; kaynak script
# düzeltildi, eski cache'i burada iyileştir (yeni cache'te etkisiz).
if [ -f "$DEPS_ROOT/arm64-v8a/lib/pkgconfig/oboe.pc" ]; then
    sed -i 's/ -log / -llog /g' "$DEPS_ROOT/arm64-v8a/lib/pkgconfig/oboe.pc"
fi

# --- /root/velsub hiyerarşisi (runner'da sudo ile kurdurulur) ---
if [ ! -d "$ROOT" ]; then
    sudo mkdir -p "$ROOT"
    sudo chown -R "$(id -u):$(id -g)" "$ROOT"
fi
[ -w "$ROOT" ] || sudo chown -R "$(id -u):$(id -g)" "$ROOT"
mkdir -p "$ROOT/ndk-deps" "$ROOT/ndk" "$ROOT/device-pkg"
[ -e "$ROOT/aegisub-upstream" ] || ln -sfn "$REPO" "$ROOT/aegisub-upstream"

mkdir -p "$REPO/tools/cross/apk-ui/seed"  # no-op garantisi

cd "$REPO"

echo "== ci-setup PHASE=$PHASE (root=$ROOT) =="

phase_deps() {
    echo "---- deps: meson araçları + wrap kaynakları ----"
    command -v meson >/dev/null || \
        (sudo apt-get update -y && sudo apt-get install -y meson ninja-build pkg-config unzip curl)
    # wrap'ler (luajit + alt-projeler) repoda KAYNAK barındırmaz — CI'da çekilir
    meson subprojects download || \
        echo "uyarı: bazı subproject kaynakları çekilemedi (repoda sabit dizin olanlar atlanır)"
    echo "---- deps: ICU+Boost ----"
    bash tools/cross/build-deps.sh arm64
    echo "---- deps: ffmpeg 6.1.2 ----"
    bash tools/cross/build-ffmpeg.sh arm64
    echo "---- deps: luajit ----"
    bash tools/cross/build-luajit.sh arm64
    echo "---- deps: oboe ----"
    bash tools/cross/build-oboe.sh arm64
}

phase_native() {
    command -v meson >/dev/null || \
        (sudo apt-get update -y && sudo apt-get install -y meson ninja-build pkg-config libfontconfig1-dev gettext)
    # güvenlik: wrap kaynakları taze checkout'ta eksikse çek
    meson subprojects download 2>/dev/null || true
    if [ ! -f build-android-arm64/build.ninja ]; then
        # cihaz build diziniyle BİREBİR aynı özellik kümesi: kullanılmayan
        # video/ses/spell kaynakları (ffms2, vapoursynth, bestsource, hunspell,
        # fftw3, openal/portaudio/pulse/alsa) KAPALI — aksi halde meson auto ile
        # subproject'leri açıyor ve cihazda hiç derlenmeyen hedefler CI'da patlıyor
        PATH=$TOOLBIN:$PATH meson setup build-android-arm64 \
            -Dvelnosub_mobile=true \
            -Dffms2=disabled -Dvapoursynth=disabled -Dbestsource=disabled \
            -Davisynth=disabled -Dhunspell=disabled -Dfftw3=disabled \
            -Dalsa=disabled -Dopenal=disabled -Dportaudio=disabled \
            -Dlibpulse=disabled -Dlocal_boost=false \
            -Denable_update_checker=false -Dbuild_appimage=false \
            -Dlibass:require-system-font-provider=false \
            --cross-file tools/cross/android-arm64.txt
    fi
    ninja -C build-android-arm64 -j"$JOBS" libvelnosub-core.so tools/cross/libvelnosub-abi.so
}

phase_rust() {
    (cd velnosub-ui && cargo build --release --target aarch64-linux-android)
}

phase_apk() {
    local DB=$ROOT/device-pkg
    local seed=$REPO/tools/cross/apk-ui/seed
    local B=$ROOT/build-android-arm64
    mkdir -p "$DB/lib" "$DB/share"
    cp -f "$seed"/default_config.json "$seed"/DejaVuSans.ttf \
          "$seed"/karaskel-dene.ass "$seed"/res-yarim.ass \
          "$seed"/ass-format-tests.ass "$DB"/
    if [ -d "$seed/share/lua" ]; then
        rm -rf "$DB/share/lua"; cp -rf "$seed/share/lua" "$DB/share/"
    fi
    # paket kütüphaneleri: subprojects ürünleri + ABI/core/UI + sysroot c++
    cp -f "$B/subprojects/libpng-1.6.37/libpng16.so"    "$DB/lib/"
    cp -f "$B/subprojects/freetype2/libfreetype.so"      "$DB/lib/"
    cp -f "$B/subprojects/fribidi/lib/libfribidi.so"     "$DB/lib/"
    cp -f "$B/subprojects/harfbuzz/src/libharfbuzz.so"   "$DB/lib/"
    cp -f "$B/subprojects/uchardet-0.0.7/src/libuchardet.so" "$DB/lib/" 2>/dev/null || true
    cp -f "$TOOLBIN/../sysroot/usr/lib/aarch64-linux-android/libc++_shared.so" "$DB/lib/"
    cp -f "$B/libvelnosub-core.so"                       "$DB/lib/"
    cp -f "$B/tools/cross/libvelnosub-abi.so"            "$DB/lib/"
    cp -f   "$REPO/velnosub-ui/target/aarch64-linux-android/release/libvelnosub_ui.so" "$DB/lib/"
    ( cd "$REPO/tools/cross/apk-ui" \
      && BT="$BT" PF="$PF" PKGSRC="$DB" \
         ABISO="$B/tools/cross/libvelnosub-abi.so" \
         UISO="$REPO/velnosub-ui/target/aarch64-linux-android/release/libvelnosub_ui.so" \
      && bash build-apk.sh
    )
    echo "APK: $(ls -l "$REPO/tools/cross/apk-ui"/velnosub-ui-*.apk | tail -2)"
}

case "$PHASE" in
    deps)   phase_deps ;;
    native) phase_native ;;
    rust)   phase_rust ;;
    apk)    phase_apk ;;
    all)    phase_deps; phase_native; phase_rust; phase_apk ;;
    *) echo "kullanım: PHASE=deps|native|rust|apk|all $0"; exit 2 ;;
esac
echo "ci-setup: PHASE=$PHASE TAMAM"
