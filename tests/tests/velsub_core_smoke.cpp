// Aegisub Project http://www.aegisub.org/
//
// Velsub Faz 1 (P1.6) — aegisub-core CLI duman testi:
// ASS aç → satır düzenle → kaydet → referansla karşılaştır.
//
// Test ikilisi yalnız libaegisub + aegisub-core bağlar (-DAGI_CORE); GUI/wx
// bağımsızdır. Seçenekler, depolanan gerçek default_config.json ile kurulur
// (kaydın "App/Save UI State" dalı masaüstüyle birebir aynı davranır).

#include <gtest/gtest.h>

#include "ass_dialogue.h"
#include "ass_file.h"
#include "ass_style.h"
#include "options.h"
#include "subtitle_format.h"

#include <libaegisub/fs.h>
#include <libaegisub/option.h>
#include <libaegisub/vfr.h>

#include <cstdlib>
#include <fstream>
#include <iterator>
#include <string>

// Masaüstünde main.cpp tanımlar; duman testinde burada sağlanır.
// AssStyleStorage config::path'i (stil kataloğu yolu), AssFile::LoadDefault
// zinciri config::opt'i (OPT_GET) çözer.
namespace config {
	agi::Options *opt = nullptr;
	agi::Path *path = nullptr;
}

namespace {
std::string const& default_config() {
	static std::string const data = [] {
		std::ifstream in("../src/libresrc/default_config.json");
		return std::string(std::istreambuf_iterator<char>{in},
		                   std::istreambuf_iterator<char>{});
	}();
	return data;
}

agi::fs::path temp_path(std::string const& name) {
	const char *env = std::getenv("TMPDIR");
	return agi::fs::path(env ? env : "/tmp") / name;
}
}

class CoreSmoke : public ::testing::Test {
protected:
	void SetUp() override {
		if (!config::opt)
			config::opt = new agi::Options(
				"", {default_config().data(), default_config().size()},
				agi::Options::FLUSH_SKIP);
	}
};

TEST_F(CoreSmoke, LoadEditSaveRoundTrip) {
	auto const in_path = temp_path("velsub-smoke-in.ass");
	auto const out_path = temp_path("velsub-smoke-out.ass");

	{
		std::ofstream in(in_path.c_str());
		in << "[Script Info]\n"
		      "Title: Velsub Smoke\n"
		      "ScriptType: v4.00+\n"
		      "PlayResX: 1920\n"
		      "PlayResY: 1080\n"
		      "\n"
		      "[V4+ Styles]\n"
		      "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n"
		      "Style: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,10,1\n"
		      "\n"
		      "[Events]\n"
		      "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n"
		      "Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,original first line\n"
		      "Dialogue: 0,0:00:02.00,0:00:03.00,Default,,0,0,0,,second line\n";
	}

	// 1) ASS aç (kayıtçının çekirdek dalı üzerinden)
	const SubtitleFormat *reader = SubtitleFormat::GetReader(in_path, "utf-8");
	ASSERT_NE(reader, nullptr);
	AssFile file;
	ASSERT_NO_THROW(reader->ReadFile(&file, in_path, agi::vfr::Framerate(), "utf-8"));
	ASSERT_EQ(file.Events.size(), 2u);
	EXPECT_EQ(file.Events.front().Text, "original first line");
	EXPECT_EQ(file.Events.back().Text, "second line");
	EXPECT_EQ(std::distance(file.Styles.begin(), file.Styles.end()), 1);

	// 2) satır düzenle (çekirdek Commit overload'uyla)
	file.Events.front().Text = "edited by velsub";
	file.Commit("velsub smoke", AssFile::COMMIT_DIAG_TEXT);

	// 3) kaydet
	const SubtitleFormat *writer = SubtitleFormat::GetWriter(out_path);
	ASSERT_NE(writer, nullptr);
	ASSERT_NO_THROW(writer->WriteFile(&file, out_path, agi::vfr::Framerate(), "utf-8"));

	// 4) referansla karşılaştır
	{
		std::ifstream out(out_path.c_str());
		std::string const content(std::istreambuf_iterator<char>{out},
		                          std::istreambuf_iterator<char>{});
		EXPECT_NE(content.find("edited by velsub"), std::string::npos);
		EXPECT_NE(content.find(
			"Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,edited by velsub\n"),
			std::string::npos);
	}

	// 5) kaydedileni yenleyip tur başına eşle
	AssFile reloaded;
	const SubtitleFormat *reader2 = SubtitleFormat::GetReader(out_path, "utf-8");
	ASSERT_NE(reader2, nullptr);
	reader2->ReadFile(&reloaded, out_path, agi::vfr::Framerate(), "utf-8");
	EXPECT_EQ(reloaded.Events.size(), 2u);
	EXPECT_EQ(reloaded.Events.front().Text, "edited by velsub");
	EXPECT_EQ(reloaded.Events.back().Text, "second line");
	EXPECT_EQ(reloaded.Events.front().Start.GetAssFormatted(), "0:00:01.00");
	EXPECT_EQ(reloaded.Events.back().Start.GetAssFormatted(), "0:00:02.00");
}
