// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 5 (P5.0) — LuaJIT 2.1 statik katılım + aegisub.*-stub dumanı.
// P5.1/P5.2: VELNOSUB_AEGISHIM ortam değişkeniyle gerçek aegisub.* yüzeyi
// (FFI→libvelnosub-abi.so) yüklenir; script dofile edildikten sonra
// aegisub.__pump pompası (--ass/--macro/--val/--btn/--save arg'ları) koşar.
//
// Kullanım: velnosub-automation [script.lua] [arg...]
//   script verilirse dofile edilir; verilmezse gömülü duman scripti koşar.
//   arg... değerleri aegisub.arg tablosunun (1..n) içine konur.
// Marker şeridi: [AU0] LuaJIT kuruldu, [AU1] aegisub tablo kayıtlı,
// [AUS0] aegisub yüzeyi yüklendi (shim), [AUP0] pompacı koştu,
// [AU2] script koştu, [AU3] tamam, [AUE] hata.

#include <lua.hpp>
#include <lualib.h>
#include <lauxlib.h>
#include <luajit.h>

#include <cstring>
#include <cstdio>
#include <cstdlib>
#include <iostream>
#include <string>
#include <vector>
#include <algorithm>
#include <dirent.h>

namespace {

int l_log(lua_State *L) {
	std::size_t n = 0;
	const char *s = luaL_checklstring(L, 1, &n);
	std::cout << "[AUL] " << std::string(s, n) << "\n";
	return 0;
}

// P5.3: BOM'suz dosya yükleme — upstream autoload scriptleri çoğu BOM'lu
// (EF BB BF) başlar; ham luaL_dofile "unexpected symbol" verir.
int yukle(lua_State *L, const char *yol) {
	FILE *f = fopen(yol, "rb");
	if (!f) {
		std::cerr << "[AUE] dosya yok: " << yol << "\n";
		return 1;
	}
	std::string metin;
	char tampon[4096];
	std::size_t n;
	while ((n = fread(tampon, 1, sizeof(tampon), f)) > 0)
		metin.append(tampon, n);
	fclose(f);
	if (metin.size() >= 3 && (unsigned char)metin[0] == 0xEF &&
	    (unsigned char)metin[1] == 0xBB && (unsigned char)metin[2] == 0xBF) {
		metin.erase(0, 3);
	}
	return luaL_dostring(L, metin.c_str());
}

int l_gettext(lua_State *L) {
	std::size_t n = 0;
	const char *s = luaL_checklstring(L, 1, &n);
	lua_pushlstring(L, s, n);
	return 1;
}

const luaL_Reg aegisub_fns[] = {
	{ "log", l_log },
	{ "gettext", l_gettext },
	{ nullptr, nullptr },
};

// Gömülü duman scripti: stub yüzeyi + JIT altında döngü + tablo işlemleri.
const char *g_builtin_script = R"LUA(
aegisub.log("P5.0 zemin dumanı başladı")
assert(aegisub.__stub == true, "stub eksik")
local n = aegisub.arg and #aegisub.arg or 0
local toplam = 0
for i = 1, 1000000 do
    toplam = toplam + (i * 3 - 2)
end
aegisub.log(("arg: %d, toplam: %d"):format(n, toplam))
)LUA";

} // namespace

