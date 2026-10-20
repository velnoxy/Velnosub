-- velnosub Faz 5 (P5.2) — demo makro scripti: iki makro kaydeder.
-- 1) "Ana harf": seçili satırın metnini büyük harfe çevirir
-- 2) "Form dene": aegisub.dialog.display pompası — dialog'dan gelen
--    metin + sayıyı 1. satırın metnine yazar
-- Koşum: velnosub-automation velnosub-demo-macro.lua --ass DOSYA
--        --macro "Form dene" --val metin=... --val sayi=7 --btn 1 --save YOL
-- (VELNOSUB_AEGISHIM ortam değişkeni aegisub yüzeyini yüklüyor)

local function ana_harf(subs, sel, aktif)
	aegisub.log(("ana harf: seçim %d, aktif %d, satır %d")
	             :format(#sel, aktif or -1, #subs))
	for _, i in ipairs(sel) do
		local l = subs[i]
		aegisub.log(("öncesi %d: %s"):format(i, l.text))
		l.text = l.text:upper()
		subs[i] = l
		aegisub.log(("sonrası %d: %s"):format(i, subs[i].text))
	end
	aegisub.progress.set_task("ana-harf bitti")
end

local function form_dene(subs, sel, aktif)
	aegisub.log(("form dene: seçim %d, aktif %d, satır %d")
	             :format(#sel, aktif or -1, #subs))
	local dugme, vals = aegisub.dialog.display({
		{ class = "label", label = "Makro formu: değerleri doldur" },
		{ class = "edit", name = "metin", label = "Metin", value = "selam" },
		{ class = "intedit", name = "sayi", label = "Sayı", value = 42, min = 1, max = 100 },
		{ class = "checkbox", name = "buyuk", label = "Büyük harf dağıt", value = true },
	}, { "Uygula", "İptal" })
	aegisub.log(("form döndü: düğme %s, metin %s, sayı %s, buyuk %s")
	             :format(tostring(dugme), tostring(vals.metin),
	                     tostring(vals.sayi), tostring(vals.buyuk)))
	local i = aktif or sel[1]
	local l = subs[i]
	if vals.buyuk then
		l.text = tostring(vals.metin):upper()
	else
		l.text = tostring(vals.metin)
	end
	l.text = l.text .. " @" .. tostring(vals.sayi)
	subs[i] = l
	aegisub.debug.out("form hattı: " .. l.text)
end

aegisub.register_macro("Ana harf", "seçili satır metnini büyük harfe çevirir", ana_harf)
aegisub.register_macro("Form dene", "renk stişi için örnek: form + satır yazımı", form_dene)

-- P5.2 devam: insert_line/delete_line yöntemleri — seçili satırın ÖNÜNE
-- kopya ekleyip ("önce: " öneki) asıl satırı siliyor (net: önekleme)
local function ekle_sil(subs, sel, aktif)
	local i = sel[1] or 1
	local l = subs[i]
	aegisub.log(("ekle sil: %d. satır öncesi metin özeneği %s")
	             :format(i, l.text:sub(1, 26)))
	local kopya = {
		text = "önce: " .. l.text,
		start_time = l.start_time,
		end_time = l.end_time,
		style = nil,
	}
	subs:insert_line(i, kopya)
	subs:delete_line(i + 1)
	aegisub.log(("ekle sil: %d. satır sonrası %s, satır adeti %d")
	             :format(i, subs[i].text:sub(1, 26), subs.n_lines))
	aegisub.debug.out("ekle sil bitti\n")
end
aegisub.register_macro("Ekle sil dene", "insert_line/delete_line dumanı", ekle_sil)

-- P5.2 dilim C: aegisub.text_extents dumanı — stil tablosuyla ölçüm +
-- sınıf tip kancası (dialog tablosu "Not a style entry" reddi)
local function olc_dene(subs, sel, aktif)
	local st = {
		class = "style",
		fontname = "DejaVu Sans",
		fontsize = 28,
		bold = false,
		italic = false,
		underline = false,
		strikeout = false,
		spacing = 0,
		scale_x = 100,
		scale_y = 100,
	}
	local g, y, d, l = aegisub.text_extents(st, "Merhaba velnosub")
	aegisub.log(("ölçüm: g=%.2f y=%.2f d=%.2f l=%.2f")
	             :format(g, y, d, l))
	local ok, cevap = pcall(function()
		return aegisub.text_extents({ class = "dialogue" }, "x")
	end)
	aegisub.log(("dialog sınıf red: %s (%s)")
	             :format(tostring(ok), tostring(cevap)))
end
aegisub.register_macro("Ölç dene", "P5.2 dilim C ölçüm dumanı", olc_dene)

-- P5 dilim V: sanal satır uzayı dumanı — ipairs(subs) info+stil+olay
-- sırasını dolaşır; #subs/n tam satır; aralık kırması upstream mesajıyla
local function dolas_dene(subs, sel, aktif)
	aegisub.log(("dolaş: toplam=%d, sel=%d"):format(#subs, sel[1] or -1))
	local sayi = 0
	for i, l in ipairs(subs) do
		sayi = sayi + 1
		if sayi > 12 then break end
		if l.class == "info" then
			aegisub.log(("dolaş %d: info %s = %s")
			             :format(i, l.key, l.value:sub(1, 30)))
		elseif l.class == "style" then
			aegisub.log(("dolaş %d: style %s (fontname=%s, yazı=%d, align=%d, marj=%d,%d,%d)")
			             :format(i, l.name, l.fontname, l.fontsize,
			                     l.align, l.margin_l, l.margin_r, l.margin_t))
		else
			aegisub.log(("dolaş %d: %s (%s) %s")
			             :format(i, l.class, tostring(l.comment), l.text:sub(1, 34)))
		end
	end
	aegisub.log(("dolaş: ilk 12 döndü (%d satırda kırıldı)")
	             :format(sayi))
	local son = subs[subs.n]
	aegisub.log(("dolaş son satır: %s — %s")
	             :format(son.class, son.text and son.text:sub(1, 24) or son.name))
	local ok1 = pcall(function() return subs[0] end)
	local ok2 = pcall(function() return subs[#subs + 1] end)
	aegisub.log(("dolaş aralık kırmıları: alt=%s, üst=%s")
	             :format(tostring(not ok1), tostring(not ok2)))
end
aegisub.register_macro("Dolaş dene", "sanal satır uzayı dumanı (dilim V)", dolas_dene)

-- P5 dilim 1: subs.styles okuma dumanı — karaskel hattı (n/s[i]/s[ad])
local function stil_dene(subs, sel, aktif)
	local s = subs.styles
	aegisub.log(("stiller: n=%d"):format(s.n))
	for i = 1, s.n do
		local st = s[i]
		aegisub.log(("stil %d: %s fontname=%s yazı=%d renk1=%s kimlik=%s")
		             :format(i, st.name, st.fontname, st.fontsize,
		                     st.color1, st.class))
	end
	local d = s["Default"]
	aegisub.log(("ad araması 'Default': %s (bold=%s, align=%d, marj=%d,%d,%d,%d, relative_to=%d)")
	             :format(tostring(d.name), tostring(d.bold), d.align,
	                     d.margin_l, d.margin_r, d.margin_t, d.margin_b,
	                     d.relative_to))
	local g = s.yok_boyle_stil
	aegisub.log(("ad araması 'yok_boyle_stil': %s"):format(tostring(g)))
	local _, y = aegisub.text_extents(d, "Ölç")
	aegisub.log(("Default ile ölçüm (text_extents hattı): y=%.2f")
	            :format(y))
end
aegisub.register_macro("Stil dene", "subs.styles okuma dumanı (dilim 1)", stil_dene)

-- P5 dilim 2: subs.script_resolution dumanı — upstream GetResolution
-- kardeşliği: PlayRes kuralları
local function coz_dene(subs, sel, aktif)
	local bw, bh = subs.script_resolution()
	aegisub.log(("çöz: %d x %d"):format(bw, bh))
end
aegisub.register_macro("Çöz dene", "script_resolution dumanı (dilim 2)", coz_dene)
