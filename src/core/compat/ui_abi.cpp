// Velsub Faz 1 (P1.3) — compat ABI'nin çekirdek (headless) varsayımları.

#include "core/compat/ui_abi.h"

namespace agi::compat {

namespace {
	ChoiceHandler choice_handler = nullptr;
}

int single_choice(std::string const& message, std::string const& title,
                  std::vector<std::string> const& choices) {
	if (choice_handler)
		return choice_handler(message, title, choices);
	(void)message; (void)title; (void)choices;
	return -1;
}

void set_single_choice_handler(ChoiceHandler handler) noexcept {
	choice_handler = handler;
}

int confirm(std::string const& message, std::string const& title, int buttons) {
	(void)message; (void)title; (void)buttons;
	return 0; // OK
}

int file_dialog(bool save, std::string const& title, std::string& path_out,
                std::string const& default_ext) {
	(void)save; (void)title; (void)default_ext;
	path_out.clear();
	return -1; // headless: yol üretilemez
}

bool clipboard_set(std::string const& utf8) {
	(void)utf8;
	return false;
}

bool clipboard_get(std::string& out) {
	out.clear();
	return false;
}

}
