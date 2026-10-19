// Aegisub Project http://www.aegisub.org/
//
// velnosub Faz 5 (P5.0) — LuaJIT 2.1 statik katılım + aegisub.*-stub dumanı.
// Çekirdek bağlaması P5.1 dilimidir; burada amaç LuaJIT çalışma zamanının
// mobil zincirde bütünlüğü ve stub API yüzeyinin koşuma hazır olmasıdır.
//
// Kullanım: velnosub-automation [script.lua] [arg...]
//   script verilirse dofile edilir; verilmezse gömülü duman scripti koşar.
//   arg... değerleri aegisub.arg tablosunun (1..n) içine konur.
// Marker şeridi: [AU0] LuaJIT kuruldu, [AU1] aegisub tablo kayıtlı,
// [AU2] script koştu, [AU3] tamam, [AUE] hata.

#include <lualib.h>
#include <lauxlib.h>

#include <cstring>
#include <iostream>
#include <string>

namespace {

int l_log(lua_State *L) {
	std::size_t n = 0;
	const char *s = luaL_checklstring(L, 1, &n);
	std::cout << "[AUL] " << std::string(s, n) << "\n";
	return 0;
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

	int rc;
	if (argc >= 2) {
		rc = luaL_dofile(L, argv[1]);
	} else {
		rc = luaL_dostring(L, g_builtin_script);
	}
	if (rc) {
		const char *err = lua_tostring(L, -1);
		std::cerr << "[AUE] script hatası: " << (err ? err : "(mesaj yok)") << "\n";
		lua_close(L);
		return 1;
	}
	std::cout << "[AU2] script koştu\n";
	std::cout << "[AU3] tamam (LuaJIT zemin dumanı)\n";
	lua_close(L);
	return 0;
}
