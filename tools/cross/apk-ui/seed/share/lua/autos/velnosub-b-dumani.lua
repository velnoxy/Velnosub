-- velnosub P5.2 dilim B dumanı — aegisub video köprüsü + zarif yüzey.
-- Beklenen: frame_from_ms/ms_from_frame/video_size → nil (video yokken
-- upstream davranışı), keyframes → boş küme, file_name → --ass temel adı,
-- lua_automation_version → 4, project_properties → 14 alan.
local function duman(subs, sel, aktif)
	aegisub.log("B-duman başladı\n")
	aegisub.log(("frame_from_ms(1000) → %s\n")
		:format(tostring(aegisub.frame_from_ms(1000))))
	aegisub.log(("ms_from_frame(24) → %s\n")
		:format(tostring(aegisub.ms_from_frame(24))))
	aegisub.log(("video_size → %s\n"):format(tostring(aegisub.video_size())))
	aegisub.log(("keyframes → %d küme\n")
		:format(#aegisub.keyframes()))
	aegisub.log(("lua_automation_version → %s\n")
		:format(tostring(aegisub.lua_automation_version)))
	aegisub.log(("file_name → %s\n"):format(tostring(aegisub.file_name())))
	aegisub.log(("gettext '%s' → %s\n")
		:format("Merhaba", tostring(aegisub.gettext("Merhaba"))))
	local ozet = aegisub.project_properties()
	local adet = 0
	local adlar = {}
	for k in pairs(ozet) do
		adet = adet + 1
		table.insert(adlar, k)
	end
	table.sort(adlar)
	aegisub.log(("project_properties → %d alan\n"):format(adet))
	for _, k in ipairs(adlar) do
		aegisub.log(("  %s = %s\n"):format(k, tostring(ozet[k])))
	end
	aegisub.log("B-duman bitti\n")
end

aegisub.register_macro("B duman", "P5.2 dilim B yüzey koşumu", duman)
