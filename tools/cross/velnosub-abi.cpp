// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 4 (P4.1) — C ABI şeridi: Rust/egui (velnosub-ui) tarafının
// bağlanacağı extern "C" yüzey. Akış, koşuma geçmiş P2.2 CLI kanalının
// fonksiyonlaştırılmış kopyasıdır: config hazırlığı tek kereliğine,
// open/edit/save/reload işlevseldir. libvelnosub-abi.so libvelnosub-core.so'ya
// bağlanır (Android'de libc++_shared zinciri CLI ile aynıdır).
//
// Rust tarafı (P4.1): velnosub-ui/abi-smoke crate'i bu yüzeyi çağırır.

#include "ass_dialogue.h"
#include "ass_file.h"
#include "ass_style.h"
#include "options.h"
#include "subtitle_format.h"

#include <libaegisub/dispatch.h>
#include <libaegisub/exception.h>
#include <libaegisub/fs.h>
#include <libaegisub/log.h>
#include <libaegisub/option.h>
#include <libaegisub/vfr.h>

#include <cstring>
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

void velnosub_abi_close(int h) {
	g_ctx.erase(h);
}

const char *velnosub_abi_last_error() {
	return g_last_error.c_str();
}

} // extern "C"
