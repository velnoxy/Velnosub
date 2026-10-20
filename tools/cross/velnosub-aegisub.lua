-- velnosub Faz 5 (P5.1/P5.2) — aegisub.* gerçek yüzeyi: LuaJIT FFI ile
-- libvelnosub-abi.so üzerinden (UI ile AYNI kol; dialog ABI P4.5 III).
-- Veri yolu: dosya ABI'da açılır, satır okunur/yazılır, makro koşar,
-- kayıt edilir, ikinci tutamacta kalıcılık teyit edilir.
-- Koşullar: LD_LIBRARY_PATH abi .so'yu çözümlemeli; VELNOSUB_CONFIG_JSON
-- (varsayılan default_config.json); koşumlarda çıktı "> dosya".
-- İmaretler (print ile, C++ [AUL] log'undan ayrık): [AUS1] dosya açıldı,
-- [AUS2] makro koştu, [AUS3] yazıldı, [AUS4] kalıcılık, [AUS5] pump tamam.

local ffi = require("ffi")

ffi.cdef[[
int velnosub_abi_init(const char *config_json_path);
int velnosub_abi_open(const char *in_path);
int velnosub_abi_event_count(int h);
int velnosub_abi_style_count(int h);
int velnosub_abi_line_text(int h, int idx, char *buf, int buflen);
int velnosub_abi_line_times(int h, int idx, char *buf, int buflen);
int velnosub_abi_line_meta(int h, int idx, char *buf, int buflen);
int velnosub_abi_line_insert(int h, int idx, const char *metin, const char *bas, const char *son, const char *stil);
int velnosub_abi_line_delete(int h, int idx);
int velnosub_abi_edit_line(int h, int idx, const char *text);
int velnosub_abi_save(int h, const char *out_path);
int velnosub_abi_close(int h);
const char *velnosub_abi_last_error();
int velnosub_abi_dialog_open(const char *spec);
int velnosub_abi_dialog_count(int h);
int velnosub_abi_dialog_btncount(int h);
int velnosub_abi_dialog_ctl(int h, int idx, char *buf, int buflen);
int velnosub_abi_dialog_button(int h, int idx, char *buf, int buflen);
int velnosub_abi_dialog_set(int h, int idx, const char *deger);
int velnosub_abi_dialog_show(int h, int dugme);
int velnosub_abi_dialog_showid(int h);
int velnosub_abi_dialog_close(int h);
]]

local abi = ffi.load("velnosub-abi")

local function hatasi(nerede)
	local e = ffi.string(abi.velnosub_abi_last_error())
	return nerede .. ": " .. (e == "" and "bilinmeyen hata" or e)
end

-- satır dizgesi okuma kuralı: rc<0 hata; rc<n-1 tamam; değilse tampon büyüt
local function oku(fn, h, idx)
	local n = 512
	while true do
		local b = ffi.new("char[?]", n)
		local rc = fn(h, idx, b, n)
		if rc < 0 then
			return nil, hatasi("oku")
		end
		if rc < n - 1 then
			return ffi.string(b, rc)
		end
		n = n * 2
	end
end

local hata_ayikla do
	local haneler = {}
	function hata_ayikla(s)
		haneler = {}
		for h in (s .. "|"):gmatch("(.-)|") do
			haneler[#haneler + 1] = h
		end
		return haneler
	end
end

-- LuaJIT (5.1) math.tointeger'sız: tam sayı kontrolü elle
local function tam_sayi(v)
	local n = tonumber(v)
	if not n or n % 1 ~= 0 or n ~= n then
		return nil
	end
	return n
end

-- ---- satır tablosu: text + start_time/end_time ----------------------
-- P5.2 dilim: zamanlar artık upstream auto4 biçiminde MS TAMSAYISI'dır
-- (eski dizge P5.1 sınırıydı): okuma ASS dizgesinden ayrışır, yazımsı
-- line_times_set ABI'sine ms olarak gider.
-- ASS zaman dizgesi → ms ("0:00:01.23", negatif "-0:00:01.23")
local function zaman_ms(z)
	local neg, h, m, s, cc =
		tostring(z):match("^%s*(-?)(%d+):(%d+):(%d+)%.(%d+)%s*$")
	if not h then return nil end
	local ms = (tonumber(h) * 3600 + tonumber(m) * 60 + tonumber(s)) * 1000
		+ tonumber(cc:sub(1, 2)) * 10
	if neg == "-" then ms = -ms end
	return ms
end
-- ms → ASS zaman dizgesi ("h:mm:ss.cc"; negatif öneki "-")
local function ms_zaman(ms)
	if type(ms) ~= "number" then return tostring(ms) end
	local v = math.floor(math.abs(ms) + 0.5)
	local cc = v % 100
	local t = math.floor(v / 100)
	local s = t % 60
	local m = math.floor(t / 60) % 60
	local h = math.floor(t / 3600)
	return ("%s%d:%02d:%02d.%02d")
		:format(ms < 0 and "-" or "", h, m, s, cc)
end
local function satir_oku(h, i0)
	local metin, e1 = oku(abi.velnosub_abi_line_text, h, i0)
	if not metin then error(e1, 2) end
	local zaman, e2 = oku(abi.velnosub_abi_line_times, h, i0)
	if not zaman then error(e2, 2) end
	local bas, son = zaman:match("^(.*) / (.*)$")
	if not bas then
		bas, son = zaman, zaman
	end
	local ms_bas = zaman_ms(bas)
	local ms_son = zaman_ms(son)
	if not ms_bas or not ms_son then
		error(("zaman dizgesi ASS biçiminde değil: %s"):format(zaman), 2)
	end
	-- P5.3: aegisub satır alanları — class (ABI olay kolunda hep
	-- "dialogue") + comment (Comment olayı bayrağı)
	local meta = oku(abi.velnosub_abi_line_meta, h, i0)
	if not meta then error(hatasi("line_meta"), 2) end
	local mhaneler = hata_ayikla(meta)
	return {
		text = metin,
		start_time = ms_bas,
		end_time = ms_son,
		class = mhaneler[1] or "dialogue",
		comment = mhaneler[2] == "1",
	}
end

local function subs_nesne(h)
	local adet = abi.velnosub_abi_event_count(h)
	if adet < 0 then error(hatasi("event_count"), 3) end
	-- newproxy:userdata satır görünümü — LuaJIT'te tablo __len güvenilmez
	-- (#subs array part'a bakar); userdata metadizisinde __len işler
	local proxy = newproxy(true)
	local mt = getmetatable(proxy)
	mt.__index = function(_, i)
		-- P5.2 devam: yöntem/özellik düz adlar (auto4 subtitle nesnesi)
		if type(i) == "string" then
			if i == "insert_line" then
				return function(_, konum, satir)
					local p0 = tam_sayi(konum)
					if not p0 or p0 < 1 or p0 > adet + 1 then
						error(("insert_line konum dışarıda: %s (satır %d)")
						      :format(tostring(konum), adet), 2)
					end
					if type(satir) ~= "table" or satir.text == nil then
						error("insert_line: satır tablosu .text gerekli", 2)
					end
					-- P5.2 dilim: zaman alanları ms tamsayısı veya ASS
					-- dizgesi olabilir; ABI ASS dizgesi ister → çevir
					local bas = type(satir.start_time) == "number"
						and ms_zaman(satir.start_time)
						or tostring(satir.start_time or "")
					local son = type(satir.end_time) == "number"
						and ms_zaman(satir.end_time)
						or tostring(satir.end_time or "")
					if abi.velnosub_abi_line_insert(h, p0 - 1,
					   tostring(satir.text), bas, son,
					   satir.style and tostring(satir.style) or "") ~= 0 then
						error(hatasi("insert_line"), 2)
					end
					adet = adet + 1
				end
			elseif i == "delete_line" then
				return function(_, konum)
					local p0 = tam_sayi(konum)
					if not p0 or p0 < 1 or p0 > adet then
						error(("delete_line konum dışarıda: %s (satır %d)")
						      :format(tostring(konum), adet), 2)
					end
					if abi.velnosub_abi_line_delete(h, p0 - 1) ~= 0 then
						error(hatasi("delete_line"), 2)
					end
					adet = adet - 1
				end
			elseif i == "n_lines" then
				return adet
			end
			return nil
		end
		local i0 = tam_sayi(i)
		if not i0 or i0 < 1 or i0 > adet then
			error(("satır aralık dışı: %s (satır %d)"):format(tostring(i), adet), 2)
		end
		return satir_oku(h, i0 - 1)
	end
	mt.__newindex = function(_, i, satir)
		local i0 = tam_sayi(i)
		if not i0 or i0 < 1 or i0 > adet then
			error(("satır aralık dışı: %s (satır %d)"):format(tostring(i), adet), 2)
		end
		if type(satir) ~= "table" then
			error("satır ataması tablo olmalı (" .. type(satir) .. ")", 2)
		end
		-- P5.2: alan yazımı — text flyweight vekiliyle, zamanlar ms
		-- tamsayısıyla; verilmeyen alanlar mevcut değerinde kalır
		if satir.text ~= nil then
			if abi.velnosub_abi_edit_line(h, i0 - 1, tostring(satir.text)) ~= 0 then
				error(hatasi("edit_line"), 2)
			end
		end
		if satir.start_time ~= nil or satir.end_time ~= nil then
			local mevcut = oku(abi.velnosub_abi_line_times, h, i0 - 1)
			if not mevcut then error(hatasi("line_times"), 2) end
			local zb, zs = mevcut:match("^(.*) / (.*)$")
			local ms_bas = zaman_ms(zb)
			local ms_son = zaman_ms(zs)
			if not ms_bas or not ms_son then
				error(("zaman dizgesi ASS biçiminde değil: %s"):format(mevcut), 2)
			end
			if type(satir.start_time) == "number" then
				ms_bas = math.floor(satir.start_time)
			elseif type(satir.start_time) == "string" then
				ms_bas = zaman_ms(satir.start_time)
				if not ms_bas then
					error(("start_time ASS biçiminde değil: %s")
					      :format(tostring(satir.start_time)), 2)
				end
			elseif satir.start_time ~= nil then
				error("start_time tip: ms sayısı veya ASS dizgesi", 2)
			end
			if type(satir.end_time) == "number" then
				ms_son = math.floor(satir.end_time)
			elseif type(satir.end_time) == "string" then
				ms_son = zaman_ms(satir.end_time)
				if not ms_son then
					error(("end_time ASS biçiminde değil: %s")
					      :format(tostring(satir.end_time)), 2)
				end
			elseif satir.end_time ~= nil then
				error("end_time tip: ms sayısı veya ASS dizgesi", 2)
			end
			if abi.velnosub_abi_line_times_set(h, i0 - 1, ms_bas, ms_son) ~= 0 then
				error(hatasi("line_times_set"), 2)
			end
		end
	end
	mt.__len = function()
		return adet
	end
	return proxy
end

-- ---- aegisub.dialog.display: dialog tanım tablosu → ABI mini DSL ------
-- kurallar (faz4 P4.5 III DSL): ctl|sınıf|ad|başlik|değer[|ek] ;
-- label 4 hücreli değersiz; intedit/floatedit ek = min,max ;
-- dropdown ek = öğeler ';' ayırımlı ; checkbox değer 0/1.
local function deger_dizgesi(c)
	local sinif = c.class or ""
	local deger = c.value
	if sinif == "checkbox" then
		deger = deger and "1" or "0"
	elseif deger == nil then
		if sinif == "dropdown" and type(c.items) == "table" then
			deger = tostring(c.items[1] or "")
		else
			deger = ""
		end
	else
		deger = tostring(deger)
	end
	return deger
end

local function kontrol_satiri(c)
	local sinif = c.class or ""
	local ad = tostring(c.name or "")
	local baslik = tostring(c.label or "")
	if sinif == "label" then
		return "ctl|label|" .. ad .. "|" .. baslik
	end
	local satir = "ctl|" .. sinif .. "|" .. ad .. "|" .. baslik .. "|"
	             .. deger_dizgesi(c)
	if sinif == "intedit" or sinif == "floatedit" then
		satir = satir .. "|" .. tostring(c.min or 0) .. "," .. tostring(c.max or 0)
	elseif sinif == "dropdown" then
		local ogeler = {}
		for _, o in ipairs(type(c.items) == "table" and c.items or {}) do
			ogeler[#ogeler + 1] = tostring(o)
		end
		satir = satir .. "|" .. table.concat(ogeler, ";")
	end
	return satir
end

-- okuma cevabında tip dönüşümü: intedit/floatedit → sayı; checkbox → bool
local function deger_coz(c, dizge)
	local sinif = c.class or ""
	if sinif == "checkbox" then
		return dizge == "1"
	elseif sinif == "intedit" then
		local n = tam_sayi(tonumber(dizge))
		return n or dizge
	elseif sinif == "floatedit" then
		local n = tonumber(dizge)
		return n or dizge
	end
	return dizge
end

local cevap_btn = nil      -- pompadan gelen düğme seçimi (1-taban)
local cevap_vals = nil     -- pompadan gelen ad→değer tablosu

local function display(dlg, buttons)
	assert(type(dlg) == "table", "dialog tanımı tablo olmalı")
	local satirlar = { "# aegisub.dialog pompası (P5.2)" }
	for _, c in ipairs(dlg) do
		satirlar[#satirlar + 1] = kontrol_satiri(c)
	end
	local dugmeler = buttons or { "Ok" }
	for _, b in ipairs(dugmeler) do
		satirlar[#satirlar + 1] = "buton|" .. tostring(b)
	end
	local cd = abi.velnosub_abi_dialog_open(table.concat(satirlar, "\n") .. "\n")
	if cd < 0 then
		error(hatasi("dialog_open"), 2)
	end
	local adet = abi.velnosub_abi_dialog_count(cd)
	local siniflar, adlar = {}, {}
	for i = 0, adet - 1 do
		local t, e = oku(abi.velnosub_abi_dialog_ctl, cd, i)
		if not t then error(e, 2) end
		local haneler = hata_ayikla(t)
		siniflar[i] = haneler[1]
		adlar[i] = haneler[2]
		if cevap_vals and adlar[i] ~= "" then
			local cevap = cevap_vals[adlar[i]]
			if cevap ~= nil then
				local dizge = cevap
				if type(cevap) == "boolean" then
					dizge = cevap and "1" or "0"
				end
				if abi.velnosub_abi_dialog_set(cd, i, tostring(dizge)) ~= 0 then
					local e2 = hatasi("dialog_set " .. adlar[i])
					abi.velnosub_abi_dialog_close(cd)
					error(e2, 2)
				end
			end
		end
	end
	local secim = cevap_btn or 1
	if abi.velnosub_abi_dialog_show(cd, secim - 1) < 0 then
		local e = hatasi("dialog_show")
		abi.velnosub_abi_dialog_close(cd)
		error(e, 2)
	end
	local vals = {}
	for i = 0, adet - 1 do
		local t, e = oku(abi.velnosub_abi_dialog_ctl, cd, i)
		if not t then error(e, 2) end
		local haneler = hata_ayikla(t)
		vals[adlar[i]] = deger_coz({ class = siniflar[i] }, haneler[4] or "")
	end
	local btnmetin, e = oku(abi.velnosub_abi_dialog_button, cd, secim - 1)
	abi.velnosub_abi_dialog_close(cd)
	if not btnmetin then error(e, 2) end
	return btnmetin, vals
end

-- ---- makro kayıtları -------------------------------------------------
local makrolar = {}
-- P5.2 dilim B: pump açık dosyayı burada tutar (aegisub.file_name kolu)
local acik_dosya_yolu = nil

local function register_macro(ad, aciklama, isleme, denetim)
	makrolar[ad] = {
		aciklama = aciklama,
		isleme = isleme,
		denetim = denetim,
	}
	print(("[AUSR] makro kaydı: %s (%s)"):format(ad, aciklama or ""))
end

-- ---- pompa: arg tablosundan --ass/--macro/--btn/--val/--sel/--save -----
local function arg_ayikla()
	local p = { val = {}, sel = {} }
	local a = aegisub.arg or {}
	local i = 1
	while a[i] do
		local k = tostring(a[i])
		if k == "--ass" or k == "--macro" or k == "--btn" or k == "--save"
		   or k == "--sel" then
			local v = a[i + 1]
			if k == "--ass" then p.ass = v
			elseif k == "--macro" then p.makro = v
			elseif k == "--btn" then p.btn = tonumber(v) or 1
			elseif k == "--save" then p.save = v
			elseif k == "--sel" then p.sel[#p.sel + 1] = tonumber(v) or 1
			end
			i = i + 2
		elseif k == "--val" then
			local ad, deger = tostring(a[i + 1] or ""):match("^(.-)=(.*)$")
			if ad ~= "" then p.val[ad] = deger end
			i = i + 2
		else
			i = i + 1
		end
	end
	return p
end

function aegisub.__pump()
	local p = arg_ayikla()
	assert(p.ass, "--ass yok: pump dosyasız koşamaz")
	local cfg = os.getenv("VELNOSUB_CONFIG_JSON") or "default_config.json"
	if abi.velnosub_abi_init(cfg) ~= 0 then
		error(hatasi("init"), 0)
	end
	print("[AUS0] yapılandırma hazır (" .. cfg .. ")")
	local h = abi.velnosub_abi_open(p.ass)
	if h < 0 then
		error(hatasi("open " .. p.ass), 0)
	end
	local adet = abi.velnosub_abi_event_count(h)
	acik_dosya_yolu = p.ass
	print(("[AUS1] dosya açıldı: %s (satır %d, seçim %s, makro %s)")
	      :format(p.ass, adet, tostring(#p.sel or 0), tostring(p.makro)))
	local subs = subs_nesne(h)
	cevap_vals = p.val
	cevap_btn = p.btn
	if p.makro then
		local m = makrolar[p.makro]
		assert(m, "makro kayıtlı değil: " .. tostring(p.makro))
		local sel = #p.sel > 0 and p.sel or { 1 }
		local okk, err = pcall(m.isleme, subs, sel, sel[1] or 1)
		if not okk then
			abi.velnosub_abi_close(h)
			error("makro hatası: " .. tostring(err), 0)
		end
		print("[AUS2] makro koştu: " .. p.makro)
	end
	if p.save then
		if abi.velnosub_abi_save(h, p.save) ~= 0 then
			local e = hatasi("save")
			abi.velnosub_abi_close(h)
			error(e, 0)
		end
		print("[AUS3] yazıldı: " .. p.save)
		print("[AUS4a] yeniden açılış öncesi")
		local h2 = abi.velnosub_abi_open(p.save)
		print("[AUS4b] yeniden açıldı tutamaç: " .. tostring(h2))
		assert(h2 >= 0, hatasi("yeniden açılış"))
		local adet2 = abi.velnosub_abi_event_count(h2)
		print("[AUS4c] olay sayısı: " .. tostring(adet2))
		local adet2 = abi.velnosub_abi_event_count(h2)
		assert(adet2 == adet,
			("kalıcılık: satır sayısı değişti %d → %d"):format(adet, adet2))
		local son = satir_oku(h2, math.min(adet, 1) - 1)
		print(("[AUS4] kalıcılık: yeniden açılış satır 1: %s"):format(son.text))
		for _, si in ipairs(p.sel and #p.sel > 0 and p.sel or { 1 }) do
			local s2 = satir_oku(h2, si - 1)
			print(("[AUSB] yeniden açılış satır %d: %s")
			      :format(si, s2.text))
		end
		abi.velnosub_abi_close(h2)
	end
	cevap_vals = nil
	cevap_btn = nil
	abi.velnosub_abi_close(h)
	print("[AUS5] pump tamam")
end

-- P5.3: aegisub 2.x include() — include dizininden BOM temizli yükleme
-- (upstream autoload scriptleri bu konvansiyonu kullanıyor)
function include(ad)
	local kurulu = os.getenv("VELNOSUB_AEGIINC") or "share/lua/inc"
	for d in tostring(kurulu):gmatch("[^:;]+") do
		local yol = d .. "/" .. tostring(ad)
		local ff = io.open(yol, "rb")
		if ff then
			local metin = ff:read("*a") or ""
			ff:close()
			-- BOM (EF BB BF) yut
			if metin:sub(1, 3) == "\239\187\191" then
				metin = metin:sub(4)
			end
			local yuk, hata = loadstring(metin, "@" .. tostring(ad))
			if not yuk then
				error("include hatası (" .. ad .. "): " .. tostring(hata), 2)
			end
			yuk()
			return true
		end
	end
	error("include yok: " .. tostring(ad) .. " (" .. kurulu .. ")")
end

-- ---- yüzey monte ----------------------------------------------------
aegisub.dialog = { display = display }
aegisub.register_macro = register_macro
aegisub.unregister_macro = function(a) makrolar[a] = nil end
aegisub.progress = {
	set = function() end,
	set_task = function(s) aegisub.log("progress: " .. tostring(s)) end,
	task = function(s) aegisub.log("progress: " .. tostring(s)) end,
	is_cancelled = function() return false end,
	title = function() end,
}
aegisub.debug = {
	out = function(...) aegisub.log(tostring((select(1, ...)))) end,
	mode = function() end,
}
-- P5.3: geri kalan aegisub.* yüzey katmanı
aegisub.set_undo_point = function(ad)
	aegisub.log("undo noktası (mobil hatta kayıt yok): " .. tostring(ad))
end
aegisub.register_filter = function(ad, aciklama, kategori, isleme)
	aegisub.log(("filter kaydı (mobil dilimde koşulmaz): %s (%d adet işlemci)")
	             :format(tostring(ad), type(isleme) == "table" and #isleme or 1))
end
aegisub.cancel = function() end
-- P5.2 dilim B: video köprüsü + zarif yüzey parçaları (upstream auto4
-- davranışı: video yüklü değilken frame_from_ms/ms_from_frame/video_size
-- nil döner; mobil hatta otomasyon kolu videoyu Lua yüzeyine açmıyor)
aegisub.lua_automation_version = 4
aegisub.frame_from_ms = function(ms)
	if tam_sayi(ms) == nil then
		error("frame_from_ms: ms tamsayı olmalı", 2)
	end
	print("[AUSB] frame_from_ms: video yüklü değil → nil")
	return nil
end
aegisub.ms_from_frame = function(k)
	if tam_sayi(k) == nil then
		error("ms_from_frame: kare tamsayı olmalı", 2)
	end
	print("[AUSB] ms_from_frame: video yüklü değil → nil")
	return nil
end
aegisub.video_size = function()
	print("[AUSB] video_size: video yüklü değil → nil")
	return nil
end
aegisub.keyframes = function()
	print("[AUSB] keyframes: video yok → boş küme")
	return {}
end
aegisub.file_name = function()
	if not acik_dosya_yolu then return nil end
	return tostring(acik_dosya_yolu):match("[^/\\]+$")
		or tostring(acik_dosya_yolu)
end
aegisub.gettext = function(s) return tostring(s) end
aegisub.project_properties = function()
	print("[AUSB] proje özetleri: mobil hatta nötr değerler")
	return {
		automation_scripts = false,
		export_filters = false,
		export_encoding = "",
		style_storage = 0,
		video_zoom = 100.0,
		ar_value = 0.0,
		scroll_position = -1,
		active_row = 0,
		ar_mode = 0,
		video_position = 0,
		audio_file = "",
		video_file = "",
		timecodes_file = "",
		keyframes_file = "",
	}
end
aegisub.decode_path = function(yol)
	local kullanici = os.getenv("VELNOSUB_USER_DIR") or (
		os.getenv("HOME") and (os.getenv("HOME") .. "/.velnosub") or "/data/local/tmp/velnosub-user")
	local oge = tostring(yol)
	oge = oge:gsub("^%?user", kullanici)
	oge = oge:gsub("^%?data", os.getenv("VELNOSUB_DATA_DIR") or ".")
	oge = oge:gsub("^%?script", os.getenv("VELNOSUB_SCRIPT_DIR") or ".")
	oge = oge:gsub("^%?temp", "/tmp")
	return oge
end
aegisub.__stub = nil
print("[AUST] aegisub yüzeyi gerçek ABI hattında (P5.1/P5.2)")
