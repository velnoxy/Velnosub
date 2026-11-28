// Copyright (c) 2005, Rodrigo Braz Monteiro, Niels Martin Hansen
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
//   * Redistributions of source code must retain the above copyright notice,
//     this list of conditions and the following disclaimer.
//   * Redistributions in binary form must reproduce the above copyright notice,
//     this list of conditions and the following disclaimer in the documentation
//     and/or other materials provided with the distribution.
//   * Neither the name of the Aegisub Group nor the names of its contributors
//     may be used to endorse or promote products derived from this software
//     without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
//
// Aegisub Project http://www.aegisub.org/

/// @file frame_main.cpp
/// @brief Main window creation and control management
/// @ingroup main_ui

#include "frame_main.h"

#include "include/aegisub/context.h"
#include "include/aegisub/menu.h"
#include "include/aegisub/toolbar.h"
#include "include/aegisub/hotkey.h"

#include "ass_file.h"
#include "async_video_provider.h"
#include "audio_controller.h"
#include "audio_box.h"
#include "base_grid.h"
#include "compat.h"
#include "command/command.h"
#include "dialog_detached_video.h"
#include "dialog_manager.h"
#include "libresrc/libresrc.h"
#include "main.h"
#include "options.h"
#include "project.h"
#include "subs_controller.h"
#include "subs_edit_box.h"
#include "utils.h"
#include "version.h"
#include "video_box.h"
#include "video_controller.h"
#include "video_display.h"

#include <libaegisub/dispatch.h>
#include <libaegisub/log.h>
#include <libaegisub/make_unique.h>

#include <algorithm>

#include <wx/button.h>
#include <wx/choice.h>
#include <wx/dnd.h>
#include <wx/notebook.h>
#include <wx/panel.h>
#include <wx/simplebook.h>
#include <wx/stattext.h>
#include <wx/msgdlg.h>
#include <wx/sizer.h>
#include <wx/statline.h>
#include <wx/sysopt.h>
#include <wx/utils.h>

enum {
	ID_APP_TIMER_STATUSCLEAR = 12002
};

#ifdef WITH_STARTUPLOG
#define StartupLog(a) MessageBox(0, a, "Aegisub startup log", 0)
#else
#define StartupLog(a) LOG_I("frame_main/init") << a
#endif

/// Handle files drag and dropped onto Aegisub
class AegisubFileDropTarget final : public wxFileDropTarget {
	agi::Context *context;
public:
	AegisubFileDropTarget(agi::Context *context) : context(context) { }
	bool OnDropFiles(wxCoord, wxCoord, wxArrayString const& filenames) override {
		std::vector<agi::fs::path> files;
		for (wxString const& fn : filenames)
			files.push_back(from_wx(fn));
		agi::dispatch::Main().Async([=] { context->project->LoadList(files); });
		return true;
	}
};

