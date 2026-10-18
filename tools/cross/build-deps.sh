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
set -euo pipefail

DEPS_ROOT=${DEPS_ROOT:-/root/velsub/ndk-deps}
WORK=${WORK:-$DEPS_ROOT/work}
API=${API:-29}
ICU_VER=75
BOOST_VER=1.84.0
SRC=$WORK/src
mkdir -p "$SRC" "$DEPS_ROOT"

NDK_ROOT=${NDK_ROOT:?NDK_ROOT is required (e.g. /root/velsub/ndk/r29)}
TOOLBIN=$(echo "$NDK_ROOT"/toolchains/llvm/prebuilt/*/bin | head -1)
[ -d "$TOOLBIN" ] || { echo "NDK toolchain bin dir not found under $NDK_ROOT"; exit 1; }
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
[ -x "$TOOLBIN/$TRIP$API-clang++" ] || { echo "missing $TOOLBIN/$TRIP$API-clang++"; exit 1; }

# ---------- ICU ----------
if [ ! -f "$PREFIX/lib/libicuuc.a" ] && [ ! -f "$PREFIX/lib/libicuuc.so" ]; then
    DL "https://github.com/unicode-org/icu/releases/download/release-$(echo "$ICU_VER" | tr -d '.')-1/icu4c-${ICU_VER}_1-src.tgz" "$SRC/icu4c-src.tgz"
    [ -d "$SRC/icu" ] || tar -xzf "$SRC/icu4c-src.tgz" -C "$SRC"

    # host (ana makine) derlemesi — cross araçları (genrb vb.) için gereklidir
    if [ ! -x "$SRC/icu-host-build/bin/genrb" ] && [ ! -x "$SRC/icu-host-build/stubdata/genrb" ]; then
        mkdir -p "$SRC/icu-host-build"
        (cd "$SRC/icu-host-build" && CXXFLAGS="-std=c++17" "$SRC/icu/source/configure" --prefix="$SRC/icu-host" >/dev/null 2>&1 && make -j"$(nproc)" -s >/dev/null 2>&1 && make -s install >/dev/null 2>&1)
    fi

# PIC zorunlu: velsub-core.so statik arşivleri shared'a taşır
    mkdir -p "$SRC/icu-cross-$ABI_DIR"
    (cd "$SRC/icu-cross-$ABI_DIR" && \
        CC="$TRIP$API-clang" CXX="$TRIP$API-clang++" \
        AR=llvm-ar RANLIB=llvm-ranlib \
        "$SRC/icu/source/configure" \
            --host="$TRIP" \
            --with-cross-build="$SRC/icu-host-build" \
            --prefix="$PREFIX" \
            --with-data-packaging=static \
            --disable-shared --enable-static \
            --disable-tests --disable-samples --disable-tools --disable-extras \
            CFLAGS="-Os -fPIC" CXXFLAGS="-Os -std=c++17 -fPIC" \
        >/dev/null && \
        make -j"$(nproc)" -s >/dev/null && make -s install >/dev/null)
fi

# ---------- Boost ----------
if [ ! -d "$PREFIX/include/boost" ]; then
    DL "https://archives.boost.io/release/$BOOST_VER/source/boost_${BOOST_VER//./_}.tar.bz2" "$SRC/boost.tar.bz2"
    [ -d "$SRC/boost_${BOOST_VER//./_}" ] || tar -xjf "$SRC/boost.tar.bz2" -C "$SRC"

    pushd "$SRC/boost_${BOOST_VER//./_}" >/dev/null
    # b2 release tar'ın içi x86_64 önceden derlenmiş gelir; aarch64 host'ta
    # bootstrap.sh ile yeniden derlenir (idempotent).
    [ -x ./b2 ] || ./bootstrap.sh --with-toolset=gcc >/dev/null
    cat > user-config-android.jam <<EOF
using clang : android : $TRIP$API-clang++ ;
EOF
    ./b2 --user-config=user-config-android.jam toolset=clang-android \
        -j1 \
        target-os=android link=static threading=multi \
        cxxflags="-I$PREFIX/include -D__ANDROID_API__=$API -Os -fPIC" \
        linkflags="-L$PREFIX/lib" \
        --with-filesystem --with-regex --with-locale --with-chrono --with-thread \
        --prefix="$PREFIX" install
    popd >/dev/null

    # Boost 1.84 GIL başlıkları clang 22 strict TE rule'unda kırılıyor
    # ('-Wmissing-template-arg-list-after-template-kw'); GIL master'daki
    # düzeltme: fonksiyon şablon çağrısındaki gereksiz `template` düşer.
    sed -i 's/::template apply(/::apply(/g' "$PREFIX/include/boost/gil/algorithm.hpp"

    # b2 .pc üretmez; meson'un boost özel-yordamı pkg-config ile bulsun:
    # modül listesi root mesonda chrono/filesystem/thread/locale/regex+system
    # hepsi kurulu olmalı, aksi halde meson libleri düşürür.
    cat > "$PREFIX/lib/pkgconfig/boost.pc" <<EOF
prefix=$PREFIX
exec_prefix=\${prefix}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: boost
Description: Boost C++ Libraries (Android cross, static build)
Version: $BOOST_VER
Cflags: -I\${includedir}
Libs: -L\${libdir} -lboost_filesystem -lboost_regex -lboost_locale -lboost_atomic -lboost_chrono -lboost_thread
EOF
fi

echo "deps ready: $PREFIX"
ls "$PREFIX/lib/pkgconfig" 2>/dev/null || true
