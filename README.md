# velnosub

velnosub, [Aegisub](https://github.com/arch1t3cht/Aegisub)'ın Android portudur.
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
ninja -C build-android-arm64 libvelnosub-core.so \
  tools/cross/velnosub-cli tools/cross/velnosub-ass-render
```

Koşum (host'ta da aynı biçim; cihazda paylaşılan kütüphaneler `lib/` klasöründe
ve `LD_LIBRARY_PATH` işaretli; reçete velnosub-docs'ta faz2-device-test):

```sh
VELNOSUB_CONFIG_JSON=src/libresrc/default_config.json \
  build-android-arm64/tools/cross/velnosub-cli in.ass out.ass --edit-first "okuma-testi"
build-android-arm64/tools/cross/velnosub-ass-render in.ass out.ppm \
  --font DejaVuSans.ttf --defaultfont DejaVuSans.ttf --t 1 --w 640 --h 360
```

Notlar:

- `build-deps.sh` GNU make iş parçacığını `JOBS` ortam değişkeniyle ayarlar
  (ögel `JOBS=1` gibi proot konaklarında derlemeyi teke düşürmek için).
- libass 0.17'de `ass_add_font`'ın ad parametresi font seçim anahtarı değildir;
  ASS stilinin istediği family eşleşmezse `--defaultfont` yolu dosyadan çözer.

## Android arayüzü ve APK

Android varsayılan olarak klasik Aegisub düzeniyle açılır. Üstte menü ve araç
çubuğu, çalışma alanında solda video, sağda ses ve satır düzenleyici, altta
ise altyazı tablosu yer alır. Dar ekranda tablonun diğer sütunları yatay
kaydırmayla açılır. Kompakt sekmeli düzen Görünüm menüsünden seçilebilir.

Video açıldığında paneldeki **Video ölçeği** menüsü görüntüyü en-boy
oranını koruyarak sığdırır veya sabit bir yakınlaştırma yüzdesi uygular.
%100’de bir video pikseli bir ekran pikseline karşılık gelir. Görüntü
panele sığmazsa video alanında kaydırılabilir.

Güncel alpha APK'sı, başarılı `apk-build-ci` çalışmasının artifact bölümünde
yer alır. Paketin desteklediği mimari varsayılan olarak arm64-v8a ve alt sınırı Android 10'dur
(API 29). [APK derlemeleri](https://github.com/velnoxy/Velnosub/actions/workflows/apk-build-ci.yml)

Temiz bir çalışma dizininde NDK r29, SDK platformu android-34, build-tools
34.0.0, JDK, Meson, Ninja, CMake, pkg-config, gettext, Rust ve Android Rust hedefini
kurduktan sonra:

```sh
rustup target add aarch64-linux-android
NDK_ROOT=/path/to/android-ndk-r29 ANDROID_HOME=/path/to/android-sdk \
  bash tools/cross/apk-ui/ci-setup.sh
```

x86_64 emülatör paketi için `rustup target add x86_64-linux-android`
çalıştır ve aynı derleme komutuna `ABI=x86_64` ekle.

Varsayılan bağımlılık dizini `.android-build` altındadır. `ROOT`, `DEPS_ROOT`,
`BT` ve `PF` ile konumları değiştirebilirsin. Android APK, `tools/cross/apk-ui`
altına yazılır. Yerel Rust testleri için host ABI kütüphanesi
`build-host-check/tools/cross` altında olmalı veya `VELNOSUB_ABI_DIR` ile
gösterilmeli. Dinamik yükleme yoluna hem ABI hem core derleme dizinini ekle ve
`velnosub-ui` dizininde `cargo test --locked --lib` çalıştır.

x86_64 APK derlemesinden sonra Android 10 emülatörü bağlıyken aynı NDK
ve derleme dizinleriyle cihaz regresyonlarını da çalıştırabilirsin:

```sh
ABI=x86_64 NDK_ROOT=/path/to/android-ndk-r29 PHASE=test-android \
  bash tools/cross/apk-ui/ci-setup.sh
```

Üretim paketi için `REL=1`, `RELEASE_KEYSTORE`, `RELEASE_KEY_ALIAS`,
`RELEASE_STORE_PASSWORD`, `RELEASE_KEY_PASSWORD`, `RELEASE_VERSION_CODE`
ve `RELEASE_VERSION_NAME` gerekir. Anahtar ve parolaları yayın ortamında
sakla. Farklı imzalı alpha ve üretim paketleri aynı uygulama kimliğinde
yerinde güncellenemez.

Alpha 16’te SRT dışa aktarma, video zaman damgası ve ana kare dizini,
ASS dosyasına gömülü font ekleme, komut paleti ve Lua/MoonScript dosyası
yükleme bulunur. Otomasyon yöneticisinde paketlenmiş karaoke şablonunu
uygulayabilir veya seçtiğin scriptin makrolarını çalıştırabilirsin.
Bir makro form açtığında, formu yanıtlayana kadar betik bekler. Makro
iptal edilirse açık belge korunur. Standart `aegisub.re`,
`aegisub.lfs`, `aegisub.unicode` ve `luabins` modülleri paketlenir.
Scriptler uygulama yetkileriyle çalışır. `os.execute` ve `io.popen`
devre dışıdır. FFI kullanan scriptler için bu bir güvenlik sınırı oluşturmaz.

Linux ABI regresyonları için, aynı `LD_LIBRARY_PATH` ile depo kökünde
`python3 tests/mobile_abi_regression.py` çalıştır. FFmpeg ve ffprobe gereklidir.

İsteğe bağlı olarak Google Maven AAPT2 ve bundletool ile imzasız bir
Android App Bundle üret:

```sh
AAPT2=/path/to/aapt2 BUNDLETOOL_JAR=/path/to/bundletool.jar \
  tools/cross/apk-ui/build-aab.sh tools/cross/apk-ui/velnosub-ui-0.1.1-alpha16.apk velnosub.aab
```

## Lisans ve atıf

- Aegisub tabanı: GPLv3.
- [`libaegisub`](libaegisub/), [`libass`](subprojects/libass) ve [`ffms2`](https://github.com/FFMS/ffms2) BSD/MIT lisanslıdır; upstream telif ve izin yorumları dosyalarda aynen korunur.
- Bu portun yeni dosyaları da aynı BSD/MIT ruhuyla yazılır; upstream atıfları saklanır.
