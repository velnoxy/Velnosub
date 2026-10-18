// Velsub Faz 1 — aegisub-core tarafındaki float_to_string tanımı.
// (Masaüstü tanımı src/utils.cpp içinde kalır; iki hedef kendi kopyasını
// derler, çakışmaz. P1.6'da masaüstü çekirdeğe bağlanınca tekilleşir.)

#include <libaegisub/format.h>

#include "core/math_util.h"

std::string float_to_string(double val, int precision) {
	std::string fmt = "%." + std::to_string(precision) + "f";
	// c_str(): masaüstünde agi::format(std::string) çağrısı src/format.h'teki
	// wxString küvü üzerinden çözülür; çekirdekte wx olmadığından const Char*
	// yüklemesine bağlanır (aynı formatter, aynı çıktı).
	std::string s = agi::format(fmt.c_str(), val);
	size_t pos = s.find_last_not_of("0");
	if (pos != s.find(".")) ++pos;
	s.erase(begin(s) + pos, end(s));
	return s;
}