FrameMain::FrameMain()
: wxFrame(nullptr, -1, "", wxDefaultPosition, wxSize(920,700), wxDEFAULT_FRAME_STYLE | wxCLIP_CHILDREN)
, context(agi::make_unique<agi::Context>())
{
	StartupLog("Entering FrameMain constructor");

#ifdef __WXGTK__
	// XXX HACK XXX
	// We need to set LC_ALL to "" here for input methods to work reliably.
	setlocale(LC_ALL, "");

	// However LC_NUMERIC must be "C", otherwise some parsing fails.
	setlocale(LC_NUMERIC, "C");
#endif

	StartupLog("Initializing context controls");
	context->ass->AddCommitListener(&FrameMain::UpdateTitle, this);
	context->subsController->AddFileOpenListener(&FrameMain::OnSubtitlesOpen, this);
	context->subsController->AddFileSaveListener(&FrameMain::UpdateTitle, this);
	context->project->AddAudioProviderListener(&FrameMain::OnAudioOpen, this);
	context->project->AddVideoProviderListener(&FrameMain::OnVideoOpen, this);

	StartupLog("Initializing context frames");
	context->parent = this;
	context->frame = this;

	StartupLog("Apply saved Maximized state");
	if (OPT_GET("App/Maximized")->GetBool()) Maximize(true);

	StartupLog("Initialize toolbar");
	wxSystemOptions::SetOption("msw.remap", 0);
	OPT_SUB("App/Show Toolbar", &FrameMain::EnableToolBar, this);
	EnableToolBar(*OPT_GET("App/Show Toolbar"));

	StartupLog("Initialize menu bar");
	menu::GetMenuBar("main", this, (wxID_HIGHEST + 1) + 10000, context.get());

	StartupLog("Create status bar");
	CreateStatusBar(2);

	StartupLog("Set icon");
#ifdef _WIN32
	SetIcon(wxICON(wxicon));
#else
	wxIcon icon;
	icon.CopyFromBitmap(GETIMAGE(wxicon));
	SetIcon(icon);
#endif

	StartupLog("Create views and inner main window controls");
	InitContents();
	if (fluentWorkspace) {
		const auto display = wxGetClientDisplayRect().GetSize();
		const auto preferred = FromDIP(wxSize(1200, 840));
		SetSize(std::min(preferred.GetWidth(), display.GetWidth()),
		        std::min(preferred.GetHeight(), display.GetHeight()));
	}
	OPT_SUB("Video/Detached/Enabled", &FrameMain::OnVideoDetach, this);

	StartupLog("Set up drag/drop target");
	SetDropTarget(new AegisubFileDropTarget(context.get()));

	StartupLog("Load default file");
	context->project->CloseSubtitles();

	StartupLog("Display main window");
	AddFullScreenButton(this);
	Show();
	SetDisplayMode(1, 1);

	StartupLog("Leaving FrameMain constructor");
}

FrameMain::~FrameMain () {
	context->project->CloseAudio();
	context->project->CloseVideo();

	DestroyChildren();
}

void FrameMain::EnableToolBar(agi::OptionValue const& opt) {
	if (opt.GetBool()) {
		if (!GetToolBar()) {
			toolbar::AttachToolbar(this, "main", context.get(), "Default");
			GetToolBar()->Realize();
		}
	}
	else if (wxToolBar *old_tb = GetToolBar()) {
		SetToolBar(nullptr);
		delete old_tb;
		Layout();
	}
}

void FrameMain::InitContents() {
	StartupLog("Create background panel");
	auto Panel = workspacePanel = new wxScrolledWindow(this, wxID_ANY, wxDefaultPosition, wxDefaultSize,
		wxTAB_TRAVERSAL | wxCLIP_CHILDREN | wxHSCROLL | wxVSCROLL);

	StartupLog("Create subtitles grid");
	context->subsGrid = new BaseGrid(Panel, context.get());

	StartupLog("Create video box");
	videoBox = new VideoBox(Panel, false, context.get());

	StartupLog("Create audio box");
	context->audioBox = audioBox = new AudioBox(Panel, context.get());

	StartupLog("Create subtitle editing box");
	editBox = new SubsEditBox(Panel, context.get());
	InitWorkspaceInspector(Panel);

	StartupLog("Arrange main sizers");
	ToolsSizer = new wxBoxSizer(wxVERTICAL);
	ToolsSizer->Add(audioBox, 0, wxEXPAND);
	ToolsSizer->Add(editBox, 1, wxEXPAND);
	TopSizer = new wxBoxSizer(wxHORIZONTAL);
	TopSizer->Add(videoBox, 0, wxEXPAND, 0);
	TopSizer->Add(ToolsSizer, 1, wxEXPAND | wxLEFT | wxRIGHT | wxBOTTOM, 5);
	MainSizer = new wxBoxSizer(wxVERTICAL);
	MainSizer->Add(new wxStaticLine(Panel),0,wxEXPAND | wxALL,0);
	MainSizer->Add(TopSizer,0,wxEXPAND | wxALL,0);
	MainSizer->Add(context->subsGrid,1,wxEXPAND | wxALL,0);
	Panel->SetSizer(MainSizer);
	SetFluentWorkspace(OPT_GET("App/Fluent Workspace")->GetBool());

	StartupLog("Perform layout");
	Layout();
	StartupLog("Leaving InitContents");
}

