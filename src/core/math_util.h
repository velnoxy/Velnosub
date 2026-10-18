// Velsub Faz 1 — wx'siz küçük yardımcılar
//
// utils.h wx tiplerini kullandığı için çekirdeğe konamıyor. Saf yardımcılar
// (mid/tabs/countof/float_to_string) bu başlıkta INLINE olarak tanımlıdır.
// utils.h bunları buradan içerir; hem masaüstü hedefi hem aegisub-core aynı
// tek tanımı kullanır (inline -> ODR güvenli).
//
// Tanımlar upstream src/utils.cpp (float_to_string) ve src/utils.h
// (mid/tabs/countof) dosyasından aynen taşınmıştır; davranış değişmedidir.

#pragma once

#include <string>

/// @brief Templated abs() function
template <typename T> T tabs(T x) { return x < 0 ? -x : x; }

/// Get the middle value of a, b, and c (i.e. clamp b to [a,c])
/// @precondition a <= c
template<typename T> inline T mid(T a, T b, T c) {
	return a > b ? a : (b > c ? c : b);
}

#define countof(array) (sizeof(array) / sizeof(array[0]))

/// Sayısal değeri sabit hassasiyetli metne çevirir; sondaki sıfırları budar.
/// Tanım: masaüstünde utils.cpp, aegisub-core'da core/math_util.cpp.
std::string float_to_string(double val, int precision = 3);
