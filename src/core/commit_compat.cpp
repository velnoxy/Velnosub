// Velsub Faz 1 — aegisub-core için wx'siz AssFile::Commit iliği.
//
// Masaüstündeki AssFile::Commit(wxString const&, ...) gövdesiyle (ass_file.cpp)
// davranış uyumludur; tek fark PushState'tir: undo yığını arabirimi UI
// tarafında olduğundan çekirdek hedefinde yoksayılır. Açıklama metni UTF-8
// tutulur; çeviriyi görüntüleyen UI yapar.

#include "ass_file.h"
#include "ass_dialogue.h"

int AssFile::Commit(std::string const& desc_utf8, int type, int commitId, AssDialogue *single_line) {
	if (type == COMMIT_NEW || (type & COMMIT_DIAG_ADDREM) || (type & COMMIT_ORDER)) {
		int i = 0;
		for (auto& event : Events)
			event.Row = i++;
	}

	(void)desc_utf8; // undo açıklaması UI tarafında üretildiğinde çevrilir.

	AnnouncePreCommit(type, single_line);
	AnnounceCommit(type, single_line);

	return commitId;
}