void FrameMain::InitWorkspaceInspector(wxPanel *parent) {
	const wxColour surface(35, 36, 48), text(244, 242, 250), accent(111, 62, 179);
	auto button = [this, accent, text](wxWindow *owner, wxString const& label, const char *command) {
		auto control = new wxButton(owner, wxID_ANY, label);
		control->SetBackgroundColour(accent);
		control->SetForegroundColour(text);
		control->Bind(wxEVT_UPDATE_UI, [this, command](wxUpdateUIEvent& event) {
			event.Enable(cmd::get(command)->Validate(context.get()));
		});
		control->Bind(wxEVT_BUTTON, [this, command](wxCommandEvent&) {
			auto action = cmd::get(command);
			if (action->Validate(context.get()))
				cmd::call(command, context.get());
		});
		return control;
	};
	playSelection = button(parent, _("Play selected line"), "video/play/line");
	playSelection->Disable();
	playSelection->Hide();

	workspaceInspector = new wxPanel(parent);
	workspaceInspector->SetBackgroundColour(surface);
	workspaceInspector->SetMinSize(wxSize(FromDIP(190), -1));
	auto inspector = new wxBoxSizer(wxVERTICAL);
	auto tabs = new wxNotebook(workspaceInspector, wxID_ANY);
	auto modes = new wxPanel(tabs);
	auto settings = new wxPanel(tabs);
	tabs->AddPage(modes, _("Modes"));
	tabs->AddPage(settings, _("Settings"));
	modes->SetBackgroundColour(surface);
	settings->SetBackgroundColour(surface);
	auto modeSizer = new wxBoxSizer(wxVERTICAL);
	auto choice = new wxChoice(modes, wxID_ANY);
	choice->Append(_("Translation"));
	choice->Append(_("Timing"));
	choice->Append(_("Typesetting"));
	choice->SetSelection(0);
	modeSizer->Add(choice, wxSizerFlags().Expand().Border(wxALL, FromDIP(6)));
	auto book = new wxSimplebook(modes, wxID_ANY);
	auto addPage = [&](std::initializer_list<std::pair<wxString, const char *>> actions) {
		auto page = new wxPanel(book);
		page->SetBackgroundColour(surface);
		auto content = new wxBoxSizer(wxVERTICAL);
		for (auto const& action : actions)
			content->Add(button(page, action.first, action.second), wxSizerFlags().Expand().Border(wxALL, FromDIP(3)));
		page->SetSizer(content);
		book->AddPage(page, wxString());
	};
	addPage({{_("Previous line"), "grid/line/prev"}, {_("Next line"), "grid/line/next"}});
	addPage({{_("Jump to line start"), "video/jump/start"}, {_("Jump to line end"), "video/jump/end"},
	         {_("Timing post-processor"), "tool/time/postprocess"}});
	addPage({{_("Position"), "video/tool/drag"}, {_("Scale"), "video/tool/scale"},
	         {_("Rotate"), "video/tool/rotate/z"}, {_("Clip"), "video/tool/clip"}});
	choice->Bind(wxEVT_CHOICE, [book](wxCommandEvent& event) { book->SetSelection(event.GetSelection()); });
	modeSizer->Add(book, 1, wxEXPAND);
	auto style = new wxButton(modes, wxID_ANY, _("Edit style…"));
	style->SetForegroundColour(text);
	style->SetBackgroundColour(accent);
	style->Bind(wxEVT_BUTTON, [this](wxCommandEvent&) { editBox->OpenStyleEditor(); });
	modeSizer->Add(style, wxSizerFlags().Expand().Border(wxALL, FromDIP(6)));
	modes->SetSizer(modeSizer);

	auto settingsSizer = new wxBoxSizer(wxVERTICAL);
	settingsSizer->Add(button(settings, _("Preferences…"), "app/options"), wxSizerFlags().Expand().Border());
	settingsSizer->Add(button(settings, _("Styles manager…"), "tool/style/manager"), wxSizerFlags().Expand().Border());
	settings->SetSizer(settingsSizer);
	inspector->Add(tabs, 1, wxEXPAND);
	workspaceInspector->SetSizer(inspector);
	workspaceInspector->Hide();
}

