// Velsub Faz 1 (P1.5) — font ölçüm ABI'sinin headless çekirdek varsayımı.

#include "core/compat/font_abi.h"

namespace agi::compat {

namespace {
	FontMeasurer font_measurer = nullptr;
}

bool measure_text(std::string const& font_family_utf8, double fontsize,
                  bool bold, bool italic, bool underline, bool strikeout,
                  int encoding,
                  double spacing,
                  std::string const& text_utf8,
                  double& width, double& height,
                  double& descent, double& extlead) {
	width = height = descent = extlead = 0;
	if (font_measurer)
		return font_measurer(font_family_utf8, fontsize, bold, italic, underline,
			strikeout, encoding, spacing, text_utf8,
			width, height, descent, extlead);
	(void)fontsize; (void)bold; (void)italic; (void)underline; (void)strikeout;
	(void)encoding; (void)spacing; (void)text_utf8;
	return false;
}

void set_font_measurer(FontMeasurer measurer) noexcept {
	font_measurer = measurer;
}

}
