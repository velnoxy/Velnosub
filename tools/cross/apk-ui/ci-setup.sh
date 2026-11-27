#!/usr/bin/env bash
# velnosub — GitHub Actions'ta tam APK derleme zinciri (root-layout replika).
# PHASE=deps|native|rust|apk|test-android|all
# NDK_ROOT, BT ve PF Android araçlarının kurulu olduğu dizinleri gösterir.
set -euo pipefail

PHASE=${PHASE:-all}
REPO=${REPO:-$(cd "$(dirname "$0")/../../.." && pwd)}
ROOT=${ROOT:-$REPO/.android-build}
NDK_ROOT=${NDK_ROOT:-$ROOT/ndk/r29}
export NDK_ROOT
export DEPS_ROOT=${DEPS_ROOT:-$ROOT/ndk-deps}
API=29
export API
BT=${BT:-${ANDROID_HOME:-/usr/local/lib/android/sdk}/build-tools/34.0.0}
PF=${PF:-${ANDROID_HOME:-/usr/local/lib/android/sdk}/platforms/android-34}
JOBS=${JOBS:-$(nproc)}
ABI=${ABI:-arm64-v8a}
case "$ABI" in
    arm64-v8a) CROSS_ARCH=arm64; TARGET=aarch64-linux-android ;;
    x86_64) CROSS_ARCH=x86_64; TARGET=x86_64-linux-android ;;
    *) echo "Desteklenmeyen Android ABI: $ABI" >&2; exit 2 ;;
