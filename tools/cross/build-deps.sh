#!/usr/bin/env bash
# Velsub Faz 2 — Android cross bağımlılık kurulumu (ICU + Boost)
#
# Çıktı: <WORK>/ndk-deps/<abi>/  (arm64-v8a | x86_64) — meson cross dosyaları
# bu prefix'i pkg-config ile kullanır. ICU önce host'ta cross-üretim araçları
# için derlenir (ICU küçük araç zinciri gerektirir), sonra hedef ABI'ye kurulur.
# Boost: filesystem + regex + locale (+system otomatik) statik kurulur; Locale
# ICU backend'iyle bağlanır (karaoke_matcher / util boost::locale kullanır).
#
# Kullanım:
#   NDK_ROOT=/root/velsub/ndk/android-ndk-r29 tools/cross/build-deps.sh arm64 x86_64
mkdir -p "$WORK"
set -euo pipefail

DEPS_ROOT=${DEPS_ROOT:-/root/velsub/ndk-deps}
WORK=${WORK:-$DEPS_ROOT/work}
API=${API:-29}
ICU_VER=75
BOOST_VER=1.84.0
SRC=$WORK/src
mkdir -p "$SRC" "$DEPS_ROOT"

NDK_ROOT=${NDK_ROOT:?NDK_ROOT is required (e.g. /root/velsub/ndk/android-ndk-r29)}
TOOLBIN=$(ls -d "$NDK_ROOT"/toolchains/llvm/prebuilt/linux-*/bin | head -1)
export PATH="$TOOLBIN:$PATH"

DL() { # url out
    [ -s "$2" ] || curl -sL -o "$2" "$1"
}

case "$1" in
arm64) ABOLU="aarch64"; TRIP=aarch64-linux-android; ABI_DIR=arm64-v8a ;;
x86_64) ABOLU="x86_64"; TRIP=x86_64-linux-android; ABI_DIR=x86_64 ;;
*) echo "usage: $0 arm64|x86_64"; exit 2 ;;
esac

PREFIX=$DEPS_ROOT/$ABI_DIR
mkdir -p "$PREFIX"

# ---------- ICU ----------
if [ ! -f "$PREFIX/lib/libicuuc.a" ] && [ ! -f "$PREFIX/lib/libicuuc.so" ]; then
    DL "https://github.com/unicode-org/icu/releases/download/release-$(echo "$ICU_VER" | tr -d '.')-1/icu4c-${ICU_VER}-1-src.tgz" "$SRC/icu4c-src.tgz"
    [ -d "$SRC/icu4c" ] || tar -xzf "$SRC/icu4c-src.tgz" -C "$SRC"

    # host (ana makine) derlemesi — cross araçları (genrb vb.) için gereklidir
    if [ ! -x "$SRC/icu-host/bin/genrb" ]; then
        mkdir -p "$SRC/icu-host-build"
        (cd "$SRC/icu-host-build" && CXXFLAGS="-std=c++17" ../icu4c/source/configure --enable-static --disable-shared --prefix="$SRC/icu-host" >/dev/null && make -j"$(nproc)" -s install >/dev/null)
    fi

    mkdir -p "$SRC/icu-cross-$ABI_DIR"
    (cd "$SRC/icu-cross-$ABI_DIR" && \
        ../icu4c/source/configure \
            --host="$TRIP" \
            --with-cross-build="$SRC/icu-host-build" \
            --prefix="$PREFIX" \
            --with-data-packaging=static \
            --disable-shared --enable-static \
            --disable-tests --disable-samples --disable-tools --disable-extras \
            CFLAGS="-Os" CXXFLAGS="-Os -std=c++17" LDFLAGS="-static" \
        >/dev/null && \
        make -j"$(nproc)" -s >/dev/null && make -s install >/dev/null)
fi

# ---------- Boost ----------
if [ ! -d "$PREFIX/include/boost" ]; then
    DL "https://archives.boost.io/release/$BOOST_VER/source/boost_${BOOST_VER//./_}.tar.bz2" "$SRC/boost.tar.bz2"
    [ -d "$SRC/boost-${BOOST_VER//./_}" ] || tar -xjf "$SRC/boost.tar.bz2" -C "$SRC"

    pushd "$SRC/boost-${BOOST_VER//./_}" >/dev/null
    cat > user-config-android.jam <<EOF
using clang : android : $TRIP$API-clang++ ;
EOF
    ./b2 --user-config=user-config-android.jam toolset=clang-android \
        target-os=android link=static threading=multi \
        cxxflags="-D__ANDROID_API__=$API -Os" \
        --without-python --without-mpi \
        --with-filesystem --with-regex --with-locale \
        --prefix="$PREFIX" install
    popd >/dev/null
fi

echo "deps ready: $PREFIX"
ls "$PREFIX/lib/pkgconfig" 2>/dev/null || true
