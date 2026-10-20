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
