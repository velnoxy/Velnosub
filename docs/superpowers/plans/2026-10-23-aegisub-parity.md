# Aegisub Parite: Yerleşim + Ses Zamanlama + Görsel Typesetting

> **For agentic workers:** Bu planı uygulamak için `superpowers:subagent-driven-development` kullanın. Adımlar checkbox (`- [ ]`) ile izlenir. Her görev ayrı commit ve ayrı inceleme kapısıdır.

**Goal:** Velnosub'un yatay ekran yerleşimini Aegisub geometrisine getirmek; ses panelinde tam zamanlama (dalga/spektrum, zoom, seçim, oynatma, satır zamanlarını yazma) ve video üzerinde tam görsel typesetting (taşı/ölçekle/döndür/clip/vektör) sağlamak.

**Architecture:** Saf hesaplar ayrı Rust modüllerinde (`layout`, `audio_timing`, `ass_tags`, `visual_geom`) tutulur ve birim testleriyle doğrulanır. C ABI yalnız eksik veriyi ekler (`audio_position`, `audio_spectrum`, `script_size`, `render_line`); mevcut ABI bozulmaz. UI (`abi.rs`) bu modülleri kullanır; mobil sekmeler korunur, yatayda Aegisub çerçevesi otomatik seçilir.

**Tech Stack:** Rust 2021, egui/eframe 0.37, libass, FFmpeg (PCM), C++17 ABI, Android NDK r29, mevcut meson/ninja hattı.

**Spec:** Bu belge tasarım ve uygulama planını birlikte taşır (superlite: tek birleşik belge).

## 1. Kullanıcı kararları

- **Yerleşim:** Yatayda otomatik Aegisub geometrisi; dikeyde mobil sekmeler.
- **Ses:** Tam zamanlama (dalga + spektrum, zoom/kaydırma, seçim, oynatma, satır zamanlarını yazma).
- **Typesetting:** Tam takım (taşı, ölçekle, Z/XY döndür, dikdörtgen clip, vektör clip/perspective).
- **Yürütme:** İnsan orkestratör — bu belge onaylanmadan ürün kodu yazılmaz.

## 2. Kapsam tanımı

### A. Yerleşim (Aegisub geometrisi)

Aegisub'un `src/frame_main.cpp` düzeni: üstte video (sol) + sağda ses ve düzen kutusu; altta tam genişlikte altyazı grid'i; menü, araç çubuğu ve durum çubuğu.

Velnosub yatay yerleşimi:

```
┌ menü ─────────────────────────────────────────────┐
│ araç çubuğu (Kaydet/Yeniden aç/Video/Ses)          │
├───────────────────────────┬────────────────────────┤
│ video (sol, en-boy)       │ ses paneli (üst ~32%)  │
│                           ├────────────────────────┤
│                           │ düzen kutusu (kalan)   │
├───────────────────────────┴────────────────────────┤
│ altyazı grid'i (tam genişlik, kalan yükseklik)     │
├────────────────────────────────────────────────────┤
│ durum çubuğu                                       │
└────────────────────────────────────────────────────┘
```

- Otomatik seçim: `genişlik > yükseklik && genişlik >= 800.0` → Aegisub; aksi halde mobil.
- Üst çalışma alanı yüksekliği kullanılabilir yükün `%55`'i; video/sağ sütun oranı video en-boyundan hesaplanır (`%32–%52` aralığına kırpılır).
- Üst alan ile grid arasında sürüklenebilir ayraç (egui `Panel::top(...).resizable(true)`).
- Mobil sekmeler (Altyazı/Video/Ses/Stil/Form/Ayarlar) aynen kalır.
- Aegisub'un wx widget'ları birebir taklit edilmez; geometri, renk ve yoğunluk yaklaştırılır. Menü/araç çubuğu egui olarak kalır.

### B. Ses zamanlama

