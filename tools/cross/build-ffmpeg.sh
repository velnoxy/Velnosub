#!/usr/bin/env bash
# velnosub Faz 3 (P3.0) — FFmpeg 6.1.2 statik headless cross derleme.
# Seçili demuxer/decoder/parser seti mobil önizleme için yeterli dardır
# (tarama ve ağ katmanı kapalı). Çıktı: <DEPS_ROOT>/<abi>/  (avcodec/avformat/
# avutil/swscale/swresample statik .a + .pc — pkg_config_libdir hattıyla meson
# bulur) velnosub-video-probe (P3.1) ve mobil video sağlayıcı (P3.2) bağlanır.
#
# Kullanım: tools/cross/build-ffmpeg.sh arm64|x86_64
# Not: build kırıntısı varsa WORK/ffmpeg-build-<abi> dizinini elle silin
# (script çıkarımını yeniden koşmaz).
set -euo pipefail

DEPS_ROOT=${DEPS_ROOT:-/root/velsub/ndk-deps}
WORK=$DEPS_ROOT/work
SRC=$WORK/src
API=${API:-29}
JOBS=${JOBS:-$(nproc)}

mode=${1:?kullanım: $0 arm64|x86_64}
case "$mode" in
    arm64) TRIP=aarch64-linux-android; ABI_DIR=arm64-v8a; ARCH=aarch64; XASM="" ;;
    x86_64) TRIP=x86_64-linux-android; ABI_DIR=x86_64; ARCH=x86_64; XASM="--disable-x86asm" ;;
    *) echo "kullanım: $0 arm64|x86_64"; exit 2 ;;
esac

FF_VER=6.1.2
mkdir -p "$SRC" "$WORK"
PREFIX=$DEPS_ROOT/$ABI_DIR

DL() { [ -s "$2" ] || curl -sL -o "$2" "$1"; }

TARBALL=$SRC/ffmpeg-$FF_VER.tar.xz
DL "https://ffmpeg.org/releases/ffmpeg-$FF_VER.tar.xz" "$TARBALL"
BUILD=$WORK/ffmpeg-build-$mode
[ -d "$BUILD" ] || tar -xf "$TARBALL" --transform "s|ffmpeg-$FF_VER|ffmpeg-build-$mode|" -C "$WORK"

TOOLBIN=$(echo /root/velsub/ndk/r29/toolchains/llvm/prebuilt/*/bin | head -1)
CCX=$TOOLBIN/$TRIP$API-clang
[ -x "$CCX" ] || { echo "eksik: $CCX"; exit 1; }

(cd "$BUILD" && \
CC="$CCX" ./configure \
    --prefix="$PREFIX" \
    --arch="$ARCH" --target-os=linux --enable-cross-compile \
    --cc="$CCX" --ar="llvm-ar" --ranlib="llvm-ranlib" \
    --nm="llvm-nm" --strip="llvm-strip" \
    --extra-cflags="-Os -fPIC -D__ANDROID_API__=$API" \
    --extra-ldflags="-D__ANDROID_API__=$API" \
    --disable-shared --enable-static --enable-pic \
    --disable-programs --disable-doc --disable-network --disable-avdevice \
    --disable-postproc --disable-avfilter --disable-autodetect \
    --enable-swresample --enable-swscale \
    --enable-demuxer=mov,matroska,avi,ogg,flv,wav,ass,srt \
    --enable-decoder=h264,hevc,vp8,vp9,aac,mp3,flac,opus,vorbis,pcm_s16le,pcm_f32le,srt \
    --enable-parser=h264,hevc,aac,vorbis,opus,flac \
    --enable-protocol=file \
    $XASM \
    > "$WORK/ffmpeg-configure-$mode.log" 2>&1) || { tail -20 "$WORK/ffmpeg-configure-$mode.log"; exit 1; }

(cp tools/cross/ffversion-fast.sh "$BUILD/ffbuild/version.sh" 2>/dev/null &&
  chmod +x "$BUILD/ffbuild/version.sh" &&
  "$BUILD/ffbuild/version.sh" "$BUILD" "$BUILD/libavutil/ffversion.h") || true

(cd "$BUILD" && make -j"$JOBS" -s >/dev/null && make -s install >/dev/null)

echo "ffmpeg $mode hazır: $PREFIX"
ls "$PREFIX/lib/pkgconfig/libavcodec.pc" 2>/dev/null || { echo "eksik: libavcodec.pc"; exit 1; }
