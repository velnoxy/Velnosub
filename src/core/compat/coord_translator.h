// velnosub Faz 1 (P1.4) — spline'ın UI koordinat çevirici bağı için ABI.
//
// Spline (spline.h) VideoDisplay koordinat dönüşümlerini yalnızca iki işlem
// için kullanır (ToScriptCoords / FromScriptCoords — spline.cpp:51,58).
// Masaüstü tarafında VisualToolBase bu arayüzü uygular; aegisub-core'da
// (P1.6 CLI) üretilecek headless çevirici aynı ABI'yi verir.
#pragma once

#include "vector2d.h"

namespace agi::compat {

class CoordTranslator {
public:
	virtual ~CoordTranslator() = default;

	/// Convert a point from video to script coordinates
	virtual Vector2D ToScriptCoords(Vector2D point) const = 0;
	/// Convert a point from script to video coordinates
	virtual Vector2D FromScriptCoords(Vector2D point) const = 0;
};

}
