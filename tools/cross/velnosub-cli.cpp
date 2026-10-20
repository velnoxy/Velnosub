// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 2 (P2.2) — mobil CLI dumanı: ASS aç → seçimli düzenleme → kaydet.
// velnosub-core.so'ya bağlanır (Android'de libc++_shared ile koşar); masaüstünde
// cross zincir olmadan hızlı duman için de kullanılabilir.
//
// Kullanım: velnosub-cli <giriş> <çıkış> [--edit-first "yeni metin"]
// Çıkış uzantısı yazıcı formatını belirler (GetWriter kuralı).
// Yapılandırma: VELNOSUB_CONFIG_JSON ortam değişkeni veya çalışma dizinindeki
// default_config.json (cihaza birlikte itilir).

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

// P2.2: config::opt / config::path tek tanım noktası core .so'dadır
// (src/core/options_globals.cpp); CLI ikilisi .so üzerinden çözer,
// kendi global kopyasını taşımaz (P1.6 statik-link yapısı değişti).

#include <cstdlib>
#include <fcntl.h>
#include <unistd.h>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>

// P5.3: flyweight Text mutasyon vekili (src/core/velnosub_core_edit.cpp) —
// cli ikilisi ayrı DSO olduğundan doğrudan Text ataması çapraz-fabrika
// bozulması üretiyor (abi TU'sundaki baş nota bak)
extern "C" int velnosub_core_edit_first_text(AssFile &file, const char *text);

namespace {
std::string read_all(std::string const& p) {
	std::ifstream in(p, std::ios::binary);
	return std::string(std::istreambuf_iterator<char>{in},
	                   std::istreambuf_iterator<char>{});
}
}

int main(int argc, char **argv) {
	if (argc < 3) {
		std::cerr << "kullanım: velnosub-cli <giriş> <çıkış> [--edit-first \"metin\"]\n";
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
		// P2.2: GUI'de main.cpp MainLoop'a rassal bağlanır; CLI'da invoke_main
		// çağrının kendisini koşturur (aynı iş parçacığı; LogSink Queue kullanır).
		agi::dispatch::Init([](agi::dispatch::Thunk f) { f(); });
		// P2.2: log sink singleton'ı GUI'de main.cpp kurar; CLI'ın kendi
		// kurulumu yoksa log yazımı null'a düşer (Options ctor log ile başlar).
		agi::log::log = new agi::log::LogSink;
		std::cerr << "[0] config okuma\n";
		// 1) seçenekler: gerçek default_config.json (masaüstüyle birebir)
		char const *cfg = std::getenv("VELNOSUB_CONFIG_JSON");
		std::string const cfg_text = read_all(cfg ? cfg : "default_config.json");
		std::cerr << "[0b] config okundu: " << cfg_text.size()
		          << " bayt\n";
		config::opt = new agi::Options(
			"", {cfg_text.data(), cfg_text.size()}, agi::Options::FLUSH_SKIP);
		std::cerr << "[1] config hazır\n";

		// 2) aç
		agi::fs::path const in(in_path);
		const SubtitleFormat *reader = SubtitleFormat::GetReader(in, "utf-8");
		if (!reader) {
			std::cerr << "okuyucu bulunamadı: " << in_path << "\n";
			return 1;
		}
		std::cerr << "[2] okuyucu hazır\n";
		AssFile file;
		reader->ReadFile(&file, in, agi::vfr::Framerate(), "utf-8");
		std::cerr << "[3] okundu: " << file.Events.size() << " satır, "
		          << std::distance(file.Styles.begin(), file.Styles.end())
		          << " stil\n";

		// 3) seçimli düzenleme (çekirdek Commit overload'u)
		if (edit_first && !file.Events.empty()) {
			// P5.3: flyweight yazımı core.so vekilinden (Commit dahil)
			velnosub_core_edit_first_text(file, edit_text.c_str());
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
		std::cerr << "hata: " << e.GetMessage() << "\n";
		return 1;
	}
	catch (std::exception const& e) {
		std::cerr << "hata: " << e.what() << "\n";
		return 1;
	}
}