int main(int argc, char **argv) {
	lua_State *L = luaL_newstate();
	if (!L) {
		std::cerr << "[AUE] lua_State oluşturulamadı\n";
		return 1;
	}
	luaL_openlibs(L);
	std::cout << "[AU0] LuaJIT kuruldu: " << LUAJIT_VERSION << "\n";

	// aegisub stub tablosu: log/gettext + __stub + arg[]
	lua_newtable(L);
	luaL_setfuncs(L, aegisub_fns, 0);
	lua_pushboolean(L, 1);
	lua_setfield(L, -2, "__stub");
	lua_newtable(L);
	for (int i = 2; i < argc; ++i) {
		lua_pushstring(L, argv[i]);
		lua_rawseti(L, -2, i - 1);
	}
	lua_setfield(L, -2, "arg");
	lua_setglobal(L, "aegisub");
	std::cout << "[AU1] aegisub tablo kayıtlı\n";

	// P5.1/P5.2: gerçek aegisub.* yüzeyi (FFI → libvelnosub-abi.so)
	const char *shim = std::getenv("VELNOSUB_AEGISHIM");
	if (shim && *shim) {
		if (yukle(L, shim)) {
			const char *err = lua_tostring(L, -1);
			std::cerr << "[AUE] shim hatası: " << (err ? err : "(mesaj yok)")
			          << "\n";
			lua_close(L);
			return 1;
		}
		std::cout << "[AUS0] aegisub yüzeyi yüklendi\n";
	}

	// P5.3: manager taraması — --include / --dir / --script arg'ları;
	// dir'de *.lua sıralı yüklenir (upstream BOM'ları dahil)
	std::vector<std::string> dizinler, scriptler, incs;
	for (int i = 2; i < argc; ++i) {
		std::string a = argv[i];
		if (a == "--dir" && i + 1 < argc) { dizinler.push_back(argv[++i]); }
		else if (a == "--script" && i + 1 < argc) { scriptler.push_back(argv[++i]); }
		else if (a == "--include" && i + 1 < argc) { incs.push_back(argv[++i]); }
	}
	for (auto const& inc : incs) {
		std::cout << "[AUZ2] include yükleniyor: " << inc << "\n";
		if (yukle(L, inc.c_str())) {
			const char *err = lua_tostring(L, -1);
			std::cerr << "[AUE] include hatası (" << inc << "): "
			          << (err ? err : "(mesaj yok)") << "\n";
			lua_close(L);
			return 1;
		}
	}
	for (auto const& d : dizinler) {
		DIR *dt = opendir(d.c_str());
		if (!dt) {
			std::cerr << "[AUE] dizin yok: " << d << "\n";
			lua_close(L);
			return 1;
		}
		std::vector<std::string> dosyalar;
		struct dirent *girdi;
		while ((girdi = readdir(dt)) != nullptr) {
			std::string ad = girdi->d_name;
			if (ad.size() > 4 && ad.compare(ad.size() - 4, 4, ".lua") == 0)
				dosyalar.push_back(ad);
		}
		closedir(dt);
		std::sort(dosyalar.begin(), dosyalar.end());
		std::cout << "[AUZ0] tarama: " << d << " (" << dosyalar.size()
		          << " script)\n";
		for (auto const& ad : dosyalar) {
			if (yukle(L, (d + "/" + ad).c_str())) {
				const char *err = lua_tostring(L, -1);
				std::cerr << "[AUE] script-dir hatası (" << ad << "): "
				          << (err ? err : "(mesaj yok)") << "\n";
				lua_close(L);
				return 1;
			}
			std::cout << "[AUZ1] yüklendi: " << ad << "\n";
		}
	}
	for (auto const& s : scriptler) {
		std::cout << "[AUZ1] script yükleniyor: " << s << "\n";
		if (yukle(L, s.c_str())) {
			const char *err = lua_tostring(L, -1);
			std::cerr << "[AUE] script hatası (" << s << "): "
			          << (err ? err : "(mesaj yok)") << "\n";
			lua_close(L);
			return 1;
		}
	}

	int rc;
	if (argc >= 2) {
		rc = yukle(L, argv[1]);
	} else {
		rc = luaL_dostring(L, g_builtin_script);
	}
	if (rc) {
		const char *err = lua_tostring(L, -1);
		std::cerr << "[AUE] script hatası: " << (err ? err : "(mesaj yok)") << "\n";
		lua_close(L);
		return 1;
	}

	// P5.2: pump — shim varsa kayıtlı makroyu dosyada koşar + kaydeder
	lua_getglobal(L, "aegisub");
	if (lua_istable(L, -1)) {
		lua_getfield(L, -1, "__pump");
		if (lua_isfunction(L, -1)) {
			if (lua_pcall(L, 0, 0, 0)) {
				const char *err = lua_tostring(L, -1);
				std::cerr << "[AUE] pump hatası: "
				          << (err ? err : "(mesaj yok)") << "\n";
				lua_close(L);
				return 1;
			}
			std::cout << "[AUP0] pompacı koştu\n";
		}
	}
	lua_settop(L, 0);

	std::cout << "[AU2] script koştu\n";
	std::cout << "[AU3] tamam (LuaJIT zemin dumanı)\n";
	lua_close(L);
	return 0;
}
