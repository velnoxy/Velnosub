// velnosub Faz 4 (P4.1/P4.2/P4.3) — C ABI şeridinin host dumanı: abi-smoke
// crate'i eframe'siz koşar (GUI yığını sonraki dilimlere kalır);
// libvelnosub-abi.so üzerinden çekirdeği sürer: init → open → sayaçlar →
// edit → save → reload → grid → video kolu (BGRA kare).
//
// Çalışma: LD_LIBRARY_PATH=<abi dizini> velnosub-abi-smoke in.ass out.ass \
//   ["--edit-first", "metin"] [video.mp4]
// Marker şeridi: [AB0] init, [AB1] açıldı, [AB2] sayaçlar, [AB3] düzenlendi,
// [AB4] kaydedildi, [AB5] tur, [AB7] tekrarlı düzenleme (P5.3 regresyon),
// [AB8] ekleme/silme (P5.2 devam), [AB9] libass önizleme (P4.6 dilim 1),
// [AG0..AG5] P4.2 grid, [AV0..AV4] P4.3 video,
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
	fn velnosub_abi_line_meta(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_line_insert(
		h: c_int, idx: c_int, metin: *const c_char, bas: *const c_char,
		son: *const c_char, stil: *const c_char,
	) -> c_int;
	fn velnosub_abi_line_delete(h: c_int, idx: c_int) -> c_int;
	// P5.2 dilim: ms zaman yazımı (start_time/end_time tamsayı yolu)
	fn velnosub_abi_line_times_set(
		h: c_int, idx: c_int, ms_bas: i64, ms_son: i64,
	) -> c_int;
	// P5.2 dilim C: aegisub.text_extents ölçüm kolu (FreeType)
	fn velnosub_abi_text_extents(
		yol_font: *const c_char, boyut: f64, kalin: c_int, italik: c_int,
		harf_bos: f64, olcek_x: f64, olcek_y: f64,
		metin: *const c_char, out4: *mut f64,
	) -> c_int;
	fn velnosub_abi_render_open(
		h: c_int, gen: c_int, yuk: c_int, yol_font: *const c_char,
		aile_font: *const c_char,
	) -> c_int;
	fn velnosub_abi_render_frame(
		h: c_int, ms: c_int, buf: *mut u8, n: c_int,
	) -> c_int;
	fn velnosub_abi_render_close(h: c_int) -> c_int;
	fn velnosub_abi_edit_line(h: c_int, idx: c_int, text: *const c_char) -> c_int;
	fn velnosub_abi_style_of(h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int) -> c_int;
	fn velnosub_abi_style_edit_size(h: c_int, idx: c_int, size: f64) -> c_int;
	fn velnosub_abi_style_edit_name(h: c_int, idx: c_int, name: *const c_char) -> c_int;
	fn velnosub_abi_style_write(h: c_int, idx: c_int, dizge: *const c_char) -> c_int;
	fn velnosub_abi_dialog_open(spec: *const c_char) -> c_int;
	fn velnosub_abi_dialog_count(h: c_int) -> c_int;
	fn velnosub_abi_dialog_btncount(h: c_int) -> c_int;
	fn velnosub_abi_dialog_ctl(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_dialog_button(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_dialog_set(h: c_int, idx: c_int, deger: *const c_char) -> c_int;
	fn velnosub_abi_dialog_show(h: c_int, dugme: c_int) -> c_int;
	fn velnosub_abi_dialog_showid(h: c_int) -> c_int;
	fn velnosub_abi_dialog_close(h: c_int);
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

/// Dialog alan dizgesini ABI'den oku (aynı kuralla).
fn dialog_ctl(h: c_int, idx: c_int) -> Result<String, String> {
	let mut len = 384usize;
	loop {
		let mut buf = vec![0u8; len];
		let n = unsafe {
			velnosub_abi_dialog_ctl(
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

/// P4.5 III: dialog düğme metnini ABI'den oku.
fn dialog_button(h: c_int, idx: c_int) -> Result<String, String> {
	let mut buf = vec![0u8; 96];
	let n = unsafe {
		velnosub_abi_dialog_button(
			h, idx,
			buf.as_mut_ptr() as *mut c_char,
			buf.len() as c_int,
		)
	};
	if n < 0 {
		return Err(last_error());
	}
	buf.truncate((n as usize).min(95));
	Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Hata çıkışı: kol kapanmadan statik yıkıcı çakışıyor (double-free) —
/// her erken dönüş bu kapıdan geçer.
fn cik(h: c_int) -> ExitCode {
	unsafe { velnosub_abi_close(h) };
	ExitCode::FAILURE
}

/// P4.5 III duman tanımı: paketteki /root/velsub/testmedia/dialog-ornak.txt
/// ile YAŞ-farklı hücre tanımları aynıdır (host+cihaz birebir şeridin anahtarı)
const ORNEK_SPEC: &str = concat!(
	"# velnosub dialog spec\n",
	"ctl|label|l1|Makro denemesi: alanları doldur\n",
	"ctl|edit|metin|Metin|selam\n",
	"ctl|intedit|sayi|Sayı|42|1,100\n",
	"ctl|floatedit|oran|Oran|1.50|0,10\n",
	"ctl|checkbox|durum|Etkin|1\n",
	"ctl|dropdown|bicim|Biçim|B|B;C;A\n",
	"ctl|textbox|not|Not|çok satırlı not\n",
	"buton|Uygula\n",
	"buton|İptal\n",
);

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
		// init başarısız: kapatılacak kol yok; küçük katkı (temiz yıkım)
		eprintln!("[ABE] init: {rc} ({})", last_error());
		return ExitCode::FAILURE;
	}
	println!("[AB0] init tamam (config: {cfg})");

	let in_c = CString::new(in_path.clone()).unwrap();
	let h = unsafe { velnosub_abi_open(in_c.as_ptr()) };
	if h < 0 {
		eprintln!("[ABE] open: {}", last_error());
		return cik(h);
	}
	println!("[AB1] açıldı (tutamaç: {h})");

	let ev = unsafe { velnosub_abi_event_count(h) };
	let st = unsafe { velnosub_abi_style_count(h) };
	if ev < 0 || st < 0 {
		eprintln!("[ABE] sayaçlar: {}", last_error());
		return cik(h);
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
				return cik(h);
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
			return cik(h);
		}
		println!("[AB3] düzenlendi: {text}");
	}

	let out_c = CString::new(out_path.clone()).unwrap();
	let rc = unsafe { velnosub_abi_save(h, out_c.as_ptr()) };
	if rc != 0 {
		eprintln!("[ABE] save: {}", last_error());
		return cik(h);
	}
	println!("[AB4] kaydedildi: {out_path}");

	let tur = unsafe { velnosub_abi_reload_count(h, out_c.as_ptr()) };
	if tur < 0 {
		eprintln!("[ABE] reload: {}", last_error());
		return cik(h);
	}
	println!("[AB5] tur: {tur} satır");

	// P4.2 grid şeridi (yazım + kalıcılık doğrulaması)
	if ev >= 5 {
		let yeni = "grid duman satır 5";
		let y_c = CString::new(yeni).unwrap();
		let rc = unsafe { velnosub_abi_edit_line(h, 4, y_c.as_ptr()) };
		if rc != 0 {
			eprintln!("[ABE] edit_line: {}", last_error());
			return cik(h);
		}
		let gerial = match line_text(h, 4) {
			Ok(t) => t,
			Err(e) => {
				eprintln!("[ABE] edit_line geri okuma: {e}");
				return cik(h);
			}
		};
		if gerial != yeni {
			eprintln!("[ABE] edit_line doğrulaması: {gerial}");
			return cik(h);
		}
		println!("[AG2] 5. satır düzenlendi + geri okundu");

		let out2_c = CString::new(out_path.clone()).unwrap();
		let rc = unsafe { velnosub_abi_save(h, out2_c.as_ptr()) };
		if rc != 0 {
			eprintln!("[ABE] grid kayıt: {}", last_error());
			return cik(h);
		}
		let h2 = unsafe { velnosub_abi_open(out2_c.as_ptr()) };
		if h2 < 0 {
			eprintln!("[ABE] ikinci açılış: {}", last_error());
			return cik(h);
		}
		let kalit = match line_text(h2, 4) {
			Ok(t) => t,
			Err(e) => {
				eprintln!("[ABE] kalıcı okuma: {e}");
				unsafe { velnosub_abi_close(h2) };
				return cik(h);
			}
		};
		unsafe { velnosub_abi_close(h2) };
		if kalit != yeni {
			eprintln!("[ABE] kalıcılık doğrulaması: {kalit}");
			return cik(h);
		}
		println!("[AG3] kayıt doğrulandı (5. satır ikinci tutamaçta)");
		println!("[AG4] ikinci tutamaç kapandı");
	}
	println!("[AG5] grid şerit tamam");

	// P5.3 tekrarlı düzenleme şeridi: flyweight çapraz-DSO yığın bozulması
	// regresyonu (kök neden: fabrika tekili DSO'ya göre bölünüyor; mutasyonlar
	// core.so vekili velnosub_core_edit_* ile tek kolde toplanır). Aynı
	// tutamaçta çok satırı çok turda yazıp her turda geri okur.
	{
		let mut hatasiz = true;
		for tur in 1..=3 {
			for idx in 1..=4 {
				let yeni = format!("ab7 tur{tur} satır{idx}");
				let y_c = CString::new(yeni.clone()).unwrap();
				let rc = unsafe { velnosub_abi_edit_line(h, idx, y_c.as_ptr()) };
				if rc != 0 {
					eprintln!("[ABE] ab7 edit r{tur} i{idx}: {}", last_error());
					hatasiz = false;
					break;
				}
				let gerial = match line_text(h, idx) {
					Ok(t) => t,
					Err(e) => {
						eprintln!("[ABE] ab7 okuma r{tur} i{idx}: {e}");
						hatasiz = false;
						break;
					}
				};
				if gerial != yeni {
					eprintln!("[ABE] ab7 doğrulama r{tur} i{idx}: {gerial}");
					hatasiz = false;
					break;
				}
			}
			if !hatasiz {
				break;
			}
		}
		if !hatasiz {
			return cik(h);
		}
		println!("[AB7] tekrarlı düzenleme: 3 tur x 4 satır birebir");
	}

	// P5.2 devam: ekleme/silme şeridi — aegisub macro pump'ının
	// insert_line/delete_line ABI'sı (flyweight mutasyonları core.so
	// vekilinde; bu da aynı çapraz-DSO kaygısını karşılıyor)
	{
		let ev0 = unsafe { velnosub_abi_event_count(h) };
		if ev0 < 1 {
			eprintln!("[ABE] ab8 sayaç: {ev0}");
			return cik(h);
		}
		let m_c = CString::new("ab8 ekleme domatesi").unwrap();
		let b_c = CString::new("0:00:01.00").unwrap();
		let s_c = CString::new("0:00:02.00").unwrap();
		let st_c = CString::new("Default").unwrap();
		let rc = unsafe {
			velnosub_abi_line_insert(h, 1, m_c.as_ptr(), b_c.as_ptr(), s_c.as_ptr(), st_c.as_ptr())
		};
		if rc != 0 {
			eprintln!("[ABE] ab8 insert: {}", last_error());
			return cik(h);
		}
		let ev1 = unsafe { velnosub_abi_event_count(h) };
		let ilk = match line_text(h, 1) {
			Ok(t) => t,
			Err(e) => {
				eprintln!("[ABE] ab8 okuma: {e}");
				return cik(h);
			}
		};
		let zaman = line_times(h, 1);
		if ev1 != ev0 + 1 || ilk != "ab8 ekleme domatesi"
			|| zaman != "0:00:01.00 / 0:00:02.00"
		{
			eprintln!("[ABE] ab8 doğrulama: {ev0}→{ev1} metin '{ilk}' zaman '{zaman}'");
			return cik(h);
		}
		println!("[AB8] eklendi: {}→{} satır, içerik+birebir zaman", ev0, ev1);
		let rc = unsafe { velnosub_abi_line_delete(h, 1) };
		if rc != 0 {
			eprintln!("[ABE] ab8 delete: {}", last_error());
			return cik(h);
		}
		let ev2 = unsafe { velnosub_abi_event_count(h) };
		if ev2 != ev0 {
			eprintln!("[ABE] ab8 silme doğrulama: {ev1}→{ev2}");
			return cik(h);
		}
		println!("[AB8] silindi: {ev1}→{ev2} satır — şerit tamam");
	}

	// P4.6 dilim 1: libass önizleme şeridi — render_open/frame/close;
	// şılırlı kare: erken ms'de glyph etki > 0, uzak geleçte 0; deposit
	// boyu reddi katı (tip reddi kanca)
	{
		let yol_font = std::env::var("VELNOSUB_FONT")
			.unwrap_or_else(|_| "./DejaVuSans.ttf".into());
		let y_c = CString::new(yol_font.clone()).unwrap();
		let a_c = CString::new("Default").unwrap();
		let rc = unsafe {
			velnosub_abi_render_open(h, 240, 180, y_c.as_ptr(), a_c.as_ptr())
		};
		if rc != 0 {
			eprintln!("[ABE] ab9 render_open: {}", last_error());
			return cik(h);
		}
		println!("[AB9] render açıldı: 240x180 ({yol_font})");
		let mut kare = vec![0u8; 240 * 180 * 4];
		let mut en_cok = 0i64;
		for ms in [0i32, 8000, 16000, 24000, 32000] {
			let gl = unsafe {
				velnosub_abi_render_frame(h, ms, kare.as_mut_ptr(), kare.len() as c_int)
			};
			if gl < 0 {
				eprintln!("[ABE] ab9 frame({ms}): {}", last_error());
				return cik(h);
			}
			if gl as i64 > en_cok {
				en_cok = gl as i64;
			}
		}
		if en_cok <= 0 {
			eprintln!("[ABE] ab9 glyph etki 0 — font eşleşmesi kontrol edin");
			return cik(h);
		}
		println!("[AB9] erken kare render: en çok glyph etki {en_cok}");
		let uzak = unsafe {
			velnosub_abi_render_frame(h, 3_600_000, kare.as_mut_ptr(), kare.len() as c_int)
		};
		if uzak != 0 {
			eprintln!("[ABE] ab9 uzak kare 0 beklenirdi: {uzak}");
			return cik(h);
		}
		println!("[AB9] uzak gele empty kare 0 — boş kare doğru");
		let rc = unsafe { velnosub_abi_render_frame(h, 0, kare.as_mut_ptr(), 3) };
		if rc == 0 {
			eprintln!("[ABE] ab9 deposit boyu reddi beklenirdi");
			return cik(h);
		}
		println!("[AB9] deposit boyu reddedildi — tip kanca birebir");
		let rc = unsafe { velnosub_abi_render_close(h) };
		if rc != 0 {
			eprintln!("[ABE] ab9 render_close: {}", last_error());
			return cik(h);
		}
		println!("[AB9] render şartı kapandı — şerit tamam");
	}

	// P5.2 dilim: ms zaman yazımı/okuma geri döngüsü — upstream auto4
	// satır alanları ms tamsayı; ABI line_times_set + ASS dizge teyit
	{
		let eski = line_times(h, 0);
		if eski.starts_with("(hata") {
			eprintln!("[ABE] ab10 eski zaman: {eski}");
			return cik(h);
		}
		let rc = unsafe { velnosub_abi_line_times_set(h, 0, 1234, 5678) };
		if rc != 0 {
			eprintln!("[ABE] ab10 set: {}", last_error());
			return cik(h);
		}
		let taze = line_times(h, 0);
		if taze != "0:00:01.23 / 0:00:05.68" {
			eprintln!("[ABE] ab10 taze zaman beklenmedik: {taze}");
			return cik(h);
		}
		println!("[AB10] zaman yazıldı: {taze}");
		let adet = unsafe { velnosub_abi_event_count(h) };
		let rc = unsafe { velnosub_abi_line_times_set(h, adet, 0, 1000) };
		if rc == 0 {
			eprintln!("[ABE] ab10 aralık dışı indeks beklenirdi");
			return cik(h);
		}
		println!("[AB10] dışarıda indeks reddi birebir");
		// eski hâle dön (sonraki şeritler orijinal durumu görsün)
		let eski_ms = |z: &str| -> Result<i64, String> {
			let (neg, govde) = match z.strip_prefix('-') {
				Some(g) => (true, g),
				None => (false, z),
			};
			let parca: Vec<&str> = govde.split(':').collect();
			if parca.len() != 3 {
				return Err(format!("zaman biçimi: {z}"));
			}
			let h0: i64 = parca[0].parse().map_err(|_| format!("saat: {z}"))?;
			let m0: i64 = parca[1].parse().map_err(|_| format!("dakika: {z}"))?;
			let kala: Vec<&str> = parca[2].split('.').collect();
			let s0: i64 = kala[0].parse().map_err(|_| format!("saniye: {z}"))?;
			let cc: i64 = kala
				.get(1)
				.and_then(|c| c.get(0..2).and_then(|d| d.parse().ok()))
				.unwrap_or(0);
			let ms = (h0 * 3600 + m0 * 60 + s0) * 1000 + cc * 10;
			Ok(if neg { -ms } else { ms })
		};
		let yarilar: Vec<&str> = eski.split(" / ").collect();
		let ms_bas = match eski_ms(yarilar[0]) {
			Ok(m) => m,
			Err(e) => {
				eprintln!("[ABE] ab10 {e}");
				return cik(h);
			}
		};
		let ms_son = match yarilar.get(1).and_then(|y| eski_ms(y).ok()) {
			Some(m) => m,
			None => ms_bas,
		};
		let rc = unsafe {
			velnosub_abi_line_times_set(h, 0, ms_bas, ms_son)
		};
		if rc != 0 {
			eprintln!("[ABE] ab10 geri al: {}", last_error());
			return cik(h);
		}
		let geri = line_times(h, 0);
		if geri != eski {
			eprintln!("[ABE] ab10 geri al uyuşmuyor: {geri} != {eski}");
			return cik(h);
		}
		println!("[AB10] zaman geri alındı — şerit tamam");
	}

	// P5.2 dilim C: aegisub.text_extents ölçüm kolu — FreeType 26.6;
	// kanıt: boyut doğrusallığı (2x ≈ %2 kusurla), aralık büyümesi,
	// boşluk genişlemesi + hata yolları ([AB11])
	{
		let yol_font = std::env::var("VELNOSUB_FONT")
			.unwrap_or_else(|_| "./DejaVuSans.ttf".into());
		let y_c = CString::new(yol_font.clone()).unwrap();
		let mut out = [0.0f64; 4];
		let metin = CString::new("HAHA VelnoSub").unwrap();
		let rc = unsafe {
			velnosub_abi_text_extents(
				y_c.as_ptr(), 28.0, 0, 0, 0.0, 100.0, 100.0,
				metin.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc != 0 {
			eprintln!("[ABE] ab11 ölçüm: {}", last_error());
			return cik(h);
		}
		let (g0, y0, d0, l0) = (out[0], out[1], out[2], out[3]);
		if !(g0 > 150.0 && g0 < 300.0 && y0 > 15.0 && y0 < 33.0) {
			eprintln!(
				"[ABE] ab11 genişlik/yükseklik beklenmedik: {:.3}/{:.3}",
				g0, y0
			);
			return cik(h);
		}
		println!(
			"[AB11] ölçüm 28px: g={:.3} y={:.3} d={:.3} l={:.3}",
			g0, y0, d0, l0
		);
		// 2x boyut → DejaVu hint'li ilerletmelerle BİREBİR 2x (probe 440)
		let rc = unsafe {
			velnosub_abi_text_extents(
				y_c.as_ptr(), 56.0, 0, 0, 0.0, 100.0, 100.0,
				metin.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc != 0 {
			eprintln!("[ABE] ab11 2x ölçüm: {}", last_error());
			return cik(h);
		}
		let oran = out[0] / g0;
		if !(oran > 1.995 && oran < 2.005) {
			eprintln!("[ABE] ab11 doğrusallık: oran {:.4}", oran);
			return cik(h);
		}
		println!("[AB11] 56px ölçüm birebir doğrusal (oran {:.4})", oran);
		// kalın: sentetik %4 ilerletme → genişlik artar
		let rc = unsafe {
			velnosub_abi_text_extents(
				y_c.as_ptr(), 28.0, 1, 0, 0.0, 100.0, 100.0,
				metin.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc != 0 {
			eprintln!("[ABE] ab11 kalin ölçüm: {}", last_error());
			return cik(h);
		}
		if !(out[0] > g0 + 8.0 && out[0] < g0 + 30.0) {
			eprintln!(
				"[ABE] ab11 kalin genişlik: {:.3} (taban {:.3})",
				out[0], g0
			);
			return cik(h);
		}
		println!(
			"[AB11] kalin: g {:.3} → {:.3} (sentetik %4 artış)",
			g0, out[0]
		);
		// italik: sentetik 0.2 eğme — ilerletme değişmez; rc yolu kanıt
		let rc = unsafe {
			velnosub_abi_text_extents(
				y_c.as_ptr(), 28.0, 0, 1, 0.0, 100.0, 100.0,
				metin.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc != 0 || out[0] <= 0.0 {
			eprintln!("[ABE] ab11 italik ölçüm: {}", last_error());
			return cik(h);
		}
		println!(
			"[AB11] italik ölçüm: g={:.3} y={:.3} (eğme ilerletmeyi değiştirmez)",
			out[0], out[1]
		);
		// scale_x %150 → genişlik 1.5x
		let rc = unsafe {
			velnosub_abi_text_extents(
				y_c.as_ptr(), 28.0, 0, 0, 0.0, 150.0, 100.0,
				metin.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc != 0 || !(out[0] > g0 * 1.49 && out[0] < g0 * 1.51) {
			eprintln!(
				"[ABE] ab11 scale_x: {:.3} (beklenen ~{:.3})",
				out[0],
				g0 * 1.5
			);
			return cik(h);
		}
		println!(
			"[AB11] scale_x %%150: g {:.3} → {:.3} (1.5x birebir)",
			g0, out[0]
		);
		// harf aralığı: spacing 4 → genişlik ≥ (% ölçekle artar)
		let rc = unsafe {
			velnosub_abi_text_extents(
				y_c.as_ptr(), 28.0, 0, 0, 4.0, 100.0, 100.0,
				metin.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc != 0 {
			eprintln!("[ABE] ab11 boşluk ölçüm: {}", last_error());
			return cik(h);
		}
		if out[0] <= g0 {
			eprintln!(
				"[ABE] ab11 spacing: {:.3} <= {:.3}",
				out[0], g0
			);
			return cik(h);
		}
		println!(
			"[AB11] spacing 4px: g {:.3} → {:.3} (artış tamam)",
			g0, out[0]
		);
		// hata yolları: boş metin → rc 0 + sıfır küme (upstream davranışı);
		// kötü font yolu → rc < 0
		let boş = CString::new("").unwrap();
		let rc = unsafe {
			velnosub_abi_text_extents(
				y_c.as_ptr(), 28.0, 0, 0, 0.0, 100.0, 100.0,
				boş.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc != 0 || out.iter().any(|v| *v != 0.0) {
			eprintln!(
				"[ABE] ab11 boş metin: rc={rc} out={:?}",
				&out[..]
			);
			return cik(h);
		}
		println!("[AB11] boş metin → sıfır küme — upstream davranışı");
		let kötü = CString::new("/tmp/opencode/yok-boyle-font.ttf")
			.unwrap();
		let rc = unsafe {
			velnosub_abi_text_extents(
				kötü.as_ptr(), 28.0, 0, 0, 0.0, 100.0, 100.0,
				metin.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc == 0 || out.iter().any(|v| *v != 0.0) {
			eprintln!("[ABE] ab11 kötü font yolu beklenirdi");
			return cik(h);
		}
		println!("[AB11] kötü font yolu red + sıfır küme — tip kanca birebir");
		let rc = unsafe {
			velnosub_abi_text_extents(
				y_c.as_ptr(), 600.0, 0, 0, 0.0, 100.0, 100.0,
				metin.as_ptr(), out.as_mut_ptr(),
			)
		};
		if rc == 0 {
			eprintln!("[ABE] ab11 boyut aralık reddi beklenirdi");
			return cik(h);
		}
		println!("[AB11] boyut aralık dışı red — şerit tamam");
	}

	// P4.5 stil şeridi: stil dizgesi + ad/boyut düzenleme + kalıcılık
	{
		let st = unsafe { velnosub_abi_style_count(h) };
		if st < 0 {
			eprintln!("[ABE] stil sayısı: {}", last_error());
			return cik(h);
		}
		println!("[AS0] stiller: {st}");
		for i in 0..st.min(2) {
			let t = match style_of(h, i) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] stil {i}: {e}");
					return cik(h);
				}
			};
			println!("[AS1] stil {i}: {t}");
		}
		if st >= 1 {
			let rc = unsafe { velnosub_abi_style_edit_size(h, 0, 24.0) };
			if rc != 0 {
				eprintln!("[ABE] stil boyutu: {}", last_error());
				return cik(h);
			}
			let ince = match style_of(h, 0) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] stil teyit: {e}");
					return cik(h);
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
				return cik(h);
			}
			println!("[AS3] stil 0 adı yazıldı (VelnoSub)");

			// tam-yazma turu: font + ana renk değişikliği (13 hücre geri döngüsü)
			let ince = match style_of(h, 0) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] tam-yazma okuma: {e}");
					return cik(h);
				}
			};
			let mut hucre: Vec<String> = ince.split('|').map(|s| s.to_string()).collect();
			if hucre.len() != 13 {
				eprintln!("[ABE] dizge hücre sayısı: {}", hucre.len());
				return cik(h);
			}
			hucre[1] = "DejaVu Sans".into();
			hucre[3] = "&H0000FFFF".into(); // sarı (BBGGRR; dizge kapanacak &siz döner)
			let yazi = hucre.join("|");
			let y_c = CString::new(yazi).unwrap();
			let rc = unsafe { velnosub_abi_style_write(h, 0, y_c.as_ptr()) };
			if rc != 0 {
				eprintln!("[ABE] tam-yazma: {}", last_error());
				return cik(h);
			}
			let dogru = match style_of(h, 0) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] tam-yazma teyit: {e}");
					return cik(h);
				}
			};
			let g: Vec<&str> = dogru.split('|').collect();
			if g.get(1) != Some(&"DejaVu Sans") || g.get(3) != Some(&"&H0000FFFF") {
				eprintln!("[ABE] tam-yazma doğrulaması: {dogru}");
				return cik(h);
			}
			println!("[AS4] tam-yazma: font {} + ana renk {} teyitli", g[1], g[3]);

			let kay_c = CString::new(out_path.clone()).unwrap();
			let rc = unsafe { velnosub_abi_save(h, kay_c.as_ptr()) };
			if rc != 0 {
				eprintln!("[ABE] stil kayıt: {}", last_error());
				return cik(h);
			}
			let h2 = unsafe { velnosub_abi_open(kay_c.as_ptr()) };
			if h2 < 0 {
				eprintln!("[ABE] stil ikinci açılış: {}", last_error());
				return cik(h);
			}
			let kalit = style_of(h2, 0);
			unsafe { velnosub_abi_close(h2) };
			let kalit = match kalit {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] kalıcı stil: {e}");
					return cik(h);
				}
			};
			let es: Vec<&str> = kalit.split('|').collect();
			if es.first() != Some(&"VelnoSub") || es.get(2) != Some(&"24.000000")
				|| es.get(1) != Some(&"DejaVu Sans") || es.get(3) != Some(&"&H0000FFFF")
			{
				eprintln!("[ABE] stil kalıcılığı: {kalit}");
				return cik(h);
			}
			println!("[AS5] kalıcılık teyidi: ad+boyut+font+renk ikinci tutamaçta birebir");
		}
		println!("[AS6] stil şerit tamam");
	}

	// P4.5 III: dialog form omurgası — alan ayrıştırma, tipli yazım, düğme
	{
		let s_c = CString::new(ORNEK_SPEC.to_string()).unwrap();
		let dh = unsafe { velnosub_abi_dialog_open(s_c.as_ptr()) };
		if dh < 0 {
			eprintln!("[ABE] dialog open: {}", last_error());
			return cik(h);
		}
		let alan = unsafe { velnosub_abi_dialog_count(dh) };
		let dugme = unsafe { velnosub_abi_dialog_btncount(dh) };
		if alan < 0 || dugme < 0 {
			eprintln!("[ABE] dialog sayım: {}", last_error());
			unsafe { velnosub_abi_dialog_close(dh) };
			return cik(h);
		}
		println!("[AD0] dialog açıldı (alan {alan}, düğme {dugme})");
		for i in 0..alan {
			match dialog_ctl(dh, i) {
				Ok(t) => println!("[AD1] ctl {i}: {t}"),
				Err(e) => {
					eprintln!("[ABE] ctl {i}: {e}");
					unsafe { velnosub_abi_dialog_close(dh) };
					return cik(h);
				}
			}
		}
		// tipli yazım: metin=1, sayi=2, oran=3, durum=4, bicim=5
		let istekler: [(c_int, &str); 5] = [
			(1, "duman metni"),
			(2, "89"),
			(3, "2.25"),
			(4, "0"),
			(5, "C"),
		];
		for (i, v) in istekler {
			let c = CString::new(v).unwrap();
			let rc = unsafe { velnosub_abi_dialog_set(dh, i, c.as_ptr()) };
			if rc != 0 {
				eprintln!("[ABE] dialog set {i}: {}", last_error());
				unsafe { velnosub_abi_dialog_close(dh) };
				return cik(h);
			}
		}
		println!("[AD2] 5 alan yazıldı (sayı/sürü/anahtar)");
		let mut tamam = true;
		for (i, v) in istekler {
			let t = match dialog_ctl(dh, i) {
				Ok(t) => t,
				Err(e) => {
					eprintln!("[ABE] dialog teyit {i}: {e}");
					unsafe { velnosub_abi_dialog_close(dh) };
					return cik(h);
				}
			};
			let hucre: Vec<&str> = t.split('|').collect();
			if hucre.get(3) != Some(&v) {
				eprintln!("[ABE] dialog teyidi {i}: {t}");
				tamam = false;
				break;
			}
		}
		if !tamam {
			unsafe { velnosub_abi_dialog_close(dh) };
			return cik(h);
		}
		println!("[AD3] teyit: beş alan ikinci okumada birebir");
		// düğme seçimi + kayıt
		let secim = unsafe { velnosub_abi_dialog_show(dh, 1) };
		let kayit = unsafe { velnosub_abi_dialog_showid(dh) };
		if secim != 1 || kayit != 1 {
			eprintln!("[ABE] dialog düğme: secim {secim} kayit {kayit}");
			unsafe { velnosub_abi_dialog_close(dh) };
			return cik(h);
		}
		match dialog_button(dh, 1) {
			Ok(ad) => println!("[AD4] düğme 1 seçildi ({ad}, kayıt tutarlı)"),
			Err(e) => {
				eprintln!("[ABE] dialog düğme metni: {e}");
				unsafe { velnosub_abi_dialog_close(dh) };
				return cik(h);
			}
		}
		// tip reddi: intedit'e harf, dropdown'a öğe dışı
		let bad1 = CString::new("abc").unwrap();
		let red1 = unsafe { velnosub_abi_dialog_set(dh, 2, bad1.as_ptr()) };
		let bad2 = CString::new("Q").unwrap();
		let red2 = unsafe { velnosub_abi_dialog_set(dh, 5, bad2.as_ptr()) };
		if red1 == 0 || red2 == 0 {
			eprintln!("[ABE] tip reddi kalkmadı: {red1} {red2}");
			unsafe { velnosub_abi_dialog_close(dh) };
			return cik(h);
		}
		println!("[AD5] tip reddi: harf sayıya, öğe dışı anahtar reddedildi ({red1},{red2})");
		unsafe { velnosub_abi_dialog_close(dh) };
		println!("[AD6] dialog kol kapandı — şerit tamam");
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
				return cik(h);
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
					return cik(h);
				}
			};
			let iki_kare = match video_kare(vh, 1) {
				Ok(v) => v,
				Err(e) => {
					eprintln!("[ABE] kare 1: {e}");
					return cik(h);
				}
			};
			if bir_kare == iki_kare {
				eprintln!("[ABE] kare 0 ve 1 aynı — kodlamada hareket bekleniyordu");
				return cik(h);
			}
			println!("[AV2] kare 0 ({B} bayt) ve kare 1 farklı: BGRA akışı canlı", B = bir_kare.len());

			let geri = match video_kare(vh, 0) {
				Ok(v) => v,
				Err(e) => {
					eprintln!("[ABE] geri sarma karesi: {e}");
					return cik(h);
				}
			};
			if geri != bir_kare {
				eprintln!("[ABE] geri sarma: kare 0 tekrar çözümü eşleşmiyor");
				return cik(h);
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
				return cik(h);
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
