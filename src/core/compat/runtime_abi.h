// velnosub Faz 1 (P1.4) — yaşam döngüsü / çalışma zamanı köprüleri.
//
// libaegisub/background_runner.h wx'sizdir (ProgressSink std::string
// tabanlıdır); çekirdek no-op koşucusunu ve main-thread yordamını sağlar.
// GUI tarafı DialogProgress ile sürer; masaüstü bağlanması P1.6+'a kalmıştır.
// Timer ABI'si (§1.2) iskelet olarak tanımlıdır — henüz çekirdek tüketici yok
// (audio/video/subs controller'lar ileride katılır), masaüstü wxTimer ile kalır.
#pragma once

#include <libaegisub/background_runner.h>

#include <chrono>
#include <functional>

namespace agi::compat {

/// wx'siz koşucu: Run(fn) fn'i hemen, no-op ProgressSink ile çalıştırır.
agi::BackgroundRunner& null_background_runner();

/// İsteğin GUI ana iş parçacığında olup olmadığı. Çekirdek (tek iş
/// parçacıklı CLI/host) true döner; GUI kolu wxThread::IsMain() ile kalır.
bool main_thread_is_current();

/// §1.2 — `agi::ui::Timer` ABI'si. wxTimer karşılığı; headless çekirdek
/// varsayımı NullTimer'dır (Start no-op). Gerçek zamanlayıcı host'tan
/// sağlanır (Faz 2: Qt timer; masaüstü: wxTimer sarmalayıcı, P1.6+).
class Timer {
public:
	using Callback = std::function<void()>;
	virtual ~Timer() = default;
	virtual void Start(std::chrono::milliseconds interval, bool one_shot = false) = 0;
	virtual void Stop() = 0;
};

/// Headless varsayım zamanlayıcı (hiç ateşlemeyen).
Timer& null_timer();

}
