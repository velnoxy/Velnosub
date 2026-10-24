// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 4 (P4.1/P4.2) — C ABI şeridi: Rust/egui (velnosub-ui) tarafının
// bağlanacağı extern "C" yüzey. Akış, koşuma geçmiş P2.2 CLI kanalının
// fonksiyonlaştırılmış kopyasıdır: config hazırlığı tek kereliğine,
// açık/edit/save/reload işlevseldir. P4.2 ekleri: grid için satır
// metin/zaman okuma + satır düzenleme (edit_first ile aynı Commit hattı).
// libvelnosub-abi.so libvelnosub-core.so'ya bağlanır
// (Android'de libc++_shared zinciri CLI ile aynıdır).
//
// Rust tarafı (P4.1): velnosub-ui/abi-smoke crate'i bu yüzeyi çağırır.

#include "ass_dialogue.h"
#include "ass_file.h"
#include "ass_info.h"
#include "ass_style.h"
#include <ass/ass.h>
#include <cstdio>
#include "include/aegisub/video_provider.h"
#include "options.h"
#include "subtitle_format.h"
#include "video_frame.h"
#include "video_provider_manager.h"

#include <libaegisub/dispatch.h>
#include <libaegisub/exception.h>
#include <libaegisub/fs.h>
#include <libaegisub/log.h>
#include <libaegisub/option.h>
#include <libaegisub/vfr.h>

// P5.5: kutu-sesi — ffmpeg linki STATİK .a hattıdır (build-apk.sh 8. adım
// sembol-kapanış denetimi build-time yakalar). DİKKAT: vendored ffmpeg
// başlıkları extern "C" sarmalı İÇERMİYOR — sarmayı biz açıyoruz; yoksa
// her av*/swr* referansı C++-mangle UND düşer (lld kanıtı:
// 'undefined symbol: avformat_open_input(...)' biçimli çıktı)
extern "C" {
#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>
#include <libavutil/channel_layout.h>
#include <libavutil/samplefmt.h>
#include <libswresample/swresample.h>
}

#ifdef VELNOSUB_ABI_OBOE
#include <oboe/Oboe.h>
#endif

#include <algorithm>
#include <atomic>
#include <cmath>
#include <memory>
#include <mutex>
#include <thread>
#include <utility>
#include <vector>

#include <cstring>
#include <cstdio>

// P5.2 dilim C: aegisub.text_extents — FreeType ölçüm kolu (upstream auto4
// wx tabanlı ölçümün mobil hattı: 26.6 sabit nokta, aynı scalex/scaley/100
// telafisi ve ölçek normallemesi; fontname eşleşmesi cihaz font-servis
// fazına kalmakta — tek paket fontuyla ölçüm; italik sentetik 0.2 eğme,
// kalin ~%4 ilerletme yaklaşması)
#include <ft2build.h>
#include FT_FREETYPE_H
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <iterator>
#include <map>
#include <memory>
#include <new>
#include <string>

// P5.3: flyweight Text mutasyon vekilleri (src/core/velnosub_core_edit.cpp,
// core.so'da derlenir — fabrika tekili DSO'ya göre bölündüğünden abi.so'dan
// doğrudan atama yığını bozuyor; ayrıntı TU'nun baş notunda)
extern "C" int velnosub_core_edit_line_text(AssFile &file, int idx, const char *text);
extern "C" int velnosub_core_edit_first_text(AssFile &file, const char *text);
extern "C" int velnosub_core_insert_line(AssFile &file, int idx, const char *metin,
                                         const char *bas_zaman, const char *son_zaman,
                                         const char *stil);
extern "C" int velnosub_core_delete_line(AssFile &file, int idx);
extern "C" int velnosub_core_set_line_times(AssFile &file, int idx,
                                            long long ms_bas, long long ms_son);
extern "C" int velnosub_core_set_line_meta(AssFile &file, int idx,
                                           const char *stil, const char *aktor,
                                           const char *etki, int yorum,
                                           int katman);

namespace {

struct Ctx {
	AssFile file;
	const SubtitleFormat *reader = nullptr;
};

bool g_ready = false;
int g_next = 1;
std::map<int, std::unique_ptr<Ctx>> g_ctx;
std::string g_last_error;

std::string read_all(std::string const& p) {
	std::ifstream in(p, std::ios::binary);
	return std::string(std::istreambuf_iterator<char>{in},
	                   std::istreambuf_iterator<char>{});
}

void set_err(std::string s) { g_last_error = std::move(s); }

std::map<int, std::unique_ptr<VideoProvider>> g_video;
int g_vnext = 0;

// ---- P4.4/P3.5 omurgası: ses kanalı (s16 PCM + peak haritası + Oboe) --

struct AudioWave {
	std::string yol;                      // ham s16 PCM yolu (oynatma yeniden açar)
	int rate = 48000, ch = 2;
	int64_t toplam_ornek = 0;             // kare sayısı (her kare ch örnek)
	int64_t kova_ornekleme = 256;         // harita parçacığı (kare)
	std::vector<int16_t> kova_min, kova_max;
};

std::map<int, std::unique_ptr<AudioWave>> g_audio;
int g_anext = 0;

// ---- P7.1: spektrogram FFT'si — Aegisub src/fft.cpp'nin bağımsız, 256
// örnekli radix-2 (iteratif Cooley–Tukey) sadeleştirmesi. Dışa açılmayan
// çekirdek sembollerine dokunmaz; tablolar ilk çağrıda bir kez kurulur. ----
constexpr int kFftN = 256;
constexpr double kPi = 3.14159265358979323846;

struct Fft256Tablari {
	int ters[kFftN];
	double cosn[kFftN / 2];
	double sinn[kFftN / 2];
	double hann[kFftN];

	Fft256Tablari() {
		for (int i = 0; i < kFftN; ++i) {
			int x = i, r = 0;
			for (int b = 0; b < 8; ++b) {
				r = (r << 1) | (x & 1);
				x >>= 1;
			}
			ters[i] = r;
		}
		for (int i = 0; i < kFftN / 2; ++i) {
			double const aci = -2.0 * kPi * i / kFftN;
			cosn[i] = std::cos(aci);
			sinn[i] = std::sin(aci);
		}
		for (int i = 0; i < kFftN; ++i)
			hann[i] = 0.5 * (1.0 - std::cos(2.0 * kPi * i / (kFftN - 1)));
	}
};

// yerinde 256'lık FFT; ger/sanal tamponları sütun döngüsü DIŞINDA ayrılır
void fft256(double *ger, double *sanal, Fft256Tablari const &t) {
	for (int i = 0; i < kFftN; ++i) {
		int const j = t.ters[i];
		if (i < j) {
			std::swap(ger[i], ger[j]);
			std::swap(sanal[i], sanal[j]);
		}
	}
	for (int uzunluk = 2; uzunluk <= kFftN; uzunluk <<= 1) {
		int const yarim = uzunluk / 2;
		int const adim = kFftN / uzunluk;
		for (int i = 0; i < kFftN; i += uzunluk) {
			for (int j = 0; j < yarim; ++j) {
				int const tw = j * adim;
				double const cr = t.cosn[tw], ci = t.sinn[tw];
				int const a = i + j, b = a + yarim;
				double const tr = ger[b] * cr - sanal[b] * ci;
				double const ti = ger[b] * ci + sanal[b] * cr;
				ger[b] = ger[a] - tr;
				sanal[b] = sanal[a] - ti;
				ger[a] += tr;
				sanal[a] += ti;
			}
		}
	}
}

#ifdef VELNOSUB_ABI_OBOE
// Oboe oynatma: dosya imlecinden akan geri çağrım; kurulum ARKA İPLİKTE —
// kısıtlı ortamlarda (proot binder) açım engellenebilir, çağıran iplik
// asla bloke olmamalı.
struct AudioPlayer final : oboe::AudioStreamDataCallback, oboe::AudioStreamErrorCallback {
	std::FILE *f = nullptr;
	std::string yol;              // kurulum ipliği bu yoldan açar
	int rate = 48000, ch_c = 2;
	int64_t d0 = 0, adet = 0;     // oynatma aralığı (kare)
	std::atomic<int> kurulma{-1}; // -1 hazırlanıyor, 0 tamam, 2 hata
	int64_t kalan = 0;            // kalan kare
	std::atomic<int> durum{0};    // 0 oynatılıyor, 1 boşalma, 2 hata
	std::atomic<int64_t> ilerleme{0}; // geri çağrımda tüketilen kare (d0'a göre)
	std::atomic<bool> kes{false};
	std::shared_ptr<oboe::AudioStream> akis; // kurulum ipliği doldurur

	~AudioPlayer() { // emniyet: stop geçmediyse akışı ve dosyayı kapat
		if (akis) { akis->stop(); akis->close(); }
		if (f) std::fclose(f);
	}

	oboe::DataCallbackResult onAudioReady(oboe::AudioStream *akis, void *veri, int32_t kare) override {
		(void)akis;
		if (kes.load() || kalan <= 0) {
			if (durum.load() == 0)
				durum.store(1);
			return oboe::DataCallbackResult::Stop;
		}
		auto *cik = static_cast<int16_t *>(veri);
		int32_t istenen = kare < kalan ? (int32_t)kare : (int32_t)kalan;
		size_t okunan = std::fread(cik, sizeof(int16_t), (size_t)istenen * ch_c, f);
		int64_t goten = (int64_t)okunan / ch_c;
		kalan -= goten;
		ilerleme.fetch_add(goten, std::memory_order_relaxed);
		for (int64_t i = goten; i < istenen; ++i)
			for (int c = 0; c < ch_c; ++c)
				cik[i * ch_c + c] = 0; // boşalma duruşu
		if (kalan <= 0 && durum.load() == 0)
			durum.store(1);
		return oboe::DataCallbackResult::Continue;
	}
	void onErrorAfterClose(oboe::AudioStream *, oboe::Result hata) override {
		(void)hata;
		kurulma.store(2);
		durum.store(2);
	}
};
#endif

} // namespace

