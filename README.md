# velnosub

velnosub, [Aegisub](https://github.com/arch1t3cht/Aegisub)'ın Android/iOS portudur.
Bu depo bağımsız bir depodur; kaynak kod tabanı [arch1t3cht/Aegisub](https://github.com/arch1t3cht/Aegisub)
(`feature` dalının `9bfd500` noktası) üzerinde yürür ve Aegisub'a atıf korunur.
Aegisub'ın özgün README'si değişmeden [`README-Aegisub.md`](README-Aegisub.md) dosyasında durur.

Port stratejisi "core-first" (çekirdek öncelikli): önce wx'siz ASS çekirdeği
(`aegisub-core`) ayrıştırıldı; sonra mobil kolda `libvelnosub-core.so` + `velnosub-cli`
üreten Android arm64-v8a cross zinciri kuruldu. Lua/MoonScript otomasyonu korunur:
LuaJIT aarch64 JIT/FFI gibi kısımlar Faz 5'e kadar saklanır.

## Proje belgeleri

Plan, künyeler ve doğrulama kanıtları [velnoxy/velnosub-docs](https://github.com/velnoxy/velnosub-docs)
deposundadır:

- Master plan: [`aegisub-mobile-port.md`](https://github.com/velnoxy/velnosub-docs/blob/main/aegisub-mobile-port.md)
- Faz 2 cross-build planı ve aşılan kapılar: [`docs/mobile/faz2-android-cross-plan.md`](https://github.com/velnoxy/velnosub-docs/blob/main/docs/mobile/faz2-android-cross-plan.md)
- Cihaz duman testi rehberi: [`docs/mobile/faz2-device-test.md`](https://github.com/velnoxy/velnosub-docs/blob/main/docs/mobile/faz2-device-test.md)

## Derleme (özet)

Masaüstü (mevcut Aegisub yolu korunur):

```sh
meson setup build && ninja -C build
```

Android arm64-v8a (NDK r29 aarch64-host; detaylar velnosub-docs'taki Faz 2 planında):

```sh
export PATH=$(echo /root/velsub/ndk/r29/toolchains/llvm/prebuilt/*/bin):$PATH
NDK_ROOT=/root/velsub/ndk/r29 tools/cross/build-deps.sh arm64
meson setup build-android-arm64 \
  -Dvelnosub_mobile=true \
  -Dalsa=disabled -Dopenal=disabled -Dlibpulse=disabled -Dportaudio=disabled \
  -Dffms2=disabled -Dbestsource=disabled -Dvapoursynth=disabled -Davisynth=disabled \
  -Dfftw3=disabled -Dhunspell=disabled \
  -Dlibass:require-system-font-provider=false -Dlibass:fontconfig=disabled \
  --cross-file tools/cross/android-arm64.txt
ninja -C build-android-arm64 libvelnosub-core.so tools/cross/velnosub-cli
```

## Lisans ve atıf

- Aegisub tabanı: GPLv3.
- [`libaegisub`](libaegisub/), [`libass`](subprojects/libass) ve [`ffms2`](https://github.com/FFMS/ffms2) BSD/MIT lisanslıdır; upstream telif ve izin yorumları dosyalarda aynen korunur.
- Bu portun yeni dosyaları da aynı BSD/MIT ruhuyla yazılır; upstream atıfları saklanır.
