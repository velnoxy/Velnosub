// velnosub Faz 4 (P4.1) — C ABI şeridinin host dumanı: abi-smoke crate'i
// eframe'siz koşar (GUI yığını sonraki dilimlere kalır); libvelnosub-abi.so
// üzerinden çekirdeği sürer: init → open → sayaçlar → edit → save → reload.
//
// Çalışma: LD_LIBRARY_PATH=<abi dizini> velnosub-abi-smoke in.ass out.ass \
//   ["--edit-first", "metin"]
// Marker şeridi: [AB0] init, [AB1] açıldı, [AB2] sayaçlar, [AB3] düzenlendi,
// [AB4] kaydedildi, [AB5] tur, [ABE] hata.

use std::ffi::{c_char, c_int, CStr, CString};
use std::process::ExitCode;

extern "C" {
	fn velnosub_abi_init(config_json_path: *const c_char) -> c_int;
	fn velnosub_abi_open(in_path: *const c_char) -> c_int;
	fn velnosub_abi_event_count(h: c_int) -> c_int;
	fn velnosub_abi_style_count(h: c_int) -> c_int;
	fn velnosub_abi_edit_first(h: c_int, text: *const c_char) -> c_int;
	fn velnosub_abi_save(h: c_int, out_path: *const c_char) -> c_int;
	fn velnosub_abi_reload_count(h: c_int, out_path: *const c_char) -> c_int;
	fn velnosub_abi_close(h: c_int);
	fn velnosub_abi_last_error() -> *const c_char;
}

fn last_error() -> String {
	unsafe {
		CStr::from_ptr(velnosub_abi_last_error())
			.to_string_lossy()
			.into_owned()
	}
}

fn main() -> ExitCode {
	let mut args = std::env::args().skip(1);
	let Some(in_path) = args.next() else {
		eprintln!("kullanım: velnosub-abi-smoke <giriş> <çıkış> [--edit-first \"metin\"]");
		return ExitCode::from(2);
	};
	let Some(out_path) = args.next() else {
		eprintln!("kullanım: velnosub-abi-smoke <giriş> <çıkış> [--edit-first \"metin\"]");
		return ExitCode::from(2);
	};
	let mut edit_text: Option<String> = None;
	let mut a = args;
	while let Some(arg) = a.next() {
		if arg == "--edit-first" {
			edit_text = Some(a.next().unwrap_or_default());
		}
	}

	let cfg = std::env::var("VELNOSUB_CONFIG_JSON")
		.unwrap_or_else(|_| "default_config.json".into());
	let cfg_c = CString::new(cfg.clone()).unwrap();
	let rc = unsafe { velnosub_abi_init(cfg_c.as_ptr()) };
	if rc != 0 {
		eprintln!("[ABE] init: {rc} ({})", last_error());
		return ExitCode::FAILURE;
	}
	println!("[AB0] init tamam (config: {cfg})");

	let in_c = CString::new(in_path.clone()).unwrap();
	let h = unsafe { velnosub_abi_open(in_c.as_ptr()) };
	if h < 0 {
		eprintln!("[ABE] open: {}", last_error());
		return ExitCode::FAILURE;
	}
	println!("[AB1] açıldı (tutamaç: {h})");

	let ev = unsafe { velnosub_abi_event_count(h) };
	let st = unsafe { velnosub_abi_style_count(h) };
	if ev < 0 || st < 0 {
		eprintln!("[ABE] sayaçlar: {}", last_error());
		return ExitCode::FAILURE;
	}
	println!("[AB2] satır: {ev} stil: {st}");

	if let Some(text) = edit_text {
		let t_c = CString::new(text.clone()).unwrap();
		let rc = unsafe { velnosub_abi_edit_first(h, t_c.as_ptr()) };
		if rc != 0 {
			eprintln!("[ABE] edit: {}", last_error());
			return ExitCode::FAILURE;
		}
		println!("[AB3] düzenlendi: {text}");
	}

	let out_c = CString::new(out_path.clone()).unwrap();
	let rc = unsafe { velnosub_abi_save(h, out_c.as_ptr()) };
	if rc != 0 {
		eprintln!("[ABE] save: {}", last_error());
		return ExitCode::FAILURE;
	}
	println!("[AB4] kaydedildi: {out_path}");

	let tur = unsafe { velnosub_abi_reload_count(h, out_c.as_ptr()) };
	if tur < 0 {
		eprintln!("[ABE] reload: {}", last_error());
		return ExitCode::FAILURE;
	}
	println!("[AB5] tur: {tur} satır");

	unsafe { velnosub_abi_close(h) };
	ExitCode::SUCCESS
}