extern "C" {

int velnosub_abi_init(const char *config_json_path) {
	if (g_ready) return 0;
	try {
		agi::dispatch::Init([](agi::dispatch::Thunk f) { f(); });
		agi::log::log = new agi::log::LogSink;
		std::string const cfg_text = read_all(
			config_json_path && *config_json_path ? config_json_path : "default_config.json");
		if (cfg_text.empty()) {
			set_err("config boş (yol cihazda geçersiz?): " +
			        std::string(config_json_path ? config_json_path : "default_config.json"));
			return -1;
		}
		config::opt = new agi::Options(
			"", {cfg_text.data(), cfg_text.size()}, agi::Options::FLUSH_SKIP);
		g_ready = true;
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_open(const char *in_path) {
	if (!g_ready) { set_err("init çağrılmadı"); return -1; }
	try {
		auto ctx = std::make_unique<Ctx>();
		agi::fs::path const in(in_path);
		ctx->reader = SubtitleFormat::GetReader(in, "utf-8");
		if (!ctx->reader) { set_err("okuyucu bulunamadı: " + std::string(in_path)); return -1; }
		ctx->reader->ReadFile(&ctx->file, in, agi::vfr::Framerate(), "utf-8");
		int h = g_ctx.empty() ? 1 : (++g_next);
		g_ctx.emplace(h, std::move(ctx));
		return h;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_event_count(int h) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	return static_cast<int>(g_ctx.at(h)->file.Events.size());
}

int velnosub_abi_style_count(int h) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	return static_cast<int>(
		std::distance(g_ctx.at(h)->file.Styles.begin(), g_ctx.at(h)->file.Styles.end()));
}

int velnosub_abi_edit_first(int h, const char *text) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	Ctx &c = *g_ctx.at(h);
	if (c.file.Events.empty()) { set_err("event boş"); return -1; }
	try {
		// P5.3: flyweight yazımı core.so vekilinden (bkz. velnosub_core_edit.cpp)
		if (velnosub_core_edit_first_text(c.file, text) != 0) {
			set_err("event boş");
			return -1;
		}
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_save(int h, const char *out_path) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	try {
		agi::fs::path const out(out_path);
		const SubtitleFormat *writer = SubtitleFormat::GetWriter(out);
		if (!writer) { set_err("yazıcı bulunamadı: " + std::string(out_path)); return -1; }
		writer->WriteFile(&g_ctx.at(h)->file, out, agi::vfr::Framerate(), "utf-8");
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_reload_count(int h, const char *out_path) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	try {
		AssFile reloaded;
		g_ctx.at(h)->reader->ReadFile(&reloaded, agi::fs::path(out_path),
		                              agi::vfr::Framerate(), "utf-8");
		return static_cast<int>(reloaded.Events.size());
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_line_text(int h, int idx, char *buf, int buflen) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		std::string const t = std::next(c.file.Events.begin(), idx)->Text;
		int const n = static_cast<int>(t.size());
		int const m = (buflen - 1) < n ? (buflen - 1) : n;
		std::memcpy(buf, t.data(), static_cast<size_t>(m));
		buf[m] = '\0';
		return n; // tam uzunluk; kısa tamponla işi tekrar çağırabilirsin
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_line_meta(int h, int idx, char *buf, int buflen) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		AssDialogue const& d = *std::next(c.file.Events.begin(), idx);
		// P5.3: aegisub satır sınıfı + yorum bayrağı — "class|comment"
		// (upstream scriptler class=="dialogue" + comment alanına bakar;
		// ABI olay kolları yalnız Dialogue satırı tuttukları için sınıf
		// hep "dialogue" döner; Comment olayı comment=1)
		// P5 dilim 4: karaskel hattı alanları — upstream LuaDialogueToLua
		// kardeşliği (auto4_lua_assfile.cpp 164-182): layer/style/actor/
		// effect/margin_l/r/t(b=t tek slot Margin[2]) kanalları eklendi
		std::string const t =
			"dialogue|" + std::string(d.Comment ? "1" : "0")
			+ "|" + std::to_string(d.Layer)
			+ "|" + d.Style.get() + "|" + d.Actor.get() + "|" + d.Effect.get()
			+ "|" + std::to_string(d.Margin[0])
			+ "|" + std::to_string(d.Margin[1])
			+ "|" + std::to_string(d.Margin[2]);
		int const n = static_cast<int>(t.size());
		int const m = (buflen - 1) < n ? (buflen - 1) : n;
		std::memcpy(buf, t.data(), static_cast<size_t>(m));
		buf[m] = '\0';
		return n;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_line_times(int h, int idx, char *buf, int buflen) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		AssDialogue const& d = *std::next(c.file.Events.begin(), idx);
		std::string const t = d.Start.GetAssFormatted() + " / " + d.End.GetAssFormatted();
		int const n = static_cast<int>(t.size());
		int const m = (buflen - 1) < n ? (buflen - 1) : n;
		std::memcpy(buf, t.data(), static_cast<size_t>(m));
		buf[m] = '\0';
		return n;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

// P5.2 dilim: satır zaman yazımı (ms) — aegisub satır alanı yazısı
// (start_time/end_time). idx 0-taban; ms_bas/ms_son tamsayı.
int velnosub_abi_line_times_set(int h, int idx, long long ms_bas, long long ms_son) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		if (velnosub_core_set_line_times(c.file, idx, ms_bas, ms_son) != 0) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_edit_line(int h, int idx, const char *text) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!text) { set_err("boş metin"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		// P5.3: flyweight Text yazımı core.so vekilinden (fabrika tekili
		// DSO'ya göre bölünüyor; abi.so'dan doğrudan atama yığını bozuyor)
		if (velnosub_core_edit_line_text(c.file, idx, text) != 0) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

// ---- P4.5 omurgası: stil kolu (alan dizgesi + ölçü/ad düzenleme) -------

// alan dizgesi ('|' ile ayrılmış 13 hane):
// 0 ad, 1 font, 2 boyut, 3-6 renkler ana/ikincil/kontur/gölge (ASS biçimi),
// 7 biçim baytları "kalın,italik,altçizgi,üstçizgi", 8 hiza,
// 9 kenarlar "B,G,D", 10 kodlama, 11 "ölçekx,ölçeks,haraba,açı",
// 12 "kenarstili,konturgenis, gölgegenis"
static std::string stil_dizgesi(AssStyle const& s) {
	auto ik = [](bool b) { return b ? "1" : "0"; };
	return s.name + "|" + s.font + "|" + std::to_string(s.fontsize)
	     + "|" + s.primary.GetAssStyleFormatted()
	     + "|" + s.secondary.GetAssStyleFormatted()
	     + "|" + s.outline.GetAssStyleFormatted()
	     + "|" + s.shadow.GetAssStyleFormatted()
	     + "|" + ik(s.bold) + "," + ik(s.italic) + "," + ik(s.underline)
	     + "," + ik(s.strikeout)
	     + "|" + std::to_string(s.alignment)
	     + "|" + std::to_string(s.Margin[0]) + "," + std::to_string(s.Margin[1])
	     + "," + std::to_string(s.Margin[2])
	     + "|" + std::to_string(s.encoding)
	     + "|" + std::to_string(s.scalex) + "," + std::to_string(s.scaley)
	     + "," + std::to_string(s.spacing) + "," + std::to_string(s.angle)
	     + "|" + std::to_string(s.borderstyle) + ","
	     + std::to_string(s.outline_w) + "," + std::to_string(s.shadow_w);
}

// P5.2 devam: satır ekleme/silme — aegisub macro pump'ının insert_line/
// delete_line çağrıları için. idx ABI kolunda 0-taban; insert düğümü idx
// konumunun önüne koyar (idx == adet ise sona). Zamanlar ASS dizgesi.
// (flyweight mutasyonları core.so vekilinde — bkz. velnosub_core_edit.cpp)
int velnosub_abi_line_insert(int h, int idx, const char *metin, const char *bas,
                             const char *son, const char *stil) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!metin) { set_err("boş metin"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx > static_cast<int>(c.file.Events.size())) {
			set_err("ekleme indeksi dışarıda: " + std::to_string(idx));
			return -1;
		}
		if (velnosub_core_insert_line(c.file, idx, metin, bas, son, stil) != 0) {
			set_err("ekleme indeksi dışarıda: " + std::to_string(idx));
			return -1;
		}
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_line_delete(int h, int idx) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		if (velnosub_core_delete_line(c.file, idx) != 0) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

// P5.4 gece dilimi: satır meta yazımı — stil/oyuncu/etki/yorum/katman.
// NULL dizge = alan dokunulmaz; yorum/katman -1 = dokunulmaz.
int velnosub_abi_line_meta_set(int h, int idx, const char *stil,
                               const char *aktor, const char *etki,
                               int yorum, int katman) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		if (velnosub_core_set_line_meta(c.file, idx, stil, aktor, etki,
		                                yorum, katman) != 0) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

// P4.6 dilim 1: libass önizleme kolü — tutamaç başına şarıtlı ASS_Renderer.
// AssStyle::GetEntryData/AssDialogue::GetEntryData okuma-kolu (fabrika
// dokunmuyor) — abi.so'dan çağrılabilir. Font: pkg'taki blob (ask..., fontconfig
// cihazda yok: ASS_FONTPROVIDER_NONE + ass_add_font; P2.3 düzendır).
struct RCtx {
	ASS_Library *lib = nullptr;
	ASS_Renderer *ren = nullptr;
	ASS_Track *track = nullptr;
	int gen = 0;
	int yuk = 0;
};

static std::map<int, RCtx> g_r;

static void r_close(RCtx &r) {
	if (r.track) ass_free_track(r.track);
	if (r.ren) ::ass_renderer_done(r.ren);
	if (r.lib) ::ass_library_done(r.lib);
	r = RCtx{};
}

// ASS serileştirme: script info (PlayRes dahil), stiller ve istenirse yalnız
// tek olay. Önizleme ile satır render'ı aynı başlığı kullanır; PlayRes yoksa
// libass 384x288 varsayar (Aegisub GetResolution ile aynı).
static std::string ass_seri(Ctx const& c, int sadece_idx) {
	std::string ser = "[Script Info]\n";
	for (auto const& info : c.file.Info)
		ser += info.GetEntryData() + "\n";
	if (c.file.GetScriptInfo("ScriptType").empty())
		ser += "ScriptType: v4.00+\n";
	ser += "\n[V4+ Styles]\n";
	for (auto const& st : c.file.Styles)
		ser += st.GetEntryData() + "\n";
	ser += "\n[Events]\n";
	int i = 0;
	for (auto const& ev : c.file.Events) {
		if (sadece_idx < 0 || i == sadece_idx)
			ser += ev.GetEntryData() + "\n";
		++i;
	}
	return ser;
}


int velnosub_abi_render_open(int h, int gen, int yuk, const char *yol_font,
                             const char *aile_font) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (g_r.count(h)) r_close(g_r.at(h)); // yeniden açık: önceki şartı yumlat
	if (gen <= 0 || gen > 4096 || yuk <= 0 || yuk > 4096) {
		set_err("render çerçeve ölçüleri aralık dışı");
		return -1;
	}
	try {
		RCtx &r = g_r[h];
		r.gen = gen;
		r.yuk = yuk;
		r.lib = ::ass_library_init();
		if (!r.lib) {
			r_close(r);
			set_err("ass_library_init bitti boş");
			return -1;
		}
		if (yol_font && *yol_font) {
			std::string fdata = read_all(yol_font);
			if (fdata.empty()) {
				r_close(r);
				set_err("font dosyası okunamadı: " + std::string(yol_font));
				return -1;
			}
			::ass_add_font(r.lib, aile_font && *aile_font ? aile_font : "Default",
			               fdata.data(), static_cast<int>(fdata.size()));
		}
		// bellekten seri: durum (düzenlemeler) birebir taşınır; PlayRes dahil
		Ctx &c = *g_ctx.at(h);
		std::string ser = ass_seri(c, -1);
		r.track = ::ass_new_track(r.lib);
		::ass_process_data(r.track, ser.data(), static_cast<int>(ser.size()));
		if (r.track->n_events <= 0) {
			r_close(r);
			set_err("libass olay listesi boş");
			return -1;
		}
		r.ren = ::ass_renderer_init(r.lib);
		if (!r.ren) {
			r_close(r);
			set_err("ass_renderer_init bitti boş");
			return -1;
		}
		::ass_set_frame_size(r.ren, r.gen, r.yuk);
		::ass_set_fonts(r.ren, yol_font && *yol_font ? yol_font : nullptr,
		                nullptr, ASS_FONTPROVIDER_NONE, nullptr, 1);
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

// Kare deposine dizgi alfa kompoziti yaz; glyph etki sayısını döndürür.
static int kare_yaz(RCtx &r, ASS_Track *track, int ms, unsigned char *buf, int n) {
	if (!buf) { set_err("boş kare depositi"); return -1; }
	if (n != r.gen * r.yuk * 4) {
		set_err("kare depositi boyu " + std::to_string(n) + " != "
		        + std::to_string(r.gen * r.yuk * 4));
		return -1;
	}
	int change = 0;
	ASS_Image *imgs = ::ass_render_frame(r.ren, track, ms, &change);
	// P4.6 dilim 2: iç kare artık DİZGİ alfa tutar (0xAARRGGBB; zemin
	// alfa 0) — egui video üstüne gerçek saydamlayış işlemiyle biner.
	// over kompozit (dizgi alfa): out_a = sa + da*(255-sa)/255,
	// out_rgb = (src*sa + dst*da*(255-sa)/255)/out_a. [AB9] glyph sayacı
	// etki koşuluna bağlı olduğu için sayı birebir kalır.
	std::vector<uint32_t> kare(static_cast<size_t>(r.gen) * r.yuk, 0u);
	unsigned long glyphs = 0;
	for (ASS_Image *img = imgs; img; img = img->next) {
		if (!img->bitmap || img->w <= 0 || img->h <= 0) continue;
		uint32_t color = img->color;
		unsigned alfa = (color >> 24) & 0xFF;      // 0 opak, 255 saydam
		unsigned kr = (color >> 16) & 0xFF;
		unsigned kg = (color >> 8) & 0xFF;
		unsigned kb = color & 0xFF;
		for (int yy = 0; yy < img->h; ++yy) {
			int py = img->dst_y + yy;
			if (py < 0 || py >= r.yuk) continue;
			unsigned char const *row = img->bitmap + yy * img->stride;
			for (int xx = 0; xx < img->w; ++xx) {
				int px = img->dst_x + xx;
				if (px < 0 || px >= r.gen) continue;
				unsigned etki = row[xx] * (255u - alfa) / 255u;
				if (!etki) continue;
				++glyphs;
				uint32_t &dst = kare[static_cast<size_t>(py) * r.gen + px];
				unsigned da = (dst >> 24) & 0xFF;
				unsigned dr = (dst >> 16) & 0xFF, dg = (dst >> 8) & 0xFF,
				         db = dst & 0xFF;
				unsigned w1 = da * (255u - etki) / 255u;
				unsigned oa = etki + w1;
				dr = (kr * etki + dr * w1) / oa;
				dg = (kg * etki + dg * w1) / oa;
				db = (kb * etki + db * w1) / oa;
				dst = (oa << 24) | (dr << 16) | (dg << 8) | db;
			}
		}
	}
	for (int y = 0; y < r.yuk; ++y) {
		unsigned char *satir = buf + (static_cast<size_t>(y) * r.gen) * 4;
		for (int x = 0; x < r.gen; ++x) {
			uint32_t p = kare[static_cast<size_t>(y) * r.gen + x];
			satir[x * 4 + 0] = static_cast<unsigned char>((p >> 16) & 0xFF);
			satir[x * 4 + 1] = static_cast<unsigned char>((p >> 8) & 0xFF);
			satir[x * 4 + 2] = static_cast<unsigned char>(p & 0xFF);
			satir[x * 4 + 3] = static_cast<unsigned char>((p >> 24) & 0xFF);
		}
	}
	return static_cast<int>(glyphs);
}

// ms içinde tüm olayları çiz; RGBA sekizli (r,g,b,ALFA), zemin alfa 0.
int velnosub_abi_render_frame(int h, int ms, unsigned char *buf, int n) {
	if (!g_r.count(h)) { set_err("tanımsız render tutamaç"); return -1; }
	RCtx &r = g_r.at(h);
	return kare_yaz(r, r.track, ms, buf, n);
}

// Yalnız idx olayını çiz — typesetting önizlemesi. Aynı renderer/font/ölçü
// ve aynı başlık serileştirmesi kullanılır; tam kare önizlemesiyle hizalıdır.
int velnosub_abi_render_line(int h, int idx, int ms, unsigned char *buf, int n) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!g_r.count(h)) { set_err("tanımsız render tutamaç"); return -1; }
	Ctx &c = *g_ctx.at(h);
	RCtx &r = g_r.at(h);
	if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
		set_err("satır dışında indeks: " + std::to_string(idx));
		return -1;
	}
	if (ms < 0) { set_err("negatif zaman"); return -1; }
	try {
		std::string ser = ass_seri(c, idx);
		ASS_Track *t = ::ass_new_track(r.lib);
		if (!t) { set_err("ass_new_track bitti boş"); return -1; }
		::ass_process_data(t, ser.data(), static_cast<int>(ser.size()));
		if (t->n_events <= 0) {
			::ass_free_track(t);
			set_err("libass geçici olay listesi boş");
			return -1;
		}
		int son = kare_yaz(r, t, ms, buf, n);
		::ass_free_track(t);
		return son;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_render_close(int h) {
	if (!g_r.count(h)) { set_err("tanımsız render tutamaç"); return -1; }
	r_close(g_r.at(h));
	g_r.erase(h);
	return 0;
}

// ASS koordinat uzayı: PlayResX/Y; eksikse Aegisub/libass uyumlu
// 384x288 (veya tek eksenden türetilen) varsayım. UI video yoksa 640x480
// kullanabilir; render hattı bu boyutu izler.
int velnosub_abi_script_size(int h, int *out_w, int *out_h) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!out_w || !out_h) { set_err("boş boyut çıktısı"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		int w = 0, hh = 0;
		c.file.GetResolution(w, hh);
		*out_w = w;
		*out_h = hh;
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_style_of(int h, int idx, char *buf, int buflen) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Styles.size())) {
			set_err("stil dışında indeks: " + std::to_string(idx));
			return -1;
		}
		std::string const t = stil_dizgesi(*std::next(c.file.Styles.begin(), idx));
		int const n = static_cast<int>(t.size());
		int const m = (buflen - 1) < n ? (buflen - 1) : n;
		std::memcpy(buf, t.data(), static_cast<size_t>(m));
		buf[m] = '\0';
		return n;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_style_edit_size(int h, int idx, double size) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (size < 2.0 || size > 9999.0) { set_err("stil boyutu dışında"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Styles.size())) {
			set_err("stil dışında indeks: " + std::to_string(idx));
			return -1;
		}
		AssStyle &s = *std::next(c.file.Styles.begin(), idx);
		s.fontsize = size;
		s.UpdateData();
		c.file.Commit("velnosub abi", AssFile::COMMIT_STYLES);
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_style_edit_name(int h, int idx, const char *name) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!name || !*name) { set_err("stil adı boş"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Styles.size())) {
			set_err("stil dışında indeks: " + std::to_string(idx));
			return -1;
		}
		AssStyle &s = *std::next(c.file.Styles.begin(), idx);
		s.name = name;
		s.UpdateData();
		c.file.Commit("velnosub abi", AssFile::COMMIT_STYLES);
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

// ---- P4.5 devam: tam stil yazma (style_of dizgesinin geri döngüsü) -----

// "&HAABBGGRR&" | "&HAABBGGRR" | "aabbggrr" → agi::Color (public alan dolu)
static bool ass_renk_ayikla(std::string metin, agi::Color &renk) {
	if (!metin.empty() && (metin.front() == '&')) metin.erase(0, 1);
	if (!metin.empty() && (metin.back() == '&')) metin.pop_back();
	if (!metin.empty() && (metin.front() == 'H' || metin.front() == 'h'))
		metin.erase(0, 1);
	if (metin.size() != 8) return false;
	unsigned long deger;
	try {
		deger = std::stoul(metin, nullptr, 16);
	} catch (std::exception const&) {
		return false;
	}
	renk.a = (unsigned char)((deger >> 24) & 0xFF);
	renk.b = (unsigned char)((deger >> 16) & 0xFF);
	renk.g = (unsigned char)((deger >> 8) & 0xFF);
	renk.r = (unsigned char)(deger & 0xFF);
	return true;
}

// "0,0,0,0" → dört bool (kalın, italik, altçizgi, üstçizgi)
static bool bi_coz(std::string const& metin, bool &b, bool &i, bool &u, bool &s) {
	size_t p1 = metin.find(',');
	if (p1 == std::string::npos) return false;
	size_t p2 = metin.find(',', p1 + 1);
	if (p2 == std::string::npos) return false;
	size_t p3 = metin.find(',', p2 + 1);
	if (p3 == std::string::npos) return false;
	auto bir = [](std::string k) { return k == "1"; };
	b = bir(metin.substr(0, p1));
	i = bir(metin.substr(p1 + 1, p2 - p1 - 1));
	u = bir(metin.substr(p2 + 1, p3 - p2 - 1));
	s = bir(metin.substr(p3 + 1));
	return true;
}

int velnosub_abi_style_write(int h, int idx, const char *dizge) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!dizge || !*dizge) { set_err("boş stil dizgesi"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Styles.size())) {
			set_err("stil dışında indeks: " + std::to_string(idx));
			return -1;
		}
		std::string const metin(dizge);
		std::vector<std::string> hucre;
		size_t bas = 0;
		while (true) {
			size_t son = metin.find('|', bas);
			hucre.push_back(metin.substr(
				bas, son == std::string::npos ? std::string::npos : son - bas));
			if (son == std::string::npos) break;
			bas = son + 1;
		}
		if (hucre.size() != 13) {
			set_err("stil dizgesi 13 hanede değil: " + std::to_string(hucre.size()));
			return -1;
		}
		// TÜM alanlar çözülmeden hiçbir alan yazılmaz (yarım durum yok)
		if (hucre[0].empty() || hucre[1].empty()) {
			set_err("stil adı/font boş");
			return -1;
		}
		double boyut;
		try {
			boyut = std::stod(hucre[2]);
		} catch (std::exception const&) {
			set_err("stil boyut sayısı değil: " + hucre[2]);
			return -1;
		}
		if (boyut < 2.0 || boyut > 9999.0) {
			set_err("stil boyutu dışında");
			return -1;
		}
		agi::Color ana, ikincil, kontur, golge;
		if (!ass_renk_ayikla(hucre[3], ana) || !ass_renk_ayikla(hucre[4], ikincil)
		    || !ass_renk_ayikla(hucre[5], kontur) || !ass_renk_ayikla(hucre[6], golge)) {
			set_err("renk dizgesi çözülemedi (ASS &HAABBGGRR)");
			return -1;
		}
		bool kb, ki, ku, ks;
		if (!bi_coz(hucre[7], kb, ki, ku, ks)) {
			set_err("biçim baytları hatalı (kalın,italik,altçizgi,üstçizgi)");
			return -1;
		}
		auto sayi_okuma = [](std::string const& metin, double &deger) {
			try {
				size_t ultani;
				deger = std::stod(metin, &ultani);
				return ultani == metin.size();
			} catch (std::exception const&) {
				return false;
			}
		};
		double hiza_d, kb_d, kg_d, kd_d, kod_d, ox_d, oy_d, hr_d, ac_d, kst_d, kg2_d, gg_d;
		std::vector<std::string> kenar, olcek; // hücre 9 / 11 parçaları
		{
			std::string &k = hucre[9];
			size_t bas = 0;
			while (true) {
				size_t son = k.find(',', bas);
				kenar.push_back(k.substr(
					bas, son == std::string::npos ? std::string::npos : son - bas));
				if (son == std::string::npos) break;
				bas = son + 1;
			}
			std::string &m = hucre[11];
			bas = 0;
			while (true) {
				size_t son = m.find(',', bas);
				olcek.push_back(m.substr(
					bas, son == std::string::npos ? std::string::npos : son - bas));
				if (son == std::string::npos) break;
				bas = son + 1;
			}
			std::string &u = hucre[12];
			size_t v1 = u.find(','); size_t v2 = u.find(',', v1 + 1);
			if (v1 == std::string::npos || v2 == std::string::npos) {
				set_err("kenar stili üçlüsü hatalı");
				return -1;
			}
			if (!sayi_okuma(hucre[8], hiza_d) || !sayi_okuma(kenar[0], kb_d)
			    || !sayi_okuma(kenar[1], kg_d) || !sayi_okuma(kenar[2], kd_d)
			    || !sayi_okuma(hucre[10], kod_d) || !sayi_okuma(olcek[0], ox_d)
			    || !sayi_okuma(olcek[1], oy_d) || !sayi_okuma(olcek[2], hr_d)
			    || !sayi_okuma(olcek[3], ac_d) || !sayi_okuma(u.substr(0, v1), kst_d)
			    || !sayi_okuma(u.substr(v1 + 1, v2 - v1 - 1), kg2_d)
			    || !sayi_okuma(u.substr(v2 + 1), gg_d)) {
				set_err("sayısal alan çözülemedi");
				return -1;
			}
		}
		if (kenar.size() < 3 || olcek.size() < 4) {
			set_err("kenar/ölçek hücreleri eksik");
			return -1;
		}
		int hiza = (int)hiza_d, kod = (int)kod_d, kst = (int)kst_d;
		if (hiza < 1 || hiza > 9) {
			set_err("hiza 1..9 olmalı");
			return -1;
		}
		if (kst != 1 && kst != 3) {
			set_err("kenar stili 1 veya 3 olmalı");
			return -1;
		}
		AssStyle &s = *std::next(c.file.Styles.begin(), idx);
		s.name = hucre[0];
		s.font = hucre[1];
		s.fontsize = boyut;
		s.primary = ana;
		s.secondary = ikincil;
		s.outline = kontur;
		s.shadow = golge;
		s.bold = kb;
		s.italic = ki;
		s.underline = ku;
		s.strikeout = ks;
		s.alignment = hiza;
		s.Margin[0] = (int)kb_d;
		s.Margin[1] = (int)kg_d;
		s.Margin[2] = (int)kd_d;
		s.encoding = kod;
		s.scalex = ox_d;
		s.scaley = oy_d;
		s.spacing = hr_d;
		s.angle = ac_d;
		s.borderstyle = kst;
		s.outline_w = kg2_d;
		s.shadow_w = gg_d;
		s.UpdateData();
		c.file.Commit("velnosub abi", AssFile::COMMIT_STYLES);
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

// ---- P8.1: tam stil yöneticisi — yeni stil / kopya / sil ----
// style_write (13 hücre) tek başına alan yazımını yapar; liste büyümesi için
// ekle/sil burada. Yeni stil ilk stilin alanlarını taban alır (renk/imza
// varsayılanları böylece dosyayla uyumlu); ad boş ya da çakışık olamaz.
extern "C" int velnosub_abi_style_add(int h, const char *ad) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!ad || !*ad) { set_err("stil adı boş"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		for (auto const &s : c.file.Styles)
			if (s.name == ad) {
				set_err("stil adı zaten var: " + std::string(ad));
				return -1;
			}
		AssStyle s;
		if (!c.file.Styles.empty()) s = *c.file.Styles.begin();
		s.name = ad;
		if (s.font.empty()) s.font = "Arial";
		if (s.fontsize <= 0.0) s.fontsize = 20.0;
		s.alignment = 2;
		s.Margin[0] = s.Margin[1] = s.Margin[2] = 10;
		s.scalex = s.scaley = 100.0;
		s.borderstyle = s.borderstyle == 3 ? 3 : 1;
		s.UpdateData();
		c.file.Styles.push_back(s);
		c.file.Commit("velnosub abi", AssFile::COMMIT_STYLES);
		return static_cast<int>(c.file.Styles.size()) - 1;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

extern "C" int velnosub_abi_style_delete(int h, int idx) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		int const n = static_cast<int>(c.file.Styles.size());
		if (idx < 0 || idx >= n) {
			set_err("stil dışında indeks: " + std::to_string(idx));
			return -1;
		}
		if (n <= 1) { set_err("son stil silinemez"); return -1; }
		auto it = std::next(c.file.Styles.begin(), idx);
		// silinen stili kullanan satırlar ilk kalan stile taşınır
		std::string kalan;
		for (auto const &s : c.file.Styles)
			if (&s != &*it) { kalan = s.name; break; }
		for (auto &e : c.file.Events)
			if (e.Style == it->name) e.Style = kalan;
		c.file.Styles.erase(it);
		c.file.Commit("velnosub abi", AssFile::COMMIT_STYLES);
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

// ---- P4.5 III / Faz 5 D:dialog formları omurgası (dinamik tanımlar) ----
// mini tanım DSL: her satır "ctl|<sınıf>|<ad>|<başlık>|<değer>[|<ek>]" ya da
// "buton|<metin>", '#' yorum. sınıflar: label edit intedit floatedit
// checkbox dropdown textbox color coloralpha (aegisub.dialog sınıfları adaysı)

struct Alan {
	std::string sinif, ad, baslik, deger;
	double alt = 0.0, ust = 0.0; // intedit/floatedit sınırları
	std::vector<std::string> ogeler;
};
struct DialogDef {
	std::vector<Alan> alanlar;
	std::vector<std::string> dugmeler;
	int son_dugme = -1;
};
std::map<int, std::unique_ptr<DialogDef>> g_dialog;
int g_dnext = 0;

static std::vector<std::string> boru_ayikla(std::string const& metin) {
	std::vector<std::string> parca;
	size_t bas = 0;
	while (true) {
		size_t son = metin.find('|', bas);
		parca.push_back(metin.substr(
			bas, son == std::string::npos ? std::string::npos : son - bas));
		if (son == std::string::npos) break;
		bas = son + 1;
	}
	return parca;
}

static bool belge_sayi(std::string const& metin, double &deger) {
	if (metin.empty()) return false;
	try {
		size_t kalan;
		deger = std::stod(metin, &kalan);
		return kalan == metin.size();
	} catch (std::exception const&) {
		return false;
	}
}

int velnosub_abi_dialog_open(const char *spec) {
	if (!g_ready) { set_err("init çağrılmadı"); return -1; }
	if (!spec || !*spec) { set_err("boş dialog tanımı"); return -1; }
	try {
		auto dd = std::make_unique<DialogDef>();
		std::string metin(spec);
		size_t bas = 0;
		bool dosya_sonu = false;
		while (!dosya_sonu) {
			size_t son = metin.find('\n', bas);
			std::string satir = metin.substr(
				bas, son == std::string::npos ? std::string::npos : son - bas);
			if (son == std::string::npos) dosya_sonu = true;
			else bas = son + 1;
			while (!satir.empty() && (satir.back() == '\r' || satir.back() == ' '))
				satir.pop_back();
			if (satir.empty() || satir[0] == '#') continue;
			auto parca = boru_ayikla(satir);
			if (parca[0] == "buton") {
				if (parca.size() < 2 || parca[1].empty()) {
					set_err("buton metni eksik");
					return -1;
				}
				dd->dugmeler.push_back(parca[1]);
				continue;
			}
			if (parca[0] != "ctl") {
				set_err("satır ctl/buton değil: " + satir);
				return -1;
			}
			if (parca.size() < 4) {
				set_err("ctl hücre eksik (4 min): " + satir);
				return -1;
			}
			Alan a;
			a.sinif = parca[1];
			a.ad = parca[2];
			a.baslik = parca[3];
			a.deger = parca.size() > 4 ? parca[4] : std::string();
			if (a.ad.empty() && a.sinif != "label") {
				set_err("ctl adı boş: " + a.sinif);
				return -1;
			}
			if (a.sinif != "label" && a.deger.empty()) {
				set_err("ctl değeri boş (label hariç): " + a.ad);
				return -1;
			}
			std::string ek = parca.size() > 5 ? parca[5] : std::string();
			auto sinir = [&](std::string const& met, double &alt, double &ust) {
				size_t v = met.find(',');
				if (v == std::string::npos) return false;
				return belge_sayi(met.substr(0, v), alt) &&
				       belge_sayi(met.substr(v + 1), ust);
			};
			auto oge_ayikla = [&](std::string const& met) {
				std::vector<std::string> dizi;
				size_t b = 0;
				while (true) {
					size_t s = met.find(';', b);
					dizi.push_back(met.substr(
						b, s == std::string::npos ? std::string::npos : s - b));
					if (s == std::string::npos) break;
					b = s + 1;
				}
				return dizi;
			};
			if (a.sinif == "intedit" || a.sinif == "floatedit") {
				if (ek.empty() || !sinir(ek, a.alt, a.ust)) {
					set_err(a.sinif + " için sınır hücresi alt,ust");
					return -1;
				}
				double y;
				if (!belge_sayi(a.deger, y)) {
					set_err(a.sinif + " değeri sayı değil: " + a.deger);
					return -1;
				}
			} else if (a.sinif == "checkbox") {
				if (a.deger != "0" && a.deger != "1") {
					set_err("checkbox değeri 0/1 olmalı");
					return -1;
				}
			} else if (a.sinif == "dropdown") {
				a.ogeler = oge_ayikla(ek);
				bool var = false;
				for (auto const& o : a.ogeler) if (o == a.deger) var = true;
				if (a.ogeler.empty() || !var) {
					set_err("dropdown değeri öğeler arasında değil");
					return -1;
				}
			} else if (a.sinif == "label" || a.sinif == "edit" || a.sinif == "textbox"
			           || a.sinif == "color" || a.sinif == "coloralpha") {
				// ek sütunu arında yok
			} else {
				set_err("bilinmeyen ctl sınıfı: " + a.sinif);
				return -1;
			}
			dd->alanlar.push_back(std::move(a));
		}
		if (dd->alanlar.empty()) { set_err("dialog alanı boş"); return -1; }
		if (dd->dugmeler.empty()) dd->dugmeler = {"Uygula"};
		int h = ++g_dnext;
		g_dialog.emplace(h, std::move(dd));
		return h;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_dialog_count(int h) {
	if (!g_dialog.count(h)) { set_err("tanımsız dialog tutamacı"); return -1; }
	return static_cast<int>(g_dialog.at(h)->alanlar.size());
}

int velnosub_abi_dialog_btncount(int h) {
	if (!g_dialog.count(h)) { set_err("tanımsız dialog tutamacı"); return -1; }
	return static_cast<int>(g_dialog.at(h)->dugmeler.size());
}

int velnosub_abi_dialog_ctl(int h, int idx, char *buf, int buflen) {
	if (!g_dialog.count(h)) { set_err("tanımsız dialog tutamacı"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	try {
		DialogDef &d = *g_dialog.at(h);
		if (idx < 0 || idx >= static_cast<int>(d.alanlar.size())) {
			set_err("alan dışında indeks: " + std::to_string(idx));
			return -1;
		}
		Alan const& a = d.alanlar[idx];
		std::string t = a.sinif + "|" + a.ad + "|" + a.baslik + "|" + a.deger;
		if (a.sinif == "intedit" || a.sinif == "floatedit") {
			t += "|" + std::to_string(a.alt) + "," + std::to_string(a.ust);
		} else if (!a.ogeler.empty()) {
			t += "|";
			for (size_t i = 0; i < a.ogeler.size(); ++i) {
				if (i) t += ";";
				t += a.ogeler[i];
			}
		}
		int const n = static_cast<int>(t.size());
		int const m = (buflen - 1) < n ? (buflen - 1) : n;
		std::memcpy(buf, t.data(), static_cast<size_t>(m));
		buf[m] = '\0';
		return n;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_dialog_button(int h, int idx, char *buf, int buflen) {
	if (!g_dialog.count(h)) { set_err("tanımsız dialog tutamacı"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	try {
		DialogDef &d = *g_dialog.at(h);
		if (idx < 0 || idx >= static_cast<int>(d.dugmeler.size())) {
			set_err("düğme dışında indeks: " + std::to_string(idx));
			return -1;
		}
		std::string const& t = d.dugmeler[idx];
		int const n = static_cast<int>(t.size());
		int const m = (buflen - 1) < n ? (buflen - 1) : n;
		std::memcpy(buf, t.data(), static_cast<size_t>(m));
		buf[m] = '\0';
		return n;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_dialog_set(int h, int idx, const char *deger) {
	if (!g_dialog.count(h)) { set_err("tanımsız dialog tutamacı"); return -1; }
	if (!deger) { set_err("boş değer"); return -1; }
	try {
		DialogDef &d = *g_dialog.at(h);
		if (idx < 0 || idx >= static_cast<int>(d.alanlar.size())) {
			set_err("alan dışında indeks: " + std::to_string(idx));
			return -1;
		}
		Alan &a = d.alanlar[idx];
		std::string metin(deger);
		auto aradi = [](std::vector<std::string> const& dizi, std::string const& m) {
			for (auto const& o : dizi) if (o == m) return true;
			return false;
		};
		if (a.sinif == "intedit") {
			double y;
			if (!belge_sayi(metin, y) || y != std::floor(y)) {
				set_err("intedit değeri tam sayı değil: " + metin);
				return -1;
			}
		} else if (a.sinif == "floatedit") {
			double y;
			if (!belge_sayi(metin, y)) {
				set_err("floatedit değeri sayı değil: " + metin);
				return -1;
			}
		} else if (a.sinif == "checkbox") {
			if (metin != "0" && metin != "1") {
				set_err("checkbox değeri 0/1 olmalı");
				return -1;
			}
		} else if (a.sinif == "dropdown") {
			if (!aradi(a.ogeler, metin)) {
				set_err("dropdown değeri öğeler arasında değil: " + metin);
				return -1;
			}
		}
		a.deger = metin;
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_dialog_show(int h, int dugme) {
	if (!g_dialog.count(h)) { set_err("tanımsız dialog tutamacı"); return -1; }
	DialogDef &d = *g_dialog.at(h);
	if (dugme < 0 || dugme >= static_cast<int>(d.dugmeler.size())) {
		set_err("düğme dışında indeks: " + std::to_string(dugme));
		return -1;
	}
	d.son_dugme = dugme;
	return dugme;
}

int velnosub_abi_dialog_showid(int h) {
	if (!g_dialog.count(h)) { set_err("tanımsız dialog tutamacı"); return -1; }
	return g_dialog.at(h)->son_dugme;
}

int velnosub_abi_dialog_close(int h) {
	if (!g_dialog.count(h)) return 0;
	g_dialog.erase(h);
	return 0;
}

// ---- P4.3 omurgası: video kolu (BGRA kare teslimi) -------------------

int velnosub_abi_video_open(const char *path) {
	if (!g_ready) { set_err("init çağrılmadı"); return -1; }
	if (!path || !*path) { set_err("video yolu boş"); return -1; }
	try {
		auto prov = VideoProviderFactory::GetProvider(agi::fs::path(path),
		                                              std::string(), nullptr);
		if (!prov) { set_err("video okuyucu bulunamadı: " + std::string(path)); return -1; }
		int h = ++g_vnext;
		g_video.emplace(h, std::move(prov));
		return h;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_video_info(int h, char *buf, int buflen) {
	if (!g_video.count(h)) { set_err("tanımsız video tutamacı"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	try {
		VideoProvider &v = *g_video.at(h);
		char fmt[160];
		std::snprintf(fmt, sizeof fmt, "%dx%d fps:%.3f kare:%d %s",
		              v.GetWidth(), v.GetHeight(), v.GetFPS().FPS(),
		              v.GetFrameCount(), v.GetDecoderName().c_str());
		int const n = static_cast<int>(std::strlen(fmt));
		int const m = (buflen - 1) < n ? (buflen - 1) : n;
		std::memcpy(buf, fmt, static_cast<size_t>(m));
		buf[m] = '\0';
		return n;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_video_frame(int h, int n_kare, unsigned char *buf, int buflen) {
	if (!g_video.count(h)) { set_err("tanımsız video tutamacı"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	try {
		VideoFrame f;
		g_video.at(h)->GetFrame(n_kare, f);
		size_t const needed = f.width * f.height * 4; // BGRA, pitch=w*4
		if (buflen < 0 || (size_t)buflen < needed)
			return static_cast<int>(needed); // daha büyük tampon çağır
		size_t const lines = f.height;
		for (size_t y = 0; y < lines; ++y)
			std::memcpy(buf + y * f.pitch, f.data.data() + y * f.pitch, f.pitch);
		return static_cast<int>(needed);
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

int velnosub_abi_video_geom(int h, int *out_w, int *out_h, double *out_fps, int *out_kare) {
	if (!g_video.count(h)) { set_err("tanımsız video tutamacı"); return -1; }
	try {
		VideoProvider &v = *g_video.at(h);
		if (out_w) *out_w = v.GetWidth();
		if (out_h) *out_h = v.GetHeight();
		if (out_fps) *out_fps = v.GetFPS().FPS();
		if (out_kare) *out_kare = v.GetFrameCount();
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
}

void velnosub_abi_video_close(int h) {
	g_video.erase(h);
}

// ---- P4.4/P3.5: ses kolu --------------------------------------------

// ---- P5.5: ham s16 PCM'i AudioWave olarak kaydet (peak haritası taranır;
// audio_open ham-PCM hem kutu-decode hattı da SONUNDA BURADAN geçer) ----
static int ses_pcm_kayit_ekle(const char *path, int rate, int ch) {
	std::FILE *f = std::fopen(path, "rb");
	if (!f) { set_err("ses dosyası açılamadı: " + std::string(path)); return -1; }
	std::fseek(f, 0, SEEK_END);
	long baytlar = std::ftell(f);
	std::fseek(f, 0, SEEK_SET);
	int64_t toplam = (int64_t)baytlar / (2LL * ch);
	if (toplam <= 0) {
		std::fclose(f);
		set_err("ses verisi yok: " + std::string(path));
		return -1;
	}
	auto aw = std::make_unique<AudioWave>();
	aw->rate = rate;
	aw->ch = ch;
	aw->toplam_ornek = toplam;
	aw->yol = path;
	// peak haritası: 256 karelik kovalar, akış-tarayıcı
	int64_t kovalar = (toplam + aw->kova_ornekleme - 1) / aw->kova_ornekleme;
	aw->kova_min.assign((size_t)kovalar, 32767);
	aw->kova_max.assign((size_t)kovalar, -32768);
	std::vector<int16_t> cane(4096);
	int64_t reset = 0;  // imleç (kare)
	int64_t kova_id = 0;
	int16_t dmin = 32767, dmax = -32768;
	size_t okunan;
	while ((okunan = std::fread(cane.data(), sizeof(int16_t), cane.size(), f)) > 0) {
		size_t kare = okunan / ch;
		for (size_t i = 0; i < kare; ++i) {
			int16_t hedef = cane[i * ch]; // tek kanal örneklemesi
			if (hedef < dmin) dmin = hedef;
			if (hedef > dmax) dmax = hedef;
			++reset;
			if (reset == aw->kova_ornekleme * (kova_id + 1)) {
				if (kova_id < kovalar) {
					aw->kova_min[(size_t)kova_id] = dmin;
					aw->kova_max[(size_t)kova_id] = dmax;
				}
				++kova_id;
				dmin = 32767;
				dmax = -32768;
			}
		}
	}
	std::fclose(f);
	// dosya imleci kapatıldı; oynatma yol'dan yeniden açar
	int h = ++g_anext;
	g_audio.emplace(h, std::move(aw));
	return h;
}

int velnosub_abi_audio_open(const char *path, int rate, int ch) {
	if (!g_ready) { set_err("init çağrılmadı"); return -1; }
	if (!path || !*path) { set_err("ses yolu boş"); return -1; }
	if (rate <= 0 || ch <= 0 || ch > 8) { set_err("geçersiz rate/kanal"); return -1; }
	return ses_pcm_kayit_ekle(path, rate, ch);
}

// P5.5: kutu-sesi (mp3/m4a/flac/mkv/mp4 …) — çekirdek ffmpeg hattıyla İLK ses
// akışını s16 stereo 48000'e çözer, önbellek dosyasına (VELNOSUB_CACHE altı)
// yazar; AudioWave kaydı o önbellekten kurulur → peak/oynatma hattı aynen
// kalır (Oboe dosyadan akar). Çok kanallı girişler kanonik stereo'ya düşer.
extern "C" int velnosub_abi_audio_file_open(const char *in_path) {
	if (!g_ready) { set_err("init çağrılmadı"); return -1; }
	if (!in_path || !*in_path) { set_err("ses yolu boş"); return -1; }
	const int hedef_rate = 48000, hedef_ch = 2;
	std::string onbellek;
	if (const char *cb = std::getenv("VELNOSUB_CACHE"); cb && *cb)
		onbellek = std::string(cb) + "/user-ses.pcm";
	else if (const char *ex = std::getenv("VELNOSUB_EXTDIR"); ex && *ex)
		onbellek = std::string(ex) + "/user-ses.pcm";
	else
		onbellek = "user-ses.pcm";
	AVFormatContext *fmt = nullptr;
	AVCodecContext *cc = nullptr;
	SwrContext *swr = nullptr;
	AVPacket *pkt = nullptr;
	AVFrame *frm = nullptr;
	std::FILE *cik = nullptr;
	bool tamam = false;
	do {
		if (avformat_open_input(&fmt, in_path, nullptr, nullptr) < 0 || !fmt) {
			avformat_free_context(fmt);
			set_err("kutu açılamadı (ffmpeg): " + std::string(in_path));
			break;
		}
		if (avformat_find_stream_info(fmt, nullptr) < 0) {
			set_err("akış bilgisi çıkarılamadı: " + std::string(in_path));
			break;
		}
		int si = -1;
		for (unsigned i = 0; i < fmt->nb_streams && si < 0; ++i)
			if (fmt->streams[i]->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) si = (int)i;
		if (si < 0) { set_err("ses akışı yok: " + std::string(in_path)); break; }
		AVStream *st = fmt->streams[si];
		const AVCodec *co = avcodec_find_decoder(st->codecpar->codec_id);
		if (!co) {
			set_err("ses çözücüsü yok: codec " +
			        std::to_string((int)st->codecpar->codec_id));
			break;
		}
		cc = avcodec_alloc_context3(co);
		if (!cc || avcodec_parameters_to_context(cc, st->codecpar) < 0) {
			set_err("çözücü bağlamı kurulamadı");
			break;
		}
		if (avcodec_open2(cc, co, nullptr) < 0) {
			set_err(std::string("çözücü açılamadı: ") + co->name);
			break;
		}
		// kanonik giriş düzeni: N kanal → varsayılan N-maske (mono/stereo/5.1)
		int64_t in_layout =
		    av_get_default_channel_layout(cc->channels > 0 ? cc->channels : 2);
		swr = swr_alloc_set_opts(nullptr, AV_CH_LAYOUT_STEREO, AV_SAMPLE_FMT_S16,
		                         hedef_rate, in_layout, cc->sample_fmt,
		                         cc->sample_rate, 0, nullptr);
		if (!swr || swr_init(swr) < 0) {
			set_err("resamplama kurulumu başarısız");
			break;
		}
		cik = std::fopen(onbellek.c_str(), "wb");
		if (!cik) { set_err("önbellek yazılamaz: " + onbellek); break; }
		pkt = av_packet_alloc();
		frm = av_frame_alloc();
		if (!pkt || !frm) { set_err("ffmpeg tamponları kurulamadı"); break; }
		std::vector<uint8_t> tampon(1 << 16);
		const int kare_bayt = hedef_ch * 2;
		while (av_read_frame(fmt, pkt) >= 0) {
			if (pkt->stream_index != si) { av_packet_unref(pkt); continue; }
			if (avcodec_send_packet(cc, pkt) < 0) { av_packet_unref(pkt); continue; }
			while (avcodec_receive_frame(cc, frm) == 0) {
				const uint8_t *gir[8] = {};
				for (int c = 0; c < 8; ++c) gir[c] = frm->data[c];
				uint8_t *cikis[1] = { tampon.data() };
				// tek çağrı yeter: kalan örnekler swr içi tampona düşer (boşalt sıfır-örnekle)
				int alind = swr_convert(swr, cikis, (int)(tampon.size() / kare_bayt),
				                        gir, frm->nb_samples);
				if (alind > 0)
					std::fwrite(tampon.data(), kare_bayt, alind, cik);
				av_frame_unref(frm);
			}
			av_packet_unref(pkt);
		}
		{ // swr içi işlenmemiş örnekleri boşalt
			uint8_t *cikis[1] = { tampon.data() };
			int alind;
			while ((alind = swr_convert(swr, cikis, (int)(tampon.size() / kare_bayt),
			                            nullptr, 0)) > 0)
				std::fwrite(tampon.data(), kare_bayt, alind, cik);
		}
		std::fclose(cik);
		cik = nullptr;
		tamam = true;
	} while (false);
	av_frame_free(&frm);
	av_packet_free(&pkt);
	swr_free(&swr);
	if (cc) avcodec_free_context(&cc);
	avformat_close_input(&fmt);
	if (cik) std::fclose(cik);
	if (!tamam) {
		std::remove(onbellek.c_str());
		return -1;
	}
	return ses_pcm_kayit_ekle(onbellek.c_str(), hedef_rate, hedef_ch);
}

// ---- P6.4: kutu-İÇİ metin altyazı (MKV S_TEXT/*, MP4 mov_text …) ------
// (1) examine: medyadaki METİN altyazı akışlarını listeler ve
//     <VELNOSUB_CACHE>/vs-subtracks.tsv'ye yazar (uidx\tcodec\tlang);
//     bitmap/PGS vb. resimli akışlar listede YOK (metinçevrilmez).
// (2) extract: seçilen akışı çözer; olay hızları pkt.pts + display
//     ofsetlerinden SRT satırlarına döşer; ass rect'lerin {…} imleri
//     yumuşar. Dönen değer: yazılan olay sayısı.
// NOT: ass/ssa/mov_text/webvtt decoder'ları (P6.4) build-ffmpeg.sh'a
// eklendi — ffmpeg yeniden derlemesi sonrası tam kapsamçok.

static bool alt_metin_kodek(int id) {
	switch (id) {
	case AV_CODEC_ID_SUBRIP:
	case AV_CODEC_ID_SSA:
	case AV_CODEC_ID_ASS:
	case AV_CODEC_ID_MOV_TEXT:
	case AV_CODEC_ID_WEBVTT:
		return true;
	default:
		return false;
	}
}

static std::string srt_saati(int64_t ms) {
	if (ms < 0) ms = 0;
	int64_t saniye = ms / 1000;
	char buf[24];
	std::snprintf(buf, sizeof buf, "%02lld:%02lld:%02lld,%03lld",
	              (long long)(saniye / 3600), (long long)((saniye / 60) % 60),
	              (long long)(saniye % 60), (long long)(ms % 1000));
	return buf;
}

// ass satırından {…} bloklarını atar; \N|\n satır-bitişine döşer
static std::string ass_pack(const char *kaynak) {
	std::string cik;
	if (!kaynak) return cik;
	bool goz = true;
	for (const char *p = kaynak; *p; ++p) {
		if (*p == '{') {
			goz = false;
			continue;
		}
		if (*p == '}') {
			goz = true;
			continue;
		}
		if (!goz) continue;
		if (p[0] == '\\' && (p[1] == 'N' || p[1] == 'n')) {
			cik += '\n';
			++p;
			continue;
		}
		cik += *p;
	}
	while (!cik.empty() && (cik.front() == ' ' || cik.front() == '\t'))
		cik.erase(cik.begin());
	while (!cik.empty() && (cik.back() == ' ' || cik.back() == '\t'))
		cik.pop_back();
	return cik;
}

extern "C" int velnosub_abi_subtitle_examine(const char *in_path) {
	if (!g_ready) { set_err("init çağrılmadı"); return -1; }
	if (!in_path || !*in_path) { set_err("medya yolu boş"); return -1; }
	std::string tsv;
	if (const char *cb = std::getenv("VELNOSUB_CACHE"); cb && *cb)
		tsv = std::string(cb) + "/vs-subtracks.tsv";
	else if (const char *ex = std::getenv("VELNOSUB_EXTDIR"); ex && *ex)
		tsv = std::string(ex) + "/vs-subtracks.tsv";
	else
		tsv = "vs-subtracks.tsv";
	AVFormatContext *fmt = nullptr;
	std::FILE *cik = nullptr;
	int sayi = 0;
	do {
		if (avformat_open_input(&fmt, in_path, nullptr, nullptr) < 0 || !fmt) {
			avformat_free_context(fmt);
			set_err("kutu açılamadı (ffmpeg): " + std::string(in_path));
			break;
		}
		cik = std::fopen(tsv.c_str(), "wb");
		if (!cik) {
			set_err("akış listesi yazılamaz: " + tsv);
			break;
		}
		for (unsigned i = 0; i < fmt->nb_streams; ++i) {
			AVStream *st = fmt->streams[i];
			if (st->codecpar->codec_type != AVMEDIA_TYPE_SUBTITLE) continue;
			if (!alt_metin_kodek((int)st->codecpar->codec_id)) continue;
			const AVCodecDescriptor *des =
			    avcodec_descriptor_get(st->codecpar->codec_id);
			const char *ko = des && des->name ? des->name : "metin";
			const AVDictionaryEntry *dil =
			    av_dict_get(st->metadata, "language", nullptr, 0);
			std::fprintf(cik, "%u\t%s\t%s\n", i, ko,
			             dil && dil->value ? dil->value : "-");
			++sayi;
		}
		std::fclose(cik);
		cik = nullptr;
	} while (false);
	avformat_close_input(&fmt);
	if (cik) std::fclose(cik);
	return sayi; // 0 = metin akışı yok; <0 = hata (set_err)
}

extern "C" int velnosub_abi_subtitle_extract(
    const char *in_path, int rock_idx, const char *dest) {
	if (!g_ready) { set_err("init çağrılmadı"); return -1; }
	if (!in_path || !*in_path || !dest || !*dest) {
		set_err("yol boş");
		return -1;
	}
	AVFormatContext *fmt = nullptr;
	AVCodecContext *cc = nullptr;
	AVPacket *pkt = nullptr;
	std::FILE *cik = nullptr;
	bool tamam = false;
	int olay = 0;
	do {
		if (avformat_open_input(&fmt, in_path, nullptr, nullptr) < 0 || !fmt) {
			avformat_free_context(fmt);
			set_err("kutu açılamadı (ffmpeg): " + std::string(in_path));
			break;
		}
		if (rock_idx < 0 || (unsigned)rock_idx >= fmt->nb_streams) {
			set_err("akış numarası dışı");
			break;
		}
		AVStream *st = fmt->streams[rock_idx];
		if (st->codecpar->codec_type != AVMEDIA_TYPE_SUBTITLE ||
		    !alt_metin_kodek((int)st->codecpar->codec_id)) {
			set_err("akış metin altyazı değil");
			break;
		}
		const AVCodec *co = avcodec_find_decoder(st->codecpar->codec_id);
		if (!co) {
			set_err(std::string("altyazı çözücüsü yok: ") +
			        (co ? co->name : "bilinmeyen"));
			break;
		}
		cc = avcodec_alloc_context3(co);
		if (!cc || avcodec_parameters_to_context(cc, st->codecpar) < 0 ||
		    avcodec_open2(cc, co, nullptr) < 0) {
			set_err(std::string("altyazı çözücüsü açılamadı: ") + co->name);
			break;
		}
		cik = std::fopen(dest, "wb");
		if (!cik) {
			set_err("srt yazılamaz: " + std::string(dest));
			break;
		}
		pkt = av_packet_alloc();
		if (!pkt) { set_err("paket tamponu kurulamadı"); break; }
		AVRational ms_q;
		ms_q.num = 1;
		ms_q.den = 1000;
		while (av_read_frame(fmt, pkt) >= 0) {
			if (pkt->stream_index != rock_idx) {
				av_packet_unref(pkt);
				continue;
			}
			AVSubtitle sub;
			std::memset(&sub, 0, sizeof sub);
			int got = 0;
			int rc =
			    avcodec_decode_subtitle2(cc, &sub, &got, pkt);
			if (rc >= 0 && got && sub.num_rects > 0) {
				int64_t taban = pkt->pts != AV_NOPTS_VALUE ? pkt->pts
				                                           : pkt->dts;
				if (taban == AV_NOPTS_VALUE) taban = 0;
				int64_t pkt_ms =
				    av_rescale_q(taban, st->time_base, ms_q);
				std::string metin;
				for (unsigned r = 0; r < sub.num_rects; ++r) {
					const AVSubtitleRect *d = sub.rects[r];
					if (r > 0) metin += '\n';
					if (d->type == SUBTITLE_TEXT && d->text)
						metin += d->text;
					else if (d->type == SUBTITLE_ASS && d->ass)
						metin += ass_pack(d->ass);
				}
				if (!metin.empty()) {
					int64_t bas = pkt_ms + sub.start_display_time;
					int64_t son = pkt_ms + sub.end_display_time;
					if (son <= bas) son = bas + 500;
					++olay;
					std::fprintf(cik, "%d\n%s --> %s\n%s\n\n", olay,
					             srt_saati(bas).c_str(),
					             srt_saati(son).c_str(), metin.c_str());
				}
			}
			if (rc >= 0) avsubtitle_free(&sub);
			av_packet_unref(pkt);
		}
		std::fclose(cik);
		cik = nullptr;
		if (olay > 0) {
			tamam = true;
		} else {
			set_err("altyazı olayı çıkarılamadı (akış boş mu?)");
		}
	} while (false);
	av_packet_free(&pkt);
	if (cc) avcodec_free_context(&cc);
	avformat_close_input(&fmt);
	if (cik) std::fclose(cik);
	if (!tamam) {
		std::remove(dest);
		return -1;
	}
	return olay;
}

int velnosub_abi_audio_len(int h) {
	if (!g_audio.count(h)) { set_err("tanımsız ses tutamacı"); return -1; }
	int64_t t = g_audio.at(h)->toplam_ornek;
	if (t > INT32_MAX) return INT32_MAX; // kare sayısı int/64 ayrımı için üst kırpma
	return (int)t;
}

int64_t velnosub_abi_audio_len64(int h) {
	if (!g_audio.count(h)) { set_err("tanımsız ses tutamacı"); return -1; }
	return g_audio.at(h)->toplam_ornek;
}

int velnosub_abi_audio_info(int h, char *buf, int buflen) {
	if (!g_audio.count(h)) { set_err("tanımsız ses tutamacı"); return -1; }
	if (!buf || buflen <= 0) { set_err("boş tampon"); return -1; }
	AudioWave &a = *g_audio.at(h);
	char fmt[128];
	std::snprintf(fmt, sizeof fmt, "s16 %dHz %dk %lld örnek (%.2f sn)",
	              a.rate, a.ch, (long long)a.toplam_ornek,
	              a.toplam_ornek / (double)a.rate);
	int const n = static_cast<int>(std::strlen(fmt));
	int const m = (buflen - 1) < n ? (buflen - 1) : n;
	std::memcpy(buf, fmt, (size_t)m);
	buf[m] = '\0';
	return n;
}

int velnosub_abi_audio_peaks(int h, int64_t d0, int64_t adet, int kovalar,
	                         int16_t *minv, int16_t *maxv) {
	if (!g_audio.count(h)) { set_err("tanımsız ses tutamacı"); return -1; }
	if (kovalar <= 0 || !minv || !maxv) { set_err("boş kova işaretçisi"); return -1; }
	AudioWave &a = *g_audio.at(h);
	int64_t kovalar_map = (int64_t)a.kova_min.size();
	for (int k = 0; k < kovalar; ++k) {
		int64_t b0 = (d0 + k * adet / kovalar) / a.kova_ornekleme;
		int64_t b1 = (d0 + (k + 1) * adet / kovalar) / a.kova_ornekleme;
		if (b0 < 0) b0 = 0;
		if (b0 >= kovalar_map) b0 = kovalar_map - 1;
		if (b1 < b0) b1 = b0;
		if (b1 >= kovalar_map) b1 = kovalar_map - 1;
		int16_t dmin = 32767, dmax = -32768;
		for (int64_t b = b0; b <= b1; ++b) {
			if (a.kova_min[(size_t)b] < dmin) dmin = a.kova_min[(size_t)b];
			if (a.kova_max[(size_t)b] > dmax) dmax = a.kova_max[(size_t)b];
		}
		if (dmax < dmin) { dmin = 0; dmax = 0; }
		minv[k] = dmin;
		maxv[k] = dmax;
	}
	return 0;
}

// spektrogram: d0..d0+adet aralığı cols sütuna bölünür; her sütun 256 karelik
// pencere (soldan hizalı) okur. Satır 0 en düşük, rows-1 en yüksek frekans;
// bin eşlemesi doğrusal: bin = row * (kFftN/2) / rows. Eksik/kısa okumalar
// sıfır doldurulur. d0+adet ses sınırını aşarsa aralık reddedilir; böylece
// sütun konumu hesabı taşmaz ve görünür aralık dışına veri okunmaz.
int velnosub_abi_audio_spectrum(int h, int64_t d0, int64_t adet,
                                int cols, int rows, unsigned char *out) {
	if (!g_audio.count(h)) { set_err("tanımsız ses tutamacı"); return -1; }
	if (cols < 1 || cols > 256 || rows < 1 || rows > 256 || !out) {
		set_err("spektrogram boyutu geçersiz");
		return -1;
	}
	if (d0 < 0 || adet <= 0) { set_err("spektrogram aralığı geçersiz"); return -1; }
	AudioWave &a = *g_audio.at(h);
	if (adet > a.toplam_ornek || d0 > a.toplam_ornek - adet) {
		set_err("spektrogram aralığı ses sınırları dışında");
		return -1;
	}
	if (a.yol.empty()) { set_err("ses yolu boş (kolu kapat-aç)"); return -1; }
	std::FILE *f = std::fopen(a.yol.c_str(), "rb");
	if (!f) { set_err("ses dosyası açılamadı: " + a.yol); return -1; }

	static Fft256Tablari const tablo;
	std::vector<int16_t> ham((size_t)kFftN * (size_t)a.ch);
	std::vector<double> ger(kFftN), sanal(kFftN);
	for (int col = 0; col < cols; ++col) {
		int64_t const bas = d0 + (int64_t)col * adet / cols;
		std::fill(ham.begin(), ham.end(), (int16_t)0);
		if (bas < a.toplam_ornek) {
			if (std::fseek(f, (long)(bas * 2LL * a.ch), SEEK_SET) == 0)
				std::fread(ham.data(), sizeof(int16_t),
				           (size_t)kFftN * (size_t)a.ch, f);
		}
		for (int n = 0; n < kFftN; ++n) {
			double mono = 0.0;
			for (int c = 0; c < a.ch; ++c)
				mono += ham[(size_t)n * a.ch + c];
			mono /= a.ch;
			ger[n] = mono * tablo.hann[n] / 32768.0;
			sanal[n] = 0.0;
		}
		fft256(ger.data(), sanal.data(), tablo);
		for (int row = 0; row < rows; ++row) {
			int const bin = row * (kFftN / 2) / rows;
			double const mag =
			    std::sqrt(ger[bin] * ger[bin] + sanal[bin] * sanal[bin]);
			// tek taraflı genlik (2/N); -80..0 dB → 0..255
			double const db = 20.0 * std::log10(2.0 * mag / kFftN + 1e-9);
			int v = (int)std::lround((db + 80.0) * (255.0 / 80.0));
			if (v < 0) v = 0;
			if (v > 255) v = 255;
			out[(size_t)col * rows + row] = (unsigned char)v;
		}
	}
	std::fclose(f);
	return 0;
}

#ifdef VELNOSUB_ABI_OBOE
std::map<int, std::shared_ptr<AudioPlayer>> g_oynatici;
std::mutex g_oynatma_kilit;

// kilit alinmisken cagrilir: akisi kapat, oynaticiyi haritadan dus
// (yikici, halen acik olan dosyayi kapatir)
static void ses_kes_kilitli(int h) {
	auto it = g_oynatici.find(h);
	if (it == g_oynatici.end()) return;
	auto pb = it->second;
	pb->kes.store(true);
	if (pb->akis) {
		pb->akis->stop();
		pb->akis->close();
		pb->akis.reset();
	}
	g_oynatici.erase(it);
}

// kurulum ipliği: dosya aç → konumlan → akış aç → başlat. Kısıtlı ortamlarda
// (proot binder) openStream süresiz bloke olabilir — çağıran iplik asla
// beklemez; kilidi yalnızca kısa kritik kesitte tutar.
static void ses_kurulum_ipligi(int h, std::shared_ptr<AudioPlayer> pbc) {
	pbc->f = std::fopen(pbc->yol.c_str(), "rb");
	if (!pbc->f) { pbc->kurulma.store(2); pbc->durum.store(2); return; }
	if (std::fseek(pbc->f, (long)(pbc->d0 * 2LL * pbc->ch_c), SEEK_SET) != 0) {
		std::fclose(pbc->f);
		pbc->f = nullptr;
		pbc->kurulma.store(2);
		pbc->durum.store(2);
		return;
	}
	pbc->kalan = pbc->adet;

	oboe::AudioStreamBuilder kurgu;
	kurgu.setDirection(oboe::Direction::Output)
	    ->setPerformanceMode(oboe::PerformanceMode::LowLatency)
	    ->setSharingMode(oboe::SharingMode::Shared)
	    ->setFormat(oboe::AudioFormat::I16)
	    ->setChannelCount(pbc->ch_c)
	    ->setSampleRate(pbc->rate)
	    ->setDataCallback(pbc.get())
	    ->setErrorCallback(pbc.get());
	std::shared_ptr<oboe::AudioStream> akis;
	oboe::Result rc = kurgu.openStream(akis);
	if (rc != oboe::Result::OK) {
		std::fclose(pbc->f);
		pbc->f = nullptr;
		pbc->kurulma.store(2);
		pbc->durum.store(2);
		return;
	}
	if (pbc->kes.load()) {
		akis->stop();
		akis->close();
		std::fclose(pbc->f);
		pbc->f = nullptr;
		return;
	}
	rc = akis->start();
	if (rc != oboe::Result::OK) {
		akis->close();
		std::fclose(pbc->f);
		pbc->f = nullptr;
		pbc->kurulma.store(2);
		pbc->durum.store(2);
		return;
	}
	{
		std::lock_guard<std::mutex> kilit(g_oynatma_kilit);
		if (pbc->kes.load()) { // stop tam buraya denk geldiyse
			akis->stop();
			akis->close();
			std::fclose(pbc->f);
			pbc->f = nullptr;
			return;
		}
		pbc->akis = akis;
		pbc->kurulma.store(0);
	}
}

int velnosub_abi_audio_play(int h, int64_t d0, int64_t adet) {
	if (!g_audio.count(h)) { set_err("tanımsız ses tutamacı"); return -1; }
	AudioWave &a = *g_audio.at(h);
	if (d0 < 0 || adet <= 0 || d0 + adet > a.toplam_ornek) {
		set_err("oynatma aralığı dışında");
		return -1;
	}
	if (a.yol.empty()) { set_err("ses yolu boş (kolu kapat-aç)"); return -1; }
	auto pbc = std::make_shared<AudioPlayer>();
	pbc->ilerleme.store(0); // yeni oynatıcı: sayaç sıfırdan başlar
	pbc->yol = a.yol;
	pbc->rate = a.rate;
	pbc->ch_c = a.ch;
	pbc->d0 = d0;
	pbc->adet = adet;
	std::lock_guard<std::mutex> kilit(g_oynatma_kilit);
	ses_kes_kilitli(h); // varsa eski akışı kes
	g_oynatici[h] = pbc;
	std::thread(ses_kurulum_ipligi, h, pbc).detach();
	return 0; // hemen döner; durum sorgusuyla izlenir
}

// durum: -1 oynatma yok, 4 hazırlanıyor, 0 oynatılıyor, 1 boşalma, 2 hata
int velnosub_abi_audio_state(int h) {
	std::lock_guard<std::mutex> kilit(g_oynatma_kilit);
	auto it = g_oynatici.find(h);
	if (it == g_oynatici.end()) return -1;
	if (it->second->kurulma.load() == -1) return 4;
	return it->second->durum.load();
}

// oynatma konumu (kare): yok/hata -1, boşalma d0+adet, aksi d0+ilerleme
int64_t velnosub_abi_audio_position(int h) {
	std::lock_guard<std::mutex> kilit(g_oynatma_kilit);
	auto it = g_oynatici.find(h);
	if (it == g_oynatici.end()) return -1;
	AudioPlayer &p = *it->second;
	if (p.kurulma.load() == 2 || p.durum.load() == 2) return -1;
	if (p.durum.load() == 1) return p.d0 + p.adet;
	return p.d0 + p.ilerleme.load(std::memory_order_relaxed);
}

int velnosub_abi_audio_stop(int h) {
	std::lock_guard<std::mutex> kilit(g_oynatma_kilit);
	ses_kes_kilitli(h);
	return 0;
}
#else
int velnosub_abi_audio_play(int, int64_t, int64_t) {
	set_err("oboe bu derlemede yok (masa kolu)");
	return -1;
}
int velnosub_abi_audio_state(int) { return 0; }
int64_t velnosub_abi_audio_position(int) { return -1; }
int velnosub_abi_audio_stop(int) { return 0; }
#endif

void velnosub_abi_audio_close(int h) {
#ifdef VELNOSUB_ABI_OBOE
	velnosub_abi_audio_stop(h);
#endif
	g_audio.erase(h);
}

void velnosub_abi_close(int h) {
	// P4.6 dilim 2: altyazı kapanınca render şartı da kapanır (sızıntı yolu)
	if (g_r.count(h)) {
		r_close(g_r.at(h));
		g_r.erase(h);
	}
	g_ctx.erase(h);
}

const char *velnosub_abi_last_error() {
	return g_last_error.c_str();
}

// P5.2 dilim C: UTF-8 kod noktası okuyucu (basit deçici; kötü bayt '?')
static unsigned long ft_utf8_oku(const unsigned char *z, int &at) {
	unsigned char c = z[at];
	if ((c & 0x80) == 0) {
		at += 1;
		return c;
	}
	int ek = 0;
	unsigned long cp = 0;
	if ((c & 0xE0) == 0xC0) {
		ek = 1;
		cp = c & 0x1F;
	} else if ((c & 0xF0) == 0xE0) {
		ek = 2;
		cp = c & 0x0F;
	} else if ((c & 0xF8) == 0xF0) {
		ek = 3;
		cp = c & 0x07;
	} else {
		at += 1;
		return '?';
	}
	for (int i = 0; i < ek; ++i) {
		unsigned char d = z[at + 1 + i];
		if ((d & 0xC0) != 0x80) {
			at += ek + 1;
			return '?';
		}
		cp = (cp << 6) | (d & 0x3F);
	}
	at += ek + 1;
	return cp ? cp : '?';
}

// P5.2 dilim C: aegisub.text_extents — upstream auto4 ölçüm kolunun
// mobil hattı. Giriş: font yolu (pkg blob), boyut (piksel, 72dpi), kalin/
// italik/harf aralığı (harf_bos), olcek_x/olcek_y (%100); çıkış out4:
// genişlik, yükseklik, descent, external leading (piksel).
// Formül upstream auto4_base.cpp CalculateTextExtents'un non-WIN32 dalıyla
// aynı yapıdadır: spacing≠0 → harf harf (kerning yok), spacing=0 → tek
// geçiş kerning'li; ölçek normallemesi fontsize/ink-yükseklik; son telafi
// scalex/scaley/100 — büyük ölçüler FreeType 26.6 sabit noktasında.
int velnosub_abi_text_extents(const char *yol_font, double boyut,
                              int kalin, int italik, double harf_bos,
                              double olcek_x, double olcek_y,
                              const char *metin, double *out4) {
	if (!out4 || !metin) {
		set_err("boş çıktı tamponu");
		return -1;
	}
	// upstream kolu girişte sıfırlar: hata bitse bile çıktı tamponu
	// sıfır kalır (arka arkaya çağrıların sayıllarını bozmaz)
	out4[0] = out4[1] = out4[2] = out4[3] = 0;
	if (!yol_font || !*yol_font) {
		set_err("boş font yolu");
		return -1;
	}
	if (boyut <= 0 || boyut > 500) {
		set_err("font boyutu aralık dışı");
		return -1;
	}
	FT_Library ft = nullptr;
	FT_Face face = nullptr;
	if (FT_Init_FreeType(&ft)) {
		set_err("freetype kurulamadı");
		return -1;
	}
	if (FT_New_Face(ft, yol_font, 0, &face)) {
		FT_Done_FreeType(ft);
		set_err(std::string("font yüzü açılamadı: ") + yol_font);
		return -1;
	}
	double size26 = boyut * 64.0;
	if (FT_Set_Char_Size(face, 0,
	                     static_cast<FT_F26Dot6>(size26 + 0.5), 72,
	                     72) != 0) {
		FT_Done_Face(face);
		FT_Done_FreeType(ft);
		set_err("font boyutu kurulamadı");
		return -1;
	}
	if (italik) {
		// sentetik italik: 0.2 eğme (libass koluyla aynı katsayı)
		FT_Matrix mat = {0x10000, 0x3333, 0, 0x10000};
		FT_Set_Transform(face, &mat, nullptr);
	}
	FT_Long yuksek26 = face->size->metrics.height;
	FT_Long asc26 = face->size->metrics.ascender;
	FT_Long desc26 = face->size->metrics.descender;
	double lead26 = static_cast<double>(yuksek26 - (asc26 - desc26));
	const double ek26 = harf_bos * 64.0;
	const bool bosluk_ekli = std::abs(ek26) > 0.5;
	const double kalin_art26 = kalin ? size26 * 0.04 : 0.0;
	double gen26 = 0.0, yuk26 = 0.0, des26 = 0.0, lea26 = 0.0;
	FT_UInt prev_gi = 0;
	int at = 0;
	const unsigned char *z = reinterpret_cast<unsigned char const*>(metin);
		// zero-ink karakter (boşluk gibi): wx kolu font metrik yüksekliği
		// verir; FreeType ink'i 0 geldiği için ascender−descender'a düş
		double b26_zemin =
			static_cast<double>(asc26 - desc26) > 0.0
				? static_cast<double>(asc26 - desc26)
				: size26;
		while (z[at]) {
			unsigned long cp = ft_utf8_oku(z, at);
			FT_UInt gi = cp ? FT_Get_Char_Index(face, cp) : 0;
			if (FT_Load_Glyph(face, gi, FT_LOAD_NO_BITMAP) != 0)
				continue;
			FT_Glyph_Metrics const& m = face->glyph->metrics;
			double a26 = static_cast<double>(face->glyph->advance.x) +
			             kalin_art26;
			if (bosluk_ekli) {
				// upstream spacing dalı: harf harf; kerning devre dışı
				double b26 = m.height > 0
				                 ? static_cast<double>(m.height)
				                 : b26_zemin;
				double scaling = size26 / b26;
				gen26 += (a26 + ek26) * scaling;
				double b_y = b26 * scaling;
				if (b26 > 0.0 && m.height > 0 && b_y > yuk26)
					yuk26 = b_y;
				FT_Long c26 = m.height > m.horiBearingY
				                  ? m.height - m.horiBearingY
				                  : 0;
				double cd =
					static_cast<double>(c26) * scaling;
				if (b26 > 0.0 && cd > des26) des26 = cd;
				double ld = lead26 * scaling;
				if (ld > lea26) lea26 = ld;
			} else {
				// upstream spacing=0 dalı: tek geçiş (kerning dahil)
				gen26 += a26;
				if (FT_HAS_KERNING(face) && prev_gi && gi) {
					FT_Vector kern;
					if (FT_Get_Kerning(face, prev_gi, gi,
					                   FT_KERNING_DEFAULT,
					                   &kern) == 0)
						gen26 += static_cast<double>(kern.x);
				}
				prev_gi = gi;
				if (m.height > 0 &&
				    static_cast<double>(m.height) > yuk26)
					yuk26 = static_cast<double>(m.height);
				FT_Long c26 = m.height > m.horiBearingY
				                  ? m.height - m.horiBearingY
				                  : 0;
				double cd = static_cast<double>(c26);
				if (cd > des26) des26 = cd;
				if (lead26 > lea26) lea26 = lead26;
			}
		}
	// scalex/scaley (%) telafisi + 26.6 → piksel bölmesi (upstream /64)
	out4[0] = olcek_x / 100.0 * gen26 / 64.0;
	out4[1] = olcek_y / 100.0 * yuk26 / 64.0;
	out4[2] = olcek_y / 100.0 * des26 / 64.0;
	out4[3] = olcek_y / 100.0 * lea26 / 64.0;
	FT_Done_Face(face);
	FT_Done_FreeType(ft);
	return 0;
}

} // extern "C"