- **Görünüm:** dalga formu ↔ spektrum geçişi; seçili satırın başlangıç/bitiş işaretleri.
- **Navigasyon:** yatay zoom (buton + iki parmak/pinch), kaydırma, görünür aralığa sığdır.
- **Etkileşim:** tıkla → imleci taşı; sürükle → zaman aralığı seç; seçimi oynat; imleçten oynat; durdur.
- **Oynatma:** playhead görünür aralıkta izlenir; `audio_position` ile 30–60 Hz yoklanır.
- **Zaman yazma:** "Başlat = seçim başı", "Bitir = seçim sonu"; satırın başlangıç/bitiş tutamaçlarını sürükleyerek `line_times_set`; kareye oturt; mevcut zaman kaydırma komutları korunur.
- **Performans:** spektrum görünür aralık için hesaplanır ve `(d0, adet, cols, rows)` anahtarıyla önbelleklenir; sütun sayısı en fazla 192, FFT 256; UI ipliği bloke edilmez (gerekirse arka plan iş parçacığı + kanal).

### C. Görsel typesetting

- Video üzerine **yalnız seçili satır** render edilir (`render_line`); diğer satırlar karışmaz.
- Araçlar:
  - **Taşı:** gövdeyi sürükle → `\pos(x,y)`.
  - **Ölçekle:** köşe tutamaçları → `\fscx`, `\fscy`.
  - **Z döndür:** tutamaç → `\frz`.
  - **XY döndür / perspektif:** yatay/dikey sürükleme → `\frx`, `\fry`.
  - **Dikdörtgen clip:** dört köşe tutamacı → `\clip(x1,y1,x2,y2)` (gerekirse `\iclip`).
  - **Vektör clip:** poligon noktalarını sürükle, kenara dokunarak nokta ekle, uzun basarak sil → `\clip(m ...)` / `\iclip(m ...)`.
- Etiket düzenleme ilk `{...}` bloğunda yapılır; varsa mevcut etiket yerinde güncellenir, yoksa eklenir. Diğer etiketler ve metin korunur.
- Sürükleme sırasında yalnız egui önizlemesi dönüştürülür; ABI'ye yazma **bırakınca** yapılır (tek commit).
- Değişiklikler için UI tarafında satır-metin geri/ileri yığını (Ctrl+Z/Ctrl+Y).
- **Kapsam dışı:** karaoke hece bölme, `\move` düzenleme (satırda `\move` varsa taşıma devre dışı + uyarı), automation/Lua, font/attachment, translation.

## 3. Mevcut durum (kanıt)

