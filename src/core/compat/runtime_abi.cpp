// Velsub Faz 1 (P1.4) — köprülerin headless çekirdek varsayımları.

#include "core/compat/runtime_abi.h"

#include <memory>

namespace agi::compat {

namespace {

class NullProgressSink final : public agi::ProgressSink {
	void SetIndeterminate() override { }
	void SetTitle(std::string const&) override { }
	void SetMessage(std::string const&) override { }
	void SetProgress(int64_t, int64_t) override { }
	void Log(std::string const&) override { }
	void SetStayOpen(bool) override { }
	bool IsCancelled() override { return false; }
};

class NullBackgroundRunner final : public agi::BackgroundRunner {
	NullProgressSink sink;

public:
	void Run(std::function<void(agi::ProgressSink*)> task) override {
		task(&sink);
	}
};

class NullTimer final : public Timer {
public:
	void Start(std::chrono::milliseconds, bool) override { }
	void Stop() override { }
};

NullBackgroundRunner null_runner;
NullTimer null_timer_instance;

}

agi::BackgroundRunner& null_background_runner() {
	return null_runner;
}

bool main_thread_is_current() {
	return true;
}

Timer& null_timer() {
	return null_timer_instance;
}

}