void FrameMain::SetFluentWorkspace(bool enabled) {
	if (fluentWorkspace == enabled) return;
	Freeze();
	fluentWorkspace = enabled;
	OPT_SET("App/Fluent Workspace")->SetBool(enabled);
	const int gap = FromDIP(6);
	if (enabled) {
		TopSizer->Detach(videoBox);
		ToolsSizer->Detach(audioBox);
		mediaSizer = new wxBoxSizer(wxVERTICAL);
		mediaSizer->Add(videoBox, 0, wxEXPAND);
		mediaSizer->Add(playSelection, wxSizerFlags().Expand().Border(wxTOP | wxBOTTOM, gap));
		TopSizer->Insert(0, mediaSizer, 0, wxEXPAND | wxALL, gap);
		TopSizer->Add(workspaceInspector, 0, wxEXPAND | wxALL, gap);
		MainSizer->Insert(2, audioBox, 0, wxEXPAND | wxLEFT | wxRIGHT, gap);
	}
	else {
		mediaSizer->Detach(videoBox);
		mediaSizer->Detach(playSelection);
		TopSizer->Remove(mediaSizer);
		mediaSizer = nullptr;
		TopSizer->Detach(workspaceInspector);
		MainSizer->Detach(audioBox);
		TopSizer->Insert(0, videoBox, 0, wxEXPAND);
		ToolsSizer->Insert(0, audioBox, 0, wxEXPAND);
	}
	workspacePanel->SetBackgroundColour(enabled ? wxColour(23, 24, 32) : GetBackgroundColour());
	playSelection->Show(enabled && showVideo && context->project->VideoProvider());
	if (!enabled)
		workspacePanel->Scroll(0, 0);
	workspacePanel->SetScrollRate(enabled ? FromDIP(12) : 0, enabled ? FromDIP(12) : 0);
	MainSizer->SetItemMinSize(context->subsGrid, wxSize(-1, enabled ? FromDIP(96) : -1));
	workspaceInspector->Show(enabled);
	context->subsGrid->SetFluentWorkspace(enabled);
	editBox->SetFluentWorkspace(enabled);
	workspacePanel->Layout();
	Layout();
	Thaw();
}

void FrameMain::SetDisplayMode(int video, int audio) {
	if (!IsShownOnScreen()) return;

	bool sv = false, sa = false;

	if (video == -1) sv = showVideo;
	else if (video)  sv = context->project->VideoProvider() && !context->dialog->Get<DialogDetachedVideo>();

	if (audio == -1) sa = showAudio;
	else if (audio)  sa = !!context->project->AudioProvider();

	// See if anything changed
	if (sv == showVideo && sa == showAudio) return;

	showVideo = sv;
	showAudio = sa;

	bool didFreeze = !IsFrozen();
	if (didFreeze) Freeze();

	context->videoController->Stop();

	videoBox->Show(showVideo);
	audioBox->Show(showAudio);
	playSelection->Show(fluentWorkspace && showVideo);

	workspacePanel->Layout();
	Layout();

	if (didFreeze) Thaw();
}

