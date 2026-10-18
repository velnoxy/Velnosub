// Copyright (c) 2014, Thomas Goyne <plorkyeran@aegisub.org>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//
// Aegisub Project http://www.aegisub.org/

#include "text_selection_controller.h"

// velnosub (P2.2): wx SC intifa area zaten yalnız ctrl non-null iken gerekli;
// AGI_CORE derlemesinde (mobil çekirdek) wx başlıkları koşturmaz, ctrl yok sayılır.
#ifndef AGI_CORE
#include <wx/stc/stc.h>
#endif

void TextSelectionController::SetControl(wxStyledTextCtrl *ctrl) {
	this->ctrl = ctrl;
#ifndef AGI_CORE
	if (ctrl)
		ctrl->Bind(wxEVT_STC_UPDATEUI, &TextSelectionController::UpdateUI, this);
#endif
}

TextSelectionController::~TextSelectionController() {
#ifndef AGI_CORE
	if (ctrl) ctrl->Unbind(wxEVT_STC_UPDATEUI, &TextSelectionController::UpdateUI, this);
#endif
}

#define GET(var, new_value) do { \
	int tmp = new_value;      \
	if (tmp != var) {         \
		var = tmp;            \
		changed = true;       \
	}                         \
} while(false)

#ifdef AGI_CORE
// velnosub (P2.2): wx SC yok — yalnız alan güncelleyip sinyali duyurur.
#define SET(var, new_value, Setter) do { \
	if (var != new_value) {              \
		var = new_value;                 \
	}                                    \
} while (false)
#else
#define SET(var, new_value, Setter) do { \
	if (var != new_value) {              \
		var = new_value;                 \
		if (ctrl) ctrl->Setter(var);     \
	}                                    \
} while (false)
#endif

void TextSelectionController::UpdateUI(wxStyledTextEvent &evt) {
#ifndef AGI_CORE
	if (changing) return;

	bool changed = false;
	GET(insertion_point, ctrl->GetInsertionPoint());
	if (evt.GetUpdated() & wxSTC_UPDATE_SELECTION) {
		GET(selection_start, ctrl->GetSelectionStart());
		GET(selection_end, ctrl->GetSelectionEnd());
	}
	else {
		GET(selection_start, insertion_point);
		GET(selection_end, insertion_point);
	}
	if (changed) AnnounceSelectionChanged();
#endif
}

void TextSelectionController::SetInsertionPoint(int position) {
	changing = true;
	SET(insertion_point, position, SetInsertionPoint);
	changing = false;
	AnnounceSelectionChanged();
}

void TextSelectionController::SetSelection(int start, int end) {
	changing = true;
	SET(selection_start, start, SetSelectionStart);
	SET(selection_end, end, SetSelectionEnd);
	changing = false;
	AnnounceSelectionChanged();
}


void TextSelectionController::CommitStagedChanges() {
	if (has_staged_selection) {
		if (staged_selection_start <= staged_selection_end) {
			SetSelection(staged_selection_start, staged_selection_end);
		} else {
			// commit some crimes to get this to work in all cases
			SetInsertionPoint(staged_selection_end == 0 ? staged_selection_start : 0);
			SetSelection(staged_selection_start, staged_selection_start);
			SetInsertionPoint(staged_selection_end);
		}
		has_staged_selection = false;
	}
}