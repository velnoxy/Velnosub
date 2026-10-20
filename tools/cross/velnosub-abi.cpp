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
#include "ass_style.h"
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

#ifdef VELNOSUB_ABI_OBOE
#include <oboe/Oboe.h>
#endif

#include <atomic>
#include <memory>
#include <mutex>
#include <thread>
#include <vector>

#include <cstring>
#include <cstdio>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <iterator>
#include <map>
#include <memory>
#include <new>
#include <string>

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
		c.file.Events.front().Text = text;
		c.file.Commit("velnosub abi", AssFile::COMMIT_DIAG_TEXT);
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

int velnosub_abi_edit_line(int h, int idx, const char *text) {
	if (!g_ctx.count(h)) { set_err("tanımsız tutamaç"); return -1; }
	if (!text) { set_err("boş metin"); return -1; }
	try {
		Ctx &c = *g_ctx.at(h);
		if (idx < 0 || idx >= static_cast<int>(c.file.Events.size())) {
			set_err("satır dışında indeks: " + std::to_string(idx));
			return -1;
		}
		std::next(c.file.Events.begin(), idx)->Text = text;
		c.file.Commit("velnosub abi", AssFile::COMMIT_DIAG_TEXT);
		return 0;
	}
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
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

int velnosub_abi_audio_open(const char *path, int rate, int ch) {
	if (!g_ready) { set_err("init çağrılmadı"); return -1; }
	if (!path || !*path) { set_err("ses yolu boş"); return -1; }
	if (rate <= 0 || ch <= 0 || ch > 8) { set_err("geçersiz rate/kanal"); return -1; }
	try {
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
	catch (agi::Exception const& e) { set_err(e.GetMessage()); }
	catch (std::exception const& e) { set_err(e.what()); }
	return -1;
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

#ifdef VELNOSUB_ABI_OBOE
std::map<int, std::shared_ptr<AudioPlayer>> g_oynatici;
std::mutex g_oynatma_kilit;

// kilit ALINMIŞken çağrılır: akışı kapat, oynatıcıyı haritadan düş
// (yıkıcı, hâlâ açık olan dosyayı kapatır)
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
int velnosub_abi_audio_stop(int) { return 0; }
#endif

void velnosub_abi_audio_close(int h) {
#ifdef VELNOSUB_ABI_OBOE
	velnosub_abi_audio_stop(h);
#endif
	g_audio.erase(h);
}

void velnosub_abi_close(int h) {
	g_ctx.erase(h);
}

const char *velnosub_abi_last_error() {
	return g_last_error.c_str();
}

} // extern "C"
