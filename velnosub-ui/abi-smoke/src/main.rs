// velnosub Faz 4 (P4.1/P4.2/P4.3) — C ABI şeridinin host dumanı: abi-smoke
// crate'i eframe'siz koşar (GUI yığını sonraki dilimlere kalır);
// libvelnosub-abi.so üzerinden çekirdeği sürer: init → open → sayaçlar →
// edit → save → reload → grid → video kolu (BGRA kare).
//
// Çalışma: LD_LIBRARY_PATH=<abi dizini> velnosub-abi-smoke in.ass out.ass \
//   ["--edit-first", "metin"] [video.mp4]
// Marker şeridi: [AB0] init, [AB1] açıldı, [AB2] sayaçlar, [AB3] düzenlendi,
// [AB4] kaydedildi, [AB5] tur, [AG0..AG5] P4.2 grid, [AV0..AV4] P4.3 video,
// [ABE] hata.

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
	fn velnosub_abi_style_of(h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int) -> c_int;
	fn velnosub_abi_style_edit_size(h: c_int, idx: c_int, size: f64) -> c_int;
	fn velnosub_abi_style_edit_name(h: c_int, idx: c_int, name: *const c_char) -> c_int;
	fn velnosub_abi_style_write(h: c_int, idx: c_int, dizge: *const c_char) -> c_int;
	fn velnosub_abi_close(h: c_int);
	fn velnosub_abi_last_error() -> *const c_char;
	fn velnosub_abi_video_open(path: *const c_char) -> c_int;
	fn velnosub_abi_video_info(
		h: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_video_frame(
		h: c_int, kare: c_int, buf: *mut u8, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_video_close(h: c_int);
	fn velnosub_abi_audio_open(
		path: *const c_char, rate: c_int, ch: c_int,
	) -> c_int;
	fn velnosub_abi_audio_len64(h: c_int) -> i64;
	fn velnosub_abi_audio_info(
		h: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_audio_peaks(
		h: c_int, d0: i64, adet: i64, kovalar: c_int,
		minv: *mut i16, maxv: *mut i16,
	) -> c_int;
	fn velnosub_abi_audio_play(h: c_int, d0: i64, adet: i64) -> c_int;
	fn velnosub_abi_audio_state(h: c_int) -> c_int;
	fn velnosub_abi_audio_stop(h: c_int) -> c_int;
	fn velnosub_abi_audio_close(h: c_int);
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

/// Stil alan dizgesini ABI'den oku (line_text okuma kuralının aynısı).
fn style_of(h: c_int, idx: c_int) -> Result<String, String> {
	let mut len = 384usize;
	loop {
		let mut buf = vec![0u8; len];
		let n = unsafe {
			velnosub_abi_style_of(
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

/// P4.3: veri karesini ABI'den BGRA olarak çek (tampon-büyütmeli).
fn video_kare(vh: c_int, idx: c_int) -> Result<Vec<u8>, String> {
	let mut len: c_int = 1 << 20;
	loop {
		let mut buf = vec![0u8; len as usize];
		let n = unsafe {
			velnosub_abi_video_frame(
				vh, idx,
				buf.as_mut_ptr(),
				buf.len() as c_int,
			)
		};
		if n < 0 {
			return Err(last_error());
		}
		if (n as usize) <= buf.len() {
			buf.truncate(n as usize);
			return Ok(buf);
		}
		if n > len {
			len = n;
		}
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
	let mut video_path: Option<String> = None;
	let mut a = args;
	while let Some(arg) = a.next() {
		if arg == "--edit-first" {
			edit_text = Some(a.next().unwrap_or_default());
		} else {
			video_path = Some(arg);
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

	// P4.5 stil şeridi: stil dizgesi + ad/boyut düzenleme + kalıcılık
	{
		let st = unsafe { velnosub_abi_style_count(h) };
		if st < 0 {
			eprintln!("[ABE] stil sayısı: {}", last_error());
			return ExitCode::FAILURE;
		}
		println!("[AS0] stiller: {st}");
		for i in 0..st.min(2) {
			let t = match style_of(h, i) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] stil {i}: {e}");
					return ExitCode::FAILURE;
				}
			};
			println!("[AS1] stil {i}: {t}");
		}
		if st >= 1 {
			let rc = unsafe { velnosub_abi_style_edit_size(h, 0, 24.0) };
			if rc != 0 {
				eprintln!("[ABE] stil boyutu: {}", last_error());
				return ExitCode::FAILURE;
			}
			let ince = match style_of(h, 0) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] stil teyit: {e}");
					return ExitCode::FAILURE;
				}
			};
			let parca: Vec<&str> = ince.split('|').collect();
			let boyut = parca.get(2).copied().unwrap_or("?");
			println!("[AS2] stil 0 boyutu 24 yazıldı; teyit: {boyut}");
			let ad = "VelnoSub";
			let ad_c = CString::new(ad).unwrap();
			let rc = unsafe { velnosub_abi_style_edit_name(h, 0, ad_c.as_ptr()) };
			if rc != 0 {
				eprintln!("[ABE] stil adı: {}", last_error());
				return ExitCode::FAILURE;
			}
			println!("[AS3] stil 0 adı yazıldı (VelnoSub)");

			// tam-yazma turu: font + ana renk değişikliği (13 hücre geri döngüsü)
			let ince = match style_of(h, 0) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] tam-yazma okuma: {e}");
					return ExitCode::FAILURE;
				}
			};
			let mut hucre: Vec<String> = ince.split('|').map(|s| s.to_string()).collect();
			if hucre.len() != 13 {
				eprintln!("[ABE] dizge hücre sayısı: {}", hucre.len());
				return ExitCode::FAILURE;
			}
			hucre[1] = "DejaVu Sans".into();
			hucre[3] = "&H0000FFFF".into(); // sarı (BBGGRR; dizge kapanacak &siz döner)
			let yazi = hucre.join("|");
			let y_c = CString::new(yazi).unwrap();
			let rc = unsafe { velnosub_abi_style_write(h, 0, y_c.as_ptr()) };
			if rc != 0 {
				eprintln!("[ABE] tam-yazma: {}", last_error());
				return ExitCode::FAILURE;
			}
			let dogru = match style_of(h, 0) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] tam-yazma teyit: {e}");
					return ExitCode::FAILURE;
				}
			};
			let g: Vec<&str> = dogru.split('|').collect();
			if g.get(1) != Some(&"DejaVu Sans") || g.get(3) != Some(&"&H0000FFFF") {
				eprintln!("[ABE] tam-yazma doğrulaması: {dogru}");
				return ExitCode::FAILURE;
			}
			println!("[AS4] tam-yazma: font {} + ana renk {} teyitli", g[1], g[3]);

			let kay_c = CString::new(out_path.clone()).unwrap();
			let rc = unsafe { velnosub_abi_save(h, kay_c.as_ptr()) };
			if rc != 0 {
				eprintln!("[ABE] stil kayıt: {}", last_error());
				return ExitCode::FAILURE;
			}
			let h2 = unsafe { velnosub_abi_open(kay_c.as_ptr()) };
			if h2 < 0 {
				eprintln!("[ABE] stil ikinci açılış: {}", last_error());
				return ExitCode::FAILURE;
			}
			let kalit = style_of(h2, 0);
			unsafe { velnosub_abi_close(h2) };
			let kalit = match kalit {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] kalıcı stil: {e}");
					return ExitCode::FAILURE;
				}
			};
			let es: Vec<&str> = kalit.split('|').collect();
			if es.first() != Some(&"VelnoSub") || es.get(2) != Some(&"24.000000")
				|| es.get(1) != Some(&"DejaVu Sans") || es.get(3) != Some(&"&H0000FFFF")
			{
				eprintln!("[ABE] stil kalıcılığı: {kalit}");
				return ExitCode::FAILURE;
			}
			println!("[AS5] kalıcılık teyidi: ad+boyut+font+renk ikinci tutamaçta birebir");
		}
		println!("[AS6] stil şerit tamam");
	}

	// P4.3 video şeridi: video argümanı verildiyse BGRA kare hattını sürer
	if let Some(vp) = video_path {
		let vp_c = CString::new(vp.clone()).unwrap();
		let vh = unsafe { velnosub_abi_video_open(vp_c.as_ptr()) };
		if vh < 0 {
			// host'ta FFmpeg sağlayıcı yoksa normal: sadece raporla
			eprintln!("[AVE] video kolu: {} (host'ta masa taban video-EX yok)", last_error());
		} else {
			println!("[AV0] video açıldı (tutamaç: {vh})");

			let mut ibuf = vec![0u8; 128];
			let n = unsafe {
				velnosub_abi_video_info(vh, ibuf.as_mut_ptr() as *mut c_char, ibuf.len() as c_int)
			};
			if n < 0 {
				eprintln!("[ABE] video bilgi: {}", last_error());
				return ExitCode::FAILURE;
			}
			let bilgi = unsafe {
				CStr::from_ptr(ibuf.as_ptr() as *const c_char)
					.to_string_lossy()
					.into_owned()
			};
			println!("[AV1] video bilgi: {bilgi}");

			let bir_kare = match video_kare(vh, 0) {
				Ok(v) => v,
				Err(e) => {
					eprintln!("[ABE] kare 0: {e}");
					return ExitCode::FAILURE;
				}
			};
			let iki_kare = match video_kare(vh, 1) {
				Ok(v) => v,
				Err(e) => {
					eprintln!("[ABE] kare 1: {e}");
					return ExitCode::FAILURE;
				}
			};
			if bir_kare == iki_kare {
				eprintln!("[ABE] kare 0 ve 1 aynı — kodlamada hareket bekleniyordu");
				return ExitCode::FAILURE;
			}
			println!("[AV2] kare 0 ({B} bayt) ve kare 1 farklı: BGRA akışı canlı", B = bir_kare.len());

			let geri = match video_kare(vh, 0) {
				Ok(v) => v,
				Err(e) => {
					eprintln!("[ABE] geri sarma karesi: {e}");
					return ExitCode::FAILURE;
				}
			};
			if geri != bir_kare {
				eprintln!("[ABE] geri sarma: kare 0 tekrar çözümü eşleşmiyor");
				return ExitCode::FAILURE;
			}
			println!("[AV3] geri sarma kanıtlandı (kare 0 birebir)");
			unsafe { velnosub_abi_video_close(vh) };
			println!("[AV4] video kol kapatıldı");
		}
	}

	// P4.4/P3.5 ses şeridi: VELNOSUB_AUDIO (yol), VELNOSUB_RATE/CH env ile
	if let Ok(ap) = std::env::var("VELNOSUB_AUDIO") {
		let rate: c_int = std::env::var("VELNOSUB_RATE").ok()
			.and_then(|v| v.parse().ok())
			.unwrap_or(48000);
		let ch: c_int = std::env::var("VELNOSUB_CH").ok()
			.and_then(|v| v.parse().ok())
			.unwrap_or(2);
		let ap_c = CString::new(ap.clone()).unwrap();
		let ah = unsafe { velnosub_abi_audio_open(ap_c.as_ptr(), rate, ch) };
		if ah < 0 {
			eprintln!("[AVE] ses kolu: {}", last_error());
		} else {
			println!("[AW0] ses açıldı (tutamaç: {ah}, rate {rate}, ch {ch})");
			let toplam = unsafe { velnosub_abi_audio_len64(ah) };
			let mut ibuf = vec![0u8; 128];
			let n = unsafe {
				velnosub_abi_audio_info(ah, ibuf.as_mut_ptr() as *mut c_char, ibuf.len() as c_int)
			};
			let bilgi = unsafe {
				CStr::from_ptr(ibuf.as_ptr() as *const c_char)
					.to_string_lossy()
					.into_owned()
			};
			println!("[AW1] ses bilgi: {bilgi} (n {n})");

			let mut minv = vec![0i16; 64];
			let mut maxv = vec![0i16; 64];
			let rc = unsafe {
				velnosub_abi_audio_peaks(ah, 0, toplam, 64, minv.as_mut_ptr(), maxv.as_mut_ptr())
			};
			if rc != 0 {
				eprintln!("[ABE] peaks: {}", last_error());
				return ExitCode::FAILURE;
			}
			let dolu = (0..64).filter(|&k| maxv[k] != 0 || minv[k] != 0).count();
			let sirali = (0..64).all(|k| minv[k] <= maxv[k]);
			println!("[AW2] peak kovaları: 64 (boş olmayan {dolu}, min<=max {sirali})");

			let oynat = unsafe { velnosub_abi_audio_play(ah, 0, toplam.min(rate as i64)) };
			if oynat != 0 {
				println!("[AVE] oynatma bu kolda yok: {}", last_error());
			} else {
				// hazırlanıyor (4) ve oynatılıyor (0) üzerinden geç; boşalma (1)
				// veya hata (2) ulaştığında ya da ~1,5 sn geçince dur
				let mut son_durum = -1;
				for _ in 0..15 {
					std::thread::sleep(std::time::Duration::from_millis(100));
					son_durum = unsafe { velnosub_abi_audio_state(ah) };
					if son_durum == 1 || son_durum == 2 {
						break;
					}
				}
				println!("[AW3] oynatma durumu: {son_durum} (-1 yok, 4 hazırlanıyor, 0 oynatılıyor, 1 boşalma, 2 hata)");
				unsafe { velnosub_abi_audio_stop(ah) };
				println!("[AW4] oynatma kesildi");
			}
			unsafe { velnosub_abi_audio_close(ah) };
			println!("[AW5] ses kol kapandı — şerit tamam");
		}
	}

	unsafe { velnosub_abi_close(h) };
	ExitCode::SUCCESS
}
