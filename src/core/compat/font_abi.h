// velnosub Faz 1 (P1.5) — metin ölçüm ABI'si (Caz§1.2 'font::Measure').
//
// auto4_base.cpp içindeki aegisub.text_extents ölçümü masaüstünde wx
// (=wxMemoryDC/wxFont; Windows'ta GDI) ile sürdüğü YERDE kalır. Faz 1'de
// çekirdekte gerçek ölçücü yoktur (headless: false döner, metrikler 0):
// gerçek ölçücü Faz 2'de FreeType+HarfBuzz ile host'tan gelir; masaüstü
// çekirdeğe bağlanırken (P1.6+) set_font_measurer ile wx impl kayıtlanır.
#pragma once

#include <string>

namespace agi::compat {

/// Ölçücü imza türü: ölçmez; dönüş false (headless).
using FontMeasurer = bool (*)(std::string const& font_family_utf8, double fontsize,
                              bool bold, bool italic, bool underline, bool strikeout,
                              int encoding,
                              double spacing,
                              std::string const& text_utf8,
                              double& width, double& height,
                              double& descent, double& extlead);

/// Aegisub.text_extents karşılığı. Dönüş: ölçümlerin gerçekten hesaplandığından.
/// False dönince sonuçların anlamı yoktur (headless).
bool measure_text(std::string const& font_family_utf8, double fontsize,
                  bool bold, bool italic, bool underline, bool strikeout,
                  int encoding,
                  double spacing,
                  std::string const& text_utf8,
                  double& width, double& height,
                  double& descent, double& extlead);

/// Host'un ölçücüsünü kaydeder; nullptr → headless varsayıma döner.
void set_font_measurer(FontMeasurer measurer) noexcept;

}
