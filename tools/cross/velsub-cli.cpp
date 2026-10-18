// Aegisub Project http://www.aegisub.org/
//
// Velsub Faz 2 (P2.2) — mobil CLI dumanı: ASS aç → seçimli düzenleme → kaydet.
// velsub-core.so'ya bağlanır (Android'de libc++_shared ile koşar); masaüstünde
// cross zincir olmadan hızlı duman için de kullanılabilir.
//
// Kullanım: velsub-cli <giriş> <çıkış> [--edit-first "yeni metin"]
// Çıkış uzantısı yazıcı formatını belirler (GetWriter kuralı).
// Yapılandırma: VELSUB_CONFIG_JSON ortam değişkeni veya çalışma dizinindeki
// default_config.json (cihaza birlikte itilir).

#include "ass_dialogue.h"
#include "ass_file.h"
#include "ass_style.h"
#include "options.h"
#include "subtitle_format.h"

#include <libaegisub/exception.h>
#include <libaegisub/fs.h>
#include <libaegisub/option.h>
#include <libaegisub/vfr.h>

#include <cstdlib>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>

// P1.6 ile aynı ABI: AssStyleStorage config::path'i, AssFile::LoadDefault
// zinciri config::opt'i (OPT_GET) çözer. CLI yalnızca opt'i kurar.
namespace config {
	agi::Options *opt = nullptr;
	agi::Path *path = nullptr;
}

namespace {
std::string read_all(std::string const& p) {
	std::ifstream in(p, std::ios::binary);
	return std::string(std::istreambuf_iterator<char>{in},
	                   std::istreambuf_iterator<char>{});
}
}

int main(int argc, char **argv) {
	if (argc < 3) {
		std::cerr << "kullanım: velsub-cli <giriş> <çıkış> [--edit-first \"metin\"]\n";
		return 2;
	}
	std::string const in_path = argv[1];
	std::string const out_path = argv[2];
	std::string edit_text;
	bool edit_first = false;
	for (int i = 3; i + 1 < argc; ++i) {
		if (std::string(argv[i]) == "--edit-first") {
			edit_first = true;
			edit_text = argv[i + 1];
			++i;
		}
	}

	try {
		// 1) seçenekler: gerçek default_config.json (masaüstüyle birebir)
		char const *cfg = std::getenv("VELSUB_CONFIG_JSON");
		std::string const cfg_text = read_all(cfg ? cfg : "default_config.json");
		config::opt = new agi::Options(
			"", {cfg_text.data(), cfg_text.size()}, agi::Options::FLUSH_SKIP);

		// 2) aç
		agi::fs::path const in(in_path);
		const SubtitleFormat *reader = SubtitleFormat::GetReader(in, "utf-8");
		if (!reader) {
			std::cerr << "okuyucu bulunamadı: " << in_path << "\n";
			return 1;
		}
		AssFile file;
		reader->ReadFile(&file, in, agi::vfr::Framerate(), "utf-8");
		std::cout << "okundu: " << file.Events.size() << " satır, "
		          << std::distance(file.Styles.begin(), file.Styles.end())
		          << " stil\n";

		// 3) seçimli düzenleme (çekirdek Commit overload'u)
		if (edit_first && !file.Events.empty()) {
			file.Events.front().Text = edit_text;
			file.Commit("velsub cli", AssFile::COMMIT_DIAG_TEXT);
			std::cout << "düzenlendi: " << edit_text << "\n";
		}

		// 4) kaydet
		agi::fs::path const out(out_path);
		const SubtitleFormat *writer = SubtitleFormat::GetWriter(out);
		if (!writer) {
			std::cerr << "yazıcı bulunamadı: " << out_path << "\n";
			return 1;
		}
		writer->WriteFile(&file, out, agi::vfr::Framerate(), "utf-8");

		// 5) tur doğrulama: kaydedileni yeniden oku
		AssFile reloaded;
		reader->ReadFile(&reloaded, out, agi::vfr::Framerate(), "utf-8");
		std::cout << "tur tamam: " << reloaded.Events.size() << " satır\n";
		if (!reloaded.Events.empty())
			std::cout << "ilk satır: " << reloaded.Events.front().Text << "\n";
		return 0;
	}
	catch (agi::Exception const& e) {
		std::cerr << "hata: " << e.what() << "\n";
		return 1;
	}
	catch (std::exception const& e) {
		std::cerr << "hata: " << e.what() << "\n";
		return 1;
	}
}