void FrameMain::UpdateTitle() {
	wxString newTitle;
	if (context->subsController->IsModified()) newTitle << "* ";
	newTitle << context->subsController->Filename().filename().wstring();

#ifndef __WXMAC__
	newTitle << " - Velnosub " << GetAegisubLongVersionString();
#endif

#if defined(__WXMAC__)
	// On Mac, set the mark in the close button
	OSXSetModified(context->subsController->IsModified());
#endif

	if (GetTitle() != newTitle) SetTitle(newTitle);
}

void FrameMain::OnVideoOpen(AsyncVideoProvider *provider) {
	if (!provider) {
		SetDisplayMode(0, -1);
		return;
	}

	Freeze();
	int vidx = provider->GetWidth(), vidy = provider->GetHeight();

	// Set zoom level based on video resolution and window size
	double zoom = context->videoDisplay->GetZoom();
	wxSize windowSize = GetSize();
	if (fluentWorkspace) {
		double fitWidth = windowSize.GetWidth() * .44 / vidx;
		double fitHeight = windowSize.GetHeight() * .48 / vidy;
		if (zoom > fitWidth || zoom > fitHeight)
			context->videoDisplay->SetWindowZoom(std::min(fitWidth, fitHeight));
	}
	else if (vidx*3*zoom > windowSize.GetX()*4 || vidy*4*zoom > windowSize.GetY()*6)
		context->videoDisplay->SetWindowZoom(zoom * .25);
	else if (vidx*3*zoom > windowSize.GetX()*2 || vidy*4*zoom > windowSize.GetY()*3)
		context->videoDisplay->SetWindowZoom(zoom * .5);

	SetDisplayMode(1,-1);

	if (OPT_GET("Video/Detached/Enabled")->GetBool() && !context->dialog->Get<DialogDetachedVideo>())
		cmd::call("video/detach", context.get());
	Thaw();
}

void FrameMain::OnVideoDetach(agi::OptionValue const& opt) {
	if (opt.GetBool())
		SetDisplayMode(0, -1);
	else if (context->project->VideoProvider())
		SetDisplayMode(1, -1);
}

void FrameMain::StatusTimeout(wxString text,int ms) {
	SetStatusText(text,1);
	StatusClear.SetOwner(this, ID_APP_TIMER_STATUSCLEAR);
	StatusClear.Start(ms,true);
}

BEGIN_EVENT_TABLE(FrameMain, wxFrame)
	EVT_TIMER(ID_APP_TIMER_STATUSCLEAR, FrameMain::OnStatusClear)
	EVT_CLOSE(FrameMain::OnCloseWindow)
	EVT_CHAR_HOOK(FrameMain::OnKeyDown)
	EVT_MOUSEWHEEL(FrameMain::OnMouseWheel)
END_EVENT_TABLE()

void FrameMain::OnCloseWindow(wxCloseEvent &event) {
	wxEventBlocker blocker(this, wxEVT_CLOSE_WINDOW);

	context->videoController->Stop();
	context->audioController->Stop();

	// Ask user if he wants to save first
	if (context->subsController->TryToClose(event.CanVeto()) == wxCANCEL) {
		event.Veto();
		return;
	}

	context->dialog.reset();

	// Store maximization state
	OPT_SET("App/Maximized")->SetBool(IsMaximized());

	Destroy();
}

void FrameMain::OnStatusClear(wxTimerEvent &) {
	SetStatusText("",1);
}

void FrameMain::OnAudioOpen(agi::AudioProvider *provider) {
	if (provider)
		SetDisplayMode(-1, 1);
	else
		SetDisplayMode(-1, 0);
}

void FrameMain::OnSubtitlesOpen() {
	context->subsGrid->ResetWorkspaceSource();
	editBox->RefreshWorkspaceSource();
	UpdateTitle();
	SetDisplayMode(1, 1);
}

void FrameMain::OnKeyDown(wxKeyEvent &event) {
	hotkey::check("Main Frame", context.get(), event);
}

void FrameMain::OnMouseWheel(wxMouseEvent &evt) {
	ForwardMouseWheelEvent(this, evt);
}
