#!/usr/bin/env bash
# velnosub P5.5 — Android Oboe (ses çıkışı) statik kütüphane kurumu.
# Çıktı: <DEPS_ROOT>/<abi>/lib/liboboe.a + pkgconfig/oboe.pc — meson
# dependency('oboe') bu .pc üzerinden link edilebilir hale gelir.
#
# Kullanım: tools/cross/build-oboe.sh arm64|x86_64
# Gerektirir: NDK root /root/velsub/ndk/r29 (+ cmake & ninja).
set -euo pipefail

OBOE_VER=1.9.0
DEPS_ROOT=${DEPS_ROOT:-/root/velsub/ndk-deps}
WORK=$DEPS_ROOT/work
SRC=$WORK/src
API=${API:-29}

mode=${1:?kullanım: $0 arm64|x86_64}
case "$mode" in
    arm64) TRIP=aarch64-linux-android; ABI_DIR=arm64-v8a; AND_ABI=arm64-v8a ;;
    x86_64) TRIP=x86_64-linux-android; ABI_DIR=x86_64; AND_ABI=x86_64 ;;
    *) echo "kullanım: $0 arm64|x86_64"; exit 2 ;;
esac
PREFIX=$DEPS_ROOT/$ABI_DIR
NDK_ROOT=${NDK_ROOT:-/root/velsub/ndk/r29}

DL() { [ -s "$2" ] || curl -sL -o "$2" "$1"; }

DL "https://github.com/google/oboe/archive/refs/tags/${OBOE_VER}.tar.gz" "$SRC/oboe-${OBOE_VER}.tar.gz"
KAYNAK=$SRC/oboe-$mode
BLD=$WORK/oboe-build-$mode
[ -d "$KAYNAK" ] || (tar -xzf "$SRC/oboe-${OBOE_VER}.tar.gz" -C "$SRC" && mv -f "$SRC/oboe-${OBOE_VER}" "$KAYNAK")

cmake -S "$KAYNAK" -B "$BLD" \
    -DCMAKE_TOOLCHAIN_FILE="$NDK_ROOT/build/cmake/android.toolchain.cmake" \
    -DANDROID_ABI="$AND_ABI" -DANDROID_PLATFORM=android-$API \
    -DCMAKE_BUILD_TYPE=Release -DOBOE_BUILD_TESTS=OFF \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" -DBUILD_SHARED_LIBS=OFF
cmake --build "$BLD" -j"${JOBS:-$(nproc)}"
cmake --install "$BLD"

mkdir -p "$PREFIX/lib/pkgconfig"
cat > "$PREFIX/lib/pkgconfig/oboe.pc" <<EOF
prefix=$PREFIX
includedir=\${prefix}/include
libdir=\${prefix}/lib

Name: oboe
Description: Oboe low-latency audio (static)
Version: ${OBOE_VER}
Cflags: -I\${includedir}
Libs: -L\${libdir} -loboe -lOpenSLES -log
EOF
# Android toolchain ${prefix}/lib/<abi>/ altına döker — düzleştir
if [ -f "$PREFIX/lib/$AND_ABI/liboboe.a" ]; then
    mv -f "$PREFIX/lib/$AND_ABI/liboboe.a" "$PREFIX/lib/liboboe.a"
fi
ls -l "$PREFIX/lib/liboboe.a" "$PREFIX/lib/pkgconfig/oboe.pc"
