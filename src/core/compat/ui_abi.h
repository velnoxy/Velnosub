// Velsub Faz 1 (P1.3) — çekirdeğin UI'ya sorduğu dört sorunun ABI'si.
//
// Çekirdek hedefi (aegisub-core) wx'sizdir: bu ABI'lerin çekirdek
// varsayımları headless'tır (çıktı yoktur, iptal/no-op dönerler).
// Masaüstü GUI bu fonksiyonları kullanmadan mevcut wx çağrılarını
// sürdürür; çekirdek tekrar masaüstüne bağlanırken (P1.6+) orada tek
// tanıma geçilecek. Runtime bağlama (set_single_choice_handler) Faz 2
// (Android/Qt) köprüsü içindir.
#pragma once

#include <string>
#include <vector>

namespace agi::compat {

/// wxGetSingleChoiceIndex karşılığı. Dönüş: seçilen dizin (0..), kullanıcı
/// iptalinde ve headless varsayımda -1.
using ChoiceHandler = int (*)(std::string const& message, std::string const& title,
                              std::vector<std::string> const& choices);

int single_choice(std::string const& message, std::string const& title,
                  std::vector<std::string> const& choices);

/// Handler'ı değiştir; nullptr → headless varsayıma döner.
void set_single_choice_handler(ChoiceHandler handler) noexcept;

/// wxMessageBox karşılığı. buttons: 0 = yalnız OK (P1.3'te tek kullanım).
/// Dönüş: 0 (OK/potansiyel affirm) — headless varsayımda OK kabul edilir.
int confirm(std::string const& message, std::string const& title, int buttons);

/// Dosya seç/kaydet diyaloğu. save=true istek kullanıcıdan bir yol bekler.
/// Dönüş: 0 = yol alındı, 1 = iptal, -1 = headless (yol verilemez).
/// path_out: başarılıysa mutlak yol UTF-8, ayrıca default_ext örk (.srt gibi).
int file_dialog(bool save, std::string const& title, std::string& path_out,
                std::string const& default_ext = std::string());

/// Pickup/attach seçenekleri — çekirdek no-op; false "gerçekten işlenmedi".
bool clipboard_set(std::string const& utf8);
bool clipboard_get(std::string& out);

}
