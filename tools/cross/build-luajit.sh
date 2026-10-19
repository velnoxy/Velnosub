#!/usr/bin/env bash
# velnosub Faz 5 (P5.0) — LuaJIT 2.1 derleme: host + Android ABI statik.
# Kaynak: subprojects/luajit (wrap revizyonu v2.1). Çıktıda luajit.pc yazılır;
# meson pkg-config'i (pkg_config_libdir veya PKG_CONFIG_PATH ile) otomatik bulur.
# Kullanım:
#   tools/cross/build-luajit.sh host           # /root/velsub/ndk-deps/luajit-host
#   tools/cross/build-luajit.sh arm64          # ndk-deps/arm64-v8a içine
#   tools/cross/build-luajit.sh x86_64         # ndk-deps/x86_64 içine
set -euo pipefail

DEPS_ROOT=${DEPS_ROOT:-/root/velsub/ndk-deps}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
LUASRC=$ROOT/subprojects/luajit
WORK=$DEPS_ROOT/work/luajit
API=${API:-29}
JOBS=${JOBS:-$(nproc)}

[ -d "$LUASRC/src" ] || { echo "eksik: $LUASRC/src yok (meson wrap güncellenmedi)"; exit 1; }

mode=${1:?kullanım: $0 host|arm64|x86_64}
mkdir -p "$WORK"

install_pc() { # $1=libdir $2=libad(a) $3=prefix
    local libdir=$1 libname=$2 prefix=$3
    mkdir -p "$libdir/pkgconfig"
    cat > "$libdir/pkgconfig/luajit.pc" <<EOF
prefix=$prefix
exec_prefix=\${prefix}
libdir=$libdir
includedir=$prefix/include

Name: LuaJIT
Description: velnosub Faz 5 zemin: LuaJIT 2.1 (statik)
Version: 2.1
Cflags: -I$prefix/include/luajit-2.1
Libs: -L$libdir -l$libname -lm
EOF
}

case "$mode" in
host)
    BUILD=$WORK/build-host
    PREFIX=$DEPS_ROOT/luajit-host
    rm -rf "$BUILD" "$PREFIX"
    mkdir -p "$BUILD" "$PREFIX"
    cp -r "$LUASRC/src/." "$BUILD/"
    (cd "$BUILD" && make -j"$JOBS" -s BUILDMODE=static >/dev/null)
    (cd "$BUILD" && make -s install PREFIX="$PREFIX" >/dev/null)
    LN=$(ls "$PREFIX/lib"/libluajit*.a | head -1 | xargs basename | sed 's/^lib//; s/\.a$//')
    install_pc "$PREFIX/lib" "$LN" "$PREFIX"
    echo "luajit host hazır: $PREFIX (lib: $LN)"
    ;;
arm64|x86_64)
    case "$mode" in
        arm64) TRIP=aarch64-linux-android; ABI_DIR=arm64-v8a ;;
        x86_64) TRIP=x86_64-linux-android; ABI_DIR=x86_64 ;;
    esac
    TOOLBIN=$(echo /root/velsub/ndk/r29/toolchains/llvm/prebuilt/*/bin | head -1)
    CCX=$TOOLBIN/$TRIP$API-clang
    [ -x "$CCX" ] || { echo "eksik: $CCX"; exit 1; }
    BUILD=$WORK/build-$mode
    rm -rf "$BUILD"
    mkdir -p "$BUILD"
    cp -r "$LUASRC/src/." "$BUILD/"
    # LuaJIT kuralları: CC = hedef derleyici, HOST_CC = host araçları
    # (minilua/buildvm aarch64 host gcc ile); statik mod yalnız .a verir.
    (cd "$BUILD" && make -j"$JOBS" -s \
        CC="$CCX" HOST_CC=cc \
        TARGET_SYS="Linux" \
        BUILDMODE=static \
        TARGET_AR="llvm-ar rcus" \
        TARGET_STRIP="llvm-strip" \
        TARGET_CFLAGS="-D__ANDROID_API__=$API" >/dev/null)
    PREFIX=$DEPS_ROOT/$ABI_DIR
    (cd "$BUILD" && make -s install PREFIX="$PREFIX" >/dev/null)
    LN=$(ls "$PREFIX/lib"/libluajit*.a | head -1 | xargs basename | sed 's/^lib//; s/\.a$//')
    install_pc "$PREFIX/lib" "$LN" "$PREFIX"
    echo "luajit $mode hazır: $PREFIX (lib: $LN)"
    ;;
*)
    echo "kullanım: $0 host|arm64|x86_64"; exit 2 ;;
esac
