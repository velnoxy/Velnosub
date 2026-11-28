#include <climits>

extern "C" int velnosub_abi_open_encoding(const char *path, const char *encoding);
extern "C" int velnosub_abi_save(int handle, const char *path);
using CancelMedia = int (*)(const void *);
extern "C" int velnosub_abi_video_open_cancel(const char *, CancelMedia, const void *);
extern "C" int velnosub_abi_audio_file_open_cancel(const char *, CancelMedia, const void *);

// Some exceptions thrown by the core carry a foreign type across the Android
// shared libraries. Keep them on the C++ side of the Rust FFI boundary.
extern "C" int velnosub_ui_open_document(const char *path, const char *encoding) noexcept {
	try {
		return velnosub_abi_open_encoding(path, encoding);
	}
	catch (...) {
		return INT_MIN;
	}
}

extern "C" int velnosub_ui_save_document(int handle, const char *path) noexcept {
	try {
		return velnosub_abi_save(handle, path);
	}
	catch (...) {
		return INT_MIN;
	}
}

extern "C" int velnosub_ui_open_video(const char *path, CancelMedia cancel, const void *state) noexcept {
	try {
		return velnosub_abi_video_open_cancel(path, cancel, state);
	}
	catch (...) {
		return INT_MIN;
	}
}

extern "C" int velnosub_ui_open_audio(const char *path, CancelMedia cancel, const void *state) noexcept {
	try {
		return velnosub_abi_audio_file_open_cancel(path, cancel, state);
	}
	catch (...) {
		return INT_MIN;
	}
}
