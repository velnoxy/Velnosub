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

#include <cstring>
#include <cstdio>
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

void velnosub_abi_close(int h) {
	g_ctx.erase(h);
}

const char *velnosub_abi_last_error() {
	return g_last_error.c_str();
}

} // extern "C"
