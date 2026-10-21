# velnosub 0.1.0-alpha — yayın el-kitabı (iç)

## Ne hazır
- **APK**: `velnosub-0.1.0-alpha.apk` (12,8 MB; aapt + zip + zipalign + apksigner hattı, imza geçerli)
- **İmza**: debug anahtar `velnosub-debug.keystore` (parola `velnosub`, alias `velnosubdebug`)
  → gerçek sürüm için yalnız SENİN anahtarınla tekrar imzalanmalı (build-apk.sh 8. adımı
  kendi keystore'laşmanla koşulu yapar ya da hazır APK'ı `apksigner sign --ks <senin>` ile
  yeniden imzala)
- **Kaynak**: `tools/cross/apk-alpha/` (AndroidManifest.xml + MainActivity.java + strings.xml
  + build-apk.sh — build hattının tamamı kayıtlı)

## APK içi nerede aldırıyor
- `lib/arm64-v8a/libvelnosub_automation.so` = `velnosub-automation` bionic aarch64 ELF (strip'li)
- `lib/arm64-v8a/*` = abi/core/harfbuzz/freetype/fribidi/uchardet/libpng/libc++_shared zinciri (strip'li)
- `assets/velnosub/` = device-pkg gövdesi (lualua yüzeyi + inc karaskel kümesi + autos +
  default_config.json + DejaVuSans.ttf + üç test .ass)
- `classes.dex` = MainActivity: "Paketi kopyala" (assets → filesDir), "Pump duman (Dolaş)"
  = ass-format-tests.ass + Dolaş dene, "Pump duman (Kara)" = karaskel-dene.ass + Kara dene

## TELEFONDA deneme sırası (sen yapacaksın)
1. APK'ı yükle (yan yana kaynaksız sürümdür; "bilinmeyen kaynak" onayı gerekir)
2. "Paketi kopyala" (ilk koşum şartsız)
3. "Pump duman (Dolaş)" — beklenen: `[AUS1] dosya açıldı … Dolaş dene` +
   `dolaş: toplam=51, sel=13` + 8 info + 2 stil + aralık kırmaları ok
4. "Pump duman (Kara)" — beklenen: 11 `kara evN [k]` satırı + `kara dene: 5 olay birebir`
5. Başlarsa alpha onaylı demektir

## Sürüm adları
- GitHub release öneri adı: `velnosub 0.1.0-alpha (mobil çekirdek dumanı)`
- Gövde notu kaynak: Aşağıdaki "slogan" bloğu aşırı kısa, taksiz ve faydalı:

> velnosub 0.1.0-alpha — Aegisub'ın Android'deki ilk dumanı.
> Mobil çekirdek: ASS ayrıştırıcı + karaskel zinciri (sanal satır uzayı, subs.styles,
> script_resolution, parse_karaoke_data, text_extents) telefon üzerinde pump makro
> arayüzüyle koşuyor. UI fazı tamamlanmak üzere (pump + management yüzeyi mevcut).
> bu sürüm "canlı" riskli değildir: yalnızca alarm düzeltme penceresi'nden dumanlı
> koşum çıktısı gösterir.

(De slogan'ı da hemen altta senin dilediğin biçime çevir.)
