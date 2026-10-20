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
	local l = subs[1]
	if vals.buyuk then
		l.text = tostring(vals.metin):upper()
	else
		l.text = tostring(vals.metin)
	end
	l.text = l.text .. " @" .. tostring(vals.sayi)
	subs[1] = l
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
