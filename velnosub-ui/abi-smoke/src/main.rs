// velnosub Faz 4 (P4.1/P4.2) — C ABI şeridinin host dumanı: abi-smoke crate'i
// eframe'siz koşar (GUI yığını sonraki dilimlere kalır); libvelnosub-abi.so
// üzerinden çekirdeği sürer: init → open → sayaçlar → edit → save → reload.
// P4.2 ekleri: satır metin/zaman okuma + satır düzenleme (grid hazırlığı).
//
// Çalışma: LD_LIBRARY_PATH=<abi dizini> velnosub-abi-smoke in.ass out.ass \
//   ["--edit-first", "metin"]
// Marker şeridi: [AB0] init, [AB1] açıldı, [AB2] sayaçlar, [AB3] düzenlendi,
// [AB4] kaydedildi, [AB5] tur, [AG0..AG5] P4.2 grid şeridi, [ABE] hata.

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
	fn velnosub_abi_line_text(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_line_times(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_edit_line(h: c_int, idx: c_int, text: *const c_char) -> c_int;
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

/// Satır metnini ABI'den oku; kısa tampon gelirse bir büyütüp tekrar dene.
fn line_text(h: c_int, idx: c_int) -> Result<String, String> {
	let mut len = 256usize;
	loop {
		let mut buf = vec![0u8; len];
		let n = unsafe {
			velnosub_abi_line_text(
				h, idx,
				buf.as_mut_ptr() as *mut c_char,
				buf.len() as c_int,
			)
		};
		if n < 0 {
			return Err(last_error());
		}
		if (n as usize) < len - 1 {
			buf.truncate(n as usize);
			return Ok(String::from_utf8_lossy(&buf).into_owned());
		}
		len *= 2;
	}
}

fn line_times(h: c_int, idx: c_int) -> String {
	let mut buf = vec![0u8; 96];
	let n = unsafe {
		velnosub_abi_line_times(
			h, idx,
			buf.as_mut_ptr() as *mut c_char,
			buf.len() as c_int,
		)
	};
	if n < 0 {
		return format!("(hata: {})", last_error());
	}
	let s = unsafe {
		CStr::from_ptr(buf.as_ptr() as *const c_char)
			.to_string_lossy()
			.into_owned()
	};
	s
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

	// P4.2 grid şeridi (okuma yarısı)
	let mut texts: Vec<String> = Vec::with_capacity(ev as usize);
	let mut nonempty = 0usize;
	for i in 0..ev {
		match line_text(h, i) {
			Ok(t) => {
				if !t.trim().is_empty() {
					nonempty += 1;
				}
				texts.push(t);
			}
			Err(e) => {
				eprintln!("[ABE] satır {i}: {e}");
				return ExitCode::FAILURE;
			}
		}
	}
	println!("[AG0] satır metinleri okundu: {} (dolu {})", texts.len(), nonempty);
	if ev >= 5 {
		let t = &texts[4];
		let goster: String = if t.chars().count() > 40 {
			format!("{}…", t.chars().take(40).collect::<String>())
		} else {
			t.clone()
		};
		println!("[AG1] 5. satır: {goster}");
		println!("[AG1] 5. satır zamanları: {}", line_times(h, 4));
	}

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

	// P4.2 grid şeridi (yazım + kalıcılık doğrulaması)
	if ev >= 5 {
		let yeni = "grid duman satır 5";
		let y_c = CString::new(yeni).unwrap();
		let rc = unsafe { velnosub_abi_edit_line(h, 4, y_c.as_ptr()) };
		if rc != 0 {
			eprintln!("[ABE] edit_line: {}", last_error());
			return ExitCode::FAILURE;
		}
		let gerial = match line_text(h, 4) {
			Ok(t) => t,
			Err(e) => {
				eprintln!("[ABE] edit_line geri okuma: {e}");
				return ExitCode::FAILURE;
			}
		};
		if gerial != yeni {
			eprintln!("[ABE] edit_line doğrulaması: {gerial}");
			return ExitCode::FAILURE;
		}
		println!("[AG2] 5. satır düzenlendi + geri okundu");

		let out2_c = CString::new(out_path.clone()).unwrap();
		let rc = unsafe { velnosub_abi_save(h, out2_c.as_ptr()) };
		if rc != 0 {
			eprintln!("[ABE] grid kayıt: {}", last_error());
			return ExitCode::FAILURE;
		}
		let h2 = unsafe { velnosub_abi_open(out2_c.as_ptr()) };
		if h2 < 0 {
			eprintln!("[ABE] ikinci açılış: {}", last_error());
			return ExitCode::FAILURE;
		}
		let kalit = match line_text(h2, 4) {
			Ok(t) => t,
			Err(e) => {
				eprintln!("[ABE] kalıcı okuma: {e}");
				unsafe { velnosub_abi_close(h2) };
				return ExitCode::FAILURE;
			}
		};
		unsafe { velnosub_abi_close(h2) };
		if kalit != yeni {
			eprintln!("[ABE] kalıcılık doğrulaması: {kalit}");
			return ExitCode::FAILURE;
		}
		println!("[AG3] kayıt doğrulandı (5. satır ikinci tutamaçta)");
		println!("[AG4] ikinci tutamaç kapandı");
	}
	println!("[AG5] grid şerit tamam");

	unsafe { velnosub_abi_close(h) };
	ExitCode::SUCCESS
}
