// velnosub Faz 5 (P5.3) — flyweight metin alanı mutasyon vekilleri.
//
// KÖK NEDEN (mini-bidirme minibindirme ile kanıtlandı, 2026-10-20):
// AssDialogueBase::Text vb. alanlar boost::flyweight<std::string>; fabrika
// (Meyers tekili) INLINE işlev yerel statikleri olduğu için DSO başına AYRI
// örneklemleniyor: libvelnosub-core.so yükleyicisi F1'e intern ediyor,
// libvelnosub-abi.so / velnosub-cli ise kendi fabrikası (F2/F3) üzerinden
// yazıyor. Çapraz-fabrika refcount düşümü + erase yabancı düğümü yığında
// "free(): invalid size" / "double free or corruption" üretiyor (tekil
// düzenlemede gizli, tekrarlı düzenlemede çökme — P5.3 Clean Tags akışı).
//
// ÇÖZÜM: flyweight alanlarına YAZAN her iş bu TU'da (core.so içinde derlenir,
// fabrika = yükleyicinin kullandığı F1) toplanır. abi/cli yalnız bu vekilleri
// çağırır; okumalar fabrikaya dokunmadığından serbest kalır.

#include "ass_file.h"
#include "ass_dialogue.h"

extern "C" int velnosub_core_edit_line_text(AssFile &file, int idx, const char *text) {
	if (idx < 0 || idx >= static_cast<int>(file.Events.size()))
		return -1;
	std::next(file.Events.begin(), idx)->Text = text;
	file.Commit("velnosub abi", AssFile::COMMIT_DIAG_TEXT);
	return 0;
}

extern "C" int velnosub_core_edit_first_text(AssFile &file, const char *text) {
	if (file.Events.empty())
		return -1;
	file.Events.front().Text = text;
	file.Commit("velnosub abi", AssFile::COMMIT_DIAG_TEXT);
	return 0;
}

// P5.2 devam: satır ekleme — idx 0-taban; düğüm idx konumunun ÖNÜNE girer
// (idx == size → sona). Zaman dizgeleri ASS biçimi ("0:00:01.00"); boşsa
// varsayılan (Start 0, End 5000) kalır. Text/Style flyweight yazımı bu
// TU'da (F1 kolu), yukarıdaki kök neden notuna bak.
extern "C" int velnosub_core_insert_line(AssFile &file, int idx, const char *metin,
                                         const char *bas_zaman, const char *son_zaman,
                                         const char *stil) {
	if (idx < 0 || idx > static_cast<int>(file.Events.size()))
		return -1;
	AssDialogue *d = new AssDialogue;
	d->Text = metin ? std::string(metin) : std::string();
	if (bas_zaman && *bas_zaman)
		d->Start = agi::Time(std::string(bas_zaman));
	if (son_zaman && *son_zaman)
		d->End = agi::Time(std::string(son_zaman));
	if (stil && *stil)
		d->Style = stil;
	auto pos = idx >= static_cast<int>(file.Events.size())
	               ? file.Events.end()
	               : std::next(file.Events.begin(), idx);
	file.Events.insert(pos, *d);
	file.Commit("velnosub abi", AssFile::COMMIT_DIAG_ADDREM);
	return 0;
}

extern "C" int velnosub_core_delete_line(AssFile &file, int idx) {
	if (idx < 0 || idx >= static_cast<int>(file.Events.size()))
		return -1;
	auto it = std::next(file.Events.begin(), idx);
	AssDialogue *d = &*it;
	file.Events.erase(it); // intrusive: düğümü listeden söker; sahiplik bizde
	delete d;
	file.Commit("velnosub abi", AssFile::COMMIT_DIAG_ADDREM);
	return 0;
}

// P5.2 dilim: ms zaman yazımı — auto4 satır alanları start_time/end_time
// ms tamsayısıdır; shim buradan yazar. Time nesneleri flyweight DEĞİL;
// yine de mutasyon tek kolda (bu TU) kalır.
extern "C" int velnosub_core_set_line_times(AssFile &file, int idx,
                                            long long ms_bas, long long ms_son) {
	if (idx < 0 || idx >= static_cast<int>(file.Events.size()))
		return -1;
	auto &d = *std::next(file.Events.begin(), idx);
	d.Start = agi::Time(static_cast<int>(ms_bas));
	d.End = agi::Time(static_cast<int>(ms_son));
	file.Commit("velnosub abi", AssFile::COMMIT_DIAG_TIME);
	return 0;
}

// P5.4 gece dilimi: satır meta yazımı — Style/Actor/Effect flyweight
// yazımı BU TU'da (F1 kolu; kök neden notuna bak). yorum=-1/katman=-1
// → alan DOKUNULMAZ (kısmi yazım: UI tek alan değişirken diğerlerini
// bozmasın).
extern "C" int velnosub_core_set_line_meta(AssFile &file, int idx,
                                           const char *stil, const char *aktor,
                                           const char *etki, int yorum,
                                           int katman) {
	if (idx < 0 || idx >= static_cast<int>(file.Events.size()))
		return -1;
	auto &d = *std::next(file.Events.begin(), idx);
	if (stil) d.Style = stil;
	if (aktor) d.Actor = aktor;
	if (etki) d.Effect = etki;
	if (yorum >= 0) d.Comment = yorum != 0;
	if (katman >= 0) d.Layer = katman;
	file.Commit("velnosub abi", AssFile::COMMIT_DIAG_META);
	return 0;
}