- `velnosub-ui/src/abi.rs` ~4.300 satır; masaüstü çerçevesi video/ses/grid/düzen kutularını **üst üste** yığıyor; mobil sekmeler var.
- Ses: `audio_peaks` ile 512 kovalı statik dalga; `audio_play/state/stop` var; **oynatma konumu ve spektrum yok**.
- Render: `render_frame` tüm track'i çiziyor; **tek satır render'ı ve PlayRes boyutu yok**.
- Satır metni: `line_text`, `edit_line` var; **override tag düzenleyici yok**.
- Aegisub kaynakları referans: `src/frame_main.cpp` (sizer'lar), `src/audio_renderer_spectrum.cpp`, `src/fft.cpp`, `src/visual_tool_*.cpp`.
- Cihazda kurulu: alpha12-r2; CI RUN23 (#46) yeşil; Shizuku sunucusu şu an kapalı (cihaz yeniden başladı).

## 4. Dosya yapısı

**Yeni:**
- `velnosub-ui/src/layout.rs` — yatay/dikey seçim ve Aegisub bölme matematiği (saf).
- `velnosub-ui/src/audio_timing.rs` — örnek↔piksel, zoom/kaydırma, seçim normalizasyonu (saf).
- `velnosub-ui/src/ass_tags.rs` — ASS override etiketi oku/yaz/sil (saf).
- `velnosub-ui/src/visual_geom.rs` — video/PlayRes/ekran eşlemesi ve tutamaç isabeti (saf).

**Değişecek:**
- `velnosub-ui/src/abi.rs` — FFI bildirimleri, Aegisub çerçevesi, ses paneli, typesetting UI.
- `velnosub-ui/src/lib.rs` — modül bildirimleri.
- `tools/cross/velnosub-abi.cpp` — dört yeni ABI ucu; render yardımcısı.
- `tools/cross/apk-ui/AndroidManifest.xml`, `tools/cross/apk-ui/build-apk.sh` — alpha13 sürümü.
- `tools/cross/apk-ui/ci-setup.sh` — gerekirse yeni ABI hedefi (değişiklik beklenmiyor).

**Testler:** her saf modülün `#[cfg(test)]` birim testleri; `cargo test` (host). UI/ABI için cihaz smoke.

## 5. ABI sözleşmeleri

Mevcut imzalar korunur. Eklenecekler:

```c
// Oynatma konumu: kare (sample) indeksi; oynatma yoksa -1.
int64_t velnosub_abi_audio_position(int h);

// Görünür aralık için spektrogram: out[col*rows + row] = 0..255.
// cols 1..=256, rows 1..=256; hata -1.
int velnosub_abi_audio_spectrum(int h, int64_t d0, int64_t adet,
                                int cols, int rows, unsigned char *out);

// ASS PlayResX/PlayResY; bulunamazsa video boyutu, o da yoksa 640x480.
int velnosub_abi_script_size(int h, int *out_w, int *out_h);

// Yalnız idx satırını ms zamanında RGBA (gen*yuk*4) çizer.
int velnosub_abi_render_line(int h, int idx, int ms,
                             unsigned char *buf, int n);
```

Semantik notlar:
- `audio_position`: `AudioPlayer::ilerleme` atomik sayacı; `d0 + ilerleme`. Oynatma bitince `d0+adet` döner; `stop` sonrası -1.
- `audio_spectrum`: `AudioWave::yol` ham s16 PCM'den okur; kanallar mono karıştırılır; Hann penceresi; radix-2 FFT (256); log-magnitüd. Ses yoksa/aralık dışıysa -1.
- `render_line`: `AssFile`'dan yalnız seçili olay + stiller ile geçici `ASS_Track` kurar, `ass_render_frame` ile çizer; mevcut RGBA kompozit yardımcısı kullanılır.
- `script_size`: `GetScriptInfoAsInt("PlayResX"/"PlayResY")`; 0 ise video bilgisi; o da yoksa 640×480.

## 6. Global kısıtlar

- Arayüz Türkçe; mevcut adlandırma ve 4×× satır stili korunur.
- Yeni ağır bağımlılık yok; FFT ABI içinde küçük ve bağımsız.
- Android arm64 hedefi; `cargo check --target aarch64-linux-android` ve `ninja -C build-android-arm64 libvelnosub-core.so tools/cross/libvelnosub-abi.so` geçmeli.
- Release **yok**; nightly ikon korunur; APK Actions artifact.
- Sürüm: `0.1.1-ui-alpha13`, versionCode 17.
- Her dilimde commit + `git push origin feature`.
- Cihaz doğrulaması Shizuku'ya bağlı; sunucu kapalıysa derleme kapısı geçer, cihaz doğrulaması "beklemede" işaretlenir ve raporlanır.
- Automation, fonts/attachments, translation bu planın dışında.

## 7. Uygulama planı

### Task 1: Yerleşim matematiği

**Files:** Create `velnosub-ui/src/layout.rs`; modify `velnosub-ui/src/lib.rs` (mod bildirimi); test aynı dosyada.

**Interfaces:**
```rust
pub enum Yerlesim { Aegisub, Mobil }
pub struct Bolmeler {
    pub ust_yuk: f32,   // menü+araç çubuğu hariç üst çalışma alanı
    pub video_gen: f32,
    pub sag_gen: f32,
    pub ses_yuk: f32,
    pub duzen_yuk: f32,
}
pub fn sec(gen: f32, yuk: f32) -> Yerlesim;
pub fn bolmeler(gen: f32, yuk: f32, video_oran: Option<f32>) -> Bolmeler;
```
- `sec`: `gen > yuk && gen >= 800.0` → `Aegisub`, değilse `Mobil`.
- `bolmeler`: `ust_yuk = yuk * 0.55`; `video_oran` yoksa 16/9; `video_gen = (ust_yuk * oran).clamp(gen*0.32, gen*0.52)`; `sag_gen = gen - video_gen`; `ses_yuk = ust_yuk * 0.32`; `duzen_yuk = ust_yuk - ses_yuk`.

- [ ] **Step 1:** `sec` ve `bolmeler` için testleri yaz (portre, 2400×1080 telefon, 2560×1600 tablet, video oranı 4/3 ve None).
- [ ] **Step 2:** `cargo test layout` çalıştır; testler derlenmeden önce FAIL.
- [ ] **Step 3:** Fonksiyonları yukarıdaki sözleşmeyle yaz.
- [ ] **Step 4:** `cargo test layout` PASS; `cargo check --target aarch64-linux-android` PASS.
- [ ] **Step 5:** Commit: `feat(ui): aegisub yerlesim matematigi (layout.rs)`.

### Task 2: Otomatik yatay Aegisub çerçevesi

**Files:** Modify `velnosub-ui/src/abi.rs` (`masaustu_cerceve`, `Duzen`, ayarlar); test yok (saf matematik Task 1'de).

**Interfaces:** `layout::sec`, `layout::bolmeler`; mevcut `video_icerik`, `ses_icerik`, `duzen_kutusu`, `grid_icerik`.

- [ ] **Step 1:** `Duzen`'a `Otomatik` ekle; varsayılan `Otomatik`; Ayarlar'da üç seçenek (Otomatik/Mobil/Masaüstü).
- [ ] **Step 2:** `masaustu_cerceve`'yi Aegisub geometrisine çevir: üst panelde `ui.horizontal`; solda video, sağda `ui.vertical` ses + düzen kutusu; merkezde grid; menü/araç çubuğu üstte, durum altta. Üst panel `resizable(true)` ve `default_height(ust_yuk)`.
- [ ] **Step 3:** Otomatik seçimi `GridApp::ui` başında uygula (`layout::sec(gen,yuk)`), kullanıcı seçimi `Otomatik` ise.
- [ ] **Step 4:** `cargo check` + yerel `cargo build --release --target aarch64-linux-android`.
- [ ] **Step 5:** Commit: `feat(ui): yatayda aegisub geometrisi, dikeyde mobil`.

### Task 3: Oynatma konumu ABI

**Files:** Modify `tools/cross/velnosub-abi.cpp` (`AudioPlayer`, `velnosub_abi_audio_position`); Modify `velnosub-ui/src/abi.rs` (FFI).

**Interfaces:** `extern "C" int64_t velnosub_abi_audio_position(int h);`

- [ ] **Step 1:** `AudioPlayer`'a `std::atomic<int64_t> ilerleme{0}` ekle; `onAudioReady` okunan kare kadar artır; kurulumda sıfırla; `audio_play` yeni oynatıcıda sıfırlar.
- [ ] **Step 2:** ABI ucunu yaz: oynatıcı yoksa `-1`, varsa `d0 + ilerleme` (durum 1 ise `d0+adet`).
- [ ] **Step 3:** Rust FFI bildirimi + `Ses` durumuna `konum: i64` alanı.
- [ ] **Step 4:** `ninja -C build-android-arm64 tools/cross/libvelnosub-abi.so` PASS; `cargo check` PASS.
- [ ] **Step 5:** Commit: `feat(abi): audio_position (oynatma konumu)`.

### Task 4: Spektrum ABI

**Files:** Modify `tools/cross/velnosub-abi.cpp` (FFT + `velnosub_abi_audio_spectrum`); Modify `velnosub-ui/src/abi.rs` (FFI).

**Interfaces:** `int velnosub_abi_audio_spectrum(int h, int64_t d0, int64_t adet, int cols, int rows, unsigned char *out);`

- [ ] **Step 1:** 256 örnekli radix-2 FFT'yi ABI dosyasına ekle (Aegisub `src/fft.cpp` algoritmasının sadeleştirilmişi); Hann penceresi ve log-magnitüd.
- [ ] **Step 2:** `AudioWave.yol` ham PCM'ini `fopen/fseek/fread` ile oku; her sütun için `fft=256` örnek, kanalları mono karıştır; `out[col*rows+row]` 0..255.
- [ ] **Step 3:** Sınır denetimleri: `cols/rows 1..=256`, `d0>=0`, `adet>0`, dosya var; hata -1.
- [ ] **Step 4:** `ninja` ABI PASS; `cargo check` PASS.
- [ ] **Step 5:** Commit: `feat(abi): audio_spectrum (FFT spektrogram)`.

### Task 5: Ses zamanlama matematiği

**Files:** Create `velnosub-ui/src/audio_timing.rs`; test aynı dosyada.

**Interfaces:**
```rust
pub struct Gorunum { pub d0: i64, pub adet: i64, pub gen: f32 }
pub fn px_to_ornek(x: f32, g: &Gorunum) -> i64;
pub fn ornek_to_px(s: i64, g: &Gorunum) -> f32;
pub fn zoom_merkez(g: &mut Gorunum, carpan: f64, capa_px: f32, toplam: i64);
pub fn kaydir(g: &mut Gorunum, dx_px: f32, toplam: i64);
pub fn secim_normalize(a: i64, b: i64) -> (i64, i64);
pub fn kareye_oturt(ornek: i64, rate: i32, fps: f64) -> i64;
```
- [ ] **Step 1:** Sınır testleri: sıfır genişlik, negatif dx, zoom min/max, seçim ters yön, kare yuvarlama.
- [ ] **Step 2:** `cargo test audio_timing` FAIL.
- [ ] **Step 3:** Fonksiyonları yaz; taşma için `i128` ara değer.
- [ ] **Step 4:** `cargo test audio_timing` PASS.
- [ ] **Step 5:** Commit: `feat(ui): ses zamanlama matematigi`.

### Task 6: Ses paneli (dalga/spektrum + tam zamanlama)

**Files:** Modify `velnosub-ui/src/abi.rs` (`ses_icerik`, `Ses` durumu, FFI).

**Interfaces:** Task 3/4/5 imzaları; mevcut `audio_peaks`, `line_times_set`, `audio_play`, `audio_stop`, `audio_len64`.

- [ ] **Step 1:** `Ses` durumuna `zoom`, `d0`, `adet`, `secim: Option<(i64,i64)>`, `mod: Dalga|Spektrum`, `spek: Option<Vec<u8>>`, `spek_anahtar`.
- [ ] **Step 2:** Çizim: dalga için `audio_peaks` (mevcut), spektrum için `audio_spectrum`; görünür aralık ve seçim işaretleri; playhead `audio_position`.
- [ ] **Step 3:** Etkileşim: tıkla→imleç, sürükle→seçim, iki parmak/tekerlek→zoom, boş alan sürükle→pan; "Seçimi oynat", "İmleçten", "Durdur"; "Başlat/Bitir" düğmeleri seçimi satıra yazar.
- [ ] **Step 4:** Seçili satırın başlangıç/bitiş tutamaçları; sürükleyince `line_times_set` + kareye oturt; notta ms gösterimi.
- [ ] **Step 5:** `cargo test` + `cargo check` + cihazda (Shizuku varsa) ses dosyasıyla smoke.
- [ ] **Step 6:** Commit: `feat(ui): ses panelinde tam zamanlama`.

### Task 7: script_size + render_line ABI

**Files:** Modify `tools/cross/velnosub-abi.cpp`; Modify `velnosub-ui/src/abi.rs` (FFI).

**Interfaces:** `velnosub_abi_script_size`, `velnosub_abi_render_line` (bkz. §5).

- [ ] **Step 1:** `script_size`: PlayResX/Y, yoksa video, yoksa 640×480.
- [ ] **Step 2:** `render_line`: `Ctx.file`'dan başlık + stiller + yalnız `idx` olayıyla geçici `ASS_Track`; `ass_render_frame`; mevcut RGBA kompozitini yardımcı fonksiyona çıkarıp iki yerde kullan.
- [ ] **Step 3:** Sınır denetimleri (idx, n, ms); hata -1.
- [ ] **Step 4:** `ninja` ABI PASS.
- [ ] **Step 5:** Commit: `feat(abi): script_size + render_line`.

### Task 8: ASS etiket motoru

**Files:** Create `velnosub-ui/src/ass_tags.rs`; Modify `lib.rs`; test aynı dosyada.

**Interfaces:**
```rust
pub fn etiket_al(metin: &str, ad: &str) -> Option<String>;
pub fn etiket_yaz(metin: &str, ad: &str, deger: &str) -> String;
pub fn etiket_sil(metin: &str, ad: &str) -> String;
pub fn sayi_al(deger: &str) -> Option<f64>;
pub fn vektor_noktalari(metin: &str, ad: &str) -> Option<Vec<(f64, f64)>>;
pub fn vektor_yaz(metin: &str, ad: &str, noktalar: &[(f64, f64)], kapali: bool) -> String;
```
- [ ] **Step 1:** Testler: `{\pos(10,20)}merhaba`, çok bloklu metin, `\fscx120\fscy90`, `\clip(1,2,3,4)`, `\clip(m 0 0 l 10 0 10 10)`, etiketsiz metin, bozuk blok.
- [ ] **Step 2:** `cargo test ass_tags` FAIL.
- [ ] **Step 3:** İlk `{...}` bloğunda güncelle/ekle; yoksa başa blok aç; diğer metni aynen koru.
- [ ] **Step 4:** `cargo test ass_tags` PASS.
- [ ] **Step 5:** Commit: `feat(ui): ass override etiket motoru`.

### Task 9: Görsel geometri

**Files:** Create `velnosub-ui/src/visual_geom.rs`; test aynı dosyada.

**Interfaces:**
```rust
pub struct Alan { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
pub struct Esleme { pub olcek: f32, pub ofs_x: f32, pub ofs_y: f32 }
pub fn esleme(ass_w: i32, ass_h: i32, alan: Alan) -> Esleme;
pub fn ass_to_ekran(x: f64, y: f64, e: &Esleme) -> (f32, f32);
pub fn ekran_to_ass(x: f32, y: f32, e: &Esleme) -> (f64, f64);
pub fn tutamac(a: (f32, f32), b: (f32, f32), yaricap: f32) -> bool;
```
- [ ] **Step 1:** Testler: letterbox (16:9 video, 4:3 alan), kare video, ters dönüşüm, tutamaç yarıçapı.
- [ ] **Step 2:** `cargo test visual_geom` FAIL.
- [ ] **Step 3:** Fonksiyonları yaz (en-boy korunur, ortalama).
- [ ] **Step 4:** `cargo test visual_geom` PASS.
- [ ] **Step 5:** Commit: `feat(ui): typesetting koordinat eslemesi`.

### Task 10: Typesetting çekirdeği (taşı/ölçek/Z döndür)

**Files:** Modify `velnosub-ui/src/abi.rs` (`VideoSection`, yeni `Typeset` durumu); kullanır: Task 7/8/9.

**Interfaces:** `Typeset { arac: Arac, satir: usize, metin: String, ... }`; `render_line`; `line_text`/`edit_line`; `script_size`.

- [ ] **Step 1:** Araç çubuğu: Taşı / Ölçek / Z Döndür / Clip / Vektör / XY. Seçili satır yoksa pasif.
- [ ] **Step 2:** Seçili satırı `render_line` ile al; video karesinin üzerine çiz; `script_size` + `esleme` ile yerleştir.
- [ ] **Step 3:** Taşı: sürükleme deltasını ASS koordinatına çevir; önizlemede ofset uygula; bırakınca `\pos` yaz (`etiket_yaz`) + `edit_line`.
- [ ] **Step 4:** Ölçek: köşe tutamaçları; `\fscx/\fscy` yüzdesi; min %10.
- [ ] **Step 5:** Z döndür: merkez açısı; `\frz` derece; normalize [-180,180].
- [ ] **Step 6:** Sürükleme sırasında ABI'ye yazma; bırakınca tek commit; Ctrl+Z için metin yığını.
- [ ] **Step 7:** `cargo test` + cihaz smoke (varsa).
- [ ] **Step 8:** Commit: `feat(ui): gorsel typesetting cekirdegi (tasi/olcek/dondur)`.

### Task 11: Clip + vektör + XY/perspektif

**Files:** Modify `velnosub-ui/src/abi.rs`, `ass_tags.rs` (vektör yardımcıları).

**Interfaces:** Task 8 vektör fonksiyonları; `\clip`/`\iclip`, `\frx`, `\fry`.

- [ ] **Step 1:** Dikdörtgen clip: dört tutamaç; `\clip(x1,y1,x2,y2)`; `\iclip` anahtarı.
- [ ] **Step 2:** Vektör clip: mevcut `m ... l ...` noktalarını göster; sürükle; kenara dokun → nokta ekle; uzun bas → sil; `\clip(m ...)` yaz.
- [ ] **Step 3:** XY döndür: yatay/dikey sürükleme → `\frx/\fry`; perspektif görünümü aynı araçta.
- [ ] **Step 4:** Etiket yoksa oluştur; varsa yerinde güncelle; `\move` varsa taşımayı devre dışı bırak + uyarı.
- [ ] **Step 5:** `cargo test` + `cargo check` + cihaz smoke (varsa).
- [ ] **Step 6:** Commit: `feat(ui): clip/vektor/xy typesetting`.

### Task 12: Sürüm, derleme, cihaz doğrulaması, CI

**Files:** Modify `AndroidManifest.xml` (code 17 / alpha13), `build-apk.sh` (OUT/echo), gerekirse `ci-setup.sh`; `docs` raporu.

- [ ] **Step 1:** alpha13 sürüm damgası; `cargo build --release --target aarch64-linux-android`.
- [ ] **Step 2:** ABI `ninja`; APK üret; md5 al.
- [ ] **Step 3:** Shizuku varsa `pm install -r`, `am start`, 5 sn sonra ekran görüntüsü + breadcrumb + logcat; yatay/dikey düzen ve ses/typesetting kontrol listesi.
- [ ] **Step 4:** Shizuku yoksa "cihaz doğrulaması beklemede" notuyla APK'yı `/sdcard/Download`'a kopyala.
- [ ] **Step 5:** Commit + push; CI `apk-build-ci.yml` artifact.
- [ ] **Step 6:** Commit: `build: alpha13 (aegisub parite dilimi)`.

## 8. Review Focus

1. **Dar yatay ekran (2400×1080 telefon):** video/sağ sütun/grid taşmamalı; edit kutusu 44pt hedefleri korumalı. → Task 1 testleri + Task 2 düzen kontrolü.
2. **Çok uzun ses (2 saat) ve aşırı zoom:** örnek indeksleri taşmamalı, zoom min/max ve kaydırma sınırları doğru. → Task 5 testleri.
3. **Spektrum performansı:** görünür aralık değişmedikçe yeniden FFT yok; UI 16 ms'lik karede bloke olmamalı. → Task 4/6 önbellek anahtarı + cihaz ölçümü.
4. **ASS etiket kenarları:** çok bloklu metin, `\move`, karaoke `\k`, bozuk `\clip` metni korunmalı. → Task 8 testleri.
5. **PlayRes ≠ video boyutu ve letterbox:** typesetting imleci görünen görüntüyle hizalanmalı; yeniden boyutlamada kaymamalı. → Task 9 testleri + Task 10 cihaz kontrolü.
6. **Cihaz erişimi yok:** Shizuku kapalıyken derleme/APK geçmeli, doğrulama beklemede kalmalı; yanlış "tamam" denmemeli. → Task 12 adım 4.

## 9. Sıra, riskler, geri dönüş

- **Sıra:** Task 1–2 (A) → Task 3–6 (B1) → Task 7–11 (B2) → Task 12. Her dilim kendi başına derlenir ve APK üretir.
- **Risk:** "Tam takım" tek oturumda bitmez; vektör clip ve XY döndür en sona bırakıldı. Ses spektrumu telefonda yavaşsa sütun/FFT küçültülür ve önbellek genişletilir.
- **Geri dönüş:** Typesetting araçları yalnız video/satır seçiliyken etkin; sorun çıkarsa `Typeset` durumu kapatılır, ses paneli dalga moduna döner. Yerleşim `Ayarlar → Düzen → Mobil` ile eski görünüme alınabilir.
- **Cihaz:** Shizuku yeniden başlatılana kadar Task 12'nin cihaz adımları beklemede; kod/derleme/CI durmaz.

## 10. Onay kapısı

Bu belge onaylanmadan ürün kodu yazılmaz. Onaydan sonra Task 1'den başlanır; her görev sonunda commit + görev incelemesi yapılır, dilim sonunda APK/CI ve mümkünse cihaz smoke gelir.