esac
BUILD=build-android-$CROSS_ARCH
TOOLBIN=$(echo "$NDK_ROOT"/toolchains/llvm/prebuilt/*/bin | head -1)
export PATH=$TOOLBIN:$PATH
export PKG_CONFIG_PATH=$DEPS_ROOT/$ABI/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}

# Geçmiş deps cache'i oboe.pc'de "-log" (tek l) taşıyor: ld bunu libog.so
# sanıp "unable to find library -log" veriyor. Doğrusu -llog; kaynak script
# düzeltildi, eski cache'i burada iyileştir (yeni cache'te etkisiz).
if [ -f "$DEPS_ROOT/$ABI/lib/pkgconfig/oboe.pc" ]; then
    sed -i 's/ -log / -llog /g' "$DEPS_ROOT/$ABI/lib/pkgconfig/oboe.pc"
fi

mkdir -p "$ROOT/device-pkg/$ABI"
if [ ! -x "$TOOLBIN/$TARGET${API}-clang" ]; then
    echo "NDK derleyicisi bulunamadı: $TOOLBIN"
    exit 1
fi
cd "$REPO"

echo "== ci-setup PHASE=$PHASE (root=$ROOT) =="

phase_deps() {
    echo "---- deps: meson araçları + wrap kaynakları ----"
    command -v meson >/dev/null || \
        (sudo apt-get update -y && sudo apt-get install -y meson ninja-build pkg-config unzip curl)
    # wrap'ler (luajit + alt-projeler) repoda KAYNAK barındırmaz — CI'da çekilir
    meson subprojects download luajit
    echo "---- deps: ICU+Boost ----"
    bash tools/cross/build-deps.sh "$CROSS_ARCH"
    echo "---- deps: ffmpeg 6.1.2 ----"
    bash tools/cross/build-ffmpeg.sh "$CROSS_ARCH"
    echo "---- deps: luajit ----"
    bash tools/cross/build-luajit.sh "$CROSS_ARCH"
    echo "---- deps: oboe ----"
    bash tools/cross/build-oboe.sh "$CROSS_ARCH"
}

phase_native() {
    command -v meson >/dev/null || \
        (sudo apt-get update -y && sudo apt-get install -y meson ninja-build pkg-config libfontconfig1-dev gettext)
    if [ ! -f "$BUILD/build.ninja" ]; then
        # cihaz build diziniyle BİREBİR aynı özellik kümesi: kullanılmayan
        # video/ses/spell kaynakları (ffms2, vapoursynth, bestsource, hunspell,
        # fftw3, openal/portaudio/pulse/alsa) KAPALI — aksi halde meson auto ile
        # subproject'leri açıyor ve cihazda hiç derlenmeyen hedefler CI'da patlıyor
        local cross=$ROOT/android-$CROSS_ARCH.txt
        python3 - "$cross" "$DEPS_ROOT" "$CROSS_ARCH" <<'PYEOF'
import pathlib
import sys

source = pathlib.Path(f"tools/cross/android-{sys.argv[3]}.txt").read_text()
pathlib.Path(sys.argv[1]).write_text(source.replace("/root/velsub/ndk-deps", sys.argv[2]))
PYEOF
        local libass_options=()
        # Android x86_64 Meson cross builds cannot compile libass NASM as PIE.
        if [ "$ABI" = x86_64 ]; then
            libass_options+=(-Dlibass:asm=disabled)
        fi
        meson setup "$BUILD" \
            -Dvelnosub_mobile=true \
            -Dffms2=disabled -Dvapoursynth=disabled -Dbestsource=disabled \
            -Davisynth=disabled -Dhunspell=disabled -Dfftw3=disabled \
            -Dalsa=disabled -Dopenal=disabled -Dportaudio=disabled \
            -Dlibpulse=disabled -Dlocal_boost=false \
            -Denable_update_checker=false -Dbuild_appimage=false \
            -Dlibass:require-system-font-provider=false \
            "${libass_options[@]}" \
            --cross-file "$cross"
    fi
    ninja -C "$BUILD" -j"$JOBS" libvelnosub-core.so tools/cross/libvelnosub-abi.so
}

phase_rust() {
    (
        cd velnosub-ui
        linker_var="CARGO_TARGET_${TARGET^^}_LINKER"
        export "${linker_var//-/_}=$TOOLBIN/$TARGET${API}-clang"
        cargo build --locked --release --target "$TARGET"
    )
}

phase_apk() {
    local DB=$ROOT/device-pkg/$ABI
    local seed=$REPO/tools/cross/apk-ui/seed
    local B=$REPO/$BUILD
    mkdir -p "$DB/lib" "$DB/share"
    cp -f "$seed"/default_config.json "$seed"/DejaVuSans.ttf \
          "$seed"/karaskel-dene.ass "$seed"/res-yarim.ass \
          "$seed"/ass-format-tests.ass "$DB"/
    if [ -d "$seed/share/lua" ]; then
        rm -rf "$DB/share/lua"; cp -rf "$seed/share/lua" "$DB/share/"
    fi
    cp "$REPO"/automation/include/*.lua "$DB/share/lua/inc/"
    cp -r "$REPO/automation/include/aegisub" "$DB/share/lua/inc/"
    # paket kütüphaneleri: subprojects ürünleri + ABI/core/UI + sysroot c++
    cp -f "$B/subprojects/libpng-1.6.37/libpng16.so"    "$DB/lib/"
    cp -f "$B/subprojects/freetype2/libfreetype.so"      "$DB/lib/"
    cp -f "$B/subprojects/fribidi/lib/libfribidi.so"     "$DB/lib/"
    cp -f "$B/subprojects/harfbuzz/src/libharfbuzz.so"   "$DB/lib/"
    cp -f "$B/subprojects/uchardet-0.0.7/src/libuchardet.so" "$DB/lib/" 2>/dev/null || true
    cp -f "$TOOLBIN/../sysroot/usr/lib/$TARGET/libc++_shared.so" "$DB/lib/"
    cp -f "$B/libvelnosub-core.so"                       "$DB/lib/"
    cp -f "$B/tools/cross/libvelnosub-abi.so"            "$DB/lib/"
    cp -f   "$REPO/velnosub-ui/target/$TARGET/release/libvelnosub_ui.so" "$DB/lib/"
    ( cd "$REPO/tools/cross/apk-ui" \
      && ABI="$ABI" BT="$BT" PF="$PF" NDKBIN="$TOOLBIN" PKGSRC="$DB" \
         ABISO="$B/tools/cross/libvelnosub-abi.so" \
         UISO="$REPO/velnosub-ui/target/$TARGET/release/libvelnosub_ui.so" \
         bash build-apk.sh
    )
    echo "APK: $(ls -l "$REPO/tools/cross/apk-ui"/velnosub-ui-*.apk | tail -2)"
}

phase_test_android() (
    local libraries cargo_log test_binary
    libraries=$(cd "$ROOT/device-pkg/$ABI/lib" && pwd -P)
    local linker_var="CARGO_TARGET_${TARGET^^}_LINKER"
    export "${linker_var//-/_}=$TOOLBIN/$TARGET${API}-clang"
    export VELNOSUB_ABI_DIR="$libraries"
    export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-Wl,-rpath-link,$libraries"
    cargo_log=$(mktemp)
    trap 'rm -f "$cargo_log"' EXIT

    cd "$REPO/velnosub-ui"
    cargo test --package velnosub-ui --release --locked --lib --target "$TARGET" --no-run --message-format=json > "$cargo_log" || {
        cat "$cargo_log"
        return 1
    }
    test_binary=$(python3 - "$cargo_log" <<'PYEOF'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as messages:
    for line in messages:
        item = json.loads(line)
        if (item.get("reason") == "compiler-artifact"
                and item["target"]["name"] == "velnosub_ui"
                and item["profile"]["test"] and item["executable"]):
            print(item["executable"])
            break
    else:
        sys.exit("Android Rust test ikilisi bulunamadı")
PYEOF
)
    bash "$REPO/tools/cross/apk-ui/test-adb.sh" "$test_binary" "$libraries"
)

case "$PHASE" in
    deps)   phase_deps ;;
    native) phase_native ;;
    rust)   phase_rust ;;
    apk)    phase_apk ;;
    test-android) phase_test_android ;;
    all)    phase_deps; phase_native; phase_rust; phase_apk ;;
    *) echo "kullanım: PHASE=deps|native|rust|apk|test-android|all $0"; exit 2 ;;
esac
echo "ci-setup: PHASE=$PHASE TAMAM"
