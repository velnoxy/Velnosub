//! velnosub Faz 4 (P4.2) — C ABI köprüsü + sanallaştırılmış altyazı grid'i.
//!
//! libvelnosub-abi.so üzerinden çekirdek: init/open/line_text/line_times/
//! edit_line/save/close. Grid görünümü ScrollArea::show_rows ile sadece
//! görünen satırları çizer (egui sanallaştırması); metinler açılışta
//! önceden çekilir (çekim maliyeti ABI başına C-str kopyasıdır).
//! Düzenleme her hücre değişiminde edit_line ile çekirdek Commit hattına
//! girer (P2.2/CLI kanalıyla aynı yol).

use std::ffi::{c_char, c_int, CStr, CString};
use std::sync::OnceLock;

use eframe::egui;
use egui::{CentralPanel, Color32, Panel, RichText, ScrollArea, TextEdit};

use crate::layout::{self, Yerlesim};
use crate::ass_tags;

/// P8.1: gömülü Material Icons kod noktaları (assets/MaterialIcons-Regular.ttf).
/// egui varsayılan yazı tiplerinde bu aralık boştur; font lib.rs'te ailelere
/// son sıradan eklenir, böylece metin glifleri bozulmadan ikonlar çözülür.
mod ikon {
	pub const PLAY: &str = "\u{e037}";
	pub const PAUSE: &str = "\u{e034}";
	pub const STOP: &str = "\u{e047}";
	pub const SKIP_PREV: &str = "\u{e045}";
	pub const SKIP_NEXT: &str = "\u{e044}";
	pub const OPEN: &str = "\u{e2c8}";
	pub const SAVE: &str = "\u{e161}";
	pub const UNDO: &str = "\u{e166}";
	pub const REDO: &str = "\u{e15a}";
	pub const SEARCH: &str = "\u{e8b5}";
	pub const SETTINGS: &str = "\u{e8b8}";
	pub const ADD: &str = "\u{e145}";
	pub const DELETE: &str = "\u{e872}";
	pub const COPY: &str = "\u{e14d}";
	pub const CLOSE: &str = "\u{e5cd}";
	pub const PALETTE: &str = "\u{e40a}";
	pub const AUTOMATION: &str = "\u{e87b}";
	pub const SUBTITLES: &str = "\u{e048}";
	pub const VIDEO: &str = "\u{e04b}";
	pub const AUDIO: &str = "\u{e3a1}";
	pub const INFO: &str = "\u{e88e}";
	pub const TEXT: &str = "\u{e262}";
	pub const BOLD: &str = "\u{e238}";
	pub const ITALIC: &str = "\u{e23f}";
	pub const UNDERLINE: &str = "\u{e249}";
	pub const STRIKE: &str = "\u{e23d}";
	pub const FULL: &str = "\u{e5d0}";
	pub const LAYERS: &str = "\u{e53b}";
	pub const TUNE: &str = "\u{e429}";
	pub const ARROW_BACK: &str = "\u{e5c4}";
	pub const ARROW_FWD: &str = "\u{e5c8}";
}

extern "C" {
	fn velnosub_abi_init(config_json_path: *const c_char) -> c_int;
	fn velnosub_abi_new_document() -> c_int;
	fn velnosub_ui_open_document(in_path: *const c_char) -> c_int;
	fn velnosub_abi_event_count(h: c_int) -> c_int;
	fn velnosub_abi_style_count(h: c_int) -> c_int;
	fn velnosub_abi_line_text(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_line_times(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_line_margins(h: c_int, idx: c_int, left: c_int, right: c_int, vertical: c_int) -> c_int;
	fn velnosub_abi_edit_line(h: c_int, idx: c_int, text: *const c_char) -> c_int;
	fn velnosub_abi_line_meta(h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int) -> c_int;
	fn velnosub_abi_line_meta_set(h: c_int, idx: c_int, stil: *const c_char,
		aktor: *const c_char, etki: *const c_char, yorum: c_int, katman: c_int,
	) -> c_int;
	fn velnosub_abi_line_times_set(h: c_int, idx: c_int, ms_bas: i64, ms_son: i64) -> c_int;
	fn velnosub_abi_snapshot(h: c_int) -> c_int;
	fn velnosub_abi_list_macros(
		shim: *const c_char, script: *const c_char, out: *mut c_char, size: c_int,
	) -> c_int;
	#[cfg(target_os = "android")]
	fn velnosub_abi_list_filters(
		shim: *const c_char, script: *const c_char, out: *mut c_char, size: c_int,
	) -> c_int;
	fn velnosub_abi_macro_start(
		handle: c_int, row: c_int, shim: *const c_char, script: *const c_char,
		name: *const c_char, path: *const c_char, video: c_int,
		token: *mut c_int, result: *mut c_int,
	) -> c_int;
	fn velnosub_abi_macro_continue(token: c_int, button: c_int, result: *mut c_int) -> c_int;
	fn velnosub_abi_macro_abort(token: c_int);
	fn velnosub_abi_run_script(
		h: c_int, row: c_int, shim: *const c_char, script: *const c_char,
		name: *const c_char, path: *const c_char,
		video: c_int, filter: c_int,
	) -> c_int;
	fn velnosub_abi_duplicate_line(h: c_int, idx: c_int) -> c_int;
	fn velnosub_abi_line_insert(h: c_int, idx: c_int, metin: *const c_char,
		bas: *const c_char, son: *const c_char, stil: *const c_char,
	) -> c_int;
	fn velnosub_abi_line_delete(h: c_int, idx: c_int) -> c_int;
	fn velnosub_abi_style_of(h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int) -> c_int;
	fn velnosub_abi_style_edit_size(h: c_int, idx: c_int, size: f64) -> c_int;
	fn velnosub_abi_style_edit_name(h: c_int, idx: c_int, name: *const c_char) -> c_int;
	fn velnosub_abi_style_write(h: c_int, idx: c_int, dizge: *const c_char) -> c_int;
	fn velnosub_abi_style_add(h: c_int, ad: *const c_char) -> c_int;
	fn velnosub_abi_style_delete(h: c_int, idx: c_int) -> c_int;
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
	fn velnosub_ui_save_document(h: c_int, out_path: *const c_char) -> c_int;
	fn velnosub_abi_attach_font(h: c_int, filename: *const c_char) -> c_int;
	fn velnosub_abi_close(h: c_int);
	fn velnosub_abi_last_error() -> *const c_char;
	fn velnosub_ui_open_video(
		path: *const c_char, cancel: extern "C" fn(*const std::ffi::c_void) -> c_int,
		state: *const std::ffi::c_void,
	) -> c_int;
	fn velnosub_abi_video_info(
		h: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_video_geom(
		h: c_int, out_w: *mut c_int, out_h: *mut c_int,
		out_fps: *mut f64, out_kare: *mut c_int,
	) -> c_int;
	fn velnosub_abi_video_frame(
		h: c_int, kare: c_int, buf: *mut u8, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_video_timeline(h: c_int, times: *mut c_int, count: c_int) -> c_int;
	fn velnosub_abi_video_keyframes(h: c_int, frames: *mut c_int, count: c_int) -> c_int;
	fn velnosub_abi_video_close(h: c_int);
	// P4.6 dilim 1 ABI: libass önizleme (C özneli; alfa deposit dilim 2)
	fn velnosub_abi_render_open(
		h: c_int, gen: c_int, yuk: c_int,
		yol_font: *const c_char, aile_font: *const c_char,
	) -> c_int;
	fn velnosub_abi_render_frame(
		h: c_int, ms: c_int, buf: *mut u8, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_render_close(h: c_int) -> c_int;
	// P8: typesetting — ASS koordinat boyu + yalnız seçili satır render'ı
	fn velnosub_abi_script_size(
		h: c_int, out_w: *mut c_int, out_h: *mut c_int,
	) -> c_int;
	fn velnosub_abi_render_line(
		h: c_int, idx: c_int, ms: c_int, buf: *mut u8, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_render_line_metin(
		h: c_int, idx: c_int, metin: *const c_char, ms: c_int,
		buf: *mut u8, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_audio_open(
		path: *const c_char, rate: c_int, ch: c_int,
	) -> c_int;
	fn velnosub_ui_open_audio(
		path: *const c_char, cancel: extern "C" fn(*const std::ffi::c_void) -> c_int,
		cancel_state: *const std::ffi::c_void,
	) -> c_int;
	// P6.4: kutu-içi metin altyazı — akış taraması + seçilen akışı SRT'ye çekme
	fn velnosub_abi_subtitle_examine(path: *const c_char) -> c_int;
	fn velnosub_abi_subtitle_extract(
		path: *const c_char,
		idx: c_int,
		dest: *const c_char,
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
	fn velnosub_abi_audio_position(h: c_int) -> i64;
	#[allow(dead_code)] // Task 6 spektrogram paneli bu ucu kullanacak
	fn velnosub_abi_audio_spectrum(
		h: c_int, d0: i64, adet: i64, cols: c_int, rows: c_int,
		out: *mut u8,
	) -> c_int;
	fn velnosub_abi_audio_stop(h: c_int) -> c_int;
	fn velnosub_abi_audio_close(h: c_int);
}

static INIT_OK: OnceLock<bool> = OnceLock::new();

fn last_error() -> String {
	unsafe {
		CStr::from_ptr(velnosub_abi_last_error())
			.to_string_lossy()
			.into_owned()
	}
}

fn document_io_result(result: c_int, failure: &str) -> Result<c_int, String> {
	match result {
		c_int::MIN => Err(failure.into()),
		0.. => Ok(result),
		_ => Err(last_error()),
	}
}

fn write_document(handle: c_int, path: &CStr) -> Result<(), String> {
	document_io_result(
		unsafe { velnosub_ui_save_document(handle, path.as_ptr()) },
		"Altyazı dosyası yazılamadı",
	)
	.map(|_| ())
}

/// Tampon-büyütmeli okuma şeridi: her okuma ABI'sı (line_text/line_times)
/// aynı kuralı izler — tampon yetmezse tam uzunluğu döner.
fn read_cstr(
	read: impl Fn(c_int, *mut c_char, c_int) -> c_int,
	h: c_int,
) -> Result<String, String> {
	let mut len = 256usize;
	loop {
		let mut buf = vec![0u8; len];
		let n = read(h, buf.as_mut_ptr() as *mut c_char, buf.len() as c_int);
		if n < 0 {
			return Err(last_error());
		}
		if (n as usize) < len.saturating_sub(1) {
			buf.truncate(n as usize);
			return Ok(String::from_utf8_lossy(&buf).into_owned());
		}
		len *= 2;
	}
}

fn line_text(h: c_int, idx: c_int) -> Result<String, String> {
	read_cstr(
		|h, buf, len| unsafe { velnosub_abi_line_text(h, idx, buf, len) },
		h,
	)
}
fn line_times(h: c_int, idx: c_int) -> Result<String, String> {
	read_cstr(
		|h, buf, len| unsafe { velnosub_abi_line_times(h, idx, buf, len) },
		h,
	)
}

/// Satır metnini çekirdeğe yaz (Commit hattı: COMMIT_DIAG_TEXT).
fn edit_line(h: c_int, idx: usize, text: &str) -> Result<(), String> {
	let c = match CString::new(text.to_string()) {
		Ok(c) => c,
		Err(e) => return Err(format!("metin kodlanamadı: {e}")),
	};
	let rc = unsafe { velnosub_abi_edit_line(h, idx as c_int, c.as_ptr()) };
	if rc == 0 {
		Ok(())
	} else {
		Err(last_error())
	}
}

/// Geçici metinle tek satır render'ı (clip/vektör/XY önizlemesi).
fn satir_metin_cek(
	h: c_int,
	idx: usize,
	metin: &str,
	ms: i32,
	buf: &mut [u8],
) -> Result<i32, String> {
	let c = CString::new(metin)
		.map_err(|e| format!("geçici metin kodlanamadı: {e}"))?;
	let n = i32::try_from(buf.len()).map_err(|_| "kare deposu çok büyük")?;
	let rc = unsafe {
		velnosub_abi_render_line_metin(
			h,
			idx as c_int,
			c.as_ptr(),
			ms,
			buf.as_mut_ptr(),
			n,
		)
	};
	if rc < 0 {
		Err(last_error())
	} else {
		Ok(rc)
	}
}

/// P5.4 gece: hat-meta okuyucu — C dizgesi
/// "class|yorum|katman|stil|oyuncu|etki|mL|mR|mV" (ABI line_meta).
fn line_meta_parts(h: c_int, idx: c_int) -> Result<Vec<String>, String> {
	let s = read_cstr(
		|h, buf, len| unsafe { velnosub_abi_line_meta(h, idx, buf, len) },
		h,
	)?;
	Ok(s.split('|').map(|p| p.to_string()).collect())
}

/// P5.4 gece: hat-meta yazısı — None dizge / None sayı = alan DOKUNULMAZ.
fn meta_set(
	h: c_int,
	idx: usize,
	stil: Option<&str>,
	oyuncu: Option<&str>,
	etki: Option<&str>,
	yorum: Option<bool>,
	katman: Option<i32>,
) -> Result<(), String> {
	let c_stil = stil.map(|s| CString::new(s).map_err(|e| e.to_string()))
		.transpose()?;
	let c_oyuncu = oyuncu.map(|s| CString::new(s).map_err(|e| e.to_string()))
		.transpose()?;
	let c_etki = etki.map(|s| CString::new(s).map_err(|e| e.to_string()))
		.transpose()?;
	let rc = unsafe {
		velnosub_abi_line_meta_set(
			h,
			idx as c_int,
			c_stil.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
			c_oyuncu.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
			c_etki.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
			yorum.map_or(-1, |b| if b { 1 } else { 0 }),
			katman.unwrap_or(-1),
		)
	};
	if rc == 0 {
		Ok(())
	} else {
		Err(last_error())
	}
}

/// P5.4 gece: satır zaman yazısı (ms).
fn times_set(h: c_int, idx: usize, ms_bas: i64, ms_son: i64) -> Result<(), String> {
	let rc = unsafe { velnosub_abi_line_times_set(h, idx as c_int, ms_bas, ms_son) };
	if rc == 0 {
		Ok(())
	} else {
		Err(last_error())
	}
}

/// P5.4 gece: satır ekleme — düğüm idx ÖNÜNE (idx == satır sayısı → sona).
/// zamanlar ASS dizgesi ("0:00:01.00"); boş geçilebilir.
fn satir_ekle(
	h: c_int,
	idx: usize,
	text: &str,
	bas_ass: &str,
	son_ass: &str,
	stil: &str,
) -> Result<(), String> {
	let me = CString::new(text).map_err(|e| e.to_string())?;
	let ba = CString::new(bas_ass).map_err(|e| e.to_string())?;
	let so = CString::new(son_ass).map_err(|e| e.to_string())?;
	let st = CString::new(stil).map_err(|e| e.to_string())?;
	let rc = unsafe {
		velnosub_abi_line_insert(h, idx as c_int, me.as_ptr(), ba.as_ptr(),
			so.as_ptr(), st.as_ptr())
	};
	if rc == 0 {
		Ok(())
	} else {
		Err(last_error())
	}
}

/// P5.4 gece: satır silme.
fn satir_sil(h: c_int, idx: usize) -> Result<(), String> {
	let rc = unsafe { velnosub_abi_line_delete(h, idx as c_int) };
	if rc == 0 {
		Ok(())
	} else {
		Err(last_error())
	}
}

/// ASS saat dizgesi → ms ("0:00:01.00" → 1000_; cs = yüzde-saniye).
fn ass_ms(s: &str) -> Result<i64, String> {
	let mut it = s.trim().split(':');
	let h: i64 = it
		.next()
		.ok_or_else(|| format!("saat dizgesi boş: {s}"))?
		.parse()
		.map_err(|e| format!("saat hatası {s}: {e}"))?;
	let m: i64 = it
		.next()
		.ok_or_else(|| format!("saat eksik (m): {s}"))?
		.parse()
		.map_err(|e| format!("dakika hatası {s}: {e}"))?;
	let sn = it
		.next()
		.ok_or_else(|| format!("saat eksik (s): {s}"))?
		.trim();
	let (int_k, cs_k) = match sn.split_once('.') {
		Some((i, c)) => (i, c),
		None => (sn, "00"),
	};
	let sec_k: i64 = int_k.parse().map_err(|e| format!("saniye hatası {s}: {e}"))?;
	let cs: i64 = format!("{:0<2}", cs_k)
		.chars()
		.take(2)
		.collect::<String>()
		.parse()
		.map_err(|e| format!("yüzde-saniye hatası {s}: {e}"))?;
	Ok(h * 3_600_000 + m * 60_000 + sec_k * 1_000 + cs * 10)
}

/// ms → ASS saat dizgesi ("0:00:01.00").
fn ms_ass(ms: i64) -> String {
	let ms = ms.max(0);
	let h = ms / 3_600_000;
	let m = (ms / 60_000) % 60;
	let s = (ms / 1_000) % 60;
	let cs = (ms % 1_000) / 10;
	format!("{h}:{m:02}:{s:02}.{cs:02}")
}

/// P4.3: video künyesindeki sayısal geometriyi çıkar.
fn video_geom(h: c_int) -> Result<(usize, usize, f64, i32), String> {
	let mut w: c_int = 0;
	let mut hgt: c_int = 0;
	let mut fps: f64 = 0.0;
	let mut kareler: c_int = 0;
	let rc = unsafe {
		velnosub_abi_video_geom(h, &mut w, &mut hgt, &mut fps, &mut kareler)
	};
	if rc != 0 {
		return Err(last_error());
	}
	Ok((w as usize, hgt as usize, fps, kareler))
}

/// P4.3: BGRA kareyi tampona al (tampon-büyütmeli).
fn video_kare(h: c_int, idx: c_int) -> Result<Vec<u8>, String> {
	let mut len: c_int = 1 << 20;
	loop {
		let mut buf = vec![0u8; len as usize];
		let n = unsafe {
			velnosub_abi_video_frame(h, idx, buf.as_mut_ptr(), buf.len() as c_int)
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

/// BGRA bayt düzenini egui ColorImage'e bük (R/B takası, alfa doygun).
fn bgra_to_image(d: &[u8], w: usize, hgt: usize) -> egui::ColorImage {
	let mut pixels = Vec::with_capacity(w * hgt);
	for p in d.chunks_exact(4) {
		pixels.push(egui::Color32::from_rgba_unmultiplied(p[2], p[1], p[0], 255));
	}
	egui::ColorImage {
		size: [w, hgt],
		pixels,
		source_size: egui::Vec2::new(w as f32, hgt as f32),
	}
}

impl VideoSection {
	fn timeline(&self) -> &[i32] {
		self.external_times.as_deref().unwrap_or(&self.zamanlar)
	}

	fn keyframes(&self) -> &[i32] {
		self.external_keys.as_deref().unwrap_or(&self.ana_kareler)
	}

	fn kare_zamani(&self, frame: i32) -> i32 {
		self.timeline()[frame.clamp(0, self.kareler - 1) as usize]
	}

	fn zamandaki_kare(&self, ms: i64) -> i32 {
		self.timeline().partition_point(|&start| i64::from(start) <= ms).saturating_sub(1) as i32
	}

	fn en_yakin_kare_zamani(&self, ms: i64) -> i64 {
		let frame = self.zamandaki_kare(ms);
		let a = i64::from(self.kare_zamani(frame));
		let b = i64::from(self.kare_zamani(frame + 1));
		if ms.abs_diff(a) <= ms.abs_diff(b) { a } else { b }
	}

	/// Karesi doku olarak ekle (her kare değişiminde çağırır).
	fn cizim_cek(&mut self, ctx: &egui::Context) -> Result<(), String> {
		let d = video_kare(self.handle, self.frame)?;
		if d.len() != self.w * self.hgt * 4 {
			return Err(format!(
				"kare uzunluğu uymadı: beklenen {}, gelen {}",
				self.w * self.hgt * 4,
				d.len()
			));
		}
		let img = bgra_to_image(&d, self.w, self.hgt);
		let ad = format!("vvideo_f{}_{}", self.frame, self.kareler);
		self.dok = Some(ctx.load_texture(&ad, img, egui::TextureOptions::default()));
		Ok(())
	}
}

/// Aegi-parite hat satırı; yerles()'in ABI taze okuması doldurur.
#[derive(Clone)]
struct Row {
	// P4.8.2/5.4: line_times'ın "S1 / S2" biçimi burada bir kez ayrıştırılır
	bas: String,
	son: String,
	text: String,
	// P5.4 gece: aegisub hat-alanları (ABI line_meta okur, meta_set yazar)
	stil: String,
	oyuncu: String,
	etki: String,
	yorum: bool,
	katman: i32,
	kenarlar: [i32; 3],
}

/// P4.3: açılı video kolu ve önizleme dokusu durumu.
/// P6.4: önbellek dizini (C hattıyla aynı kural: VELNOSUB_CACHE →
/// VELNOSUB_EXTDIR → boş-görece)
fn alt_onbellek_ana() -> String {
	if let Ok(cb) = std::env::var("VELNOSUB_CACHE") {
		return cb;
	}
	if let Ok(ex) = std::env::var("VELNOSUB_EXTDIR") {
		return ex;
	}
	String::new()
}

/// P6.4: examine'in yazdığı vs-subtracks.tsv okuması —
/// (akış idx, codec, dil) üçlüleri
fn alt_tsv_okes() -> Vec<(i64, String, String)> {
	let ana = alt_onbellek_ana();
	let yol = if ana.is_empty() {
		std::path::PathBuf::from("vs-subtracks.tsv")
	} else {
		std::path::PathBuf::from(ana).join("vs-subtracks.tsv")
	};
	let Ok(veri) = std::fs::read(yol) else {
		return Vec::new();
	};
	let metin = String::from_utf8_lossy(&veri).into_owned();
	metin
		.lines()
		.filter_map(|s| {
			let mut dil = s.splitn(3, '\t');
			let i = dil.next()?.parse::<i64>().ok()?;
			let ko = dil.next()?.trim().to_string();
			let di = dil.next().unwrap_or("-").trim().to_string();
			Some((i, ko, di))
		})
		.collect()
}

	pub struct VideoSection {
	handle: c_int,
	w: usize,
	hgt: usize,
	fps: f64,
	kareler: i32,
	zamanlar: Vec<i32>,
	ana_kareler: Vec<i32>,
	external_times: Option<Vec<i32>>,
	external_keys: Option<Vec<i32>>,
	frame: i32,
	info: String,
	dok: Option<egui::TextureHandle>,
	olcek_yuzdesi: Option<u16>,
	preview_ratio: Option<f32>,
	// P6.4: medya yolu iç altyazı seçici; akslar = (stream idx, codec, dil);
	// oynatma çiftes (kare-timer) + birim kare biriktirici
	yol: String,
	akslar: Vec<(i64, String, String)>,
	oynat: bool,
	son_tik: Option<std::time::Instant>,
	// P7.1 aegisub player mantığı: birim = ms bekçisi (satır penceresi boyunca)
	birim: f32,
	// P7.1: oynat penceresi (ms) — satır başı + uzunluk; başlangıç karesi cb
	satir_bas: i64,
	satir_uzun: i64,
	// P6.5: son kare-decode süresi — cihazhızı kanıtlanır (ms/kare)
	coz_ms: u32,
}

/// P4.6 dilim 2: ASS önizleme kolu — altyazı kolu başına libass şartı,
/// kare zamanı tabloyu/video fps ile çevrilir; deposit egui saydam dokusu
/// olur (C kompoziti artık dizgi alfa tutuyor).
pub struct Oniz {
	h: c_int,
	gen: usize,
	yuk: usize,
	acik: bool,
	denendi: bool,
	son_ms: i32,
	etki: i32,
	tampon: Vec<u8>,
	dok: Option<egui::TextureHandle>,
	hata: Option<String>,
}

impl Default for Oniz {
	fn default() -> Self {
		Self {
			h: -1,
			gen: 0,
			yuk: 0,
			acik: false,
			denendi: false,
			son_ms: -1,
			etki: 0,
			tampon: Vec::new(),
			dok: None,
			hata: None,
		}
	}
}

/// Typesetting aracı — Task 10 çekirdeği; clip/vektör Task 11.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arac {
	Tasi,
	Olcek,
	ZDondur,
	Clip,
	Vektor,
	XY,
}

struct Surukleme {
	bas_pointer: egui::Pos2,
	pivot: egui::Pos2,
	bas_pos_ass: (f64, f64),
	bas_fscx: f64,
	bas_fscy: f64,
	bas_frz: f64,
	bas_metin: String,
}

/// Seçili satırın video üzerindeki typesetting durumu.
struct Typeset {
	satir: usize,
	arac: Arac,
	metin: String,
	script_w: i32,
	script_h: i32,
	// render_line önbelleği
	son_satir: usize,
	son_ms: i32,
	son_metin: String,
	tampon: Vec<u8>,
	dok: Option<egui::TextureHandle>,
	// doku pikseli cinsinden alfa sınırı
	bbox: Option<egui::Rect>,
	surukleme: Option<Surukleme>,
	// sürükleme sırasındaki egui önizleme dönüşümü
	oniz_ofs: egui::Vec2,
	oniz_olcek: f32,
	oniz_aci: f32,
	// clip / vektör / XY geçici önizleme metni
	gecici: Option<String>,
	clip: Option<(f64, f64, f64, f64)>,
	clip_hazir: bool,
	iclip: bool,
	clip_surukle: Option<usize>,
	vektor: Option<Vec<(f64, f64)>>,
	vektor_hazir: bool,
	vektor_surukle: Option<usize>,
	frx: f64,
	fry: f64,
	xy_hazir: bool,
	xy_surukle: Option<(egui::Pos2, f64, f64)>,
	hata: Option<String>,
}

impl Typeset {
	fn yeni(satir: usize, metin: String, script_w: i32, script_h: i32) -> Self {
		Self {
			satir,
			arac: Arac::Tasi,
			metin,
			script_w: script_w.max(1),
			script_h: script_h.max(1),
			son_satir: usize::MAX,
			son_ms: i32::MIN,
			son_metin: String::new(),
			tampon: Vec::new(),
			dok: None,
			bbox: None,
			surukleme: None,
			oniz_ofs: egui::Vec2::ZERO,
			oniz_olcek: 1.0,
			oniz_aci: 0.0,
			gecici: None,
			clip: None,
			clip_hazir: false,
			iclip: false,
			clip_surukle: None,
			vektor: None,
			vektor_hazir: false,
			vektor_surukle: None,
			frx: 0.0,
			fry: 0.0,
			xy_hazir: false,
			xy_surukle: None,
			hata: None,
		}
	}

	fn oniz_sifirla(&mut self) {
		self.surukleme = None;
		self.oniz_ofs = egui::Vec2::ZERO;
		self.oniz_olcek = 1.0;
		self.oniz_aci = 0.0;
		self.gecici = None;
	}

	fn arac_sec(&mut self, a: Arac) {
		self.arac = a;
		self.clip_hazir = false;
		self.vektor_hazir = false;
		self.xy_hazir = false;
		self.clip_surukle = None;
		self.vektor_surukle = None;
		self.xy_surukle = None;
		self.hata = None;
		self.oniz_sifirla();
	}
}

/// Etiket değerindeki sonlu sayıları sırayla çöz (`\pos(10,20)` gibi).
fn sayilar_al(deger: &str) -> Vec<f64> {
	deger
		.split(|c: char| c == ',' || c == '(' || c == ')' || c.is_whitespace())
		.filter_map(|s| s.parse::<f64>().ok())
		.filter(|v| v.is_finite())
		.collect()
}

/// Dereceyi (-180, 180] aralığına indir.
fn aci_derece(v: f64) -> f64 {
	let mut a = (v + 180.0).rem_euclid(360.0) - 180.0;
	if a == -180.0 {
		a = 180.0;
	}
	a
}

/// Ekran noktasını `pivot` çevresinde ölçekle/döndür, sonra kaydır.
fn ts_donusum(
	p: egui::Pos2,
	pivot: egui::Pos2,
	olcek: f32,
	aci: f32,
	ofs: egui::Vec2,
) -> egui::Pos2 {
	let v = p - pivot;
	let (s, c) = aci.sin_cos();
	pivot + egui::vec2(v.x * c - v.y * s, v.x * s + v.y * c) * olcek + ofs
}

/// Kare dokusunda alfa > 4 olan en küçük dikdörtgen (piksel).
fn alfa_bbox(d: &[u8], gen: usize, yuk: usize) -> Option<egui::Rect> {
	let mut minx = gen;
	let mut miny = yuk;
	let mut maxx = 0usize;
	let mut maxy = 0usize;
	let mut var = false;
	for y in 0..yuk {
		for x in 0..gen {
			let a = d.get((y * gen + x) * 4 + 3).copied().unwrap_or(0);
			if a > 4 {
				var = true;
				minx = minx.min(x);
				miny = miny.min(y);
				maxx = maxx.max(x);
				maxy = maxy.max(y);
			}
		}
	}
	if !var {
		return None;
	}
	Some(egui::Rect::from_min_max(
		egui::pos2(minx as f32, miny as f32),
		egui::pos2((maxx + 1) as f32, (maxy + 1) as f32),
	))
}

/// ASS hizasına göre çizilmiş satırın çapa noktası.
fn hiza_ankraj(bb: egui::Rect, hiza: i32) -> egui::Pos2 {
	let h = if (1..=9).contains(&hiza) { hiza } else { 2 };
	let x = match (h - 1) % 3 {
		0 => bb.left(),
		1 => bb.center().x,
		_ => bb.right(),
	};
	let y = match (h - 1) / 3 {
		0 => bb.bottom(),
		1 => bb.center().y,
		_ => bb.top(),
	};
	egui::pos2(x, y)
}

/// Döndürme tutamacı: çizim kutusunun üst ortasından yukarı doğru.
fn dondur_tutamac(bb: egui::Rect, aci: f32) -> egui::Pos2 {
	let tc = egui::pos2(bb.center().x, bb.top());
	let up = egui::vec2(0.0, -30.0);
	let (s, c) = aci.sin_cos();
	tc + egui::vec2(up.x * c - up.y * s, up.x * s + up.y * c)
}

/// Mevcut dikdörtgen clip'i oku. Vektör/bozuk clip `Err` döner.
fn clip_dik_oku(metin: &str) -> Result<Option<((f64, f64, f64, f64), bool)>, ()> {
	let (ad, ham) = if let Some(v) = ass_tags::etiket_al(metin, "clip") {
		("clip", v)
	} else if let Some(v) = ass_tags::etiket_al(metin, "iclip") {
		("iclip", v)
	} else {
		return Ok(None);
	};
	if ass_tags::vektor_noktalari(metin, ad).is_some() {
		return Err(());
	}
	let s = sayilar_al(&ham);
	if s.len() != 4 {
		return Err(());
	}
	let x1 = s[0].min(s[2]);
	let y1 = s[1].min(s[3]);
	let x2 = s[0].max(s[2]);
	let y2 = s[1].max(s[3]);
	Ok(Some(((x1, y1, x2, y2), ad == "iclip")))
}

/// Dikdörtgen clip yaz; eski clip/iclip etiketlerini değiştirir.
fn clip_dik_yaz(metin: &str, r: (f64, f64, f64, f64), iclip: bool) -> String {
	let m = ass_tags::etiket_sil(metin, "clip");
	let m = ass_tags::etiket_sil(&m, "iclip");
	ass_tags::etiket_yaz(
		&m,
		if iclip { "iclip" } else { "clip" },
		&format!("({:.1},{:.1},{:.1},{:.1})", r.0, r.1, r.2, r.3),
	)
}

/// Vektör clip yaz; eski clip/iclip etiketlerini değiştirir.
fn clip_vektor_yaz(
	metin: &str,
	noktalar: &[(f64, f64)],
	iclip: bool,
) -> String {
	let m = ass_tags::etiket_sil(metin, "clip");
	let m = ass_tags::etiket_sil(&m, "iclip");
	ass_tags::vektor_yaz(
		&m,
		if iclip { "iclip" } else { "clip" },
		noktalar,
		true,
	)
}

/// Mevcut vektör clip noktalarını oku.
fn clip_vektor_oku(metin: &str) -> Option<(Vec<(f64, f64)>, bool)> {
	for ad in ["clip", "iclip"] {
		if let Some(mut p) = ass_tags::vektor_noktalari(metin, ad) {
			// `vektor_yaz(kapali=true)` ilk noktaya dönen kapanışı ekler;
			// düzenleme listesinde bu yinelenen noktayı göstermeyiz.
			if p.len() > 1 && p.first() == p.last() {
				p.pop();
			}
			if !p.is_empty() {
				return Some((p, ad == "iclip"));
			}
		}
	}
	None
}

/// XY perspektif etiketlerini yaz; mevcut `\frx/\fry` yerinde güncellenir.
fn xy_yaz(metin: &str, frx: f64, fry: f64) -> String {
	let m = ass_tags::etiket_yaz(metin, "frx", &format!("{:.1}", aci_derece(frx)));
	ass_tags::etiket_yaz(&m, "fry", &format!("{:.1}", aci_derece(fry)))
}

/// Noktanın doğru parçasına ekran mesafesi (nokta ekleme isabeti).
fn kenar_mesafe(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
	let ab = b - a;
	let l2 = ab.length_sq();
	if l2 <= 1e-6 {
		return (p - a).length();
	}
	let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
	(p - (a + ab * t)).length()
}


/// Ses çizim modu — Aegisub "Ses → Spektrum" geçişi.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SesMod {
	Dalga,
	Spektrum,
}

/// P4.4: açılı ses kolu + görünür aralık dalga/spektrum durumu.
pub struct AudioSection {
	handle: c_int,
	path: String,
	// Gerçek künye: `audio_info` dizgesinden çözülür (yoksa 48000/2).
	rate: c_int,
	ch: c_int,
	toplam: i64,
	info: String,
	ses_durum: String,
	oynatma_denendi: bool,
	konum: i64,
	// Görünür aralık (örnek) ve imleç/seçim.
	d0: i64,
	adet: i64,
	imlec: i64,
	secim: Option<(i64, i64)>,
	mod_: SesMod,
	takip: bool,
	// Spektrum önbelleği: (d0, adet, sütun, satır) anahtarı + doku.
	spek: Option<Vec<u8>>,
	spek_anahtar: Option<(i64, i64, i32, i32)>,
	spek_doku: Option<egui::TextureHandle>,
	spek_hata: Option<String>,
	// Dalga gövdesinde seçim sürüklemesinin çıpası (örnek).
	surukleme: Option<i64>,
	// Satır başı/sonu tutamaçlarının sürükleme sırasındaki geçici konumu.
	tut_bas: Option<i64>,
	tut_son: Option<i64>,
}

fn audio_peaks_cek(
	h: c_int,
	d0: i64,
	adet: i64,
	kovalar: c_int,
) -> Result<(Vec<i16>, Vec<i16>), String> {
	let k = kovalar.clamp(1, 2048);
	let mut minv = vec![0i16; k as usize];
	let mut maxv = vec![0i16; k as usize];
	if adet <= 0 {
		return Ok((minv, maxv));
	}
	let rc = unsafe {
		velnosub_abi_audio_peaks(
			h, d0, adet, k, minv.as_mut_ptr(), maxv.as_mut_ptr(),
		)
	};
	if rc != 0 {
		return Err(last_error());
	}
	Ok((minv, maxv))
}

/// Örnek → ms (görünür aralık/oynatma konumu).
fn ornek_ms(ornek: i64, rate: i32) -> i64 {
	if rate <= 0 {
		return 0;
	}
	((ornek as i128 * 1000) / rate as i128) as i64
}

/// ms → örnek.
fn ms_ornek(ms: i64, rate: i32) -> i64 {
	if rate <= 0 {
		return 0;
	}
	((ms as i128 * rate as i128) / 1000) as i64
}

fn ses_gorunum(a: &AudioSection, gen: f32) -> crate::audio_timing::Gorunum {
	crate::audio_timing::Gorunum { d0: a.d0, adet: a.adet, gen }
}

fn ses_gorunum_uygula(a: &mut AudioSection, g: crate::audio_timing::Gorunum) {
	a.d0 = g.d0;
	a.adet = g.adet;
}

/// Görünür aralık için 512 kovalı dalga çiz.
fn ses_dalga_ciz(
	painter: &egui::Painter,
	a: &AudioSection,
	dik: egui::Rect,
	_g: &crate::audio_timing::Gorunum,
) {
	let (minv, maxv) = match audio_peaks_cek(a.handle, a.d0, a.adet, 512) {
		Ok(v) => v,
		Err(_) => {
			painter.text(
				dik.center(),
				egui::Align2::CENTER_CENTER,
				"dalga okunamadı",
				egui::FontId::proportional(12.0),
				egui::Color32::from_rgb(220, 120, 90),
			);
			return;
		}
	};
	let orta = dik.center().y;
	let yarim = dik.height() * 0.5 - 2.0;
	painter.line_segment(
		[egui::pos2(dik.left(), orta), egui::pos2(dik.right(), orta)],
		egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
	);
	let adim = dik.width() / minv.len().max(1) as f32;
	let kalem = egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 180, 250));
	for k in 0..minv.len() {
		let x = dik.left() + adim * (k as f32 + 0.5);
		let ymin = orta - yarim * (maxv[k] as f32 / 32768.0);
		let ymax = orta - yarim * (minv[k] as f32 / 32768.0);
		painter.line_segment([egui::pos2(x, ymin), egui::pos2(x, ymax)], kalem);
	}
}

/// Görünür aralık spektrumunu önbellekten hazırla; `(d0, adet, cols, rows)`
/// değişmedikçe ABI yeniden çağrılmaz. Hata durumunda dalga çizimine düşülür.
fn ses_spek_hazirla(
	ctx: &egui::Context,
	a: &mut AudioSection,
	cols: i32,
	rows: i32,
) -> bool {
	let cols = cols.clamp(1, 192);
	let rows = rows.clamp(1, 256);
	let anahtar = (a.d0, a.adet, cols, rows);
	if a.spek_anahtar == Some(anahtar) {
		return a.spek_doku.is_some();
	}
	let gecerli = a.d0 >= 0
		&& a.adet > 0
		&& a.d0.saturating_add(a.adet) <= a.toplam;
	if !gecerli {
		a.spek = None;
		a.spek_doku = None;
		a.spek_anahtar = Some(anahtar);
		a.spek_hata = Some("görünür aralık ses sınırları dışında".into());
		return false;
	}
	let mut ham = vec![0u8; cols as usize * rows as usize];
	let rc = unsafe {
		velnosub_abi_audio_spectrum(
			a.handle,
			a.d0,
			a.adet,
			cols,
			rows,
			ham.as_mut_ptr(),
		)
	};
	if rc != 0 {
		a.spek = None;
		a.spek_doku = None;
		a.spek_anahtar = Some(anahtar);
		a.spek_hata = Some(last_error());
		return false;
	}
	let mut pikseller = vec![egui::Color32::BLACK; ham.len()];
	for col in 0..cols as usize {
		for row in 0..rows as usize {
			let v = ham[col * rows as usize + row] as f32 / 255.0;
			// satır 0 = düşük frekans; görüntüde alta yerleşir (dikey çevir).
			let pik = egui::Color32::from_rgb(
				(255.0 * v).round() as u8,
				(255.0 * v * v).round() as u8,
				(30.0 + 90.0 * (1.0 - v)).round() as u8,
			);
			pikseller[(rows as usize - 1 - row) * cols as usize + col] = pik;
		}
	}
	let img = egui::ColorImage::new([cols as usize, rows as usize], pikseller);
	let ad = format!("ses-spek-{}-{}-{}-{}", a.d0, a.adet, cols, rows);
	a.spek_doku = Some(ctx.load_texture(ad, img, egui::TextureOptions::LINEAR));
	a.spek = Some(ham);
	a.spek_anahtar = Some(anahtar);
	a.spek_hata = None;
	true
}

/// P4.5: ABI stil dizgesinin ayrıştırılmış bir stil satırı.
#[derive(Clone)]
pub struct StyleRow {
	name: String,
	font: String,
	size: f64,
	primary: String,
	secondary: String,
	outline: String,
	shadow: String,
	flags: String,
	alignment: i32,
	margins: String,
	encoding: i32,
	scale: String,
	edge: String,
}

/// ABI stil dizgesini büyüt- yeniden-dene kuralıyla oku (line_text örneği).
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
		if (n as usize) < len.saturating_sub(1) {
			buf.truncate(n as usize);
			return Ok(String::from_utf8_lossy(&buf).into_owned());
		}
		len *= 2;
	}
}

/// P4.5 devam: mevcut dizgeyi al, seçili hücreleri değiştir, ABI'ye yaz —
/// ABI tüm alanları çözmeyen herhangi bir hücreyi reddeder (yarım durum yok)
/// P8.1: StyleRow → 13 hücreli stil dizgesi (style_write biçimi)
fn stil_hucreleri(s: &StyleRow) -> Vec<String> {
	vec![
		s.name.clone(),
		s.font.clone(),
		format!("{}", s.size),
		s.primary.clone(),
		s.secondary.clone(),
		s.outline.clone(),
		s.shadow.clone(),
		s.flags.clone(),
		s.alignment.to_string(),
		s.margins.clone(),
		s.encoding.to_string(),
		s.scale.clone(),
		s.edge.clone(),
	]
}

/// P8.1: ASS hizalama kodu → okunur ad (1-3 alt, 4-6 orta, 7-9 üst)
fn hiza_ad(n: i32) -> &'static str {
	match n {
		1 => "1 sol-alt",
		2 => "2 orta-alt",
		3 => "3 sağ-alt",
		4 => "4 sol-orta",
		5 => "5 orta-orta",
		6 => "6 sağ-orta",
		7 => "7 sol-üst",
		8 => "8 orta-üst",
		9 => "9 sağ-üst",
		_ => "—",
	}
}

fn stil_yaz(
	h: c_int,
	idx: c_int,
	degistir: impl FnOnce(&mut Vec<String>),
) -> Result<(), String> {
	let t = style_of(h, idx)?;
	let mut hucre: Vec<String> = t.split('|').map(|s| s.to_string()).collect();
	if hucre.len() != 13 {
		return Err(format!("stil dizgesi 13 hücre değil: {}", hucre.len()));
	}
	degistir(&mut hucre);
	let metin = hucre.join("|");
	let c = CString::new(metin).map_err(|e| format!("dizge kodlanamadı: {e}"))?;
	let rc = unsafe { velnosub_abi_style_write(h, idx, c.as_ptr()) };
	if rc != 0 {
		return Err(last_error());
	}
	Ok(())
}

/// ASS stil rengi "&HAABBGGRR" → egui Color32 (alpha tersinde)
fn ass_to_color32(s: &str) -> Option<egui::Color32> {
	let t = s.trim().trim_start_matches('&').trim_end_matches('&').trim();
	let t = t.strip_prefix('H').or_else(|| t.strip_prefix('h'))?;
	if t.len() < 8 {
		return None;
	}
	let aa = u8::from_str_radix(&t[0..2], 16).ok()?;
	let bb = u8::from_str_radix(&t[2..4], 16).ok()?;
	let gg = u8::from_str_radix(&t[4..6], 16).ok()?;
	let rr = u8::from_str_radix(&t[6..8], 16).ok()?;
	Some(egui::Color32::from_rgba_unmultiplied(rr, gg, bb, 255 - aa))
}

/// egui Color32 → ASS stil dizgesi
fn color32_to_ass(c: egui::Color32) -> String {
	format!(
		"&H{:02X}{:02X}{:02X}{:02X}",
		255 - c.a(), c.b(), c.g(), c.r()
	)
}

/// Dialog alan dizgesini ABI'den oku (izadorlama büyüt-vekik kuralı).
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
		if (n as usize) < len.saturating_sub(1) {
			buf.truncate(n as usize);
			return Ok(String::from_utf8_lossy(&buf).into_owned());
		}
		len *= 2;
	}
}

/// P4.5 III: dinamik form satırı (ABI alan dizgesinden ayrıştırılmış).
struct AlanRow {
	sinif: String,
	ad: String,
	baslik: String,
	deger: String,
	ogeler: Vec<String>,
	alt: f64,
	ust: f64,
}

/// Açılı makro-form kolu: tanım + SATIR AYRIŞTIRMA zapşı.
struct DialogSection {
	handle: c_int,
	alanlar: Vec<AlanRow>,
	dugmeler: Vec<String>,
}

struct MacroForm {
	token: c_int,
	name: String,
	dialog: DialogSection,
}

fn dialog_ayikla(t: &String) -> Option<AlanRow> {
	let p: Vec<&str> = t.split('|').collect();
	if p.len() < 4 {
		return None;
	}
	let sinif = p[0].to_string();
	let (alt, ust) = if (sinif == "intedit" || sinif == "floatedit") && p.len() >= 5 {
		let v: Vec<&str> = p[4].split(',').collect();
		if v.len() == 2 {
			(v[0].parse().unwrap_or(0.0), v[1].parse().unwrap_or(0.0))
		} else {
			(0.0, 0.0)
		}
	} else {
		(0.0, 0.0)
	};
	let ogeler = if sinif == "dropdown" && p.len() >= 5 {
		p[4].split(';').map(|s| s.to_string()).collect()
	} else {
		Vec::new()
	};
	Some(AlanRow {
		sinif,
		ad: p[1].into(),
		baslik: p[2].into(),
		deger: p[3].into(),
		ogeler,
		alt,
		ust,
	})
}

fn load_dialog(yolu: &str) -> Result<DialogSection, String> {
	let metin =
		std::fs::read_to_string(yolu).map_err(|e| format!("spec okunamadı: {e}"))?;
	let c = CString::new(metin).map_err(|e| format!("spec kodlanamadı: {e}"))?;
	let handle = unsafe { velnosub_abi_dialog_open(c.as_ptr()) };
	if handle < 0 {
		return Err(last_error());
	}
	dialog_model(handle)
}

fn dialog_model(dh: c_int) -> Result<DialogSection, String> {
	let n = unsafe { velnosub_abi_dialog_count(dh) };
	if n <= 0 {
		let e = last_error();
		unsafe { velnosub_abi_dialog_close(dh) };
		return Err(e);
	}
	let bn = unsafe { velnosub_abi_dialog_btncount(dh) };
	let mut alanlar = Vec::new();
	for i in 0..n {
		match dialog_ctl(dh, i) {
			Ok(t) => {
				if let Some(row) = dialog_ayikla(&t) {
					alanlar.push(row);
				}
			}
			Err(e) => {
				unsafe { velnosub_abi_dialog_close(dh) };
				return Err(e);
			}
		}
	}
	let mut dugmeler = Vec::new();
	for i in 0..bn.max(0) {
		let oku = |h: c_int, idx: c_int| -> Result<String, String> {
			let mut buf = vec![0u8; 96];
			let n2 = unsafe {
				velnosub_abi_dialog_button(
					h, idx,
					buf.as_mut_ptr() as *mut c_char,
					buf.len() as c_int,
				)
			};
			if n2 < 0 {
				return Err(last_error());
			}
			buf.truncate((n2 as usize).min(95));
			Ok(String::from_utf8_lossy(&buf).into_owned())
		};
		match oku(dh, i) {
			Ok(t) => dugmeler.push(t),
			Err(e) => {
				unsafe { velnosub_abi_dialog_close(dh) };
				return Err(e);
			}
		}
	}
	Ok(DialogSection {
		handle: dh,
		alanlar,
		dugmeler,
	})
}

/// P4.2 uygulama durumu: tek açık tutamaç + satır önbelleği.
/// P5.5: dosya seçici — SAF'siz read_dir tarayıcı; READ_EXTERNAL_STORAGE ile
/// /sdcard ağacı okunur (izin smoke.sh `pm grant` ile verilir). İçerik:
/// dizin gezinti + uzantı süzgeci + tek-dokunuş seçim + AÇ onayı.
#[derive(Clone, Copy, PartialEq)]
enum SecimTur {
	Timecodes,
	Keyframes,
	DictionaryRules,
	DictionaryWords,
	Altyazi,
	Video,
	Ses,
	Lua,
	Font,
}

#[derive(Clone)]
struct Tarayici {
	acik: bool,
	tur: SecimTur,
	yol: std::path::PathBuf,
	secili: Option<String>,
	hata: Option<String>,
}

impl Tarayici {
	fn yeni() -> Self {
		Self {
			acik: false,
			tur: SecimTur::Altyazi,
			yol: std::path::PathBuf::from("/storage/emulated/0"),
			secili: None,
			hata: None,
		}
	}

	fn uzantilar(&self) -> &'static [&'static str] {
		match self.tur {
			SecimTur::Timecodes | SecimTur::Keyframes => &["txt", "v1", "v2"],
			SecimTur::DictionaryRules => &["aff"],
			SecimTur::DictionaryWords => &["dic"],
			SecimTur::Altyazi => &["ass", "ssa", "srt", "txt"],
			SecimTur::Video => &["mkv", "mp4", "webm", "avi", "mov", "m4v", "ts"],
			SecimTur::Lua => &["lua", "moon"],
			SecimTur::Font => &["ttf", "ttc", "otf"],
			SecimTur::Ses => &["mp3", "m4a", "aac", "flac", "opus", "ogg", "wav", "mka", "wv"],
		}
	}
}

#[derive(Clone)]
enum PendingDocumentAction {
	Open(String),
	New,
	#[cfg(target_os = "android")]
	Close,
}

#[cfg(target_os = "android")]
static DOCUMENT_UI: std::sync::Mutex<Option<egui::Context>> = std::sync::Mutex::new(None);

#[cfg(target_os = "android")]
extern "C" fn wake_document_ui() {
	if let Ok(context) = DOCUMENT_UI.lock() {
		if let Some(context) = context.as_ref() {
			context.request_repaint();
		}
	}
}

#[cfg(target_os = "android")]
struct MacroExport {
	path: std::path::PathBuf,
	document: Option<c_int>,
	name: String,
}

#[cfg(target_os = "android")]
impl Drop for MacroExport {
	fn drop(&mut self) {
		if let Some(document) = self.document.take() {
			unsafe { velnosub_abi_close(document) };
		}
		GridApp::remove_saf_stage(&self.path);
	}
}

#[cfg(target_os = "android")]
enum AndroidBelge {
	MacroOpen,
	MacroSave(MacroExport),
	Ac(SecimTur),
	Kaydet(std::path::PathBuf, bool),
}

#[cfg(target_os = "android")]
extern "C" {
	fn velnosub_abi_document_pick(
		vm: *mut std::ffi::c_void, activity: *mut std::ffi::c_void,
		kind: c_int, path: *const c_char, title: *const c_char,
	) -> c_int;
	fn velnosub_abi_document_poll(path: *mut c_char, len: usize) -> c_int;
	fn velnosub_abi_document_take_close() -> c_int;
	fn velnosub_abi_document_set_wake(wake: Option<extern "C" fn()>);
}

struct PendingVideo {
	cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
	result: std::sync::mpsc::Receiver<Result<(OpenedVideo, String), String>>,
}

impl Drop for PendingVideo {
	fn drop(&mut self) {
		self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
	}
}

struct OpenedVideo(Option<c_int>);

impl Drop for OpenedVideo {
	fn drop(&mut self) {
		if let Some(h) = self.0 {
			unsafe { velnosub_abi_video_close(h) };
		}
	}
}

struct PendingAudio {
	cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
	result: std::sync::mpsc::Receiver<Result<(OpenedAudio, String), String>>,
}

struct OpenedAudio(Option<c_int>);

impl Drop for OpenedAudio {
	fn drop(&mut self) {
		if let Some(h) = self.0 {
			unsafe { velnosub_abi_audio_close(h) };
		}
	}
}

impl Drop for PendingAudio {
	fn drop(&mut self) {
		self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
	}
}

extern "C" fn audio_cancelled(state: *const std::ffi::c_void) -> c_int {
	let state = unsafe { &*state.cast::<std::sync::atomic::AtomicBool>() };
	state.load(std::sync::atomic::Ordering::Relaxed).into()
}

struct LuaScript {
	path: String,
	source: CString,
	names: Vec<String>,
	#[cfg(target_os = "android")]
	filters: Vec<String>,
}

struct HistoryEntry {
	handle: c_int,
	revision: u64,
}

struct Misspelling {
	row: usize,
	span: std::ops::Range<usize>,
	revision: u64,
	word: String,
	replacement: String,
	suggestions: Vec<String>,
}

struct Translation {
	revision: u64,
	row: usize,
	text: String,
	drafts: std::collections::BTreeMap<usize, String>,
}

pub struct GridApp {
	multi_select: bool,
	marked_rows: std::collections::BTreeSet<usize>,
	translation: Option<Translation>,
	spelling_open: bool,
	dictionary_rules: Option<String>,
	dictionary_words: Option<String>,
	dictionary: Option<spellbook::Dictionary>,
	misspelling: Option<Misspelling>,
	lua_script: Option<LuaScript>,
	macro_form: Option<MacroForm>,
	#[cfg(target_os = "android")]
	macro_export: Option<MacroExport>,
	lua_window: bool,
	command_query: Option<String>,
	handle: Option<c_int>,
	rows: Vec<Row>,
	path: Option<String>,
	save_path: Option<String>,
	current_revision: u64,
	saved_revision: u64,
	next_revision: u64,
	bekleyen_belge: Option<PendingDocumentAction>,
	kayit_hedefi: Option<String>,
	#[cfg(target_os = "android")]
	android_belge: Option<AndroidBelge>,
	#[cfg(target_os = "android")]
	android_saved_snapshot: Option<std::path::PathBuf>,
	#[cfg(target_os = "android")]
	android_ready: bool,
	note: String,
	video: Option<VideoSection>,
	pending_video: Option<PendingVideo>,
	audio: Option<AudioSection>,
	pending_audio: Option<PendingAudio>,
	// P5.5: kendi dosyalarını açma — uygulamaiçi tarayıcı
	tarayici: Tarayici,
	stiller: Vec<StyleRow>,
	dialog: Option<DialogSection>,
	// P4.6 dilim 2: ASS önizleme (video üstünde saydam dokuya binen kare)
	oniz: Oniz,
	// P8: seçili satır üzerinde görsel typesetting (taşı/ölçek/Z döndür)
	typeset: Option<Typeset>,
	// P4.6 dilim 2: arayüz düzeni — varsayılan Otomatik (yatayda Aegisub,
	// dikeyde mobil); Mobil/Masaüstü ayarlardan/seçimden sabitlenir
	// (`velnosub-ui.json`; VELNOSUB_UI_JSON ortam değişkeniyle yol geçilir)
	duzen: Duzen,
	// P4.8.1: masaüstü yüzeyi ölçeği — ppp = ppp_base × zoom; Mobil'e
	// dönüşte tabana döner
	zoom: f32,
	ppp_base: Option<f32>,
	aktif: Sekme,
	// masaüstü düzeninde Ayarlar penceresi açık mı (mobilde ayrı sekme var)
	ayar_win: bool,
	// Task 6: kompakt ses şeridinden açılan tam zamanlama penceresi
	ses_win: bool,
	// P4.8.2: seçili satır (aegisub edit kutusu bu satıra bağlanır)
	secili: Option<usize>,
	// P5.4 gece: geri/ileri anlık görüntüleri + koalema zamanı
	undo_s: Vec<HistoryEntry>,
	redo_s: Vec<HistoryEntry>,
	son_an: Option<(usize, std::time::Instant)>,
	// P5.4 gece: Ara-Bul + zaman kaydırıcı pencere durumları
	ara_win: bool,
	ara_metin: String,
	ara_yeni: String,
	ara_i: usize,
	shift_win: bool,
	shift_ms: i32,
	shift_tum: bool,
	// P8.1: tam stil yöneticisi penceresi + seçili stil
	stil_win: bool,
	stil_sec: usize,
	// P8.1: dosya özellikleri (Aegisub File → Properties portu)
	ozellik_win: bool,
}

/// Arayüz düzeni. `Otomatik` ekran yönüne göre seçer (yatay + geniş →
/// Aegisub, aksi mobil); `Secim` yalnızca elle açılan ilk seçim ekranıdır.
#[derive(PartialEq, Clone, Copy)]
enum Duzen {
	Otomatik,
	Secim,
	Mobil,
	Masaustu,
}

/// Mobil düzenin alt sekmesi. Sekmeler yalnız gerçek içeriğe oyulur
/// (boş sekme yok); Altyazı + Ayarlar her zaman var.
#[derive(PartialEq, Clone, Copy)]
enum Sekme {
	Altyazi,
	Video,
	Ses,
	Stil,
	Form,
	Ayarlar,
}

/// Arayüz ayar dosyası yolu: `VELNOSUB_UI_JSON` varsa o, yoksa çalışma
/// dizinindeki `velnosub-ui.json` (ABI config yolu konvensiyonuyla aynı).
fn ayar_yolu() -> String {
	std::env::var("VELNOSUB_UI_JSON").unwrap_or_else(|_| "velnosub-ui.json".into())
}

/// Arayüz teması: koyu varsayılan; açık tema Ayarlar'dan seçilir
/// (denetim bulgu 7).
#[derive(PartialEq, Clone, Copy)]
pub enum Tema {
	Koyu,
	Acik,
}

/// Ayar dosyasında okunan değerler; eksik alan None kalır (o alan için
/// varsayılan kullanılır: düzen Otomatik, tema koyu).
pub struct Ayar {
	pub duzen: Option<String>,
	pub tema: Option<String>,
}

/// `{ "anahtar": "deger" }` biçimi için sade tarayıcı: anahtarı bulup
/// ondan sonraki ilk tırnaklı parçayı döner. Yalnız bizim yazdığımız
/// `velnosub-ui.json` beklenir; dosya yoksa/okunamazsa varsayılan Otomatik
/// kullanılır (None döner).
fn deger_bul(icerik: &str, anahtar: &str) -> Option<String> {
	let cap = format!("\"{anahtar}\"");
	let bas = icerik.find(&cap)? + cap.len();
	let kalan = &icerik[bas..];
	let i = kalan.find('"')? + 1;
	let kalan2 = &kalan[i..];
	let j = kalan2.find('"')?;
	Some(kalan2[..j].to_string())
}

/// Ayar dosyasını okur; dosya yoksa boş Ayar döner — bu durumda uygulama
/// varsayılan Otomatik/koyu ile açılır (düzen seçimi sorulmaz).
pub fn ayar_yukle() -> Ayar {
	let bos = Ayar {
		duzen: None,
		tema: None,
	};
	let icerik = match std::fs::read_to_string(ayar_yolu()) {
		Ok(k) => k,
		Err(_) => return bos,
	};
	Ayar {
		duzen: deger_bul(&icerik, "ui"),
		tema: deger_bul(&icerik, "tema"),
	}
}

pub fn ayar_yaz(duzen: &str, tema: &str) -> std::io::Result<()> {
	std::fs::write(
		ayar_yolu(),
		format!("{{ \"ui\": \"{duzen}\", \"tema\": \"{tema}\" }}"),
	)
}

/// WCAG 2.1 göreli parlaklığı (sRGB doğrusallaştırma) — ölçüm ve tamir
/// aynı formülü paylaşır.
fn parlaklik(c: egui::Color32) -> f64 {
	fn lin(v: f64) -> f64 {
		if v <= 0.04045 {
			v / 12.92
		} else {
			((v + 0.055) / 1.055).powf(2.4)
		}
	}
	0.2126 * lin(c.r() as f64 / 255.0)
		+ 0.7152 * lin(c.g() as f64 / 255.0)
		+ 0.0722 * lin(c.b() as f64 / 255.0)
}

/// WCAG kontrast oranı (1:1 .. 21:1) — denetim 2'nin ölçüm kolu.
pub fn kontrast(ust: egui::Color32, alt: egui::Color32) -> f64 {
	let a = parlaklik(ust);
	let b = parlaklik(alt);
	let (acik, koyu) = if a > b { (a, b) } else { (b, a) };
	(acik + 0.05) / (koyu + 0.05)
}

/// Kontrast tamiri (denetim 2): egui'nin varsayılan soluk metni koyu
/// zeminde 4.5:1'ın altında kalıyor; tamir temasına uygun kaynak-griye
/// sabitler (koyuda 170, aç beyazda 90). Ölçüyü tests::mod doğrular.
pub fn stil_tamir(style: &mut egui::Style, tema: Tema) {
	let zayif = match tema {
		Tema::Koyu => egui::Color32::from_gray(170),
		Tema::Acik => egui::Color32::from_gray(90),
	};
	style.visuals.weak_text_color = Some(zayif);
}

/// Anlık üçüncü kolu oku: bu çerçevedeki etkin tema (Ui stilinden).
fn tema_simdi(ui: &egui::Ui) -> Tema {
	if ui.style().visuals.dark_mode {
		Tema::Koyu
	} else {
		Tema::Acik
	}
}

impl GridApp {
	/// config'i bir kez hazırla; `open` verilmişse dosyayı aç.
	pub fn new(open: Option<String>, save: Option<String>) -> Self {
		if INIT_OK.get().is_none() {
			let cfg = std::env::var("VELNOSUB_CONFIG_JSON")
				.unwrap_or_else(|_| "default_config.json".into());
			let ok_rc = CString::new(cfg)
				.map(|ok| unsafe { velnosub_abi_init(ok.as_ptr()) })
				.unwrap_or(-1);
			let _ = INIT_OK.set(ok_rc == 0);
		}
		let mut app = Self {
			multi_select: false,
			marked_rows: Default::default(),
			translation: None,
			spelling_open: false,
			dictionary_rules: None,
			dictionary_words: None,
			dictionary: None,
			misspelling: None,
			lua_script: None,
			macro_form: None,
			#[cfg(target_os = "android")]
			macro_export: None,
			lua_window: false,
			command_query: None,
			handle: None,
			rows: Vec::new(),
			path: None,
			save_path: None,
			current_revision: 0,
			saved_revision: 0,
			next_revision: 1,
			bekleyen_belge: None,
			kayit_hedefi: None,
			#[cfg(target_os = "android")]
			android_belge: None,
			#[cfg(target_os = "android")]
			android_saved_snapshot: None,
			#[cfg(target_os = "android")]
			android_ready: false,
			note: "açık dosya yok".into(),
			video: None,
			pending_video: None,
			audio: None,
			pending_audio: None,
			tarayici: Tarayici::yeni(),
			stiller: Vec::new(),
			dialog: None,
			duzen: if cfg!(target_os = "android") { Duzen::Masaustu } else { Duzen::Otomatik },
			zoom: 0.8,
			ppp_base: None,
			secili: None,
			undo_s: Vec::new(),
			redo_s: Vec::new(),
			son_an: None,
			ara_win: false,
			ara_metin: String::new(),
			ara_yeni: String::new(),
			ara_i: 0,
			shift_win: false,
			shift_ms: 1000,
			shift_tum: true,
			stil_win: false,
			stil_sec: 0,
			ozellik_win: false,
			aktif: Sekme::Altyazi,
			ayar_win: false,
			ses_win: false,
			oniz: Oniz::default(),
			typeset: None,
		};
		match ayar_yukle().duzen.as_deref() {
			Some("otomatik") => app.duzen = Duzen::Otomatik,
			Some("mobil") => app.duzen = Duzen::Mobil,
			Some("masaustu") => app.duzen = Duzen::Masaustu,
			_ => {}
		}
		if let Some(p) = open {
			if let Err(e) = app.load(&p) {
				app.note = format!("açılamadı: {e}");
			} else if let Some(save) = save {
				app.save_path = Some(save);
			}
		} else {
			app.new_document();
		}
		if let Some(v) = std::env::var("VELNOSUB_VIDEO").ok() {
			app.video_ac(&v);
		}
		if let Some(s) = std::env::var("VELNOSUB_AUDIO").ok() {
			app.ses_ac(&s);
		}
		if let Some(s) = std::env::var("VELNOSUB_DIALOG").ok() {
			match load_dialog(&s) {
				Ok(d) => {
					app.note = format!("form: {} alan", d.alanlar.len());
					app.dialog = Some(d);
				}
				Err(e) => app.note = format!("form açılamadı: {e}"),
			}
		}
		app
	}

	/// P5.4 gece: satır kümesini ABI'den yeniden çeker (load/ekle/sil/undo
	/// sonrası indeks kaçmasını tek noktadan çözer)
	fn yerles(&mut self) {
		let Some(h) = self.handle else { return; };
		let ev = unsafe { velnosub_abi_event_count(h) }.max(0);
		let mut rows = Vec::with_capacity(ev as usize);
		for i in 0..ev {
			let times = line_times(h, i)
				.unwrap_or_else(|e| format!("(zaman hatası: {e})"));
			let text = line_text(h, i).unwrap_or_else(|e| format!("(hata: {e})"));
			let mp = line_meta_parts(h, i).unwrap_or_default();
			let (bir, son) = match times.split_once(" / ") {
				Some((b, s)) => (b.to_string(), s.to_string()),
				None => (times.clone(), String::new()),
			};
			// meta: [0]=class [1]=yorum [2]=katman [3]=stil [4]=oyuncu
			// [5]=etki [6..8]=kenarlar
			rows.push(Row {
				bas: bir,
				son,
				text,
				stil: mp.get(3).cloned().unwrap_or_default(),
				oyuncu: mp.get(4).cloned().unwrap_or_default(),
				etki: mp.get(5).cloned().unwrap_or_default(),
				yorum: mp.get(1).map(|v| v == "1").unwrap_or(false),
				katman: mp.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
				kenarlar: std::array::from_fn(|i| mp.get(6 + i).and_then(|part| part.parse().ok()).unwrap_or_default()),
			});
		}
		self.rows = rows;
		self.secili = self.secili.filter(|&i| i < self.rows.len());
	}

	const UNO_LIMIT: usize = 24;

	fn belge_degisti(&self) -> bool {
		self.current_revision != self.saved_revision
	}

	fn gecmisi_temizle(gecmis: &mut Vec<HistoryEntry>) {
		for entry in gecmis.drain(..) {
			unsafe { velnosub_abi_close(entry.handle) };
		}
	}

	fn an_tut(&mut self, metin_satiri: Option<usize>) -> bool {
		let Some(h) = self.handle else { return false };
		let simdi = std::time::Instant::now();
		if let (Some(i), Some((onceki, zaman))) = (metin_satiri, self.son_an) {
			if i == onceki && simdi.duration_since(zaman) < std::time::Duration::from_secs(2) {
				self.son_an = Some((i, simdi));
				return true;
			}
		}
		let an = unsafe { velnosub_abi_snapshot(h) };
		if an < 0 {
			self.note = format!("geçmiş kaydedilemedi: {}", last_error());
			return false;
		}
		self.gecmise_ekle(an);
		self.son_an = metin_satiri.map(|i| (i, simdi));
		true
	}

	fn gecmise_ekle(&mut self, h: c_int) {
		self.undo_s.push(HistoryEntry { handle: h, revision: self.current_revision });
		self.current_revision = self.next_revision;
		self.next_revision += 1;
		if self.undo_s.len() > Self::UNO_LIMIT {
			unsafe { velnosub_abi_close(self.undo_s.remove(0).handle) };
		}
		Self::gecmisi_temizle(&mut self.redo_s);
	}

	fn font_ekle(&mut self, filename: &str) {
		let Some(h) = self.handle else {
			self.note = "font eklemek için bir altyazı aç".into();
			return;
		};
		let Ok(name) = CString::new(filename) else {
			self.note = "geçersiz font yolu".into();
			return;
		};
		let snapshot = unsafe { velnosub_abi_snapshot(h) };
		if snapshot < 0 {
			self.note = last_error();
			return;
		}
		if unsafe { velnosub_abi_attach_font(h, name.as_ptr()) } < 0 {
			self.note = last_error();
			unsafe { velnosub_abi_close(snapshot) };
			return;
		}
		self.gecmise_ekle(snapshot);
		self.son_an = None;
		self.oniz = Oniz::default();
		unsafe { velnosub_abi_render_close(h) };
		self.note = "Yazı tipi ASS belgesine eklendi".into();
	}

	fn load_spelling_file(&mut self, path: &str, rules: bool) {
		let content = match std::fs::read_to_string(path) {
			Ok(content) => content,
			Err(e) => {
				self.note = format!("UTF-8 sözlük dosyası okunamadı: {e}");
				return;
			}
		};
		if rules { self.dictionary_rules = Some(content); }
		else { self.dictionary_words = Some(content); }
		self.misspelling = None;
		self.spelling_open = true;
		let Some((aff, dic)) = self.dictionary_rules.as_ref().zip(self.dictionary_words.as_ref()) else {
			return;
		};
		match spellbook::Dictionary::new(aff, dic) {
			Ok(dictionary) => {
				self.dictionary = Some(dictionary);
				self.note = "Sözlük yüklendi".into();
			}
			Err(e) => self.note = format!("Sözlük yüklenemedi: {e}"),
		}
	}

	fn find_misspelling(&mut self, start_row: usize, start_byte: usize) {
		let Some(dictionary) = self.dictionary.as_ref() else { return };
		self.misspelling = self.rows.iter().enumerate().skip(start_row).find_map(|(row, line)| {
			if line.yorum { return None; }
			let span = crate::spelling::words(&line.text).find(|span| {
				(row != start_row || span.start >= start_byte) && !dictionary.check(&line.text[span.clone()])
			})?;
			let word = line.text[span.clone()].to_owned();
			let mut suggestions = Vec::new();
			dictionary.suggest(&word, &mut suggestions);
			Some(Misspelling {
				row, span, revision: self.current_revision, replacement: word.clone(), word, suggestions,
			})
		});
		self.note = if self.misspelling.is_some() { String::new() } else { "Başka sözcük bulunamadı".into() };
	}

	fn spelling_window(&mut self, ctx: &egui::Context) {
		let mut open = self.spelling_open;
		if !open { return; }
		if self.misspelling.as_ref().is_some_and(|word| word.revision != self.current_revision) {
			self.misspelling = None;
		}
		egui::Window::new("Yazım denetimi").open(&mut open).show(ctx, |ui| {
			ui.horizontal_wrapped(|ui| {
				if ui.button(".aff aç…").clicked() { self.tarayici_ac(SecimTur::DictionaryRules); }
				if ui.button(".dic aç…").clicked() { self.tarayici_ac(SecimTur::DictionaryWords); }
				if ui.add_enabled(self.dictionary.is_some(), egui::Button::new("Baştan tara")).clicked() {
					self.find_misspelling(0, 0);
				}
			});
			if let Some(mut word) = self.misspelling.take() {
				ui.label(format!("Satır {} · {}", word.row + 1, word.word));
				ui.horizontal_wrapped(|ui| {
					for suggestion in word.suggestions.iter().take(6) {
						if ui.button(suggestion).clicked() { word.replacement.clone_from(suggestion); }
					}
				});
				ui.text_edit_singleline(&mut word.replacement);
				if ui.button("Değiştir").clicked() {
					let mut text = self.rows[word.row].text.clone();
					text.replace_range(word.span.clone(), &word.replacement);
					let next = word.span.start + word.replacement.len();
					if self.apply_row_batch(vec![word.row], "Yazım düzeltildi", |handle, row| edit_line(handle, row, &text)) {
						self.find_misspelling(word.row, next);
					} else {
						self.misspelling = Some(word);
					}
				} else if ui.button("Atla").clicked() {
					self.find_misspelling(word.row, word.span.end);
				} else {
					self.misspelling = Some(word);
				}
			}
			ui.label(&self.note);
		});
		self.spelling_open = open;
	}

	fn open_translation(&mut self) {
		let row = self.secili.unwrap_or(0);
		if self.rows.get(row).is_some() {
			self.translation = Some(Translation {
				revision: self.current_revision,
				row,
				text: String::new(),
				drafts: Default::default(),
			});
		}
	}

	fn translation_window(&mut self, ctx: &egui::Context) {
		let Some(mut state) = self.translation.take() else { return };
		if state.revision != self.current_revision { return; }
		let mut cancel = false;
		let mut apply = false;
		let mut next = state.row;
		egui::Modal::new(egui::Id::unique("translation")).show(ctx, |ui| {
			ui.heading("Çeviri asistanı");
			ui.label(format!("Satır {} / {}", state.row + 1, self.rows.len()));
			ui.label(&self.rows[state.row].text);
			if ui.add(TextEdit::multiline(&mut state.text).hint_text("Çeviriyi yaz")).changed() {
				state.drafts.insert(state.row, state.text.clone());
			}
			ui.horizontal_wrapped(|ui| {
				if ui.add_enabled(state.row > 0, egui::Button::new("Önceki")).clicked() { next -= 1; }
				if ui.add_enabled(state.row + 1 < self.rows.len(), egui::Button::new("Sonraki")).clicked() { next += 1; }
				apply = ui.add_enabled(!state.drafts.is_empty(), egui::Button::new("Uygula")).clicked();
				cancel = ui.button("Vazgeç").clicked();
			});
		});
		if apply {
			let rows = state.drafts.keys().copied().collect();
			if !self.apply_row_batch(rows, "Çeviri uygulandı", |handle, row| {
				edit_line(handle, row, &state.drafts[&row])
			}) {
				self.translation = Some(state);
			}
		} else if !cancel {
			if next != state.row {
				state.row = next;
				state.text = state.drafts.get(&next).cloned().unwrap_or_default();
			}
			self.translation = Some(state);
		}
	}

	fn komut_paleti(&mut self, ctx: &egui::Context) {
		let Some(mut query) = self.command_query.take() else { return };
		let mut open = true;
		let selected = egui::Window::new("Komut paleti").open(&mut open).show(ctx, |ui| {
			ui.text_edit_singleline(&mut query);
			let terms = query.to_lowercase();
			let commands: [(&str, fn(&mut Self)); 14] = [
				("Yeni altyazı", Self::request_new_document),
				("Altyazı aç", |app| app.tarayici_ac(SecimTur::Altyazi)),
				("Video aç", |app| app.tarayici_ac(SecimTur::Video)),
				("Ses aç", |app| app.tarayici_ac(SecimTur::Ses)),
				("Kaydet", Self::kaydet),
				("Geri al", Self::geri_al),
				("İleri al", Self::ileri_al),
				("Ara ve değiştir", |app| app.ara_win = true),
				("Çeviri asistanı", Self::open_translation),
				("Yazım denetimi", |app| app.spelling_open = true),
				("Zaman kaydır", |app| app.shift_win = true),
				("Stil yöneticisi", |app| app.stil_win = true),
				("Otomasyon", |app| app.lua_window = true),
				("Yazı tipi ekle", |app| app.tarayici_ac(SecimTur::Font)),
			];
			let mut selected = None;
			for (name, action) in commands {
				if name.to_lowercase().contains(&terms) && ui.button(name).clicked() {
					selected = Some(action);
				}
			}
			selected
		}).and_then(|window| window.inner).flatten();
		if let Some(action) = selected {
			action(self);
		} else if open {
			self.command_query = Some(query);
		}
	}

	fn lua_yukle(&mut self, path: &str) {
		let result = (|| {
			let extension = std::path::Path::new(path).extension()
				.and_then(|ext| ext.to_str()).unwrap_or_default().to_ascii_lowercase();
			if extension != "lua" && extension != "moon" {
				return Err("Lua veya MoonScript dosyası seç (.lua, .moon)".into());
			}
			let source = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
			let source = source.trim_start_matches('\u{feff}');
			let code = if extension == "moon" {
				use std::fmt::Write;
				let mut literal = String::with_capacity(source.len() * 4);
				for byte in source.bytes() {
					write!(literal, "\\{byte:03}").unwrap();
				}
				let compiler = include_str!("../../automation/include/moonscript.lua");
				format!("local moon = (function()\n{compiler}\nend)()\nassert(moon.loadstring(\"{literal}\"))()")
			} else {
				source.to_owned()
			};
			let source = CString::new(code).map_err(|err| err.to_string())?;
			let shim = CString::new(include_str!("../../tools/cross/velnosub-aegisub.lua")).unwrap();
			let mut buf = vec![0u8; 16384];
			let length = unsafe {
				velnosub_abi_list_macros(shim.as_ptr(), source.as_ptr(), buf.as_mut_ptr().cast(), buf.len() as c_int)
			};
			if length < 0 {
				return Err(last_error());
			}
			let names = String::from_utf8_lossy(&buf[..length as usize]).lines().map(str::to_owned).collect();
			#[cfg(target_os = "android")]
			let filters = {
				let length = unsafe {
					velnosub_abi_list_filters(shim.as_ptr(), source.as_ptr(), buf.as_mut_ptr().cast(), buf.len() as c_int)
				};
				if length < 0 { return Err(last_error()); }
				String::from_utf8_lossy(&buf[..length as usize]).lines().map(str::to_owned).collect()
			};
			Ok(LuaScript {
				path: path.to_owned(), source, names,
				#[cfg(target_os = "android")]
				filters,
			})
		})();
		match result {
			Ok(script) => {
				self.note = format!("{} makro yüklendi", script.names.len());
				self.lua_script = Some(script);
			}
			Err(error) => self.note = format!("Script yüklenemedi: {error}"),
		}
		self.lua_window = true;
	}

	fn lua_yonet(&mut self, ctx: &egui::Context) {
		let mut open = self.lua_window;
		if !open { return; }
		egui::Window::new("Otomasyon").open(&mut open).show(ctx, |ui| {
			if ui.button("Lua / MoonScript dosyası aç…").clicked() {
				self.tarayici_ac(SecimTur::Lua);
			}
			if ui.button("Ana harf").clicked() {
				self.makro_uygula();
			}
			if ui.button("Karaoke şablonunu uygula").clicked() {
				let script = CString::new(include_str!("../../automation/autoload/kara-templater.lua")).unwrap();
				self.makro_calistir(&script, "Apply karaoke template", None);
			}
			#[cfg(target_os = "android")]
			if ui.button("Karaoke şablonuyla ASS dışa aktar…").clicked() {
				let script = CString::new(include_str!("../../automation/autoload/kara-templater.lua")).unwrap();
				self.filtreli_disari_aktar(&script, "Karaoke template");
			}

			if let Some(script) = &self.lua_script {
				let path = script.path.clone();
				let mut selected = None;
				ui.separator();
				for name in &script.names {
					if ui.button(name).clicked() {
						selected = Some(name.clone());
					}
				}
				#[cfg(target_os = "android")]
				let filter = script.filters.iter().find(|name| {
					ui.button(format!("{name} ile ASS dışa aktar…")).clicked()
				}).cloned();

				if let Some(name) = selected {
					let source = self.lua_script.as_ref().unwrap().source.clone();
					self.makro_calistir(&source, &name, Some(&path));
				}
				#[cfg(target_os = "android")]
				if let Some(filter) = filter {
					let source = self.lua_script.as_ref().unwrap().source.clone();
					self.filtreli_disari_aktar(&source, &filter);
				}

				if ui.button("Yeniden yükle").clicked() {
					self.lua_yukle(&path);
				}
				if ui.button("Scripti kaldır").clicked() {
					self.lua_script = None;
				}
			}
			ui.label(&self.note);
		});
		self.lua_window = open;
	}

	fn makro_uygula(&mut self) {
		let script = CString::new(include_str!("../../tools/cross/velnosub-demo-macro.lua")).unwrap();
		self.makro_calistir(&script, "Ana harf", None);
	}

	fn makro_calistir(&mut self, script: &CStr, name: &str, script_path: Option<&str>) {
		let selected = self.selected_rows();
		let active = self.secili.filter(|row| selected.contains(row)).or_else(|| selected.first().copied());
		let (Some(h), Some(row)) = (self.handle, active) else {
			self.note = "makro için bir satır seç".into();
			return;
		};
		use std::fmt::Write;
		let mut shim = String::from("aegisub.__mobile_context = { selected = {");
		for row in selected { write!(shim, "{},", row + 1).unwrap(); }
		shim.push_str("},");
		for (key, path) in [
			("script", script_path),
			("video_file", self.video.as_ref().map(|video| video.yol.as_str())),
			("audio_file", self.audio.as_ref().map(|audio| audio.path.as_str())),
		] {
			if let Some(path) = path {
				write!(shim, "{key}=\"").unwrap();
				for byte in path.bytes() { write!(shim, "\\{byte:03}").unwrap(); }
				shim.push_str("\",");
			}
		}
		if let Some(video) = &self.video {
			write!(shim, "position={},zoom={},ratio={},", video.frame, video.olcek_yuzdesi.unwrap_or(100),
				video.preview_ratio.unwrap_or(video.w as f32 / video.hgt as f32)).unwrap();
			for (key, values) in [
				("times", video.external_times.as_ref()),
				("keyframes", video.external_keys.as_ref()),
			] {
				if let Some(values) = values {
					write!(shim, "{key}={{").unwrap();
					for value in values { write!(shim, "{value},").unwrap(); }
					shim.push_str("},");
				}
			}
		}
		shim.push_str("}\n");
		shim.push_str(include_str!("../../tools/cross/velnosub-aegisub.lua"));
		let shim = CString::new(shim).unwrap();
		let Ok(macro_name) = CString::new(name) else { return };
		let path = CString::new(self.path.as_deref().unwrap_or("")).unwrap_or_default();
		let (mut token, mut result) = (-1, -1);
		let status = unsafe {
			velnosub_abi_macro_start(h, row as c_int, shim.as_ptr(), script.as_ptr(),
				macro_name.as_ptr(), path.as_ptr(), self.video.as_ref().map_or(-1, |v| v.handle),
				&mut token, &mut result)
		};
		self.macro_step(name, token, status, result);
	}

	fn macro_step(&mut self, name: &str, token: c_int, status: c_int, next: c_int) {
		match status {
			0 => {
				#[cfg(target_os = "android")]
				if let Some(mut export) = self.macro_export.take() {
					export.document = Some(next);
					if !export.path.is_file() {
						self.note = "Makro kaydedilecek dosyayı yazmadı".into();
						return;
					}
					let path = export.path.clone();
					let title = path.file_name().unwrap_or_default().to_string_lossy();
					self.android_belge_sec(AndroidBelge::MacroSave(export), 3, &path.to_string_lossy(), &title);
					return;
				}
				self.apply_macro_snapshot(name, next);
			}
			1 => match dialog_model(next) {
				Ok(dialog) => self.macro_form = Some(MacroForm { token, name: name.to_owned(), dialog }),
				Err(error) => {
					unsafe { velnosub_abi_macro_abort(token) };
					#[cfg(target_os = "android")]
					{ self.macro_export = None; }
					self.note = format!("Makro formu açılamadı: {error}");
				}
			},
			_ => {
				#[cfg(target_os = "android")]
				{ self.macro_export = None; }
				self.note = format!("Makro hatası: {}", last_error());
			}
		}
	}

	fn apply_macro_snapshot(&mut self, name: &str, next: c_int) {
		let Some(h) = self.handle else {
			unsafe { velnosub_abi_close(next) };
			return;
		};
		self.gecmise_ekle(h);
		self.handle = Some(next);
		unsafe { velnosub_abi_render_close(h) };
		self.son_an = None;
		self.oniz = Oniz::default();
		self.typeset = None;
		self.yerles();
		self.load_styles();
		self.note = format!("{name} makrosu uygulandı");
	}

	fn gecmiste_ilerle(&mut self, geri: bool) {
		let Some(h) = self.handle else { return };
		let (kaynak, hedef) = if geri {
			(&mut self.undo_s, &mut self.redo_s)
		} else {
			(&mut self.redo_s, &mut self.undo_s)
		};
		let Some(an) = kaynak.pop() else {
			self.note = "geçmişte başka adım yok".into();
			return;
		};
		unsafe { velnosub_abi_render_close(h) };
		hedef.push(HistoryEntry { handle: h, revision: self.current_revision });
		self.handle = Some(an.handle);
		self.current_revision = an.revision;
		self.son_an = None;
		self.yerles();
		self.load_styles();
		self.typeset = None;
		self.oniz = Oniz::default();
		self.secili = self.secili.filter(|&i| i < self.rows.len());
		self.note = if geri { "geri alındı" } else { "ileri alındı" }.into();
	}

	fn geri_al(&mut self) {
		self.gecmiste_ilerle(true);
		self.clear_grid_selection();
	}

	fn ileri_al(&mut self) {
		self.gecmiste_ilerle(false);
		self.clear_grid_selection();
	}

	// ---------------- P5.4 gece: satır ekle/sil --------------------------
	fn selected_rows(&self) -> Vec<usize> {
		if self.multi_select {
			self.marked_rows.iter().copied().filter(|&row| row < self.rows.len()).collect()
		} else {
			self.secili.into_iter().filter(|&row| row < self.rows.len()).collect()
		}
	}

	fn select_grid_row(&mut self, row: usize) {
		self.secili = Some(row);
		if self.multi_select && !self.marked_rows.insert(row) {
			self.marked_rows.remove(&row);
		}
	}

	fn clear_grid_selection(&mut self) {
		self.multi_select = false;
		self.marked_rows.clear();
	}

	fn apply_row_batch(&mut self, rows: Vec<usize>, label: &str,
		edit: impl Fn(c_int, usize) -> Result<(), String>,
	) -> bool {
		let Some(h) = self.handle.filter(|_| !rows.is_empty()) else { return false };
		let draft = unsafe { velnosub_abi_snapshot(h) };
		if draft < 0 {
			self.note = last_error();
			return false;
		}
		for &row in rows.iter().rev() {
			if let Err(e) = edit(draft, row) {
				unsafe { velnosub_abi_close(draft) };
				self.note = format!("{label}: {e}");
				return false;
			}
		}
		self.gecmise_ekle(h);
		self.handle = Some(draft);
		unsafe { velnosub_abi_render_close(h) };
		self.son_an = None;
		self.typeset = None;
		self.oniz = Oniz::default();
		self.yerles();
		self.note = format!("{label} · {} satır", rows.len());
		true
	}

	fn toggle_selected_comments(&mut self) {
		let rows = self.selected_rows();
		let comment = rows.iter().any(|&row| !self.rows[row].yorum);
		self.apply_row_batch(rows, "Yorum durumu değiştirildi", |handle, row| {
			meta_set(handle, row, None, None, None, Some(comment), None)
		});
	}

	fn satir_ekle_tetik(&mut self, cogalt: bool) {
		if cogalt && self.multi_select {
			let rows = self.selected_rows();
			if self.apply_row_batch(rows, "Satırlar çoğaltıldı", |handle, row| {
				if unsafe { velnosub_abi_duplicate_line(handle, row as c_int) } == 0 {
					Ok(())
				} else {
					Err(last_error())
				}
			}) {
				self.clear_grid_selection();
			}
			return;
		}
		let Some(h) = self.handle else {
			self.note = "açık dosya yok".into();
			return;
		};
		let i = self.secili.unwrap_or(0);
		let idx = (i + 1).min(self.rows.len());
		if cogalt && i >= self.rows.len() {
			self.note = "satır yok".into();
			return;
		}
		if !self.an_tut(None) {
			return;
		}
		let sonuc = if cogalt {
			if unsafe { velnosub_abi_duplicate_line(h, i as c_int) } == 0 {
				Ok(())
			} else {
				Err(last_error())
			}
		} else {
			satir_ekle(h, idx, "", "", "", "")
		};
		match sonuc {
			Ok(()) => {
				self.yerles();
				self.secili = Some(idx);
				self.clear_grid_selection();
				self.note = if cogalt {
					format!("satır çoğaltıldı → {}", idx + 1)
				} else {
					format!("satır eklendi → {}", idx + 1)
				};
			}
			Err(e) => self.note = format!("satır eklenemedi: {e}"),
		}
	}

	fn satir_sil_tetik(&mut self) {
		if self.multi_select {
			let rows = self.selected_rows();
			if self.apply_row_batch(rows, "Satırlar silindi", satir_sil) {
				self.secili = None;
				self.clear_grid_selection();
			}
			return;
		}
		let Some(h) = self.handle else {
			self.note = "açık dosya yok".into();
			return;
		};
		let Some(i) = self.secili else {
			self.note = "satır seç".into();
			return;
		};
		if !self.an_tut(None) { return; }
		match satir_sil(h, i) {
			Ok(()) => {
				self.yerles();
				self.secili = None;
				self.note = format!("satır {} silindi", i + 1);
			}
			Err(e) => self.note = format!("silinemedi: {e}"),
		}
	}

	// ---------------- P5.4 gece: Ara-Bul / Değiştir ----------------------
	fn ara_ileri(&mut self) {
		let terim = self.ara_metin.to_lowercase();
		if terim.is_empty() {
			self.note = "aranacak dizge boş".into();
			return;
		}
		let n = self.rows.len();
		if n == 0 {
			return;
		}
		for k in 1..=n {
			let ix = (self.ara_i + k) % n;
			if self.rows[ix].text.to_lowercase().contains(&terim) {
				self.ara_i = ix;
				self.secili = Some(ix);
				self.note = format!("bulundu: satır {}", ix + 1);
				return;
			}
		}
		self.note = "bundan başka benzemez hiçbiri yok".into();
	}

	fn ara_degistir(&mut self) {
		let Some(i) = self.secili else {
			self.note = "satır seç".into();
			return;
		};
		let (ara, yeni) = (self.ara_metin.clone(), self.ara_yeni.clone());
		if ara.is_empty() {
			self.note = "aranacak dizge boş".into();
			return;
		}
		let Some(r) = self.rows.get(i) else {
			return;
		};
		if !r.text.contains(&ara) {
			self.note = "seçili satırda benzemez yok".into();
			return;
		}
		let t = r.text.replace(&ara, &yeni);
		self.ara_i = i;
		// an'ı zorla: tek değişim tek geri-adım olsun
		self.son_an = None;
		self.yazlari_uygula(vec![Yazi::MetinVal(i, t)]);
		self.note = format!("değiştirildi: satır {}", i + 1);
	}

	fn ara_degistir_tum(&mut self) {
		let (ara, yeni) = (self.ara_metin.clone(), self.ara_yeni.clone());
		if ara.is_empty() {
			self.note = "aranacak dizge boş".into();
			return;
		}
		let mut bek: Vec<Yazi> = Vec::new();
		for (i, r) in self.rows.iter().enumerate() {
			if r.text.contains(&ara) {
				bek.push(Yazi::MetinVal(i, r.text.replace(&ara, &yeni)));
			}
		}
		let k = bek.len();
		self.son_an = None;
		self.yazlari_uygula(bek);
		self.note = if k > 0 {
			format!("{k} satır değiştirildi")
		} else {
			"benzemez yok".into()
		};
	}

	// ---------------- P5.4 gece: zaman kaydırma --------------------------
	fn zaman_kaydir(&mut self, d: i32) {
		if d == 0 {
			self.note = "kaydırma sıfır".into();
			return;
		}
		let rows: Vec<usize> = if self.shift_tum {
			(0..self.rows.len()).collect()
		} else {
			self.selected_rows()
		};
		let mut times = std::collections::BTreeMap::new();
		for row in rows {
			let (Ok(start), Ok(end)) = (ass_ms(&self.rows[row].bas), ass_ms(&self.rows[row].son)) else {
				self.note = format!("{} numaralı satırın zamanı geçersiz", row + 1);
				return;
			};
			times.insert(row, ((start + i64::from(d)).max(0), (end + i64::from(d)).max(0)));
		}
		if times.is_empty() {
			self.note = "Kaydırılacak satır seç".into();
			return;
		}
		self.apply_row_batch(times.keys().copied().collect(), &format!("Kaydırıldı: {d} ms"), |handle, row| {
			let (start, end) = times[&row];
			times_set(handle, row, start, end)
		});
	}

	// ---------------- P5.4 gece: paylaşımlı diyalog pencereleri ---------
	// masaüstü + mobil aynı; Ayarlar penceresi de buraya taşındı
	/// P5.5: tarayıcıyı istenen türle açar
	fn tarayici_ac(&mut self, tur: SecimTur) {
		#[cfg(target_os = "android")]
		{
			let kind = match tur {
				SecimTur::Altyazi => 0,
				SecimTur::Video => 1,
				SecimTur::Ses => 2,
				SecimTur::Lua => 4,
				SecimTur::Font => 5,
				SecimTur::Timecodes => 6,
				SecimTur::Keyframes => 7,
				SecimTur::DictionaryRules => 8,
				SecimTur::DictionaryWords => 9,
			};
			self.android_belge_sec(AndroidBelge::Ac(tur), kind, "", "");
		}
		#[cfg(not(target_os = "android"))]
		{
			self.tarayici = Tarayici {
				acik: true,
				tur,
				yol: std::env::current_dir().unwrap_or_else(|_| ".".into()),
				secili: None,
				hata: None,
			};
		}
	}

	/// P5.5: tarayıcı penceresi (diyaloglar'dan çizilir — iki yüzeyde ortak)
	fn tarayici_pencere(&mut self, ui: &mut egui::Ui) {
		if !self.tarayici.acik {
			return;
		}
		let mut ap = self.tarayici.acik;
		egui::Window::new("Dosya seç")
			.open(&mut ap)
			.resizable(true)
			.default_width(430.0)
			.default_height(370.0)
			.show(ui.ctx(), |p| {
				p.horizontal(|h| {
					// hızlı köprüler
					for (ad, ks) in [
						("Ana", "/storage/emulated/0"),
						("Download", "/storage/emulated/0/Download"),
						("Filmler", "/storage/emulated/0/Movies"),
						("Müzik", "/storage/emulated/0/Music"),
						("DCIM", "/storage/emulated/0/DCIM"),
					] {
						if h.small_button(ad).clicked() {
							self.tarayici.yol = std::path::PathBuf::from(ks);
							self.tarayici.secili = None;
							self.tarayici.hata = None;
						}
					}
					if h.button("yukarı").clicked() {
						self.tarayici.yol.pop();
						self.tarayici.secili = None;
					}
				});
				p.label(
					RichText::new(self.tarayici.yol.display().to_string())
						.weak()
						.size(11.0),
				);
				p.add_space(2.0);
				let uz = self.tarayici.uzantilar();
				let mut girdiler: Vec<(std::path::PathBuf, bool)> = Vec::new();
				match std::fs::read_dir(&self.tarayici.yol) {
					Ok(gk) => {
						for g in gk.flatten() {
							girdiler.push((g.path(), g.path().is_dir()));
						}
					}
					Err(e) => {
						self.tarayici.hata = Some(format!(
							"{} okunamadı: {}",
							self.tarayici.yol.display(),
							e
						));
					}
				}
				girdiler.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
				egui::ScrollArea::vertical().show(p, |s| {
					for (yolq, dizin) in &girdiler {
						let ad = yolq
							.file_name()
							.map(|f| f.to_string_lossy().into_owned())
							.unwrap_or_else(|| yolq.display().to_string());
						if *dizin {
							if s.button(RichText::new(format!("{}/", ad)).strong())
								.clicked()
							{
								self.tarayici.yol = yolq.clone();
								self.tarayici.secili = None;
							}
						} else {
							let uyum = yolq
								.extension()
								.and_then(|x| x.to_str())
								.is_some_and(|x| uz.contains(&x.to_ascii_lowercase().as_str()));
							if uyum {
								if s.button(ad.clone()).clicked() {
									self.tarayici.secili =
										Some(yolq.to_string_lossy().into_owned());
								}
							} else {
								s.label(RichText::new(ad).weak().size(11.0));
							}
						}
					}
				});
				p.add_space(3.0);
				let tur_ad = match self.tarayici.tur {
					SecimTur::Altyazi => "AÇ (altyazı)",
					SecimTur::Video => "AÇ (video)",
					SecimTur::Ses => "AÇ (ses)",
					SecimTur::Lua => "YÜKLE (script)",
					SecimTur::Font => "EKLE (font)",
					SecimTur::Timecodes | SecimTur::Keyframes |
					SecimTur::DictionaryRules | SecimTur::DictionaryWords => "Aç",
				};
				p.horizontal(|h| {
					if h.add(
						egui::Button::new(tur_ad).min_size(egui::vec2(120.0, 36.0)),
					)
					.clicked()
					{
						if let Some(yolc) = self.tarayici.secili.clone() {
							match self.tarayici.tur {
								SecimTur::Altyazi => {
									if let Err(e) = self.belge_ac(&yolc) {
										self.note = format!("açılamadı: {e}");
									}
								}
								SecimTur::Video => self.video_ac(&yolc),
								SecimTur::Ses => self.ses_ac(&yolc),
								SecimTur::Lua => self.lua_yukle(&yolc),
								SecimTur::Font => self.font_ekle(&yolc),
								SecimTur::Timecodes | SecimTur::Keyframes => self.load_video_metadata(&yolc, matches!(self.tarayici.tur, SecimTur::Timecodes)),
								SecimTur::DictionaryRules | SecimTur::DictionaryWords => self.load_spelling_file(&yolc, matches!(self.tarayici.tur, SecimTur::DictionaryRules)),
							}
							self.tarayici.acik = false;
						} else {
							self.tarayici.hata = Some("önce listeden dosya seç".into());
						}
					}
					if let Some(ht) = &self.tarayici.hata {
						h.label(RichText::new(ht.clone()).color(egui::Color32::ORANGE).size(12.0));
					}
				});
			});
		self.tarayici.acik = ap;
	}

	fn diyaloglar(&mut self, ui: &mut egui::Ui) {
		self.tarayici_pencere(ui);
		if self.ara_win {
			let mut ap = self.ara_win;
			egui::Window::new("Ara-Bul")
				.open(&mut ap)
				.resizable(false)
				.show(ui.ctx(), |su| {
					su.horizontal(|h| {
						h.label("ara:");
						h.text_edit_singleline(&mut self.ara_metin);
					});
					su.horizontal(|h| {
						h.label("yeni:");
						h.text_edit_singleline(&mut self.ara_yeni);
					});
					su.horizontal(|h| {
						if h.button("Bul (ileri)").clicked() {
							self.ara_ileri();
						}
						if h.button("Değiştir").clicked() {
							self.ara_degistir();
						}
						if h.button("Tümünü değiştir").clicked() {
							self.ara_degistir_tum();
						}
					});
					su.add_space(4.0);
					let n = self.note.clone();
					su.label(RichText::new(n).weak());
				});
			self.ara_win = ap;
		}
		if self.shift_win {
			let mut sp = self.shift_win;
			egui::Window::new("Zaman kaydır")
				.open(&mut sp)
				.resizable(false)
				.show(ui.ctx(), |su| {
					su.horizontal(|h| {
						h.label("ms:");
						h.add(egui::DragValue::new(&mut self.shift_ms).speed(100.0));
					});
					su.horizontal(|h| {
						if h.button("− 1 sn").clicked() {
							self.shift_ms -= 1000;
						}
						if h.button("+ 1 sn").clicked() {
							self.shift_ms += 1000;
						}
					});
					su.checkbox(&mut self.shift_tum, "tüm satırlar (kapalı: seçili)");
					su.horizontal(|h| {
						if h.button("geri kaydır").clicked() {
							self.zaman_kaydir(-self.shift_ms);
						}
						if h.button("ileri kaydır").clicked() {
							self.zaman_kaydir(self.shift_ms);
						}
					});
				});
			self.shift_win = sp;
		}
		if self.ayar_win {
			let mut ap = self.ayar_win;
			egui::Window::new("Ayarlar")
				.open(&mut ap)
				.resizable(false)
				.show(ui.ctx(), |su| {
					self.ayarlar_icerik(su);
				});
			self.ayar_win = ap;
		}
		self.stil_penceresi(ui);
		self.ozellik_penceresi(ui);
	}

	/// P8.1: dosya özellikleri — yol/satır/stil/video/ses özeti (Aegisub
	/// File → Properties portu; ABI'de özellik setter'ı olmadığından salt okur).
	fn ozellik_penceresi(&mut self, ui: &mut egui::Ui) {
		if !self.ozellik_win {
			return;
		}
		let mut acik = self.ozellik_win;
		egui::Window::new(format!("{} Dosya Özellikleri", ikon::INFO))
			.open(&mut acik)
			.default_width(380.0)
			.resizable(false)
			.show(ui.ctx(), |su| {
				egui::Grid::new("ozellik_grid")
					.num_columns(2)
					.spacing([12.0, 4.0])
					.show(su, |g| {
						g.label("dosya");
						g.label(
							self.path
								.clone()
								.unwrap_or_else(|| "—".into()),
						);
						g.end_row();
						g.label("satır");
						g.label(format!("{}", self.rows.len()));
						g.end_row();
						g.label("stil");
						g.label(format!("{}", self.stiller.len()));
						g.end_row();
						if let Some(v) = self.video.as_ref() {
							g.label("video");
							g.label(format!(
								"{}×{} · {:.3} fps · {} kare",
								v.w, v.hgt, v.fps, v.kareler
							));
							g.end_row();
							g.label("medya");
							g.label(v.info.clone());
							g.end_row();
						}
						if let Some(a) = self.audio.as_ref() {
							g.label("ses");
							g.label(format!(
								"{} Hz · {} kanal · {} örnek",
								a.rate, a.ch, a.toplam
							));
							g.end_row();
							g.label("ses bilgisi");
							g.label(a.info.clone());
							g.end_row();
						}
						g.label("kayıt");
						g.label(
							self.save_path
								.clone()
								.unwrap_or_else(|| "—".into()),
						);
						g.end_row();
					});
			});
		self.ozellik_win = acik;
	}

	/// P8.1: seçili satırın baş/bitişini video kare ızgarasına oturt
	/// (Aegisub Timing menüsü karşılığı → kare = round(ms·fps/1000)).
	fn kareye_oturt(&mut self, bas: bool, son: bool) {
		let Some(h) = self.handle else {
			self.note = "kareye oturt: dosya yok".into();
			return;
		};
		let Some(video) = self.video.as_ref() else {
			self.note = "kareye oturt: video yok".into();
			return;
		};
		let Some(i) = self.secili else {
			self.note = "kareye oturt: satır seç".into();
			return;
		};
		let Some(sat) = self.rows.get(i) else {
			return;
		};
		let mb = ass_ms(&sat.bas).unwrap_or(0);
		let ms = ass_ms(&sat.son).unwrap_or(0);
		let yb = if bas { video.en_yakin_kare_zamani(mb) } else { mb };
		let ys = if son { video.en_yakin_kare_zamani(ms) } else { ms };
		if ys <= yb {
			self.note = "kareye oturt: geçersiz süre".into();
			return;
		}
		if !self.an_tut(None) { return; }
		match times_set(h, i, yb, ys) {
			Ok(()) => {
				self.yerles();
				self.note = format!("kareye oturtuldu: {yb}–{ys} ms");
			}
			Err(e) => self.note = format!("kareye oturt: {e}"),
		}
	}

	/// P8.1: tam stil yöneticisi — liste + 13 hücrenin tamamı (Aegisub
	/// Stil Yöneticisi portu). Her alan değişimi style_write hattına girer.
	fn stil_penceresi(&mut self, ui: &mut egui::Ui) {
		if !self.stil_win {
			return;
		}
		let mut acik = self.stil_win;
		let mut sec = self.stil_sec;
		let h = self.handle;
		egui::Window::new(format!("{} Stil Yöneticisi", ikon::PALETTE))
			.open(&mut acik)
			.default_width(640.0)
			.resizable(true)
			.show(ui.ctx(), |su| {
				su.horizontal(|sat| {
					sat.vertical(|v| {
						v.set_min_width(170.0);
						v.label(
							RichText::new(format!("{} stiller", ikon::SUBTITLES))
								.strong(),
						);
						egui::ScrollArea::vertical()
							.max_height(320.0)
							.show(v, |sv| {
								for (i, sr) in
									self.stiller.iter().enumerate()
								{
									if sv.selectable_label(
										i == sec,
										format!("{} {}", ikon::PALETTE, sr.name),
									)
									.clicked()
									{
										sec = i;
									}
								}
							});
						v.add_space(4.0);
						v.horizontal(|b| {
							if b.button(format!("{} yeni", ikon::ADD)).clicked() {
								if let Some(hh) = h {
									let ad = format!(
										"Yeni Stil {}",
										self.stiller.len() + 1
									);
									if let Ok(c) = CString::new(ad) {
									if !self.an_tut(None) { return; }
										let rc = unsafe {
											velnosub_abi_style_add(hh, c.as_ptr())
										};
										if rc >= 0 {
											self.load_styles();
											sec = rc as usize;
											self.note = "yeni stil eklendi".into();
										} else {
											self.note = format!(
												"stil eklenemedi: {}",
												last_error()
											);
										}
									}
								}
							}
							if b.button(format!("{} kopyala", ikon::COPY)).clicked() {
								if let (Some(hh), Some(s0)) =
									(h, self.stiller.get(sec).cloned())
								{
									let ad = format!("{} kopya", s0.name);
									if let Ok(c) = CString::new(ad.clone()) {
									if !self.an_tut(None) { return; }
										let rc = unsafe {
											velnosub_abi_style_add(hh, c.as_ptr())
										};
										if rc >= 0 {
											let mut hc = stil_hucreleri(&s0);
											hc[0] = ad;
											let metin = hc.join("|");
											if let Ok(cm) = CString::new(metin) {
												unsafe {
													velnosub_abi_style_write(
														hh, rc, cm.as_ptr(),
													)
												};
											}
											self.load_styles();
											sec = rc as usize;
											self.note = "stil kopyalandı".into();
										} else {
											self.note = format!(
												"kopya eklenemedi: {}",
												last_error()
											);
										}
									}
								}
							}
							if b.button(format!("{} sil", ikon::DELETE)).clicked() {
								if let Some(hh) = h {
									if !self.an_tut(None) { return; }
									let rc = unsafe {
										velnosub_abi_style_delete(
											hh, sec as c_int,
										)
									};
									if rc == 0 {
										self.load_styles();
										sec = sec.saturating_sub(1);
										self.note = "stil silindi".into();
									} else {
										self.note = format!(
											"stil silinemedi: {}",
											last_error()
										);
									}
								}
							}
						});
					});
					sat.separator();
					sat.vertical(|v| {
						let Some(s0) = self.stiller.get(sec).cloned() else {
							v.label("stil seç");
							return;
						};
						let mut ad = s0.name.clone();
						v.horizontal(|r| {
							r.label(format!("{} ad", ikon::TEXT));
							if r.add(
								TextEdit::singleline(&mut ad)
									.desired_width(180.0),
							)
							.lost_focus()
								&& ad != s0.name
								&& !ad.trim().is_empty()
							{
								self.stil_alan_yaz(sec, 0, ad.clone());
							}
						});
						let mut font = s0.font.clone();
						v.horizontal(|r| {
							r.label("font");
							if r.add(
								TextEdit::singleline(&mut font)
									.desired_width(180.0),
							)
							.lost_focus()
								&& font != s0.font
								&& !font.trim().is_empty()
							{
								self.stil_alan_yaz(sec, 1, font.clone());
							}
						});
						let mut boy = s0.size;
						v.horizontal(|r| {
							r.label("boyut");
							if r.add(
								egui::DragValue::new(&mut boy)
									.speed(0.5)
									.range(2.0f64..=9999.0)
									.suffix(" pts"),
							)
							.changed()
							{
								self.stil_alan_yaz(
									sec,
									2,
									format!("{}", boy),
								);
							}
						});
						v.horizontal(|r| {
							r.label("renkler:");
							for (etiket, hucre, kaynak) in [
								("ana", 3usize, &s0.primary),
								("ikincil", 4, &s0.secondary),
								("kontur", 5, &s0.outline),
								("gölge", 6, &s0.shadow),
							] {
								let mut c = ass_to_color32(kaynak)
									.unwrap_or(Color32::WHITE);
								if r.color_edit_button_srgba(&mut c)
									.changed()
								{
									self.stil_alan_yaz(
										sec,
										hucre,
										color32_to_ass(c),
									);
								}
								r.label(RichText::new(etiket).size(10.0));
							}
						});
						let bay: Vec<bool> = s0
							.flags
							.split(',')
							.map(|b| b.trim() == "1")
							.collect();
						let mut kb = *bay.first().unwrap_or(&false);
						let mut ki = *bay.get(1).unwrap_or(&false);
						let mut ku = *bay.get(2).unwrap_or(&false);
						let mut ks = *bay.get(3).unwrap_or(&false);
						v.horizontal(|r| {
							r.label("biçim:");
							let degisti = r
								.checkbox(
									&mut kb,
									format!("{} kalın", ikon::BOLD),
								)
								.changed()
								| r.checkbox(
									&mut ki,
									format!("{} italik", ikon::ITALIC),
								)
								.changed()
								| r.checkbox(
									&mut ku,
									format!("{} altçizgi", ikon::UNDERLINE),
								)
								.changed()
								| r.checkbox(
									&mut ks,
									format!("{} üstçizgi", ikon::STRIKE),
								)
								.changed();
							if degisti {
								let f = format!(
									"{},{},{},{}",
									kb as u8,
									ki as u8,
									ku as u8,
									ks as u8
								);
								self.stil_alan_yaz(sec, 7, f);
							}
						});
						v.horizontal(|r| {
							r.label("hizalama");
							let mut hz = s0.alignment;
							egui::ComboBox::from_id_salt("stil_hiza")
								.selected_text(hiza_ad(hz))
								.show_ui(r, |cb| {
									for i in 1..=9 {
										cb.selectable_value(
											&mut hz,
											i,
											hiza_ad(i),
										);
									}
								});
							if hz != s0.alignment {
								self.stil_alan_yaz(
									sec,
									8,
									hz.to_string(),
								);
							}
						});
						let mar: Vec<i32> = s0
							.margins
							.split(',')
							.map(|x| x.trim().parse().unwrap_or(0))
							.collect();
						let mut ml = *mar.first().unwrap_or(&0);
						let mut mr = *mar.get(1).unwrap_or(&0);
						let mut mv = *mar.get(2).unwrap_or(&0);
						v.horizontal(|r| {
							r.label("kenar L/R/D");
							let d = r
								.add(
									egui::DragValue::new(&mut ml)
										.range(0..=9999),
								)
								.changed()
								| r.add(
									egui::DragValue::new(&mut mr)
										.range(0..=9999),
								)
								.changed()
								| r.add(
									egui::DragValue::new(&mut mv)
										.range(0..=9999),
								)
								.changed();
							if d {
								self.stil_alan_yaz(
									sec,
									9,
									format!("{ml},{mr},{mv}"),
								);
							}
						});
						let ol: Vec<f64> = s0
							.scale
							.split(',')
							.map(|x| x.trim().parse().unwrap_or(100.0))
							.collect();
						let mut ox = *ol.first().unwrap_or(&100.0);
						let mut oy = *ol.get(1).unwrap_or(&100.0);
						let mut hr = *ol.get(2).unwrap_or(&0.0);
						let mut ac = *ol.get(3).unwrap_or(&0.0);
						v.horizontal(|r| {
							r.label("ölçek/boşluk/açı");
							let d = r
								.add(
									egui::DragValue::new(&mut ox)
										.speed(1.0)
										.range(1.0..=1000.0)
										.suffix("%x"),
								)
								.changed()
								| r.add(
									egui::DragValue::new(&mut oy)
										.speed(1.0)
										.range(1.0..=1000.0)
										.suffix("%y"),
								)
								.changed()
								| r.add(
									egui::DragValue::new(&mut hr)
										.speed(0.2)
										.range(-99.0..=99.0),
								)
								.changed()
								| r.add(
									egui::DragValue::new(&mut ac)
										.speed(0.5)
										.suffix("°"),
								)
								.changed();
							if d {
								self.stil_alan_yaz(
									sec,
									11,
									format!("{ox},{oy},{hr},{ac}"),
								);
							}
						});
						let ke: Vec<f64> = s0
							.edge
							.split(',')
							.map(|x| x.trim().parse().unwrap_or(0.0))
							.collect();
						let mut kst = ke.first().copied().unwrap_or(1.0).max(1.0);
						let mut kg = *ke.get(1).unwrap_or(&2.0);
						let mut gg = *ke.get(2).unwrap_or(&2.0);
						v.horizontal(|r| {
							r.label("kenarlık");
							let d = r
								.add(
									egui::DragValue::new(&mut kst)
										.range(1..=3),
								)
								.changed()
								| r.add(
									egui::DragValue::new(&mut kg)
										.speed(0.2)
										.range(0.0..=99.0)
										.prefix("k "),
								)
								.changed()
								| r.add(
									egui::DragValue::new(&mut gg)
										.speed(0.2)
										.range(0.0..=99.0)
										.prefix("g "),
								)
								.changed();
							if d {
								self.stil_alan_yaz(
									sec,
									12,
									format!("{kst},{kg},{gg}"),
								);
							}
						});
						v.horizontal(|r| {
							r.label("kodlama");
							let mut kod = s0.encoding;
							if r.add(
								egui::DragValue::new(&mut kod)
									.range(0..=255),
							)
							.changed()
							{
								self.stil_alan_yaz(
									sec,
									10,
									kod.to_string(),
								);
							}
						});
					});
				});
			});
		self.stil_win = acik;
		if self.stiller.is_empty() {
			self.stil_sec = 0;
		} else {
			self.stil_sec = sec.min(self.stiller.len() - 1);
		}
	}

	/// P8.1: tek stil hücresini yaz + listeyi tazele.
	fn stil_alan_yaz(&mut self, idx: usize, alan: usize, deger: String) {
		let Some(h) = self.handle else {
			self.note = "stil: açık dosya yok".into();
			return;
		};
		if !self.an_tut(None) { return; }
		match stil_yaz(h, idx as c_int, |hc| {
			if let Some(hucre) = hc.get_mut(alan) {
				*hucre = deger;
			}
		}) {
			Ok(()) => self.load_styles(),
			Err(e) => self.note = format!("stil yazılamadı: {e}"),
		}
	}

	fn load(&mut self, path: &str) -> Result<(), String> {
		if !INIT_OK.get().copied().unwrap_or(false) {
			return Err("init olmadı (config yolu?)".into());
		}
		if !std::fs::metadata(path).map_err(|error| error.to_string())?.is_file() {
			return Err("Altyazı yolu bir dosya değil".into());
		}
		let in_c = CString::new(path.to_string()).map_err(|e| e.to_string())?;
		let h = document_io_result(
			unsafe { velnosub_ui_open_document(in_c.as_ptr()) },
			"Altyazı dosyası okunamadı",
		)?;
		let ev = unsafe { velnosub_abi_event_count(h) };
		if ev < 0 {
			let e = last_error();
			unsafe { velnosub_abi_close(h) };
			return Err(e);
		}
		let st = unsafe { velnosub_abi_style_count(h) };
		self.activate_document(h, Some(path));
		self.note = format!("{path} — {ev} satır, {st} stil");
		Ok(())
	}

	fn request_new_document(&mut self) {
		if self.belge_degisti() {
			self.bekleyen_belge = Some(PendingDocumentAction::New);
		} else {
			self.new_document();
		}
	}

	fn new_document(&mut self) {
		let handle = unsafe { velnosub_abi_new_document() };
		if handle < 0 {
			self.note = format!("Yeni altyazı oluşturulamadı: {}", last_error());
			return;
		}
		self.activate_document(handle, None);
		self.note = "Yeni altyazı".into();
	}

	fn activate_document(&mut self, h: c_int, path: Option<&str>) {
		if let Some(onceki) = self.handle.replace(h) {
			unsafe { velnosub_abi_close(onceki) };
		}
		self.yerles();
		self.secili = None;
		self.clear_grid_selection();
		self.misspelling = None;
		self.translation = None;
		self.son_an = None;
		self.typeset = None;
		self.oniz = Oniz::default();
		Self::gecmisi_temizle(&mut self.undo_s);
		Self::gecmisi_temizle(&mut self.redo_s);
		if path.is_none() || self.path.as_deref() != path {
			self.save_path = None;
			#[cfg(target_os = "android")]
			if let Some(snapshot) = self.android_saved_snapshot.take() {
				Self::remove_saf_stage(&snapshot);
			}
		}
		self.path = path.map(str::to_owned);
		self.current_revision = 0;
		self.saved_revision = 0;
		self.next_revision = 1;
		self.load_styles();
	}

	/// P4.5: stilleri ABI dizgesinden ayıkla; load/yeniden-aç sonrası taze.
	fn load_styles(&mut self) {
		self.stiller.clear();
		let Some(h) = self.handle else {
			return;
		};
		let st = unsafe { velnosub_abi_style_count(h) };
		if st <= 0 {
			return;
		}
		for i in 0..st {
			let Ok(t) = style_of(h, i) else {
				continue;
			};
			let p: Vec<&str> = t.split('|').collect();
			if p.len() < 13 {
				continue;
			}
			self.stiller.push(StyleRow {
				name: p[0].into(),
				font: p[1].into(),
				size: p[2].parse().unwrap_or(0.0),
				primary: p[3].into(),
				secondary: p[4].into(),
				outline: p[5].into(),
				shadow: p[6].into(),
				flags: p[7].into(),
				alignment: p[8].parse().unwrap_or(0),
				margins: p[9].into(),
				encoding: p[10].parse().unwrap_or(0),
				scale: p[11].into(),
				edge: p[12].into(),
			});
		}
	}

	fn belge_ac(&mut self, path: &str) -> Result<(), String> {
		if self.belge_degisti() {
			self.bekleyen_belge = Some(PendingDocumentAction::Open(path.to_owned()));
			return Ok(());
		}
		self.load(path)
	}

	fn kaydet(&mut self) {
		if let Some(path) = self.save_path.clone() {
			self.save_to(&path);
			return;
		}
		self.kayit_yeri_sec();
	}

	fn belge_bicimi(&self) -> &'static str {
		let extension = self.path.as_deref()
			.and_then(|path| std::path::Path::new(path).extension())
			.and_then(|extension| extension.to_str());
		if extension.is_some_and(|extension| extension.eq_ignore_ascii_case("srt")) {
			"srt"
		} else {
			"ass"
		}
	}

	fn kayit_yeri_sec(&mut self) {
		self.kayit_bicimi(self.belge_bicimi(), false);
	}

	fn kayit_bicimi(&mut self, format: &str, _export_only: bool) {
		let varsayilan = self.path.as_deref()
			.and_then(|path| std::path::Path::new(path).file_stem())
			.and_then(|name| name.to_str()).unwrap_or("altyazi");
		#[cfg(target_os = "android")]
		{
			let Some(h) = self.handle else {
				self.note = "kayıt: açık dosya yok".into();
				return;
			};
			self.saf_yaz(h, &varsayilan.to_owned(), format, _export_only);
		}
		#[cfg(not(target_os = "android"))]
		{
			let dizin = std::env::var("VELNOSUB_EXTDIR").unwrap_or_else(|_| ".".into());
			self.kayit_hedefi = Some(std::path::Path::new(&dizin)
				.join(format!("{varsayilan}-duzenlenen.{format}")).display().to_string());
		}
	}

	#[cfg(target_os = "android")]
	fn saf_yaz(&mut self, h: c_int, name: &str, format: &str, export_only: bool) {
		if self.android_belge.is_some() { return; }

		let dir = std::env::var_os("VELNOSUB_CACHE").map(std::path::PathBuf::from)
			.unwrap_or_else(std::env::temp_dir);
		static STAGE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
		let id = STAGE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		let folder = dir.join(format!("velnosub-export-{}-{id}", std::process::id()));
		let name = format!("{name}.{format}");
		let path = folder.join(&name);
		let Ok(cpath) = CString::new(path.as_os_str().as_encoded_bytes()) else { return };
		if let Err(error) = std::fs::create_dir(&folder) {
			self.note = format!("Kayıt hazırlığı başarısız: {error}");
			return;
		}
		if let Err(error) = write_document(h, &cpath) {
			self.note = format!("kayıt hatası: {error}");
			Self::remove_saf_stage(&path);
			return;
		}
		self.android_belge_sec(AndroidBelge::Kaydet(path.clone(), export_only), 3, &path.to_string_lossy(), &name);
		if self.android_belge.is_none() {
			Self::remove_saf_stage(&path);
		}
	}

	#[cfg(target_os = "android")]
	fn remove_saf_stage(path: &std::path::Path) {
		let _ = std::fs::remove_file(path);
		if let Some(folder) = path.parent() {
			let _ = std::fs::remove_dir(folder);
		}
	}

	#[cfg(target_os = "android")]
	fn filtreli_disari_aktar(&mut self, script: &CStr, name: &str) {
		let Some(h) = self.handle else {
			self.note = "Dışa aktarmak için altyazı aç".into();
			return;
		};
		let shim = CString::new(include_str!("../../tools/cross/velnosub-aegisub.lua")).unwrap();
		let path = CString::new(self.path.as_deref().unwrap_or("")).unwrap_or_default();
		let Ok(filter) = CString::new(name) else { return };
		let copy = unsafe {
			velnosub_abi_run_script(h, -1, shim.as_ptr(), script.as_ptr(), filter.as_ptr(), path.as_ptr(),
				self.video.as_ref().map_or(-1, |v| v.handle), 1)
		};
		if copy < 0 {
			self.note = format!("Filtre hatası: {}", last_error());
			return;
		}
		self.saf_yaz(copy, "altyazi-filtreli", "ass", true);
		unsafe { velnosub_abi_close(copy) };
	}

	#[cfg(target_os = "android")]
	fn android_belge_sec(&mut self, belge: AndroidBelge, kind: c_int, path: &str, title: &str) {
		if self.android_belge.is_some() {
			return;
		}
		let app = crate::DOCUMENT_ACTIVITY.lock().unwrap();
		let Some(app) = app.as_ref() else { return };
		let (Ok(path), Ok(title)) = (CString::new(path), CString::new(title)) else { return };
		let rc = unsafe { velnosub_abi_document_pick(
			app.vm_as_ptr(), app.activity_as_ptr(), kind, path.as_ptr(), title.as_ptr(),
		) };
		if rc == 0 {
			self.android_belge = Some(belge);
		} else {
			self.note = "Android belge seçicisi açılamadı".into();
		}
	}

	#[cfg(target_os = "android")]
	fn android_belge_sonucu(&mut self, ctx: &egui::Context) {
		if self.android_belge.is_none() {
			return;
		}
		let mut buf = [0u8; 4096];
		let status = unsafe { velnosub_abi_document_poll(buf.as_mut_ptr().cast(), buf.len()) };
		if status == 0 {
			egui::Modal::new(egui::Id::unique("android_belge")).show(ctx, |ui| {
				ui.spinner();
				ui.label("Dosya işlemi bekleniyor…");
			});
			ctx.request_repaint_after(std::time::Duration::from_millis(120));
			return;
		}
		let belge = self.android_belge.take().unwrap();
		if status != 1 {
			if let AndroidBelge::Kaydet(path, _) = &belge {
				Self::remove_saf_stage(path);
			}
			self.note = if status == 2 { "dosya işlemi iptal edildi" } else { "dosya okunamadı veya yazılamadı" }.into();
			if status == 2 && matches!(belge, AndroidBelge::MacroOpen) {
				if let Some(form) = self.macro_form.take() { self.submit_macro_form(form, 1); }
			}
			return;
		}
		let path = CStr::from_bytes_until_nul(&buf).unwrap().to_string_lossy();
		match belge {
			AndroidBelge::Ac(SecimTur::Altyazi) => {
				if let Err(e) = self.belge_ac(&path) { self.note = e; }
			}
			AndroidBelge::Ac(SecimTur::Video) => self.video_ac(&path),
			AndroidBelge::Ac(SecimTur::Ses) => self.ses_ac(&path),
			AndroidBelge::Ac(SecimTur::Lua) => self.lua_yukle(&path),
			AndroidBelge::Ac(SecimTur::Font) => self.font_ekle(&path),
			AndroidBelge::Ac(SecimTur::Timecodes) => self.load_video_metadata(&path, true),
			AndroidBelge::Ac(SecimTur::Keyframes) => self.load_video_metadata(&path, false),
			AndroidBelge::Ac(SecimTur::DictionaryRules) => self.load_spelling_file(&path, true),
			AndroidBelge::Ac(SecimTur::DictionaryWords) => self.load_spelling_file(&path, false),
			AndroidBelge::MacroOpen => self.finish_macro_open(&path),
			AndroidBelge::MacroSave(mut output) => {
				let next = output.document.take().unwrap();
				self.apply_macro_snapshot(&output.name, next);
			}
			AndroidBelge::Kaydet(staged, true) => {
				Self::remove_saf_stage(&staged);
				self.note = "Dosya dışa aktarıldı".into();
			}
			AndroidBelge::Kaydet(staged, false) => {
				self.path = Some(staged.to_string_lossy().into_owned());
				if let Some(previous) = self.android_saved_snapshot.replace(staged) {
					Self::remove_saf_stage(&previous);
				}
				self.saved_revision = self.current_revision;
				self.son_an = None;
				self.note = "dosya kaydedildi".into();
				self.finish_pending_document();
			}
		}
	}

	fn finish_pending_document(&mut self) {
		match self.bekleyen_belge.take() {
			Some(PendingDocumentAction::Open(path)) => {
				if let Err(error) = self.load(&path) {
					self.note = error;
				}
			}
			Some(PendingDocumentAction::New) => self.new_document(),
			#[cfg(target_os = "android")]
			Some(PendingDocumentAction::Close) => {
				let activity = crate::DOCUMENT_ACTIVITY.lock().unwrap();
				if let Some(app) = activity.as_ref() {
					let rc = unsafe {
						velnosub_abi_document_pick(app.vm_as_ptr(), app.activity_as_ptr(),
							-1, c"".as_ptr(), c"".as_ptr())
					};
					if rc != 0 {
						self.note = "Android etkinliği kapatılamadı".into();
					}
				}
			}
			None => (),
		}
	}

	fn belge_pencereleri(&mut self, ctx: &egui::Context) {
		if self.kayit_hedefi.is_none() {
			if let Some(action) = self.bekleyen_belge.clone() {
				let (message, discard) = match action {
					PendingDocumentAction::Open(_) => (
						"Başka bir dosya açmadan önce değişiklikleri kaydetmek ister misin?",
						"Kaydetmeden aç",
					),
					PendingDocumentAction::New => (
						"Yeni altyazı açmadan önce değişiklikleri kaydetmek ister misin?",
						"Kaydetmeden devam et",
					),
					#[cfg(target_os = "android")]
					PendingDocumentAction::Close => (
						"Çıkmadan önce değişiklikleri kaydetmek ister misin?", "Kaydetmeden çık",
					),
				};
				egui::Modal::new(egui::Id::unique("kaydedilmemis")).show(ctx, |ui| {
					ui.heading("Kaydedilmemiş değişiklikler");
					ui.label(message);
					ui.horizontal_wrapped(|ui| {
						if ui.button("Kaydet").clicked() {
							self.kaydet();
							if !self.belge_degisti() {
								self.finish_pending_document();
							}
						}
						if ui.button(discard).clicked() {
							self.finish_pending_document();
						}
						if ui.button("İptal").clicked() {
							self.bekleyen_belge = None;
						}
					});
				});
			}
		}
		if let Some(mut hedef) = self.kayit_hedefi.take() {
			let mut kapat = false;
			egui::Modal::new(egui::Id::unique("kayit_hedefi")).show(ctx, |ui| {
				ui.heading("Farklı kaydet");
				ui.label("Çıktı dosyasının yolu (.ass veya .srt)");
				ui.text_edit_singleline(&mut hedef);
				ui.label(&self.note);
				if ui.button("Kaydet").clicked() {
					if std::path::Path::new(&hedef).exists() {
						self.note = "Bu dosya zaten var. Yeni bir dosya adı gir.".into();
					} else if self.save_to(&hedef) {
						kapat = true;
						self.finish_pending_document();
					}
				}
				if ui.button("İptal").clicked() { kapat = true; }
			});
			if !kapat { self.kayit_hedefi = Some(hedef); }
		}
	}

	fn save_to(&mut self, path: &str) -> bool {
		let Some(h) = self.handle else {
			self.note = "kayıt: açık dosya yok".into();
			return false;
		};
		let result = CString::new(path).map_err(|error| error.to_string())
			.and_then(|out_path| write_document(h, &out_path));
		self.note = match &result {
			Ok(()) => {
				self.path = Some(path.to_owned());
				self.save_path = Some(path.to_owned());
				self.saved_revision = self.current_revision;
				self.son_an = None;
				format!("kaydedildi: {path}")
			}
			Err(e) => format!("kayıt hatası: {e}"),
		};
		result.is_ok()
	}

	/// P4.3: video kolu aç; bir kare ertelendi (ui'da doku kurulumu).
	fn load_video(&mut self, mut opened: OpenedVideo, ender: &str) -> Result<(), String> {
		let yolu = CString::new(ender).map_err(|e| e.to_string())?;
		let h = opened.0.take().expect("video handle");
		let (w, hgt, fps, kareler) = match video_geom(h) {
			Ok(geom) => geom,
			Err(e) => {
				unsafe { velnosub_abi_video_close(h) };
				return Err(e);
			}
		};
		if kareler <= 0 {
			unsafe { velnosub_abi_video_close(h) };
			return Err("video karesi yok".into());
		}
		let mut zamanlar = vec![0; kareler as usize];
		if unsafe { velnosub_abi_video_timeline(h, zamanlar.as_mut_ptr(), kareler) } < 0 {
			let error = last_error();
			unsafe { velnosub_abi_video_close(h) };
			return Err(error);
		}
		let keys = unsafe { velnosub_abi_video_keyframes(h, std::ptr::null_mut(), 0) };
		let mut ana_kareler = vec![0; keys.max(0) as usize];
		if keys > 0 {
			unsafe { velnosub_abi_video_keyframes(h, ana_kareler.as_mut_ptr(), keys) };
		}
		let mut obuf = vec![0u8; 160];
		let n = unsafe {
			velnosub_abi_video_info(h, obuf.as_mut_ptr() as *mut c_char, obuf.len() as c_int)
		};
		let bilgi = if n < 0 {
			last_error()
		} else {
			unsafe {
				CStr::from_ptr(obuf.as_ptr() as *const c_char)
					.to_string_lossy()
					.into_owned()
			}
		};
		if let Some(v) = self.video.take() {
			unsafe { velnosub_abi_video_close(v.handle) };
		}
		self.video = Some(VideoSection {
			handle: h,
			w,
			hgt,
			fps,
			kareler,
			zamanlar,
			ana_kareler,
			external_times: None,
			external_keys: None,
			frame: 0,
			info: format!("fps:{fps:.3} {bilgi}"),
			dok: None,
			olcek_yuzdesi: None,
			preview_ratio: None,
			yol: ender.to_string(),
			akslar: Vec::new(),
			oynat: false,
			son_tik: None,
			birim: 0.0,
			satir_bas: 0,
			satir_uzun: 0,
			coz_ms: 0,
		});
		// P6.4: iç altyazı akış taraması — YALNIZ SEÇİCİ kuralı (auto-load
		// yok); metin kodekli akışlar sayılır, pane ComboBox'ından yüklenir
		let sayi = unsafe { velnosub_abi_subtitle_examine(yolu.as_ptr()) };
		self.note = format!("video açıldı: {w}x{hgt}, {kareler} kare");
		if sayi > 0 {
			let akslar = alt_tsv_okes();
			if let Some(v) = self.video.as_mut() {
				v.akslar = akslar;
			}
			self.note = format!(
				"video açıldı: {w}x{hgt}, {kareler} kare — {sayi} metin altyazı akışı (pane'de seç)"
			);
		}
		Ok(())
	}

	fn audio_open(ender: &str, cancel: &std::sync::atomic::AtomicBool) -> Result<OpenedAudio, String> {
		let yol = CString::new(ender).map_err(|e| e.to_string())?;
		let uzanti = std::path::Path::new(ender).extension().and_then(|ext| ext.to_str());
		let h = if uzanti.is_some_and(|ext| ext.eq_ignore_ascii_case("pcm")) {
			unsafe { velnosub_abi_audio_open(yol.as_ptr(), 48000, 2) }
		} else {
			unsafe {
				velnosub_ui_open_audio(
					yol.as_ptr(), audio_cancelled, (cancel as *const std::sync::atomic::AtomicBool).cast(),
				)
			}
		};
		document_io_result(h, "Ses dosyası açılamadı").map(|h| OpenedAudio(Some(h)))
	}

	fn load_audio(&mut self, mut opened: OpenedAudio, path: String) -> Result<(), String> {
		let h = opened.0.take().expect("audio handle");
		let mut obuf = vec![0u8; 160];
		let n = unsafe {
			velnosub_abi_audio_info(h, obuf.as_mut_ptr() as *mut c_char, obuf.len() as c_int)
		};
		let bilgi = if n < 0 {
			last_error()
		} else {
			unsafe {
				CStr::from_ptr(obuf.as_ptr() as *const c_char)
					.to_string_lossy()
					.into_owned()
			}
		};
		let toplam = unsafe { velnosub_abi_audio_len64(h) };
		if toplam <= 0 {
			unsafe { velnosub_abi_audio_close(h) };
			return Err("ses verisi yok".into());
		}
		// Task 6: süre hesabı gerçek örnekleme hızıyla yapılır. ABI künyesi
		// "s16 48000Hz 2k ..." biçiminde; çözülemezse 48000/2 varsayılanı.
		let (rate, ch) = crate::audio_timing::ses_kunye_coz(&bilgi);
		if let Some(a) = self.audio.take() {
			unsafe { velnosub_abi_audio_close(a.handle) };
		}
		self.audio = Some(AudioSection {
			handle: h,
			path,
			rate,
			ch,
			toplam,
			info: bilgi,
			ses_durum: "—".into(),
			oynatma_denendi: false,
			konum: 0,
			d0: 0,
			adet: toplam,
			imlec: 0,
			secim: None,
			mod_: SesMod::Dalga,
			takip: true,
			spek: None,
			spek_anahtar: None,
			spek_doku: None,
			spek_hata: None,
			surukleme: None,
			tut_bas: None,
			tut_son: None,
		});
		self.note = format!("ses açıldı: {toplam} örnek · {rate} Hz · {ch} kanal");
		Ok(())
	}
}

impl GridApp {
	// ---- şerit gövdeleri: masaüstü panel ve mobil sekme AYNI gövdeyi paylaşır;
	// `mobil` bayrağı yalnız dokunma hedefi boyunu ve satır düzenini değiştirir ----

	/// P4.6 dilim 2: ASS önizleme kolu — şart ömrü altyazı kolunu izler;
	/// kareler video fps ile ms'e çevrilir; her ms değişiminde bir render.
	fn oniz_cek(
		&mut self, ctx: &egui::Context, h: c_int,
		gen: usize, yuk: usize, ms: i32,
	) {
		if self.oniz.h != h {
			// altyazı kolu değişti: C tarafı abi_close ile şartı zaten
			// yumlatır; burada yalnız durum sıfır küsur
			let mut taze = Oniz::default();
			taze.h = h;
			taze.gen = gen;
			taze.yuk = yuk;
			self.oniz = taze;
		} else if self.oniz.gen != gen || self.oniz.yuk != yuk {
			if self.oniz.acik {
				unsafe { velnosub_abi_render_close(h); }
			}
			let mut taze = Oniz::default();
			taze.h = h;
			taze.gen = gen;
			taze.yuk = yuk;
			self.oniz = taze;
		}
		if !self.oniz.denendi {
			self.oniz.denendi = true;
			let yol = std::env::var("VELNOSUB_FONT")
				.unwrap_or_else(|_| "./DejaVuSans.ttf".into());
			match CString::new(yol.clone()) {
				Ok(y_c) => {
					let a_c = CString::new("Default").unwrap_or_default();
					let rc = unsafe {
						velnosub_abi_render_open(
							h, gen as c_int, yuk as c_int,
							y_c.as_ptr(), a_c.as_ptr(),
						)
					};
					if rc == 0 {
						self.oniz.acik = true;
						self.oniz.tampon = vec![0u8; gen * yuk * 4];
						self.oniz.son_ms = -1;
						self.oniz.hata = None;
					} else {
						self.oniz.hata = Some(format!(
							"önizleme açılamadı: {}",
							last_error()
						));
					}
				}
				Err(e) => {
					self.oniz.hata =
						Some(format!("font yolu kodlanamadı: {e}"))
				}
			}
		}
		if !self.oniz.acik {
			return;
		}
		if ms != self.oniz.son_ms {
			let n = self.oniz.gen * self.oniz.yuk * 4;
			if self.oniz.tampon.len() != n {
				self.oniz.tampon = vec![0u8; n];
				self.oniz.son_ms = -1;
			}
			let rc = unsafe {
				velnosub_abi_render_frame(
					h, ms,
					self.oniz.tampon.as_mut_ptr(),
					n as c_int,
				)
			};
			if rc < 0 {
				self.oniz.hata =
					Some(format!("önizleme karesi: {}", last_error()));
				return;
			}
			self.oniz.son_ms = ms;
			self.oniz.etki = rc;
			let img = egui::ColorImage::from_rgba_unmultiplied(
				[self.oniz.gen, self.oniz.yuk],
				&self.oniz.tampon,
			);
			let ad = format!("voniz_h{}_{}", h, ms);
			self.oniz.dok = Some(
				ctx.load_texture(&ad, img, egui::TextureOptions::default()),
			);
		}
	}

	/// Çekirdekten ASS PlayRes boyutunu oku (Task 7 ABI).
	fn script_size(&self) -> Option<(i32, i32)> {
		let h = self.handle?;
		let mut w = 0;
		let mut yuk = 0;
		let rc = unsafe { velnosub_abi_script_size(h, &mut w, &mut yuk) };
		if rc == 0 && w > 0 && yuk > 0 {
			Some((w, yuk))
		} else {
			None
		}
	}

	/// Video panelinin üstündeki typeset girişi/araç çubuğu.
	fn typeset_arac_cubugu(&mut self, ui: &mut egui::Ui) {
		if self.video.is_none() || self.secili.is_none() {
			self.typeset = None;
			return;
		}
		if let Some(t) = &self.typeset {
			if Some(t.satir) != self.secili {
				self.typeset = None;
			}
		}
		let aktif = self.typeset.is_some();
		let arac = self.typeset.as_ref().map(|t| t.arac);
		let sec = self.secili.unwrap();
		let mut ters_degis = false;
		ui.horizontal_wrapped(|su| {
			if !aktif {
				let b = su
					.add(egui::Button::new(format!("{} Typeset", ikon::TUNE)))
					.on_hover_text(
						"seçili satırı video üzerinde taşı, ölçekle, döndür, clip/vektör/XY düzenle",
					);
				if b.clicked() {
					self.typeset_basla(su.ctx(), sec);
				}
			} else {
				su.label(RichText::new(format!("typeset · satır {}", sec + 1)).weak());
				for (a, ad) in [
					(Arac::Tasi, "Taşı"),
					(Arac::Olcek, "Ölçek"),
					(Arac::ZDondur, "Z Döndür"),
					(Arac::Clip, "Clip"),
					(Arac::Vektor, "Vektör"),
					(Arac::XY, "XY"),
				] {
					if su.selectable_label(arac == Some(a), ad).clicked() {
						if let Some(t) = self.typeset.as_mut() {
							t.arac_sec(a);
						}
					}
				}
				if matches!(arac, Some(Arac::Clip | Arac::Vektor)) {
					let ic = self
						.typeset
						.as_ref()
						.map(|t| t.iclip)
						.unwrap_or(false);
					if su.selectable_label(ic, "ters (iclip)").clicked() {
						ters_degis = true;
					}
				}
				if su.button(format!("{} kapat", ikon::CLOSE)).clicked() {
					self.typeset = None;
				}
			}
		});
		if ters_degis {
			self.typeset_iclip_degistir(sec);
		}
		ui.add_space(2.0);
	}

	/// Seçili satırı typeset kipine al; satır görünmüyorsa başına atla.
	fn typeset_basla(&mut self, ctx: &egui::Context, idx: usize) {
		if self.handle.is_none() {
			self.note = "typeset: açık dosya yok".into();
			return;
		}
		let Some(metin) = self.rows.get(idx).map(|r| r.text.clone()) else {
			self.note = "typeset: satır bulunamadı".into();
			return;
		};
		let (sw, sh) = self
			.script_size()
			.or_else(|| {
				self.video
					.as_ref()
					.map(|v| (v.w as i32, v.hgt as i32))
			})
			.unwrap_or((384, 288));
		let bas = self
			.rows
			.get(idx)
			.and_then(|r| ass_ms(&r.bas).ok());
		let son = self
			.rows
			.get(idx)
			.and_then(|r| ass_ms(&r.son).ok());
		let mut hata = None;
		if let Some(v) = self.video.as_mut() {
			v.oynat = false;
			v.son_tik = None;
			if v.kareler > 0 && v.fps > 0.0 {
				if let (Some(b), Some(s)) = (bas, son) {
					let ms = i64::from(v.kare_zamani(v.frame));
					if ms < b || ms > s || s <= b {
						let k = v.zamanlar.partition_point(|&start| i64::from(start) < b).min(v.zamanlar.len() - 1) as i32;
						if k != v.frame {
							v.frame = k;
							if let Err(e) = v.cizim_cek(ctx) {
								hata = Some(e);
							}
						}
					}
				}
			}
		}
		self.typeset = Some(Typeset::yeni(idx, metin, sw, sh));
		if let Some(a) = self.audio.as_ref() {
			unsafe {
				velnosub_abi_audio_stop(a.handle);
			}
		}
		if let Some(e) = hata {
			if let Some(t) = self.typeset.as_mut() {
				t.hata = Some(format!("video karesi: {e}"));
			}
		}
		self.oniz.son_ms = -1;
		self.note = format!("typeset: satır {} hazır", idx + 1);
	}

	/// Seçili satırın stil hizası (1..9); geçersizse alt-orta.
	fn satir_hiza(&self, idx: usize) -> i32 {
		let Some(r) = self.rows.get(idx) else {
			return 2;
		};
		self.stiller
			.iter()
			.find(|s| s.name.trim() == r.stil.trim())
			.map(|s| s.alignment)
			.filter(|h| (1..=9).contains(h))
			.unwrap_or(2)
	}

	/// Stilin etkin ScaleX/ScaleY yüzdesi (satır override'ı yokken taban).
	fn satir_olcek(&self, idx: usize) -> (f64, f64) {
		let Some(r) = self.rows.get(idx) else {
			return (100.0, 100.0);
		};
		let Some(s) = self
			.stiller
			.iter()
			.find(|s| s.name.trim() == r.stil.trim())
		else {
			return (100.0, 100.0);
		};
		let mut it = s.scale.split(',').filter_map(|v| {
			let x = v.trim().parse::<f64>().ok()?;
			(x > 0.0).then_some(x)
		});
		(
			it.next().unwrap_or(100.0),
			it.next().unwrap_or(100.0),
		)
	}

	/// `\clip` ↔ `\iclip` anahtarını mevcut geometriyle değiştir.
	fn typeset_iclip_degistir(&mut self, idx: usize) {
		let Some(t) = self.typeset.as_ref() else {
			return;
		};
		let metin = t.metin.clone();
		let arac = t.arac;
		let yeni_ic = !t.iclip;
		if ass_tags::etiket_al(&metin, "clip").is_none()
			&& ass_tags::etiket_al(&metin, "iclip").is_none()
		{
			self.note = "ters: önce bir clip çiz".into();
			return;
		}
		if arac == Arac::Clip && t.clip.is_none() {
			self.note =
				"ters: mevcut clip vektör; önce Vektör aracını kullan".into();
			return;
		}
		if arac == Arac::Vektor && t.vektor.is_none() {
			self.note = "ters: vektör noktaları okunamadı".into();
			return;
		}
		let yeni = if arac == Arac::Vektor {
			let p = t.vektor.clone().unwrap_or_else(|| {
				vec![
					(0.0, 0.0),
					(t.script_w as f64, 0.0),
					(t.script_w as f64, t.script_h as f64),
					(0.0, t.script_h as f64),
				]
			});
			clip_vektor_yaz(&metin, &p, yeni_ic)
		} else {
			let r = t.clip.unwrap_or((
				0.0,
				0.0,
				t.script_w as f64,
				t.script_h as f64,
			));
			clip_dik_yaz(&metin, r, yeni_ic)
		};
		if self.typeset_uygula(idx, yeni) {
			if let Some(t) = self.typeset.as_mut() {
				t.iclip = yeni_ic;
			}
		}
	}


	/// Seçili satırı video üzerine çiz; sürüklemede geçici metin/egui
	/// önizlemesi, bırakınca tek `edit_line` commit'i.
	fn typeset_cek(
		&mut self,
		ui: &mut egui::Ui,
		dik: egui::Rect,
		gen: usize,
		yuk: usize,
		ms: i32,
	) {
		let Some(h) = self.handle else {
			self.typeset = None;
			return;
		};
		let Some(sec) = self.secili else {
			self.typeset = None;
			return;
		};
		let Some(t0) = self.typeset.as_ref() else {
			return;
		};
		if t0.satir != sec || gen == 0 || yuk == 0 || !self.oniz.acik {
			return;
		}
		let metin = self
			.rows
			.get(sec)
			.map(|r| r.text.clone())
			.unwrap_or_default();
		let hiza_stil = self.satir_hiza(sec);
		let (stil_fscx, stil_fscy) = self.satir_olcek(sec);
		let n = gen * yuk * 4;
		let commit = {
			let Some(t) = self.typeset.as_mut() else {
				return;
			};
			if t.metin != metin {
				t.gecici = None;
				t.surukleme = None;
				t.clip_surukle = None;
				t.vektor_surukle = None;
				t.xy_surukle = None;
				t.clip_hazir = false;
				t.vektor_hazir = false;
				t.xy_hazir = false;
			}
			t.metin = metin.clone();
			let render_metin = t
				.gecici
				.clone()
				.unwrap_or_else(|| metin.clone());
			if t.tampon.len() != n {
				t.tampon = vec![0u8; n];
				t.son_satir = usize::MAX;
				t.son_ms = i32::MIN;
				t.son_metin.clear();
				t.dok = None;
				t.bbox = None;
			}
			if t.son_satir != sec
				|| t.son_ms != ms
				|| t.son_metin != render_metin
				|| t.dok.is_none()
			{
				match satir_metin_cek(
					h,
					sec,
					&render_metin,
					ms,
					&mut t.tampon,
				) {
					Err(e) => {
						t.hata = Some(e);
						t.dok = None;
						t.bbox = None;
					}
					Ok(_) => {
						let img = egui::ColorImage::from_rgba_unmultiplied(
							[gen, yuk],
							&t.tampon,
						);
						let ad =
							format!("vtypeset_{}_{}_{}", h, sec, ms);
						t.dok = Some(ui.ctx().load_texture(
							ad,
							img,
							egui::TextureOptions::LINEAR,
						));
						t.bbox = alfa_bbox(&t.tampon, gen, yuk);
						t.son_satir = sec;
						t.son_ms = ms;
						t.son_metin = render_metin;
						t.hata = None;
					}
				}
			}
			if t.arac == Arac::Tasi
				&& ass_tags::etiket_al(&metin, "move").is_some()
			{
				t.hata = Some(
					"\\move satırı taşınamaz; önce \\move kaldırılmalı".into(),
				);
			}
			let bbpx = match t.bbox {
				Some(b) => b,
				None => {
					if matches!(
						t.arac,
						Arac::Clip | Arac::Vektor | Arac::XY
					) {
						egui::Rect::from_min_max(
							egui::pos2(0.0, 0.0),
							egui::pos2(gen as f32, yuk as f32),
						)
					} else {
						let hata = t.hata.clone().unwrap_or_else(|| {
							"bu karede satır görünmüyor".into()
						});
						ui.label(
							RichText::new(format!("typeset: {hata}"))
								.weak(),
						);
						return;
					}
				}
			};
			let e = crate::visual_geom::esleme_doldur(
				t.script_w,
				t.script_h,
				crate::visual_geom::Alan {
					x: dik.left(),
					y: dik.top(),
					w: dik.width(),
					h: dik.height(),
				},
			);
			let kx = dik.width() / gen as f32;
			let ky = dik.height() / yuk as f32;
			let bb = egui::Rect::from_min_max(
				dik.min + egui::vec2(bbpx.min.x * kx, bbpx.min.y * ky),
				dik.min + egui::vec2(bbpx.max.x * kx, bbpx.max.y * ky),
			);
			let hiza = ass_tags::etiket_al(&metin, "an")
				.and_then(|v| sayilar_al(&v).first().copied())
				.map(|v| v as i32)
				.filter(|v| (1..=9).contains(v))
				.unwrap_or(hiza_stil);
			let pos_ass = ass_tags::etiket_al(&metin, "pos").and_then(|v| {
				let s = sayilar_al(&v);
				if s.len() >= 2 {
					Some((s[0], s[1]))
				} else {
					None
				}
			});
			let org_ass = ass_tags::etiket_al(&metin, "org").and_then(|v| {
				let s = sayilar_al(&v);
				if s.len() >= 2 {
					Some((s[0], s[1]))
				} else {
					None
				}
			});
			let pivot = if let Some((x, y)) = org_ass.or(pos_ass) {
				let (px, py) = crate::visual_geom::ass_to_ekran(x, y, &e);
				egui::pos2(px, py)
			} else {
				hiza_ankraj(bb, hiza)
			};
			let tutamaclar = [
				bb.left_top(),
				bb.right_top(),
				bb.right_bottom(),
				bb.left_bottom(),
			];
			if t.arac == Arac::Clip && !t.clip_hazir {
				t.clip_hazir = true;
				match clip_dik_oku(&metin) {
					Ok(Some((r, ic))) => {
						t.clip = Some(r);
						t.iclip = ic;
					}
					Ok(None) => {
						t.clip = Some((
							0.0,
							0.0,
							t.script_w as f64,
							t.script_h as f64,
						));
						t.iclip = false;
					}
					Err(()) => {
						t.clip = None;
						t.hata = Some(
							"mevcut clip vektör/bozuk; Vektör aracını kullan"
								.into(),
						);
					}
				}
			}
			if t.arac == Arac::Vektor && !t.vektor_hazir {
				t.vektor_hazir = true;
				if let Some((p, ic)) = clip_vektor_oku(&metin) {
					t.vektor = Some(p);
					t.iclip = ic;
				} else {
					match clip_dik_oku(&metin) {
						Ok(Some((r, ic))) => {
							t.vektor = Some(vec![
								(r.0, r.1),
								(r.2, r.1),
								(r.2, r.3),
								(r.0, r.3),
							]);
							t.iclip = ic;
						}
						Ok(None) => {
							t.vektor = Some(vec![
								(0.0, 0.0),
								(t.script_w as f64, 0.0),
								(t.script_w as f64, t.script_h as f64),
								(0.0, t.script_h as f64),
							]);
							t.iclip = false;
						}
						Err(()) => {
							t.vektor = None;
							t.hata = Some(
								"bozuk clip; vektör noktaları okunamadı"
									.into(),
							);
						}
					}
				}
			}
			if t.arac == Arac::XY && !t.xy_hazir {
				t.xy_hazir = true;
				t.frx = ass_tags::etiket_al(&metin, "frx")
					.and_then(|v| sayilar_al(&v).first().copied())
					.unwrap_or(0.0);
				t.fry = ass_tags::etiket_al(&metin, "fry")
					.and_then(|v| sayilar_al(&v).first().copied())
					.unwrap_or(0.0);
			}
			if t.arac == Arac::Clip && t.clip.is_none() {
				t.hata = Some(
					"mevcut clip vektör/bozuk; Vektör aracını kullan"
						.into(),
				);
			}
			if t.arac == Arac::Vektor && t.vektor.is_none() {
				t.hata =
					Some("bozuk clip; vektör noktaları okunamadı".into());
			}
			let clip_kose = t.clip.map(|r| {
				let k = [
					crate::visual_geom::ass_to_ekran(r.0, r.1, &e),
					crate::visual_geom::ass_to_ekran(r.2, r.1, &e),
					crate::visual_geom::ass_to_ekran(r.2, r.3, &e),
					crate::visual_geom::ass_to_ekran(r.0, r.3, &e),
				];
				k.map(|(x, y)| egui::pos2(x, y))
			});
			let vektor_ekran: Option<Vec<egui::Pos2>> = t.vektor.as_ref().map(|p| {
				p.iter()
					.map(|(x, y)| {
						let (sx, sy) =
							crate::visual_geom::ass_to_ekran(*x, *y, &e);
						egui::pos2(sx, sy)
					})
					.collect()
			});
			let resp = ui.interact(
				dik,
				ui.scope_id().with(("typeset", sec)),
				egui::Sense::click_and_drag(),
			);
			let p = resp
				.interact_pointer_pos()
				.or_else(|| ui.input(|i| i.pointer.interact_pos()))
				.or_else(|| ui.input(|i| i.pointer.latest_pos()));
			let fscx = ass_tags::etiket_al(&metin, "fscx")
				.and_then(|v| sayilar_al(&v).first().copied())
				.filter(|v| *v > 0.0)
				.unwrap_or(stil_fscx);
			let fscy = ass_tags::etiket_al(&metin, "fscy")
				.and_then(|v| sayilar_al(&v).first().copied())
				.filter(|v| *v > 0.0)
				.unwrap_or(stil_fscy);
			let frz = ass_tags::etiket_al(&metin, "frz")
				.and_then(|v| sayilar_al(&v).first().copied())
				.unwrap_or(0.0);
			if resp.drag_started() {
				if let Some(p) = p {
					let mut basla = false;
					match t.arac {
						Arac::Tasi => {
							if ass_tags::etiket_al(&metin, "move").is_some() {
								t.hata = Some(
									"\\move satırı taşınamaz; önce \\move kaldırılmalı"
										.into(),
								);
							} else if bb.expand(8.0).contains(p) {
								basla = true;
							}
						}
						Arac::Olcek => {
							for c in tutamaclar {
								if crate::visual_geom::tutamac(
									(p.x, p.y),
									(c.x, c.y),
									24.0,
								) {
									basla = true;
									break;
								}
							}
						}
						Arac::ZDondur => {
							let c = dondur_tutamac(bb, t.oniz_aci);
							if crate::visual_geom::tutamac(
								(p.x, p.y),
								(c.x, c.y),
								24.0,
							) {
								basla = true;
							}
						}
						Arac::Clip => {
							if let Some(kose) = clip_kose {
								for (i, c) in kose.iter().enumerate() {
									if crate::visual_geom::tutamac(
										(p.x, p.y),
										(c.x, c.y),
										24.0,
									) {
										t.clip_surukle = Some(i);
										break;
									}
								}
							}
						}
						Arac::Vektor => {
							if let Some(noktalar) = &vektor_ekran {
								for (i, c) in noktalar.iter().enumerate() {
									if crate::visual_geom::tutamac(
										(p.x, p.y),
										(c.x, c.y),
										20.0,
									) {
										t.vektor_surukle = Some(i);
										break;
									}
								}
							}
						}
						Arac::XY => {
							if bb.expand(8.0).contains(p) {
								t.xy_surukle =
									Some((p, t.frx, t.fry));
							}
						}
					}
					if basla {
						let bas_pos = pos_ass.unwrap_or_else(|| {
							let a = hiza_ankraj(bb, hiza);
							crate::visual_geom::ekran_to_ass(a.x, a.y, &e)
						});
						t.oniz_ofs = egui::Vec2::ZERO;
						t.oniz_olcek = 1.0;
						t.oniz_aci = 0.0;
						t.surukleme = Some(Surukleme {
							bas_pointer: p,
							pivot,
							bas_pos_ass: bas_pos,
							bas_fscx: fscx,
							bas_fscy: fscy,
							bas_frz: frz,
							bas_metin: metin.clone(),
						});
					}
				}
			}
			if resp.dragged() {
				if let Some(p) = p {
					if let Some(s) = t.surukleme.as_ref() {
						match t.arac {
							Arac::Tasi => {
								t.oniz_ofs = p - s.bas_pointer;
							}
							Arac::Olcek => {
								let d0 = (s.bas_pointer - s.pivot).length();
								if d0 > 4.0 {
									let d = (p - s.pivot).length();
									let oran = (d / d0).clamp(0.05, 20.0);
									let yeni =
										(s.bas_fscx * oran as f64).max(10.0);
									t.oniz_olcek =
										(yeni / s.bas_fscx.max(1.0)) as f32;
								}
							}
							Arac::ZDondur => {
								let v0 = s.bas_pointer - s.pivot;
								let v1 = p - s.pivot;
								let mut d =
									v1.y.atan2(v1.x) - v0.y.atan2(v0.x);
								while d > std::f32::consts::PI {
									d -= std::f32::consts::TAU;
								}
								while d < -std::f32::consts::PI {
									d += std::f32::consts::TAU;
								}
								t.oniz_aci = d;
							}
							_ => {}
						}
					}
					match t.arac {
						Arac::Clip => {
							if let (Some(i), Some(mut r)) =
								(t.clip_surukle, t.clip)
							{
								let (ax, ay) =
									crate::visual_geom::ekran_to_ass(
										p.x, p.y, &e,
									);
								match i {
									0 => {
										r.0 = ax;
										r.1 = ay;
									}
									1 => {
										r.2 = ax;
										r.1 = ay;
									}
									2 => {
										r.2 = ax;
										r.3 = ay;
									}
									_ => {
										r.0 = ax;
										r.3 = ay;
									}
								}
								let x1 = r.0.min(r.2);
								let y1 = r.1.min(r.3);
								let x2 = r.0.max(r.2);
								let y2 = r.1.max(r.3);
								let r = (x1, y1, x2, y2);
								t.clip = Some(r);
								t.gecici =
									Some(clip_dik_yaz(&metin, r, t.iclip));
							}
						}
						Arac::Vektor => {
							if let (Some(i), Some(mut pts)) =
								(t.vektor_surukle, t.vektor.clone())
							{
								if i < pts.len() {
									let (ax, ay) =
										crate::visual_geom::ekran_to_ass(
											p.x, p.y, &e,
										);
									pts[i] = (
										ax.clamp(0.0, t.script_w as f64),
										ay.clamp(0.0, t.script_h as f64),
									);
									t.vektor = Some(pts.clone());
									t.gecici = Some(clip_vektor_yaz(
										&metin,
										&pts,
										t.iclip,
									));
								}
							}
						}
						Arac::XY => {
							if let Some((bas, fx, fy)) = t.xy_surukle {
								let dx = (p.x - bas.x) as f64;
								let dy = (p.y - bas.y) as f64;
								t.fry = aci_derece(fy + dx * 0.5);
								t.frx = aci_derece(fx - dy * 0.5);
								t.gecici = Some(xy_yaz(
									&metin, t.frx, t.fry,
								));
							}
						}
						_ => {}
					}
				}
			}
			let mut out = None;
			if resp.drag_stopped() {
				if let Some(s) = t.surukleme.take() {
					let p = p.unwrap_or(s.bas_pointer);
					let yeni = match t.arac {
						Arac::Tasi => {
							let kx = if e.olcek_x > 1e-6 {
								e.olcek_x as f64
							} else {
								1.0
							};
							let ky = if e.olcek_y > 1e-6 {
								e.olcek_y as f64
							} else {
								1.0
							};
							let dx = (p.x - s.bas_pointer.x) as f64 / kx;
							let dy = (p.y - s.bas_pointer.y) as f64 / ky;
							if ass_tags::etiket_al(&s.bas_metin, "move").is_some() {
								t.hata = Some(
									"\\move satırı taşınamaz; önce \\move kaldırılmalı"
										.into(),
								);
								None
							} else {
								Some(ass_tags::etiket_yaz(
									&s.bas_metin,
									"pos",
									&format!(
										"({:.1},{:.1})",
										s.bas_pos_ass.0 + dx,
										s.bas_pos_ass.1 + dy
									),
								))
							}
						}
						Arac::Olcek => {
							let oran = t.oniz_olcek as f64;
							let fx = (s.bas_fscx * oran).max(10.0);
							let fy = (s.bas_fscy * oran).max(10.0);
							let m = ass_tags::etiket_yaz(
								&s.bas_metin,
								"fscx",
								&format!("{fx:.1}"),
							);
							Some(ass_tags::etiket_yaz(
								&m,
								"fscy",
								&format!("{fy:.1}"),
							))
						}
						Arac::ZDondur => {
							let d = t.oniz_aci.to_degrees() as f64;
							let a = aci_derece(s.bas_frz + d);
							Some(ass_tags::etiket_yaz(
								&s.bas_metin,
								"frz",
								&format!("{a:.1}"),
							))
						}
						_ => None,
					};
					if yeni.is_some() {
						out = yeni;
					}
					t.oniz_ofs = egui::Vec2::ZERO;
					t.oniz_olcek = 1.0;
					t.oniz_aci = 0.0;
				}
				if t.clip_surukle.take().is_some() {
					if let Some(y) = t.gecici.clone() {
						out = Some(y);
					}
				}
				if t.vektor_surukle.take().is_some() {
					if let Some(y) = t.gecici.clone() {
						out = Some(y);
					}
				}
				if t.xy_surukle.take().is_some() {
					if let Some(y) = t.gecici.clone() {
						out = Some(y);
					}
				}
			}
			// Vektör: kenara tıkla → nokta ekle; uzun bas/sağ tık → nokta sil.
			if t.arac == Arac::Vektor {
				if let Some(p) = p {
					if let Some(ekran) = t.vektor.as_ref().map(|pts| {
						pts.iter()
							.map(|(x, y)| {
								let (sx, sy) =
									crate::visual_geom::ass_to_ekran(*x, *y, &e);
								egui::pos2(sx, sy)
							})
							.collect::<Vec<_>>()
					}) {
						let yakin = ekran.iter().position(|c| {
							crate::visual_geom::tutamac(
								(p.x, p.y),
								(c.x, c.y),
								20.0,
							)
						});
						if resp.long_touched() || resp.secondary_clicked() {
							if let Some(i) = yakin {
								if let Some(mut pts) = t.vektor.clone() {
									if pts.len() > 3 {
										pts.remove(i);
										t.vektor_surukle = None;
										t.vektor = Some(pts.clone());
										out = Some(clip_vektor_yaz(
											&metin,
											&pts,
											t.iclip,
										));
									}
								}
							}
						} else if resp.clicked() && yakin.is_none() {
							if ekran.len() >= 2 {
								let mut en = (0usize, f32::MAX);
								for i in 0..ekran.len() {
									let d = kenar_mesafe(
										p,
										ekran[i],
										ekran[(i + 1) % ekran.len()],
									);
									if d < en.1 {
										en = (i, d);
									}
								}
								if en.1 <= 14.0 {
									if let Some(mut pts) = t.vektor.clone() {
										let (ax, ay) =
											crate::visual_geom::ekran_to_ass(
												p.x, p.y, &e,
											);
										pts.insert(
											en.0 + 1,
											(
												ax.clamp(
													0.0,
													t.script_w as f64,
												),
												ay.clamp(
													0.0,
													t.script_h as f64,
												),
											),
										);
										t.vektor = Some(pts.clone());
										out = Some(clip_vektor_yaz(
											&metin,
											&pts,
											t.iclip,
										));
									}
								}
							}
						}
					}
				}
			}
			// Önizleme dokusu ve tutamaçlar.
			let ofs = t.oniz_ofs;
			let olcek = t.oniz_olcek;
			let aci = t.oniz_aci;
			let pv = |p: egui::Pos2| ts_donusum(p, pivot, olcek, aci, ofs);
			let ciz = ui.painter().with_clip_rect(dik);
			if let Some(d) = &t.dok {
				let mut mesh = egui::Mesh::with_texture(d.id());
				let kose = [
					dik.left_top(),
					dik.right_top(),
					dik.right_bottom(),
					dik.left_bottom(),
				];
				let uv = [
					egui::pos2(0.0, 0.0),
					egui::pos2(1.0, 0.0),
					egui::pos2(1.0, 1.0),
					egui::pos2(0.0, 1.0),
				];
				let taban = mesh.vertices.len() as u32;
				for (p, uv) in kose.into_iter().zip(uv) {
					mesh.vertices.push(egui::epaint::Vertex {
						pos: pv(p),
						uv,
						color: egui::Color32::WHITE,
					});
				}
				mesh.add_triangle(taban, taban + 1, taban + 2);
				mesh.add_triangle(taban, taban + 2, taban + 3);
				ciz.add(egui::Shape::mesh(mesh));
			}
			let sari = egui::Color32::from_rgb(255, 210, 70);
			match t.arac {
				Arac::Tasi | Arac::Olcek | Arac::ZDondur => {
					let kose: Vec<egui::Pos2> = [
						bb.left_top(),
						bb.right_top(),
						bb.right_bottom(),
						bb.left_bottom(),
					]
					.into_iter()
					.map(pv)
					.collect();
					for i in 0..4 {
						ciz.line_segment(
							[kose[i], kose[(i + 1) % 4]],
							egui::Stroke::new(1.5, sari),
						);
					}
					match t.arac {
						Arac::Tasi => {
							ciz.circle_filled(pivot + ofs, 3.0, sari);
						}
						Arac::Olcek => {
							for c in &kose {
								ciz.circle_filled(*c, 6.0, sari);
								ciz.circle_stroke(
									*c,
									6.0,
									egui::Stroke::new(
										1.0,
										egui::Color32::BLACK,
									),
								);
							}
						}
						_ => {
							let tc = pv(egui::pos2(
								bb.center().x,
								bb.top(),
							));
							let up = egui::vec2(0.0, -30.0);
							let (s, c) = aci.sin_cos();
							let u = egui::vec2(
								up.x * c - up.y * s,
								up.x * s + up.y * c,
							) * olcek;
							let hnd = tc + u;
							ciz.line_segment(
								[tc, hnd],
								egui::Stroke::new(1.5, sari),
							);
							ciz.circle_filled(hnd, 6.0, sari);
							ciz.circle_stroke(
								hnd,
								6.0,
								egui::Stroke::new(
									1.0,
									egui::Color32::BLACK,
								),
							);
							ciz.circle_filled(pivot + ofs, 3.0, sari);
						}
					}
				}
				Arac::Clip => {
					if let Some(r) = t.clip {
						let renk = if t.iclip {
							egui::Color32::from_rgb(255, 150, 60)
						} else {
							sari
						};
						let k = [
							crate::visual_geom::ass_to_ekran(r.0, r.1, &e),
							crate::visual_geom::ass_to_ekran(r.2, r.1, &e),
							crate::visual_geom::ass_to_ekran(r.2, r.3, &e),
							crate::visual_geom::ass_to_ekran(r.0, r.3, &e),
						]
						.map(|(x, y)| egui::pos2(x, y));
						let dolgu = egui::Color32::from_rgba_unmultiplied(
							renk.r(),
							renk.g(),
							renk.b(),
							20,
						);
						ciz.rect_filled(
							egui::Rect::from_two_pos(k[0], k[2]),
							0.0,
							dolgu,
						);
						for i in 0..4 {
							ciz.line_segment(
								[k[i], k[(i + 1) % 4]],
								egui::Stroke::new(1.8, renk),
							);
						}
						for c in k {
							ciz.circle_filled(c, 7.0, renk);
							ciz.circle_stroke(
								c,
								7.0,
								egui::Stroke::new(
									1.0,
									egui::Color32::BLACK,
								),
							);
						}
					}
				}
				Arac::Vektor => {
					if let Some(pts) = t.vektor.as_ref() {
						let ekran: Vec<egui::Pos2> = pts
							.iter()
							.map(|(x, y)| {
								let (sx, sy) =
									crate::visual_geom::ass_to_ekran(
										*x, *y, &e,
									);
								egui::pos2(sx, sy)
							})
							.collect();
						if ekran.len() >= 2 {
							for i in 0..ekran.len() {
								ciz.line_segment(
									[
										ekran[i],
										ekran[(i + 1)
											% ekran.len()],
									],
									egui::Stroke::new(1.8, sari),
								);
							}
						}
						for (i, c) in ekran.iter().enumerate() {
							let aktif = t.vektor_surukle == Some(i);
							let r = if aktif { 9.0 } else { 6.0 };
							ciz.circle_filled(*c, r, sari);
							ciz.circle_stroke(
								*c,
								r,
								egui::Stroke::new(
									1.0,
									egui::Color32::BLACK,
								),
							);
						}
						if ekran.len() >= 4 {
							// ilk nokta halkası: kapalı vektör başlangıcı
							ciz.circle_stroke(
								ekran[0],
								11.0,
								egui::Stroke::new(1.2, sari),
							);
						}
					}
				}
				Arac::XY => {
					let kose: Vec<egui::Pos2> = [
						bb.left_top(),
						bb.right_top(),
						bb.right_bottom(),
						bb.left_bottom(),
					]
					.into_iter()
					.collect();
					let renk = egui::Color32::from_rgb(110, 190, 255);
					for i in 0..4 {
						ciz.line_segment(
							[kose[i], kose[(i + 1) % 4]],
							egui::Stroke::new(1.2, renk),
						);
					}
					ciz.circle_filled(bb.center(), 3.0, renk);
					let metin_xy = format!(
						"XY  frx {:.0}°  fry {:.0}°",
						t.frx, t.fry
					);
					ciz.text(
						bb.left_top() + egui::vec2(4.0, -16.0),
						egui::Align2::LEFT_BOTTOM,
						metin_xy,
						egui::FontId::proportional(12.0),
						renk,
					);
				}
			}
			if let Some(hata) = &t.hata {
				ui.label(RichText::new(format!("typeset: {hata}")).weak());
			}
			out
		};
		if let Some(yeni) = commit {
			self.typeset_uygula(sec, yeni);
		}
	}

	/// Typeset sonucunu tek Commit olarak yaz; geri al/ileri anını tazele.
	fn typeset_uygula(&mut self, idx: usize, yeni: String) -> bool {
		let Some(h) = self.handle else {
			self.note = "typeset: açık dosya yok".into();
			return false;
		};
		self.son_an = None;
		if !self.an_tut(None) { return false; }
		match edit_line(h, idx, &yeni) {
			Ok(()) => {
				if let Some(r) = self.rows.get_mut(idx) {
					r.text = yeni.clone();
				}
				if let Some(t) = self.typeset.as_mut() {
					t.metin = yeni;
					t.son_satir = usize::MAX;
					t.son_metin.clear();
					t.clip_hazir = false;
					t.vektor_hazir = false;
					t.xy_hazir = false;
					t.oniz_sifirla();
				}
				self.oniz.son_ms = -1;
				self.note = format!("typeset yazıldı: satır {}", idx + 1);
				true
			}
			Err(e) => {
				if let Some(t) = self.typeset.as_mut() {
					t.clip_hazir = false;
					t.vektor_hazir = false;
					t.xy_hazir = false;
					t.oniz_sifirla();
				}
				self.note = format!("typeset yazılamadı: {e}");
				false
			}
		}
	}

	fn load_video_metadata(&mut self, path: &str, times: bool) {
		let Some(video) = self.video.as_mut() else { return };
		let result = std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|text| {
			if times {
				video.external_times = Some(crate::video_metadata::timecodes(&text, video.kareler as usize)?);
			} else {
				video.external_keys = Some(crate::video_metadata::keyframes(&text, video.kareler)?);
			}
			Ok(())
		});
		self.note = match result {
			Ok(()) => {
				if times { self.stop_timeline_playback(); }
				"Video zaman dosyası yüklendi".into()
			}
			Err(e) => e,
		};
	}

	fn stop_timeline_playback(&mut self) {
		if let Some(video) = self.video.as_mut() {
			video.oynat = false;
			video.son_tik = None;
		}
		if let Some(audio) = self.audio.as_ref() {
			unsafe { velnosub_abi_audio_stop(audio.handle) };
		}
	}

	fn video_metadata_menu(&mut self, ui: &mut egui::Ui) {
		let Some(video) = self.video.as_ref() else { return };
		let (has_times, has_keys) = (video.external_times.is_some(), video.external_keys.is_some());
		ui.menu_button("Zaman dosyaları", |menu| {
			if menu.button("Timecode aç…").clicked() {
				self.tarayici_ac(SecimTur::Timecodes);
				menu.close();
			}
			if menu.button("Keyframe aç…").clicked() {
				self.tarayici_ac(SecimTur::Keyframes);
				menu.close();
			}
			if menu.add_enabled(has_times, egui::Button::new("Kaynak kare zamanlarına dön")).clicked() {
				self.video.as_mut().unwrap().external_times = None;
				self.stop_timeline_playback();
				menu.close();
			}
			if menu.add_enabled(has_keys, egui::Button::new("Kaynak ana karelerine dön")).clicked() {
				self.video.as_mut().unwrap().external_keys = None;
				menu.close();
			}
		});
	}

	fn video_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		self.video_metadata_menu(ui);
		// P4.3: video şeridi — sanallaştırılmış grid'in üstünde; hata üstüne note
		let mut v_hata: Option<String> = None;
		// Task 10: typeset girişi/araç çubuğu — video ve seçili satır varsa
		self.typeset_arac_cubugu(ui);
		// P7.1: aegisub player penceresi (aktif satır) — v-blok ÖNCESİNDEN
		// hesap (self ödünç çakışması yok)
		let (_oyn_bas, oyn_son, oyn_d0, oyn_rate, oyn_toplam) =
			self.oynat_penceresi();
		// P6.4: oynat/dur/akış seçimi bayrakları — v-ödünç kapanışının
		// DIŞINDA ustalıkla işlenir (self çift-borçlanma yok)
		let mut oynat_istek = false;
		let mut oynat_d0ornek = 0i64;
		let mut oynat_adet = 0i64;
		let mut dur_istek = false;
		let mut aks_sec: Option<i64> = None;
		if let Some(v) = self.video.as_mut() {
			if v.dok.is_none() {
				if let Err(e) = v.cizim_cek(ui.ctx()) {
					v_hata = Some(e);
				}
			}
			let mut geri = egui::Button::new(format!("{} geri", ikon::SKIP_PREV));
			let mut ileri = egui::Button::new(format!("ileri {}", ikon::SKIP_NEXT));
			if mobil {
				geri = geri.min_size(egui::vec2(0.0, 44.0));
				ileri = ileri.min_size(egui::vec2(0.0, 44.0));
			}
			ui.horizontal(|su| {
				if su.add(geri).clicked() && v.frame > 0 {
					v.frame -= 1;
					if let Err(e) = v.cizim_cek(su.ctx()) {
						v_hata = Some(e);
					}
				}
				if su.add(ileri).clicked() && v.frame + 1 < v.kareler {
					v.frame += 1;
					if let Err(e) = v.cizim_cek(su.ctx()) {
						v_hata = Some(e);
					}
				}
				// P6.4: oynat/dur tekili; ses hattı varsa bayrak COMBO ile
				// v-blok sonrası berabence işlenir
				let mut oynat = egui::Button::new(if v.oynat {
					format!("{} dur", ikon::PAUSE)
				} else {
					format!("{} oynat", ikon::PLAY)
				});
				if mobil {
					oynat = oynat.min_size(egui::vec2(0.0, 44.0));
				}
				if su.add(oynat).clicked() {
					if v.oynat {
						v.oynat = false;
						v.son_tik = None;
						dur_istek = true;
					} else {
						// P7.1: oynat = aktif satır penceresi; başlangıç =
						// geçerli karenin ms'i (kare-odak kuralı)
						v.oynat = true;
						v.son_tik = Some(std::time::Instant::now());
						v.birim = 0.0;
						v.satir_bas = oyn_d0;
						v.satir_uzun = (oyn_son - oyn_d0).max(200);
						if oyn_rate > 0 {
							// ses örneği ofseti + adet — v-blok dışı uygulama
							oynat_d0ornek = (oyn_d0 * oyn_rate / 1000)
								.clamp(0, oyn_toplam.max(0));
							oynat_adet = ((oyn_son - oyn_d0).max(200)
								* oyn_rate
								/ 1000)
								.min((oyn_toplam - oynat_d0ornek).max(0));
						}
						// ilk kare = d0'ın karesi (senkron çapa)
						let hk = v.zamandaki_kare(oyn_d0);
						if hk != v.frame {
							v.frame = hk;
							let t0 = std::time::Instant::now();
							if let Err(e) = v.cizim_cek(su.ctx()) {
								v_hata = Some(e);
							}
							v.coz_ms = t0.elapsed().as_millis() as u32;
						}
						oynat_istek = true;
					}
				}
				for (label, forward) in [("Önceki ana kare", false), ("Sonraki ana kare", true)] {
					let index = v.keyframes().partition_point(|&frame| {
						frame < v.frame || (forward && frame == v.frame)
					});
					let target = if forward { Some(index) } else { index.checked_sub(1) };
					if let Some(&frame) = target.and_then(|index| v.keyframes().get(index)) {
						if su.small_button(label).clicked() {
							let previous = v.frame;
							v.frame = frame;
							if let Err(error) = v.cizim_cek(su.ctx()) {
								v.frame = previous;
								v_hata = Some(error);
							}
						}
					}
				}
				su.label(RichText::new(format!(
					"kare {}/{}",
					v.frame + 1, v.kareler
				)).weak());
			});
			ui.horizontal_wrapped(|controls| {
				controls.label("Video ölçeği");
				egui::ComboBox::from_id_salt("video-zoom")
					.selected_text(v.olcek_yuzdesi.map_or_else(|| "Sığdır".into(), |percent| format!("%{percent}")))
					.show_ui(controls, |choices| {
						choices.selectable_value(&mut v.olcek_yuzdesi, None, "Sığdır");
						for percent in [25, 50, 75, 100, 125, 150, 200] {
							choices.selectable_value(&mut v.olcek_yuzdesi, Some(percent), format!("%{percent}"));
						}
					});
				controls.label("En-boy oranı");
				egui::ComboBox::from_id_salt("video-aspect")
					.selected_text(v.preview_ratio.map_or_else(|| "Kaynak".into(), |ratio| format!("{ratio:.2}:1")))
					.show_ui(controls, |choices| {
						choices.selectable_value(&mut v.preview_ratio, None, "Kaynak");
						for (label, ratio) in [("4:3", 4.0 / 3.0), ("16:9", 16.0 / 9.0), ("2.39:1", 2.39)] {
							choices.selectable_value(&mut v.preview_ratio, Some(ratio), label);
						}
						if let Some(ratio) = v.preview_ratio.as_mut() {
							choices.add(egui::DragValue::new(ratio).range(0.25..=4.0).speed(0.01).fixed_decimals(3));
						}
					});
			});

			// P6.4: oynatma tick — fps-timer; kare başına bir decode; cihaz
			// yeterli hızda değilse adım kendiliğinden yavaşlar (gerçek
			// senkron yok: kare süresine göre ilerletme)
			// P7.1: aegisub player — master saat duvar; birim = ms bekçisi;
			// kare = (satir_bas + ms)·fps/1000; satır bitince otomatik dur
			if v.oynat {
				let simdi = std::time::Instant::now();
				let dsms = v
					.son_tik
					.map_or(0.0f32, |s| {
						simdi.duration_since(s).as_secs_f32() * 1000.0
					});
				v.son_tik = Some(simdi);
				v.birim += dsms;
				let hedef_ms = v.satir_bas as f64 + v.birim as f64;
				let hedef = v.zamandaki_kare(hedef_ms as i64);
				if hedef != v.frame {
					v.frame = hedef;
					// P6.5: kare-decode süresi ölçüyü — cihaz-hızı kanıtı
					let t0 = std::time::Instant::now();
					if let Err(e) = v.cizim_cek(ui.ctx()) {
						v_hata = Some(e);
					}
					v.coz_ms = t0.elapsed().as_millis() as u32;
				}
				if v.satir_uzun > 0 && v.birim >= v.satir_uzun as f32 {
					// satır sonu — aegisub kuralı: otomatik dur (ses kesilir)
					v.oynat = false;
					v.son_tik = None;
					dur_istek = true;
				} else {
					let tık = if v.fps > 0.5 { 1.0 / v.fps } else { 0.125 };
					ui.ctx()
						.request_repaint_after(
							std::time::Duration::from_secs_f64(
								tık.clamp(0.033, 0.25),
							),
						);
				}
			}
		}

		if let Some((video_handle, texture, width, height, ms, zoom, ratio)) = self.video.as_ref()
			.and_then(|video| Some((
				video.handle, video.dok.as_ref()?.id(), video.w, video.hgt,
				video.kare_zamani(video.frame), video.olcek_yuzdesi, video.preview_ratio,
			)))
		{
			let panel = [
				ui.available_width().min(ui.clip_rect().width()).max(1.0),
				(ui.clip_rect().bottom() - ui.cursor().min.y.max(ui.clip_rect().top()) - 28.0).max(72.0),
			];
			let size = layout::video_frame_size(
				[ratio.map_or(width as f32, |ratio| height as f32 * ratio), height as f32], panel, zoom, ui.ctx().pixels_per_point(),
			);
			if let Some(handle) = self.handle {
				self.oniz_cek(ui.ctx(), handle, width, height, ms);
			}
			let scroll_source = egui::scroll_area::ScrollSource {
				drag: if self.typeset.is_some() {
					egui::scroll_area::DragScroll::Never
				} else {
					egui::scroll_area::DragScroll::Always
				},
				..Default::default()
			};
			egui::ScrollArea::both()
				.id_salt(("video-frame", video_handle))
				.max_width(panel[0])
				.max_height(panel[1])
				.scroll_source(scroll_source)
				.show(ui, |frame| {
					let rect = frame.image((texture, egui::Vec2::from(size))).rect;
					if self.typeset.is_some() {
						self.typeset_cek(frame, rect, width, height, ms);
					} else if self.handle.is_some() && self.oniz.etki > 0 {
						if let Some(overlay) = &self.oniz.dok {
							frame.painter().image(
								overlay.id(), rect,
								egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
								Color32::WHITE,
							);
						}
					}
				});
			if self.handle.is_some() {
				if let Some(error) = &self.oniz.hata {
					ui.label(RichText::new(error).weak());
				}
			}
		}

		if let Some(v) = self.video.as_ref() {
			ui.label(RichText::new(if v.oynat || v.coz_ms > 0 {
				// P6.5: oynatma/decode kanıtı — kare ilerliyorsa görünür
				format!("{} · oynatma: {} ms/kare", &v.info, v.coz_ms)
			} else {
				v.info.clone()
			}).weak());
			// P6.4: iç altyazı akışı seçici — YALNIZ SEÇİCİ (auto-load yok);
			// seçim aks_sec'e yazılır, v-blok sonrası alt_yukle işler
			if !v.akslar.is_empty() {
				let akslar = v.akslar.clone();
				ui.horizontal(|ha| {
					ha.label(RichText::new("altyazı akışı:").weak());
					egui::ComboBox::from_id_salt("ic-altyazi-akisi")
						.selected_text(RichText::new("seç…").weak())
						.show_ui(ha, |c| {
							for (i, ko, di) in &akslar {
								if c
									.selectable_label(
										true,
										format!("akış {i}: {ko} {di}"),
									)
									.clicked()
								{
									aks_sec = Some(*i);
								}
							}
						});
				});
			}
			ui.add_space(4.0);
		}
		if let Some(e) = v_hata {
			self.note = format!("video doku hatası: {e}");
		}
		// P6.4: berabence — oynat/dur tetikleri SES hattına da dokunur
		// (aynı çerçeve; senkron garantisi yok — fare-deneysel kural)
		if dur_istek {
			if let Some(a) = self.audio.as_ref() {
				unsafe {
					velnosub_abi_audio_stop(a.handle);
				}
			}
		}
		if oynat_istek {
			if let Some(a) = self.audio.as_mut() {
				// P7.1: ses = aktif satır penceresinden başlar (d0 örnek
				// ofseti, aegisub "satırı oynat"); video karesi izler.
				// P6.4'ün 5-sn kesiti bitti: pencere döngüde gerçek.
				let d0 = oynat_d0ornek.min(a.toplam.max(0));
				let adet = oynat_adet.min((a.toplam - d0).max(0));
				if adet > 0 {
					let rc = unsafe {
						velnosub_abi_audio_play(a.handle, d0, adet)
					};
					a.oynatma_denendi = true;
					a.ses_durum = if rc == 0 {
						"oynatma istendi".into()
					} else {
						format!("oynatma olmadı: {}", last_error())
					};
				}
			}
		}
		if let Some(aks) = aks_sec {
			self.alt_yukle(aks);
		}
	}

	fn ses_icerik(&mut self, ui: &mut egui::Ui, mobil: bool, kompakt: bool) {
		// Ses paneli durumu `self.audio` içinde; satır zamanı yazımı ve video
		// fps'i için `self`e de erişmek gerektiğinden kol geçici olarak alınır.
		let Some(mut a) = self.audio.take() else { return };
		self.ses_govde(ui, &mut a, mobil, kompakt);
		self.audio = Some(a);
	}

	fn ses_govde(
		&mut self,
		ui: &mut egui::Ui,
		a: &mut AudioSection,
		mobil: bool,
		kompakt: bool,
	) {
		if !kompakt {
			ui.add_space(4.0);
		}
		let hedef = if mobil { 44.0 } else if kompakt { 18.0 } else { 24.0 };
		let satir_zaman = self
			.secili
			.and_then(|i| self.rows.get(i).map(|r| (i, r.bas.clone(), r.son.clone())))
			.and_then(|(i, b, s)| Some((i, ass_ms(&b).ok()?, ass_ms(&s).ok()?)));

		self.ses_oynatma_yokla(a);
		if a.oynatma_denendi {
			ui.ctx()
				.request_repaint_after(std::time::Duration::from_millis(33));
		}

		if kompakt {
			self.ses_kompakt_cubuk(ui, a, hedef);
		} else {
			self.ses_arac_cubugu(ui, a, mobil, hedef, satir_zaman);
		}

		let genislik = ui.available_width().max(1.0);
		let yuk = if kompakt {
			24.0
		} else if mobil {
			104.0
		} else {
			84.0
		};
		let (rul, rul_yanit) = ui.allocate_exact_size(
			egui::vec2(genislik, 14.0),
			egui::Sense::click_and_drag(),
		);
		let (dik, yanit) = ui.allocate_exact_size(
			egui::vec2(genislik, yuk),
			egui::Sense::click_and_drag(),
		);

		let zemin = ui.visuals().extreme_bg_color;
		ui.painter().rect_filled(rul, 2.0, zemin);
		ui.painter().rect_filled(dik, 2.0, ui.visuals().faint_bg_color);
		// cetvel çentikleri
		for i in 0..=8 {
			let x = rul.left() + rul.width() * (i as f32 / 8.0);
			let uzun = if i % 2 == 0 { 5.0 } else { 3.0 };
			ui.painter().line_segment(
				[
					egui::pos2(x, rul.bottom() - uzun),
					egui::pos2(x, rul.bottom()),
				],
				egui::Stroke::new(1.0, egui::Color32::from_gray(90)),
			);
		}

		// --- gezinme: cetvel sürükle = kaydır; tekerlek/pinch = imleç
		// altında yakınlaştır ---
		let mut g = ses_gorunum(a, genislik);
		if rul_yanit.dragged() {
			// `drag_delta()` sürükleme başından bu yana toplam deltadır;
			// her karede tekrar uygulanırsa görünüm hızlanır. Karelik
			// işaretçi deltasını kullan.
			let dx = ui.input(|i| i.pointer.delta().x);
			crate::audio_timing::kaydir(&mut g, dx, a.toplam);
			ses_gorunum_uygula(a, g);
		}
		if rul_yanit.clicked() {
			if let Some(p) = rul_yanit.interact_pointer_pos() {
				a.imlec = crate::audio_timing::px_to_ornek(
					p.x - dik.left(),
					&g,
				);
				a.imlec = a.imlec.clamp(0, a.toplam);
				a.secim = None;
			}
		}
		if rul_yanit.hovered() || yanit.hovered() {
			let (zd, sd) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
			let mut carpan = zd as f64;
			if sd.y.abs() > 0.01 {
				// tekerlek: yukarı = yakınlaş, aşağı = uzaklaş
				carpan *= (sd.y as f64 * 0.0025).exp();
			}
			if (carpan - 1.0).abs() > 1e-4 {
				let p = ui.input(|i| i.pointer.hover_pos());
				let capa = p
					.map(|p| (p.x - dik.left()).clamp(0.0, genislik))
					.unwrap_or(genislik * 0.5);
				crate::audio_timing::zoom_merkez(
					&mut g,
					carpan,
					capa,
					a.toplam,
				);
				ses_gorunum_uygula(a, g);
			}
		}

		// --- gövde: tıkla = imleç, sürükle = seçim ---
		let g = ses_gorunum(a, genislik);
		if yanit.drag_started() {
			if let Some(p) = yanit.interact_pointer_pos() {
				let s = crate::audio_timing::px_to_ornek(
					p.x - dik.left(),
					&g,
				)
				.clamp(0, a.toplam);
				a.surukleme = Some(s);
				a.secim = Some((s, s));
				a.imlec = s;
			}
		}
		if yanit.dragged() {
			if let Some(anchor) = a.surukleme {
				if let Some(p) = yanit.interact_pointer_pos() {
					let s = crate::audio_timing::px_to_ornek(
						p.x - dik.left(),
						&g,
					)
					.clamp(0, a.toplam);
					a.secim = Some(crate::audio_timing::secim_normalize(anchor, s));
					a.imlec = s;
				}
			}
		}
		if yanit.drag_stopped() {
			a.surukleme = None;
			if let Some((b, s)) = a.secim {
				if s <= b {
					a.secim = None;
				}
			}
		}
		if yanit.clicked() {
			if let Some(p) = yanit.interact_pointer_pos() {
				a.imlec = crate::audio_timing::px_to_ornek(
					p.x - dik.left(),
					&g,
				)
				.clamp(0, a.toplam);
				a.secim = None;
			}
		}

		// --- seçili satırın başlangıç/bitiş tutamaçları (bırakınca yaz) ---
		if let Some((idx, bas_ms, son_ms)) = satir_zaman {
			let bas_sabit = ms_ornek(bas_ms, a.rate).clamp(0, a.toplam);
			let son_sabit = ms_ornek(son_ms, a.rate).clamp(0, a.toplam);
			let bas_konum = a.tut_bas.unwrap_or(bas_sabit);
			let son_konum = a.tut_son.unwrap_or(son_sabit);
			let bx = dik.left() + crate::audio_timing::ornek_to_px(bas_konum, &g);
			let sx = dik.left() + crate::audio_timing::ornek_to_px(son_konum, &g);
			let bas_tut = egui::Rect::from_center_size(
				egui::pos2(bx, dik.top() + 8.0),
				egui::vec2(15.0, 14.0),
			);
			let son_tut = egui::Rect::from_center_size(
				egui::pos2(sx, dik.top() + 8.0),
				egui::vec2(15.0, 14.0),
			);
			let sag = a.d0.saturating_add(a.adet);
			let yb = (bas_konum >= a.d0 && bas_konum <= sag).then(|| {
				ui.interact(
					bas_tut,
					ui.scope_id().with(("ses-tut-bas", idx)),
					egui::Sense::drag(),
				)
			});
			let ys = (son_konum >= a.d0 && son_konum <= sag).then(|| {
				ui.interact(
					son_tut,
					ui.scope_id().with(("ses-tut-son", idx)),
					egui::Sense::drag(),
				)
			});
			if let Some(yb) = yb {
				if yb.drag_started() {
					a.tut_bas = Some(bas_sabit);
				}
				if yb.dragged() {
					if let Some(p) = yb.interact_pointer_pos() {
						a.tut_bas = Some(
							crate::audio_timing::px_to_ornek(
								p.x - dik.left(),
								&g,
							)
							.clamp(0, a.toplam),
						);
					}
				}
				if yb.drag_stopped() {
					if let Some(v) = a.tut_bas.take() {
						self.ses_tut_birak(a, idx, true, v);
					}
				}
				if yb.clicked() {
					a.imlec = bas_sabit;
				}
			}
			if let Some(ys) = ys {
				if ys.drag_started() {
					a.tut_son = Some(son_sabit);
				}
				if ys.dragged() {
					if let Some(p) = ys.interact_pointer_pos() {
						a.tut_son = Some(
							crate::audio_timing::px_to_ornek(
								p.x - dik.left(),
								&g,
							)
							.clamp(0, a.toplam),
						);
					}
				}
				if ys.drag_stopped() {
					if let Some(v) = a.tut_son.take() {
						self.ses_tut_birak(a, idx, false, v);
					}
				}
				if ys.clicked() {
					a.imlec = son_sabit;
				}
			}
		}

		// --- görünür aralık: spektrum (önbellekli) veya dalga ---
		let mut spek_cizildi = false;
		if a.mod_ == SesMod::Spektrum {
			let cols = ((genislik * 0.5).round() as i32).clamp(1, 192);
			let rows = if kompakt { 32 } else { 96 };
			if ses_spek_hazirla(ui.ctx(), a, cols, rows) {
				spek_cizildi = true;
			}
		}
		if spek_cizildi {
			if let Some(dok) = a.spek_doku.as_ref() {
				ui.painter().image(
					dok.id(),
					dik,
					egui::Rect::from_min_max(
						egui::pos2(0.0, 0.0),
						egui::pos2(1.0, 1.0),
					),
					egui::Color32::WHITE,
				);
			}
		} else {
			ses_dalga_ciz(ui.painter(), a, dik, &g);
		}

		// --- katmanlar: seçim, imleç, oynatma başı, satır tutamaçları ---
		let boya = ui.painter();
		if let Some((b, s)) = a.secim {
			if s > b {
				let x1 = dik.left() + crate::audio_timing::ornek_to_px(b, &g);
				let x2 = dik.left() + crate::audio_timing::ornek_to_px(s, &g);
				let sec = egui::Rect::from_min_max(
					egui::pos2(x1, dik.top()),
					egui::pos2(x2, dik.bottom()),
				);
				boya.rect_filled(
					sec,
					0.0,
					egui::Color32::from_rgba_unmultiplied(80, 170, 240, 46),
				);
				for x in [x1, x2] {
					boya.line_segment(
						[
							egui::pos2(x, dik.top()),
							egui::pos2(x, dik.bottom()),
						],
						egui::Stroke::new(
							1.0,
							egui::Color32::from_rgba_unmultiplied(120, 190, 255, 180),
						),
					);
				}
			}
		}
		let imlec_x = dik.left() + crate::audio_timing::ornek_to_px(a.imlec, &g);
		if a.imlec >= a.d0 && a.imlec <= a.d0 + a.adet {
			boya.line_segment(
				[
					egui::pos2(imlec_x, dik.top()),
					egui::pos2(imlec_x, dik.bottom()),
				],
				egui::Stroke::new(1.5, egui::Color32::from_rgb(245, 205, 80)),
			);
		}
		if a.konum >= 0 && a.konum >= a.d0 && a.konum <= a.d0 + a.adet {
			let x = dik.left() + crate::audio_timing::ornek_to_px(a.konum, &g);
			boya.line_segment(
				[
					egui::pos2(x, dik.top()),
					egui::pos2(x, dik.bottom()),
				],
				egui::Stroke::new(1.5, egui::Color32::from_rgb(90, 230, 140)),
			);
		}
		if let Some((_idx, bas_ms, son_ms)) = satir_zaman {
			let bas_sabit = ms_ornek(bas_ms, a.rate).clamp(0, a.toplam);
			let son_sabit = ms_ornek(son_ms, a.rate).clamp(0, a.toplam);
			let bas = a.tut_bas.unwrap_or(bas_sabit);
			let son = a.tut_son.unwrap_or(son_sabit);
			for (ornek, ad, renk) in [
				(bas, "B", egui::Color32::from_rgb(255, 170, 70)),
				(son, "S", egui::Color32::from_rgb(190, 120, 255)),
			] {
				if ornek < a.d0 || ornek > a.d0 + a.adet {
					continue;
				}
				let x = dik.left() + crate::audio_timing::ornek_to_px(ornek, &g);
				boya.line_segment(
					[
						egui::pos2(x, dik.top()),
						egui::pos2(x, dik.bottom()),
					],
					egui::Stroke::new(1.0, renk),
				);
				let tut = egui::Rect::from_center_size(
					egui::pos2(x, dik.top() + 8.0),
					egui::vec2(15.0, 14.0),
				);
				boya.rect_filled(tut, 2.0, renk);
				boya.text(
					tut.center(),
					egui::Align2::CENTER_CENTER,
					ad,
					egui::FontId::proportional(9.0),
					egui::Color32::BLACK,
				);
			}
		}

		if !kompakt {
			ui.add_space(2.0);
			ui.horizontal_wrapped(|h| {
				match a.secim {
					Some((b, s)) => h.label(format!(
						"seçim {}–{} ms",
						ornek_ms(b, a.rate),
						ornek_ms(s, a.rate)
					)),
					None => h.label("seçim yok"),
				};
				h.label(format!("imleç {} ms", ornek_ms(a.imlec, a.rate)));
				if let Some((_, b, s)) = satir_zaman {
					h.label(format!("satır {b}–{s} ms"));
				} else {
					h.label("satır seç");
				}
			});
			ui.label(RichText::new(&a.info).weak());
			ui.label(RichText::new(&a.ses_durum).weak());
			if a.secim.is_none() || satir_zaman.is_none() {
				ui.label(
					RichText::new(
						"Başlat/Bitir için dalgada seçim yap ve grid'den satır seç.",
					)
					.weak(),
				);
			}
			if let Some(e) = &a.spek_hata {
				ui.label(
					RichText::new(format!("spektrum: {e}"))
						.color(egui::Color32::from_rgb(220, 120, 90)),
				);
			}
		} else if let Some(e) = &a.spek_hata {
			ui.label(
				RichText::new(format!("spektrum: {e}"))
					.color(egui::Color32::from_rgb(220, 120, 90)),
			);
		} else if a.ses_durum.starts_with("oynatma olmadı")
			|| a.ses_durum.starts_with("dalga okunamadı")
		{
			ui.label(
				RichText::new(a.ses_durum.clone())
					.color(egui::Color32::from_rgb(220, 120, 90)),
			);
		}
		// Oynatma bu karede araç çubuğundan başladıysa yukarıdaki erken
		// istek kaçırılır; kare sonunda da iste.
		if a.oynatma_denendi {
			ui.ctx()
				.request_repaint_after(std::time::Duration::from_millis(33));
		}
	}

	/// Kompakt Aegisub ses şeridi: tek satır + 24 pt dalga; durum etiketi yok.
	fn ses_kompakt_cubuk(
		&mut self,
		ui: &mut egui::Ui,
		a: &mut AudioSection,
		hedef: f32,
	) {
		let gen = ui.available_width().max(1.0);
		let capa = gen * 0.5;
		ui.horizontal(|h| {
			if h
				.add(egui::Button::new(ikon::PLAY).min_size(egui::vec2(24.0, hedef)))
				.clicked()
			{
				self.ses_imlecten_oynat(a);
			}
			if h
				.add(egui::Button::new(ikon::STOP).min_size(egui::vec2(24.0, hedef)))
				.clicked()
			{
				self.ses_durdur(a);
			}
			let mod_ad = if a.mod_ == SesMod::Dalga { "Dalga" } else { "Spek" };
			if h
				.add(egui::Button::new(mod_ad).min_size(egui::vec2(0.0, hedef)))
				.clicked()
			{
				a.mod_ = if a.mod_ == SesMod::Dalga {
					SesMod::Spektrum
				} else {
					SesMod::Dalga
				};
			}
			if h
				.add(egui::Button::new("−").min_size(egui::vec2(22.0, hedef)))
				.clicked()
			{
				let mut g = ses_gorunum(a, gen);
				crate::audio_timing::zoom_merkez(&mut g, 0.5, capa, a.toplam);
				ses_gorunum_uygula(a, g);
			}
			if h
				.add(egui::Button::new("+").min_size(egui::vec2(22.0, hedef)))
				.clicked()
			{
				let mut g = ses_gorunum(a, gen);
				crate::audio_timing::zoom_merkez(&mut g, 2.0, capa, a.toplam);
				ses_gorunum_uygula(a, g);
			}
			if h
				.add(egui::Button::new("Büyüt").min_size(egui::vec2(0.0, hedef)))
				.clicked()
			{
				self.ses_win = true;
			}
		});
	}

	/// Tam ses araç çubuğu (masaüstü, mobil sekme ve zamanlama penceresi).
	fn ses_arac_cubugu(
		&mut self,
		ui: &mut egui::Ui,
		a: &mut AudioSection,
		_mobil: bool,
		hedef: f32,
		satir: Option<(usize, i64, i64)>,
	) {
		let gen = ui.available_width().max(1.0);
		let capa = gen * 0.5;
		let secim = a.secim.filter(|(b, s)| s > b);
		ui.horizontal_wrapped(|h| {
			for (mod_, ad) in [(SesMod::Dalga, "Dalga"), (SesMod::Spektrum, "Spektrum")]
			{
				if h
					.add(
						egui::Button::new(ad)
							.selected(a.mod_ == mod_)
							.min_size(egui::vec2(0.0, hedef)),
					)
					.clicked()
				{
					a.mod_ = mod_;
				}
			}
			h.separator();
			if h
				.add(egui::Button::new("−").min_size(egui::vec2(26.0, hedef)))
				.clicked()
			{
				let mut g = ses_gorunum(a, gen);
				crate::audio_timing::zoom_merkez(&mut g, 0.5, capa, a.toplam);
				ses_gorunum_uygula(a, g);
			}
			if h
				.add(egui::Button::new("+").min_size(egui::vec2(26.0, hedef)))
				.clicked()
			{
				let mut g = ses_gorunum(a, gen);
				crate::audio_timing::zoom_merkez(&mut g, 2.0, capa, a.toplam);
				ses_gorunum_uygula(a, g);
			}
			if h
				.add(egui::Button::new("Sığdır").min_size(egui::vec2(0.0, hedef)))
				.clicked()
			{
				a.d0 = 0;
				a.adet = a.toplam.max(1);
			}
			h.separator();
			if h
				.add(egui::Button::new(ikon::ARROW_BACK).min_size(egui::vec2(26.0, hedef)))
				.clicked()
			{
				let mut g = ses_gorunum(a, gen);
				crate::audio_timing::kaydir(&mut g, gen * 0.5, a.toplam);
				ses_gorunum_uygula(a, g);
			}
			if h
				.add(egui::Button::new(ikon::ARROW_FWD).min_size(egui::vec2(26.0, hedef)))
				.clicked()
			{
				let mut g = ses_gorunum(a, gen);
				crate::audio_timing::kaydir(&mut g, -gen * 0.5, a.toplam);
				ses_gorunum_uygula(a, g);
			}
			h.separator();
			if h
				.add_enabled(
					secim.is_some(),
					egui::Button::new(format!("{} seçim", ikon::PLAY))
						.min_size(egui::vec2(0.0, hedef)),
				)
				.clicked()
			{
				if let Some((b, s)) = secim {
					self.ses_oynat(a, b, s - b);
				}
			}
			if h
				.add(egui::Button::new(format!("{} imleç", ikon::PLAY)).min_size(egui::vec2(0.0, hedef)))
				.clicked()
			{
				self.ses_imlecten_oynat(a);
			}
			if h
				.add(egui::Button::new(ikon::STOP).min_size(egui::vec2(0.0, hedef)))
				.clicked()
			{
				self.ses_durdur(a);
			}
			h.separator();
			let yazilabilir = secim.is_some() && satir.is_some();
			if h
				.add_enabled(
					yazilabilir,
					egui::Button::new("Başlat")
						.min_size(egui::vec2(0.0, hedef)),
				)
				.clicked()
			{
				if let (Some((b, _)), Some((idx, _, son))) = (secim, satir) {
					self.ses_sinir_yaz(a, idx, true, b, son);
				}
			}
			if h
				.add_enabled(
					yazilabilir,
					egui::Button::new("Bitir")
						.min_size(egui::vec2(0.0, hedef)),
				)
				.clicked()
			{
				if let (Some((_, s)), Some((idx, bas, _))) = (secim, satir) {
					self.ses_sinir_yaz(a, idx, false, bas, s);
				}
			}
			if h
				.add(
					egui::Button::new("Kareye oturt")
						.min_size(egui::vec2(0.0, hedef)),
				)
				.clicked()
			{
				self.ses_kareye_oturt(a, satir);
			}
			h.checkbox(&mut a.takip, "takip");
			if self.ses_win
				&& h
					.add(egui::Button::new("Küçült").min_size(egui::vec2(0.0, hedef)))
					.clicked()
			{
				self.ses_win = false;
			}
		});
	}

	/// Oynatma durumunu yokla; takip açıksa oynatma başını görünür aralıkta tut.
	fn ses_oynatma_yokla(&mut self, a: &mut AudioSection) {
		if !a.oynatma_denendi {
			return;
		}
		let d = unsafe { velnosub_abi_audio_state(a.handle) };
		let ad = match d {
			-1 => "oynatma yok",
			4 => "hazırlanıyor",
			0 => "oynatılıyor",
			1 => "boşalma",
			2 => "hata",
			_ => "bilinmeyen",
		};
		if d == 0 || d == 1 {
			let p = unsafe { velnosub_abi_audio_position(a.handle) };
			if p >= 0 {
				a.konum = p;
				a.imlec = p.clamp(0, a.toplam);
			}
			if d == 0 && a.takip {
				// Sayfa %80'i: her karede yeniden hesaplamak yerine ancak
				// oynatma başı görünür aralıktan çıkınca kaydır.
				let sag = a.d0.saturating_add(a.adet);
				if a.konum >= 0 && (a.konum < a.d0 || a.konum > sag) {
					let adet = a.adet.max(1);
					let maks = (a.toplam - adet).max(0);
					a.d0 = (a.konum - adet * 4 / 5).clamp(0, maks);
				}
			}
		}
		if d == 1 {
			a.oynatma_denendi = false;
			a.ses_durum = "oynatma bitti".into();
		} else if d == 2 || d == -1 {
			a.oynatma_denendi = false;
			a.ses_durum = format!("durum {d} — {ad}");
		} else if d == 0 || d == 1 || d == 4 {
			a.ses_durum = if a.konum >= 0 {
				format!(
					"durum {d} — {ad} — {} ms",
					ornek_ms(a.konum, a.rate)
				)
			} else {
				format!("durum {d} — {ad}")
			};
		}
	}

	fn ses_oynat(&mut self, a: &mut AudioSection, bas: i64, adet: i64) {
		if a.toplam <= 0 {
			self.note = "ses yok".into();
			return;
		}
		let bas = bas.clamp(0, a.toplam - 1);
		let son = bas.saturating_add(adet.max(0)).min(a.toplam);
		if son <= bas {
			self.note = "oynatma aralığı boş".into();
			return;
		}
		let rc = unsafe { velnosub_abi_audio_play(a.handle, bas, son - bas) };
		if rc == 0 {
			a.oynatma_denendi = true;
			a.konum = bas;
			a.takip = true;
			a.ses_durum = format!(
				"oynatma {}–{} ms",
				ornek_ms(bas, a.rate),
				ornek_ms(son, a.rate)
			);
		} else {
			a.oynatma_denendi = false;
			a.konum = -1;
			a.ses_durum = format!("oynatma olmadı: {}", last_error());
			self.note = a.ses_durum.clone();
		}
	}

	fn ses_imlecten_oynat(&mut self, a: &mut AudioSection) {
		if a.toplam <= 0 {
			self.note = "ses yok".into();
			return;
		}
		if a.imlec >= a.toplam {
			a.imlec = a.toplam - 1;
		}
		let adet = a.toplam - a.imlec;
		self.ses_oynat(a, a.imlec, adet);
	}

	fn ses_durdur(&mut self, a: &mut AudioSection) {
		unsafe { velnosub_abi_audio_stop(a.handle) };
		a.oynatma_denendi = false;
		a.konum = -1;
		a.ses_durum = "kesildi".into();
	}

	fn ses_zaman_yaz(
		&mut self,
		idx: usize,
		ms_bas: i64,
		ms_son: i64,
	) -> Result<(), String> {
		let h = self.handle.ok_or_else(|| "altyazı açık değil".to_string())?;
		if !self.an_tut(None) { return Err(self.note.clone()); }
		times_set(h, idx, ms_bas, ms_son)?;
		if let Some(r) = self.rows.get_mut(idx) {
			r.bas = ms_ass(ms_bas);
			r.son = ms_ass(ms_son);
		}
		Ok(())
	}

	/// "Başlat" = seçim başı, "Bitir" = seçim sonu; diğer uç satırdan gelir.
	fn ses_sinir_yaz(
		&mut self,
		a: &AudioSection,
		idx: usize,
		bas_mi: bool,
		sinir_ornek: i64,
		diger_ms: i64,
	) {
		let yeni_ms = ornek_ms(sinir_ornek.clamp(0, a.toplam), a.rate).max(0);
		let (mb, ms) = if bas_mi {
			(yeni_ms.min(diger_ms), diger_ms)
		} else {
			(diger_ms, yeni_ms.max(diger_ms))
		};
		match self.ses_zaman_yaz(idx, mb, ms) {
			Ok(()) => self.note = format!("satır {}: {mb}–{ms} ms", idx + 1),
			Err(e) => self.note = format!("zaman yazılamadı: {e}"),
		}
	}

	/// Tutamaç bırakıldı: video varsa kareye oturt, sonra tek ABI yazımı.
	fn ses_tut_birak(
		&mut self,
		a: &mut AudioSection,
		idx: usize,
		bas_mi: bool,
		ornek: i64,
	) {
		let mut hedef = ornek.clamp(0, a.toplam);
		if let Some(video) = &self.video {
			let ms = video.en_yakin_kare_zamani(ornek_ms(hedef, a.rate));
			hedef = ms_ornek(ms, a.rate).clamp(0, a.toplam);
		} else {
			self.note = "video yok, kareye oturtma kapalı".into();
		}
		let Some((mb, ms)) = self
			.rows
			.get(idx)
			.and_then(|r| Some((ass_ms(&r.bas).ok()?, ass_ms(&r.son).ok()?)))
		else {
			return;
		};
		let (yb, ys) = if bas_mi {
			(ornek_ms(hedef, a.rate).min(ms), ms)
		} else {
			(mb, ornek_ms(hedef, a.rate).max(mb))
		};
		match self.ses_zaman_yaz(idx, yb, ys) {
			Ok(()) => {
				a.imlec = hedef;
				self.note = format!("satır {}: {yb}–{ys} ms", idx + 1);
			}
			Err(e) => self.note = format!("tutamaç yazılamadı: {e}"),
		}
	}

	fn ses_kareye_oturt(
		&mut self,
		a: &mut AudioSection,
		satir: Option<(usize, i64, i64)>,
	) {
		let Some(video) = &self.video else {
			self.note = "video yok, kareye oturtma kapalı".into();
			return;
		};
		let oturt = |sample| ms_ornek(video.en_yakin_kare_zamani(ornek_ms(sample, a.rate)), a.rate);
		if let Some((idx, mb, ms)) = satir {
			let nb = oturt(ms_ornek(mb, a.rate))
			.clamp(0, a.toplam);
			let ns = oturt(ms_ornek(ms, a.rate))
			.clamp(nb, a.toplam);
			let (nb_ms, ns_ms) = (ornek_ms(nb, a.rate), ornek_ms(ns, a.rate));
			match self.ses_zaman_yaz(idx, nb_ms, ns_ms) {
				Ok(()) => {
					a.secim = Some((nb, ns));
					a.imlec = nb;
					self.note = format!("satır {} kareye oturtuldu", idx + 1);
				}
				Err(e) => self.note = format!("kareye oturtma yazılamadı: {e}"),
			}
			return;
		}
		let Some((b, s)) = a.secim else {
			a.imlec = oturt(a.imlec)
				.clamp(0, a.toplam);
			return;
		};
		let nb = oturt(b)
			.clamp(0, a.toplam);
		let ns = oturt(s)
			.clamp(nb, a.toplam);
		a.secim = Some((nb, ns));
		a.imlec = nb;
	}

	fn stil_icerik(&mut self, ui: &mut egui::Ui, _mobil: bool) {
		// P4.5: stiller şeridi — ad/font özeti + canlı boyut düzenleme
		if !self.stiller.is_empty() {
			ui.add_space(4.0);
			ui.label(
				RichText::new(format!("Stiller ({})", self.stiller.len()))
					.strong(),
			);
			if ui
				.button(format!("{} Stil Yöneticisi…", ikon::PALETTE))
				.clicked()
			{
				self.stil_win = true;
			}
			let kopya = self.stiller.clone();
			for (i, r) in kopya.iter().enumerate() {
				ui.horizontal(|ui| {
					let mut v = r.size;
					let yanit = ui.add(
						egui::DragValue::new(&mut v)
							.speed(0.5)
							.range(2.0f64..=9999.0)
							.suffix(" pts"),
					);
					if yanit.changed() {
						if let Some(h) = self.handle {
							if !self.an_tut(None) { return; }
							let rc = unsafe {
								velnosub_abi_style_edit_size(h, i as c_int, v)
							};
							if rc == 0 {
								self.stiller[i].size = v;
								self.note =
									format!("stil {}: boyut {:.1}", r.name, v);
							} else {
								self.note = format!("stil boyutu hatası: {}", last_error());
							}
						}
					}
					// font kutusu: odak kaybında ABI yazma hattı
					let mut font_dsk = r.font.clone();
					let font_yanit = ui.add(
						egui::TextEdit::singleline(&mut font_dsk).desired_width(96.0),
					);
					if font_yanit.lost_focus()
						&& font_dsk != r.font
						&& !font_dsk.trim().is_empty()
					{
						if let Some(h) = self.handle {
							let yazi = font_dsk.trim().to_string();
							let yazi_not = yazi.clone();
							if !self.an_tut(None) { return; }
							match stil_yaz(h, i as c_int, |hc| hc[1] = yazi) {
								Ok(()) => {
									self.note = format!(
										"stil {}: font {}",
										r.name, yazi_not
									);
									self.load_styles();
								}
								Err(e) => self.note = format!("stil font hatası: {e}"),
							}
						}
					}
					// ana renk: ASS <-> Color32 akışı (alpha tersi ABI'da)
					let mut renk = ass_to_color32(&r.primary)
						.unwrap_or(egui::Color32::WHITE);
					if ui
						.color_edit_button_srgba(&mut renk)
						.changed()
					{
						let ass = color32_to_ass(renk);
						if let Some(h) = self.handle {
							if !self.an_tut(None) { return; }
							match stil_yaz(h, i as c_int, |hc| hc[3] = ass.clone()) {
								Ok(()) => {
									self.note = format!("stil {}: ana renk {ass}", r.name);
									self.load_styles();
								}
								Err(e) => self.note = format!("stil renk hatası: {e}"),
							}
						}
					}
					// hiza 1..9 açılır listesi
					let eski_hiza = r.alignment;
					let mut secim = r.alignment.clamp(1, 9);
					egui::ComboBox::from_id_salt(format!("hiza{i}"))
						.selected_text(format!("\u{5c}an{secim}"))
						.width(56.0)
						.show_ui(ui, |kut| {
							for k in 0..9 {
								kut.selectable_value(
									&mut secim, k as i32 + 1,
									format!("\u{5c}an{}", k + 1),
								);
							}
						});
					if secim != eski_hiza.clamp(1, 9) {
						if let Some(h) = self.handle {
							if !self.an_tut(None) { return; }
							match stil_yaz(
								h, i as c_int,
								|hc| hc[8] = secim.to_string(),
							) {
								Ok(()) => {
									self.note =
										format!("stil {}: hiza \\an{secim}", r.name);
									self.load_styles();
								}
								Err(e) => self.note = format!("stil hiza hatası: {e}"),
							}
						}
					}
					ui.label(RichText::new(format!(
						"{} — {} · p{} s{} o{} k{} · bi{} ken[{}] kod{} ölç[{}] {}",
						r.name, r.font, r.primary, r.secondary, r.outline,
						r.shadow, r.flags, r.margins, r.encoding, r.scale, r.edge
					))
					.weak());
				});
			}
			ui.add_space(4.0);
		}
	}

	fn dialog_fields(ui: &mut egui::Ui, fd: &mut DialogSection) {
		for (i, a) in fd.alanlar.iter_mut().enumerate() {
			match a.sinif.as_str() {
				"label" => {
					ui.label(RichText::new(if a.deger.is_empty() {
						a.baslik.clone()
					} else {
						a.deger.clone()
					})
					.weak());
				}
				"edit" => {
					let gen = ui.available_width() - 120.0;
					ui.horizontal(|kut| {
						kut.label(RichText::new(&a.baslik).strong());
						kut.add(
							egui::TextEdit::singleline(&mut a.deger)
								.desired_width(gen),
						);
					});
				}
				"textbox" => {
					ui.label(RichText::new(&a.baslik).strong());
					ui.add(
						egui::TextEdit::multiline(&mut a.deger)
							.desired_rows(3),
					);
				}
				"intedit" | "floatedit" => {
					ui.horizontal(|kut| {
						kut.label(RichText::new(&a.baslik).strong());
						if a.sinif == "intedit" {
							let mut sayi =
								a.deger.parse::<i64>().unwrap_or(a.alt as i64);
							if kut.add(
								egui::DragValue::new(&mut sayi)
									.speed(1.0)
									.range(a.alt as i64..=a.ust as i64),
							).changed() {
								a.deger = sayi.to_string();
							}
						} else {
							let mut y = a.deger.parse::<f64>().unwrap_or(a.alt);
							if kut.add(
								egui::DragValue::new(&mut y)
									.speed(0.05)
									.range(a.alt..=a.ust),
							).changed() {
								a.deger = format!("{y:.2}");
							}
						}
					});
				}
				"checkbox" => {
					let mut sec = a.deger == "1";
					if ui.checkbox(&mut sec, RichText::new(&a.baslik).strong()).changed() {
						a.deger = if sec { "1".into() } else { "0".into() };
					}
				}
				"dropdown" => {
					ui.horizontal(|kut| {
						kut.label(RichText::new(&a.baslik).strong());
						egui::ComboBox::from_id_salt(format!("dsec{i}"))
							.selected_text(&a.deger)
							.show_ui(kut, |ac| {
								let secenekler: Vec<String> = a.ogeler.clone();
								for oge in secenekler {
									ac.selectable_value(
										&mut a.deger, oge.clone(), oge,
									);
								}
							});
					});
				}
				"color" | "coloralpha" => {
					ui.horizontal(|kut| {
						kut.label(RichText::new(&a.baslik).strong());
						let mut renk = ass_to_color32(&a.deger)
							.unwrap_or(egui::Color32::WHITE);
						if kut.color_edit_button_srgba(&mut renk).changed() {
							a.deger = color32_to_ass(renk);
						}
					});
				}
				_ => {
					ui.label(RichText::new(format!("? {:?}", a.sinif)).weak());
				}
			}
		}
	}

	#[cfg(target_os = "android")]
	fn stage_macro_export(&mut self, name: &str, suggested: &str) -> Result<String, String> {
		if self.macro_export.is_some() {
			return Err("Android makroları bir çalıştırmada tek çıktı dosyası kaydedebilir".into());
		}
		static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
		let cache = std::env::var_os("VELNOSUB_CACHE").map(std::path::PathBuf::from)
			.unwrap_or_else(std::env::temp_dir);
		let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		let folder = cache.join(format!("velnosub-macro-{}-{id}", std::process::id()));
		std::fs::create_dir(&folder).map_err(|e| e.to_string())?;
		let filename = suggested.rsplit(['/', '\\']).next()
			.filter(|name| !name.is_empty() && *name != "." && *name != "..")
			.unwrap_or("makro.txt");
		let path = folder.join(filename);
		let result = path.to_string_lossy().into_owned();
		self.macro_export = Some(MacroExport { path, document: None, name: name.to_owned() });
		Ok(result)
	}

	#[cfg(target_os = "android")]
	fn finish_macro_open(&mut self, path: &str) {
		if let Some(mut form) = self.macro_form.take() {
			if let Some(field) = form.dialog.alanlar.iter_mut().find(|f| f.ad == "__velnosub_open_file") {
				field.deger = path.to_owned();
				self.submit_macro_form(form, 0);
			} else {
				self.macro_form = Some(form);
			}
		}
	}

	fn abort_macro(&mut self, form: MacroForm) {
		unsafe { velnosub_abi_macro_abort(form.token) };
		#[cfg(target_os = "android")]
		{ self.macro_export = None; }
		self.note = "Makro iptal edildi".into();
	}

	fn macro_dialog(&mut self, ctx: &egui::Context) {
		let Some(mut form) = self.macro_form.take() else { return };
		let mut selected = None;
		let mut abort = false;
		egui::Modal::new(egui::Id::new("lua_macro_dialog")).show(ctx, |ui| {
			ui.heading(&form.name);
			egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
				Self::dialog_fields(ui, &mut form.dialog);
			});
			ui.horizontal_wrapped(|ui| {
				for (i, button) in form.dialog.dugmeler.iter().enumerate() {
					if ui.button(button).clicked() { selected = Some(i); }
				}
				if ui.button("Makroyu iptal et").clicked() { abort = true; }
			});
			ui.label(&self.note);
		});
		if abort {
			self.abort_macro(form);
			return;
		}
		let Some(button) = selected else {
			self.macro_form = Some(form);
			return;
		};
		#[cfg(target_os = "android")]
		if button == 0 {
			if form.dialog.alanlar.iter().any(|field| field.ad == "__velnosub_open_file") {
				self.macro_form = Some(form);
				self.android_belge_sec(AndroidBelge::MacroOpen, 10, "", "");
				return;
			}
			if let Some(field) = form.dialog.alanlar.iter_mut().find(|f| f.ad == "__velnosub_save_file") {
				match self.stage_macro_export(&form.name, &field.deger) {
					Ok(path) => field.deger = path,
					Err(error) => {
						self.note = error;
						self.macro_form = Some(form);
						return;
					}
				}
			}
		}
		self.submit_macro_form(form, button);
	}

	fn submit_macro_form(&mut self, form: MacroForm, button: usize) {
		for (index, field) in form.dialog.alanlar.iter().enumerate() {
			if field.sinif == "label" { continue; }
			let status = CString::new(field.deger.as_str())
				.map(|value| unsafe { velnosub_abi_dialog_set(form.dialog.handle, index as c_int, value.as_ptr()) });
			if !matches!(status, Ok(0)) {
				self.note = format!("Geçersiz form alanı: {}", field.ad);
				self.macro_form = Some(form);
				return;
			}
		}
		let mut result = -1;
		let status = unsafe { velnosub_abi_macro_continue(form.token, button as c_int, &mut result) };
		self.macro_step(&form.name, form.token, status, result);
	}

	fn form_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		// P4.5 III: dialog form şeridi — sınıflara göre egui widget eşlemesi;
		// düğme bölümü alan ödünç alanı bırakıldıktan sonra gelir
		let dug = match self.dialog.as_mut() {
			None => None,
			Some(fd) => {
				ui.add_space(4.0);
				ui.label(
					RichText::new(format!("Makro formu: {} alan, {} düğme",
						fd.alanlar.len(), fd.dugmeler.len()))
					.strong(),
				);
				Self::dialog_fields(ui, fd);
				Some(fd.dugmeler.clone())
			}
		};
		let Some(dug) = dug else { return };
		ui.add_space(6.0);
		if !mobil {
			ui.horizontal(|kut| {
				for (bi, ad) in dug.iter().enumerate() {
					if kut.button(RichText::new(ad).strong()).clicked() {
						self.form_gonder(bi, ad);
					}
				}
			});
		} else {
			// mobil: her düğme dolu genişlik ve 44pt yükseklik
			let gen = ui.available_width();
			for (bi, ad) in dug.iter().enumerate() {
				if ui.add(
					egui::Button::new(RichText::new(ad).strong())
						.min_size(egui::vec2(gen, 44.0)),
				).clicked() {
					self.form_gonder(bi, ad);
				}
			}
		}
		ui.add_space(4.0);
	}

	fn durum_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		// P5.5: kendi dosyalarını açma — iki yüzeyde de erişilir dokunuşlar
		ui.horizontal(|uh| {
			let boy = if mobil { 15.0 } else { 13.0 };
			if uh.button("Komutlar").clicked() {
				self.command_query = Some(String::new());
			}
			if uh.button(RichText::new("Yeni altyazı").size(boy)).clicked() {
				self.request_new_document();
			}
			if uh.button(RichText::new("Altyazı aç…").size(boy)).clicked() {
				self.tarayici_ac(SecimTur::Altyazi);
			}
			if uh.button(RichText::new("Video aç…").size(boy)).clicked() {
				self.tarayici_ac(SecimTur::Video);
			}
			if uh.button(RichText::new("Ses aç…").size(boy)).clicked() {
				self.tarayici_ac(SecimTur::Ses);
			}
		});
		if mobil {
			ui.add_space(4.0);
		} else {
			ui.add_space(2.0);
		}
		if !mobil {
			ui.add_space(6.0);
			ui.horizontal(|ui| {
				if ui.button("Kaydet").clicked() { self.kaydet(); }
				if ui.button("Farklı kaydet").clicked() { self.kayit_yeri_sec(); }
				if ui.button("Yeniden aç").clicked() {
					if let Some(p) = self.path.clone() {
						if let Err(e) = self.belge_ac(&p) {
							self.note = format!("açılamadı: {e}");
						}
					}
				}
				// denetim 4: erişim her iki düzende gerçek eylem — masaüstünde
				// Ayarlar düğmesi, mobilde Ayarlar sekmesi
				if ui.button("Ayarlar").clicked() {
					self.ayar_win = true;
				}
				ui.label(RichText::new(&self.note).weak());
			});
			ui.add_space(6.0);
		} else {
			#[cfg(target_os = "android")]
			if ui.button("SRT dışa aktar…")
				.on_hover_text("ASS stilleri ve özel etiketler SRT’ye aktarılmaz").clicked() {
				self.kayit_bicimi("srt", true);
			}
			// mobil: Kaydet/Yeniden aç en az 44pt, not tüm genişlikte
			ui.add_space(8.0);
			let gen = ui.available_width();
			if ui.add(
				egui::Button::new(RichText::new("Kaydet").strong())
					.min_size(egui::vec2(gen, 44.0)),
			).clicked() {
				self.kaydet();
			}
			ui.add_space(8.0);
			if ui.add(
				egui::Button::new("Yeniden aç")
					.min_size(egui::vec2(gen, 44.0)),
			).clicked() {
				if let Some(p) = self.path.clone() {
					if let Err(e) = self.belge_ac(&p) {
						self.note = format!("açılamadı: {e}");
					}
				}
			}
			ui.add_space(6.0);
			if !self.note.is_empty() {
				ui.label(RichText::new(&self.note).weak());
			}
			ui.add_space(6.0);
		}
	}
}

/// Pending row edits committed together through the document's undo history.
enum Yazi {
	Metin(usize),
	Yorum(usize, bool),
	BasVal(usize, i64),
	SonVal(usize, i64),
	MetinVal(usize, String),
	StilVal(usize, String),
	OyuncuVal(usize, String),
	EtkiVal(usize, String),
	// P8.2: katman (Aegisub Layer alanı; meta_set üzerinden)
	KatmanVal(usize, i32),
	Kenarlar(usize, [i32; 3]),
}

impl GridApp {
	fn grid_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		ui.horizontal_wrapped(|ui| {
			let height = if mobil { 44.0 } else { 24.0 };
			if ui.add(egui::Button::selectable(self.multi_select, "Çoklu seç")
				.min_size(egui::vec2(0.0, height))).clicked() {
				if self.multi_select { self.clear_grid_selection(); }
				else { self.multi_select = true; self.marked_rows.extend(self.secili); }
			}
			if self.multi_select {
				if ui.button("Tümünü seç").clicked() {
					self.marked_rows = (0..self.rows.len()).collect();
				}
				ui.label(format!("{} seçili", self.marked_rows.len()));
				ui.add_enabled_ui(!self.marked_rows.is_empty(), |ui| {
					if ui.button("Çoğalt").clicked() { self.satir_ekle_tetik(true); }
					if ui.button("Sil").clicked() { self.satir_sil_tetik(); }
					if ui.button("Yorum").clicked() { self.toggle_selected_comments(); }
				});
			}
		});
		if !mobil {
			self.classic_grid(ui);
			return;
		}
		let mut edits = Vec::new();
		let row_height = ui.text_style_height(&egui::TextStyle::Body) * 3.2;
		ScrollArea::vertical().auto_shrink(false).show_rows(
			ui, row_height, self.rows.len(), |ui, visible| {
				for i in visible {
					if self.multi_select {
						let row = &self.rows[i];
						if ui.add(egui::Button::selectable(self.marked_rows.contains(&i),
							format!("{}  {}", i + 1, row.text)).min_size(egui::vec2(0.0, 44.0))).clicked() {
							self.select_grid_row(i);
						}
						continue;
					}
					let row = &mut self.rows[i];
					ui.label(format!("{} → {}", row.bas, row.son));
					let response = ui.add(TextEdit::singleline(&mut row.text).desired_width(f32::INFINITY));
					if response.clicked() || response.gained_focus() {
						self.secili = Some(i);
					}
					if response.changed() {
						edits.push(Yazi::Metin(i));
					}
				}
			},
		);
		self.yazlari_uygula(edits);
	}

	fn classic_grid(&mut self, ui: &mut egui::Ui) {
		const COLUMNS: [(&str, f32); 11] = [
			("#", 36.0), ("Katman", 56.0), ("Başlangıç", 87.0), ("Bitiş", 87.0),
			("Stil", 94.0), ("Oyuncu", 84.0), ("Efekt", 80.0), ("Sol", 40.0),
			("Sağ", 40.0), ("Dikey", 48.0), ("Metin", 220.0),
		];
		let row_height = if cfg!(target_os = "android") { 31.0 } else { 24.0 };
		let min_width: f32 = COLUMNS.iter().map(|column| column.1).sum();
		let width = ui.available_width().max(min_width);
		let captions = COLUMNS.map(|column| column.0);
		let draw_cells = |ui: &egui::Ui, rect: egui::Rect, cells: [&str; 11], color: Color32| {
			let mut left = rect.left();
			let font = egui::TextStyle::Body.resolve(ui.style());
			for (index, ((_, column_width), text)) in COLUMNS.into_iter().zip(cells).enumerate() {
				let column_width = if index + 1 == COLUMNS.len() {
					rect.right() - left
				} else {
					column_width
				};
				let cell = egui::Rect::from_min_size(
					egui::pos2(left, rect.top()), egui::vec2(column_width, rect.height()));
				let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(cell.shrink2(egui::vec2(3.0, 0.0))));
				painter.text(
					egui::pos2(left + 5.0, rect.center().y), egui::Align2::LEFT_CENTER,
					text, font.clone(), color);
				left += column_width;
			}
		};
		ScrollArea::horizontal().id_salt("classic-grid-columns").auto_shrink(false).show(ui, |ui| {
			ui.set_min_width(width);
			let (header, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
			ui.painter().rect_filled(header, 0, ui.visuals().widgets.noninteractive.bg_fill);
			draw_cells(ui, header, captions, ui.visuals().strong_text_color());
			if self.rows.is_empty() {
				ui.label("Altyazı satırı yok");
				return;
			}
			ScrollArea::vertical().id_salt("classic-grid-rows").auto_shrink(false).show_rows(
				ui, row_height, self.rows.len(), |ui, visible| {
					for i in visible {
						let (rect, response) = ui.allocate_exact_size(
							egui::vec2(width, row_height), egui::Sense::click());
						if response.clicked() {
							self.select_grid_row(i);
						}
						let row = &self.rows[i];
						let selected = if self.multi_select {
							self.marked_rows.contains(&i)
						} else {
							self.secili == Some(i)
						};
						let visuals = ui.visuals();
						let fill = if selected {
							visuals.selection.bg_fill
						} else if response.hovered() || i % 2 != 0 {
							visuals.faint_bg_color
						} else {
							visuals.extreme_bg_color
						};
						ui.painter().rect_filled(rect, 0, fill);
						let text_color = if selected {
							visuals.selection.stroke.color
						} else if row.yorum {
							visuals.weak_text_color()
						} else {
							visuals.text_color()
						};
						let number = (i + 1).to_string();
						let layer = row.katman.to_string();
						let [left, right, vertical] = row.kenarlar.map(|margin| margin.to_string());
						draw_cells(ui, rect, [
							&number, &layer, &row.bas, &row.son, &row.stil, &row.oyuncu, &row.etki,
							&left, &right, &vertical, &row.text,
						], text_color);
					}
				},
			);
		});
	}

	/// P5.4 gece: ABI yazım hattı — önce koalemalı anlık (undo), sonra
	/// çekirdek yazısı; zaman yazımsında ASS kanonizasyonu da burada.
	/// Grid ve edit-kutusu ortak boru.
	fn yazlari_uygula(&mut self, yazi: Vec<Yazi>) {
		if yazi.is_empty() {
			return;
		}
		let metin_satiri = match yazi.as_slice() {
			[Yazi::Metin(i) | Yazi::MetinVal(i, _)] => Some(*i),
			_ => None,
		};
		if !self.an_tut(metin_satiri) {
			self.yerles();
			return;
		}
		for y in yazi {
			let Some(h) = self.handle else { break };
			let hata: Result<(), String> = match y {
				Yazi::Metin(i) => {
					let t = self.rows.get(i).map(|r| r.text.clone()).unwrap_or_default();
					edit_line(h, i, &t)
				}
				Yazi::MetinVal(i, ref t) => {
					if let Some(r) = self.rows.get_mut(i) {
						r.text = t.clone();
					}
					edit_line(h, i, t)
				}
				Yazi::BasVal(i, mb) => {
					let sn = self
						.rows
						.get(i)
						.and_then(|r| ass_ms(&r.son).ok())
						.unwrap_or(0);
					if let Some(r) = self.rows.get_mut(i) {
						r.bas = ms_ass(mb);
						r.son = ms_ass(sn);
					}
					times_set(h, i, mb, sn)
				}
				Yazi::SonVal(i, ms) => {
					let mb = self
						.rows
						.get(i)
						.and_then(|r| ass_ms(&r.bas).ok())
						.unwrap_or(0);
					if let Some(r) = self.rows.get_mut(i) {
						r.bas = ms_ass(mb);
						r.son = ms_ass(ms);
					}
					times_set(h, i, mb, ms)
				}
				Yazi::StilVal(i, ref s) => {
					if let Some(r) = self.rows.get_mut(i) {
						r.stil = s.clone();
					}
					meta_set(h, i, Some(s), None, None, None, None)
				}
				Yazi::OyuncuVal(i, ref s) => {
					if let Some(r) = self.rows.get_mut(i) {
						r.oyuncu = s.clone();
					}
					meta_set(h, i, None, Some(s), None, None, None)
				}
				Yazi::EtkiVal(i, ref s) => {
					if let Some(r) = self.rows.get_mut(i) {
						r.etki = s.clone();
					}
					meta_set(h, i, None, None, Some(s), None, None)
				}
				Yazi::Yorum(i, v) => {
					meta_set(h, i, None, None, None, Some(v), None)
				}
				Yazi::Kenarlar(i, margins) => {
					let [left, right, vertical] = margins;
					let result = unsafe {
						velnosub_abi_line_margins(h, i as c_int, left, right, vertical)
					};
					if result == 0 {
						if let Some(row) = self.rows.get_mut(i) {
							row.kenarlar = margins;
						}
						Ok(())
					} else {
						Err(last_error())
					}
				}
				Yazi::KatmanVal(i, v) => {
					if let Some(r) = self.rows.get_mut(i) {
						r.katman = v;
					}
					meta_set(h, i, None, None, None, None, Some(v))
				}
			};
			if let Err(e) = hata {
				self.yerles();
				self.note = format!("işleme hatası: {e}");
				break;
			}
		}
	}

	/// Elle açılan düzen seçim ekranı. Seçim dosyaya yazılır; dosyaya
	/// yazılamazsa oturum seçilmiş düzenle devam eder ve uyarı dürüstçe
	/// gösterilir.
	fn secim_ekrani(&mut self, ui: &mut egui::Ui) {
		ui.add_space(24.0);
		ui.vertical_centered(|ui| {
			ui.label(RichText::new("velnosub").size(30.0).strong());
			ui.add_space(4.0);
			ui.label(RichText::new("Arayüz düzeni").weak());
			ui.add_space(10.0);
			ui.label("Hangi düzeni kullanmak istersin?");
			ui.label(
				RichText::new("Ayarlar bölümünden sonra da değiştirebilirsin.")
					.weak(),
			);
			ui.add_space(20.0);
		});
		let gen = ui.available_width().min(360.0);
		if ui.add(
			egui::Button::new(
				RichText::new("Mobil UI").size(18.0),
			)
			.min_size(egui::vec2(gen, 96.0)),
		).clicked() {
			self.duzen_sec(tema_simdi(ui), Duzen::Mobil);
		}
		ui.label(
			RichText::new("Alt sekme çubuğu ve geniş dokunma alanları; küçük ekranda tek elle kullanım için.").weak(),
		);
		ui.add_space(10.0);
		if ui.add(
			egui::Button::new(
				RichText::new("Masaüstü UI").size(18.0),
			)
			.min_size(egui::vec2(gen, 96.0)),
		).clicked() {
			self.duzen_sec(tema_simdi(ui), Duzen::Masaustu);
		}
		ui.label(
			RichText::new("Şeritler üst üste; satır başına iki sütunlu mevcut düzen.").weak(),
		);
		ui.add_space(16.0);
		if self.note.starts_with("ayar yazılamadı") {
			ui.label(
				RichText::new(&self.note)
					.color(egui::Color32::from_rgb(220, 120, 90)),
			);
		}
		ui.label(
			RichText::new(format!("ayar dosyası: {}", ayar_yolu())).weak(),
		);
	}

	fn duzen_sec(&mut self, tema: Tema, duzen: Duzen) {
		let kayit = match duzen {
			Duzen::Otomatik => "otomatik",
			Duzen::Mobil => "mobil",
			Duzen::Masaustu => "masaustu",
			Duzen::Secim => return,
		};
		let tema_kel = match tema {
			Tema::Koyu => "koyu",
			Tema::Acik => "acik",
		};
		match ayar_yaz(kayit, tema_kel) {
			Ok(()) => self.note = String::new(),
			Err(e) => {
				self.note =
					format!("ayar yazılamadı ({}): {e}", ayar_yolu());
			}
		}
		self.duzen = duzen;
		self.aktif = Sekme::Altyazi;
	}

	/// Ayarlar bölümü: düzen seçimi canlı etkilidir ve hep dosyaya yazılır;
	/// ayrıca ilk seçim ekranına dönüş gerçek bir eylemdir.
	fn ayarlar_bolum(&mut self, ui: &mut egui::Ui) {
		egui::ScrollArea::vertical()
			.id_salt("mobil-ayarlar")
			.show(ui, |ui| {
				ui.add_space(8.0);
				ui.heading("Ayarlar");
				self.ayarlar_icerik(ui);
			});
	}

	/// Ayarlar içeriği: mobil Ayarlar sekmesinde ve masaüstündeki Ayarlar
	/// penceresinde AYNI gövde kullanılır.
	fn ayarlar_icerik(&mut self, ui: &mut egui::Ui) {
		ui.add_space(8.0);
		ui.label(RichText::new("Arayüz düzeni").strong());
		ui.add_space(4.0);
		let gen = ui.available_width().min(360.0);
		if ui.add(
			egui::Button::new(RichText::new("Otomatik").size(18.0))
				.min_size(egui::vec2(gen, 44.0))
				.selected(self.duzen == Duzen::Otomatik),
		).clicked() {
			self.duzen_sec(tema_simdi(ui), Duzen::Otomatik);
			return;
		}
		if ui.add(
			egui::Button::new(RichText::new("Mobil UI").size(18.0))
				.min_size(egui::vec2(gen, 44.0))
				.selected(self.duzen == Duzen::Mobil),
		).clicked() {
			self.duzen_sec(tema_simdi(ui), Duzen::Mobil);
			return;
		}
		if ui.add(
			egui::Button::new(RichText::new("Masaüstü UI").size(18.0))
				.min_size(egui::vec2(gen, 44.0))
				.selected(self.duzen == Duzen::Masaustu),
		).clicked() {
			self.duzen_sec(tema_simdi(ui), Duzen::Masaustu);
			return;
		}
		ui.label(
			RichText::new(match self.duzen {
				Duzen::Otomatik if cfg!(target_os = "android") => "seçili: klasik Aegisub",
				Duzen::Otomatik => "seçili: otomatik (yatayda Aegisub, dikeyde mobil)",
				Duzen::Mobil => "seçili: mobil UI",
				Duzen::Masaustu => "seçili: masaüstü UI",
				Duzen::Secim => "seçili: ilk seçim ekranı",
			})
			.weak(),
		);
		// denetim 7: açık tema seceneği — seçim anında uygulanır ve
		// dosyaya kalıcı yazılır
		ui.add_space(12.0);
		ui.label(RichText::new("Tema").strong());
		ui.add_space(4.0);
		let koyu_simdi = tema_simdi(ui) == Tema::Koyu;
		if ui.add(
			egui::Button::new(RichText::new("Koyu tema").size(18.0))
				.min_size(egui::vec2(gen, 44.0))
				.selected(koyu_simdi),
		).clicked() {
			self.tema_sec(ui, Tema::Koyu);
		}
		if ui.add(
			egui::Button::new(RichText::new("Açık tema").size(18.0))
				.min_size(egui::vec2(gen, 44.0))
				.selected(!koyu_simdi),
		).clicked() {
			self.tema_sec(ui, Tema::Acik);
		}
		ui.label(
			RichText::new(if koyu_simdi {
				"seçili: koyu tema"
			} else {
				"seçili: açık tema"
			})
			.weak(),
		);
		ui.add_space(12.0);
		if ui.add(
			egui::Button::new("İlk seçim ekranını tekrar göster")
				.min_size(egui::vec2(gen, 44.0)),
		).clicked() {
			self.duzen = Duzen::Secim;
		}
		ui.add_space(12.0);
		if self.note.starts_with("ayar yazılamadı") {
			ui.label(
				RichText::new(&self.note)
					.color(egui::Color32::from_rgb(220, 120, 90)),
			);
		}
	}

	fn tema_sec(&mut self, ui: &egui::Ui, tema: Tema) {
		let kayit = match tema {
			Tema::Koyu => "koyu",
			Tema::Acik => "acik",
		};
		ui.ctx().all_styles_mut(|st| {
			st.visuals = match tema {
				Tema::Koyu => egui::Visuals::dark(),
				Tema::Acik => egui::Visuals::light(),
			};
			stil_tamir(st, tema);
		});
		let duzen_kel = match self.duzen {
			Duzen::Otomatik => "otomatik",
			Duzen::Mobil => "mobil",
			Duzen::Masaustu => "masaustu",
			Duzen::Secim => "mobil",
		};
		match ayar_yaz(duzen_kel, kayit) {
			Ok(()) => self.note = String::new(),
			Err(e) => {
				self.note =
					format!("ayar yazılamadı ({}): {e}", ayar_yolu());
			}
		}
	}

	/// Mobil çerçeve: alt sekme çubuğu + tek bölme içerik. Sekmeler yalnız
	/// yüklenen içeriğe oyulur; boş sekme yok.
	fn mobil_cerceve(&mut self, ui: &mut egui::Ui) {
		let mut etiketler: Vec<(Sekme, String)> = vec![(
			Sekme::Altyazi,
			format!("{}\nAltyazı", ikon::SUBTITLES),
		)];
		if self.video.is_some() {
			etiketler.push((Sekme::Video, format!("{}\nVideo", ikon::VIDEO)));
		}
		if self.audio.is_some() {
			etiketler.push((Sekme::Ses, format!("{}\nSes", ikon::AUDIO)));
		}
		if !self.stiller.is_empty() {
			etiketler.push((Sekme::Stil, format!("{}\nStil", ikon::PALETTE)));
		}
		if self.dialog.is_some() {
			etiketler.push((Sekme::Form, format!("{}\nForm", ikon::TUNE)));
		}
		etiketler.push((Sekme::Ayarlar, format!("{}\nAyarlar", ikon::SETTINGS)));
		if !etiketler.iter().any(|(s, _)| *s == self.aktif) {
			self.aktif = Sekme::Altyazi;
		}
		Panel::bottom("vtabs").show(ui, |su| {
			su.add_space(6.0);
			let adet = etiketler.len().max(1) as f32;
			let bosluk = 4.0;
			let gen = ((su.available_width() - bosluk * (adet - 1.0)) / adet)
				.floor()
				.max(48.0);
			su.spacing_mut().item_spacing.x = bosluk;
			su.horizontal_wrapped(|k| {
				k.spacing_mut().button_padding = egui::vec2(2.0, 4.0);
				for (s, ad) in &etiketler {
					let metin_boyu = (gen * 0.24).clamp(11.5, 15.0);
					if k.add(
						egui::Button::new(RichText::new(ad).size(metin_boyu))
							.min_size(egui::vec2(gen, 48.0))
							.selected(self.aktif == *s),
					).clicked() {
						self.aktif = *s;
					}
				}
			});
			su.add_space(6.0);
		});
		CentralPanel::default().show_inside(ui, |su| {
			match self.aktif {
				Sekme::Altyazi => {
					// P5.4 gece (antislop-layoutmobile): edit kutusu mobili
					// ayrı duruma taşınır — 44px hedef bandı, çok dizen
					// metin kutusu, yatay taşma yok (horizontal_wrapped)
					self.duzen_kutusu(su, true);
					self.durum_icerik(su, true);
					if su.button("Otomasyon…").clicked() {
						self.lua_window = true;
					}
					self.grid_icerik(su, true);
				}
				Sekme::Video => self.video_icerik(su, true),
				Sekme::Ses => {
					egui::ScrollArea::vertical()
						.id_salt("mobil-ses")
						.auto_shrink([false, false])
						.show(su, |s| {
							self.ses_icerik(s, true, false);
						});
				}
				Sekme::Stil => self.stil_icerik(su, true),
				Sekme::Form => self.form_icerik(su, true),
				Sekme::Ayarlar => self.ayarlar_bolum(su),
			}
		});
		// P5.4 gece: mobil yüzede paylaşımlı diyaloglar (Ara-Bul / zaman
		// kaydır / Ayarlar pencere) — masaüstü ile aynı kanal
		self.diyaloglar(ui);
	}


	/// Video/ses dosyası yükler ve notu tazeler (menü/toolbar çağrısı)
	fn video_ac(&mut self, path: &str) {
		let input = path.to_owned();
		let (send, result) = std::sync::mpsc::channel();
		let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
		let worker_cancel = cancel.clone();
		match std::thread::Builder::new().name("velnosub-video".into()).spawn(move || {
			let result = CString::new(input.as_str()).map_err(|error| error.to_string()).and_then(|path| {
				let h = unsafe {
					velnosub_ui_open_video(path.as_ptr(), audio_cancelled,
						std::sync::Arc::as_ptr(&worker_cancel).cast())
				};
				document_io_result(h, "Video dosyası açılamadı").map(|h| (OpenedVideo(Some(h)), input))
			});
			let _ = send.send(result);
		}) {
			Ok(_) => {
				self.pending_video = Some(PendingVideo { cancel, result });
				self.note = "Video kareleri dizinleniyor…".into();
			}
			Err(error) => self.note = format!("Video yükleyicisi başlatılamadı: {error}"),
		}
	}

	fn video_yukleme_sonucu(&mut self, ui: &mut egui::Ui) {
		let Some(pending) = &self.pending_video else { return };
		match pending.result.try_recv() {
			Ok(result) => {
				self.pending_video = None;
				if let Err(error) = result.and_then(|(opened, path)| self.load_video(opened, &path)) {
					self.note = format!("Video açılamadı: {error}");
				}
			}
			Err(std::sync::mpsc::TryRecvError::Disconnected) => {
				self.pending_video = None;
				self.note = "Video yükleyicisi kapandı".into();
			}
			Err(std::sync::mpsc::TryRecvError::Empty) => {
				ui.horizontal(|ui| {
					ui.spinner();
					ui.label("Video kareleri dizinleniyor…");
					if ui.button("İptal").clicked() {
						self.pending_video = None;
						self.note = "Video yükleme iptal edildi".into();
					}
				});
				ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
			}
		}
	}

	fn ses_ac(&mut self, yol: &str) {
		let (send, result) = std::sync::mpsc::channel();
		let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
		let worker_cancel = cancel.clone();
		let yol = yol.to_owned();
		match std::thread::Builder::new().name("velnosub-audio".into()).spawn(move || {
			let _ = send.send(Self::audio_open(&yol, &worker_cancel).map(|opened| (opened, yol)));
		}) {
			Ok(_) => {
				self.pending_audio = Some(PendingAudio { cancel, result });
				self.note = "ses yükleniyor…".into();
			}
			Err(e) => self.note = format!("ses yükleyicisi başlatılamadı: {e}"),
		}
	}

	fn ses_yukleme_sonucu(&mut self, ui: &mut egui::Ui) {
		let Some(pending) = &self.pending_audio else { return };
		match pending.result.try_recv() {
			Ok(result) => {
				self.pending_audio = None;
				if let Err(e) = result.and_then(|(opened, path)| self.load_audio(opened, path)) {
					self.note = format!("ses açılamadı: {e}");
				}
			}
			Err(std::sync::mpsc::TryRecvError::Disconnected) => {
				self.pending_audio = None;
				self.note = "ses yükleyicisi kapandı".into();
			}
			Err(std::sync::mpsc::TryRecvError::Empty) => {
				ui.horizontal(|ui| {
					ui.spinner();
					ui.label("Ses yükleniyor…");
					if ui.button("İptal").clicked() {
						self.pending_audio = None;
						self.note = "ses yükleme iptal edildi".into();
					}
				});
				ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
			}
		}
	}

	/// Kapsayıcıdaki ASS kaynaklarını biçim bilgisiyle aç.
	fn alt_yukle(&mut self, aks: i64) {
		if aks < 0 || aks > i32::MAX as i64 {
			self.note = "akış numarası geçersiz".into();
			return;
		}
		let Some(video) = self.video.as_ref() else { return };
		let yol = video.yol.clone();
		let ass = video.akslar.iter().any(|(i, codec, _)| *i == aks && codec == "ass");
		let bicim = if ass { "ass" } else { "srt" };
		let ana = alt_onbellek_ana();
		let an = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
			.map_or(0, |sure| sure.as_nanos());
		let cik = std::path::Path::new(&ana)
			.join(format!("altyazi-{}-{an}-{aks}.{bicim}", std::process::id()))
			.display().to_string();
		let yolu = CString::new(yol).map_err(|e| e.to_string());
		let srit = CString::new(cik.clone()).map_err(|e| e.to_string());
		let (yolu, srit) = match (yolu, srit) {
			(Ok(a), Ok(b)) => (a, b),
			(Err(e), _) | (_, Err(e)) => {
				self.note = format!("yol CString: {e}");
				return;
			}
		};
		let son = unsafe {
			velnosub_abi_subtitle_extract(
				yolu.as_ptr(),
				aks as c_int,
				srit.as_ptr(),
			)
		};
		if son < 0 {
			self.note = format!("altyazı akışı çıkarılamadı: {}", last_error());
			return;
		}
		if let Err(e) = self.belge_ac(&cik) {
			self.note = format!("altyazı yüklenemedi: {e}");
			return;
		}
		self.note = format!("iç altyazı akışı {aks}: {son} olay ({bicim})");
	}

	/// P7.1: aegisub player mantığı — "aktif satırı oynat": pencere =
	/// seçili satırın (baş, son) ms'i; seçili yoksa tüm ses süresi.
	/// Başlangıç = geçerli karenin ms'i, pencereye kırpılır.
	/// Dönüş: (baş_ms, son_ms, d0_ms, rate, toplam_örnek)
	fn oynat_penceresi(&self) -> (i64, i64, i64, i64, i64) {
		let rate = self.audio.as_ref().map(|a| a.rate as i64).unwrap_or(0);
		let toplam = self.audio.as_ref().map(|a| a.toplam).unwrap_or(0);
		let (bas, son) = self
			.secili
			.and_then(|i| self.rows.get(i))
			.map(|r| (ass_ms(&r.bas).unwrap_or(0), ass_ms(&r.son).unwrap_or(0)))
			.filter(|(b, s)| s > b)
			.unwrap_or((0, if rate > 0 { toplam * 1000 / rate } else { 0 }));
		let mevcut = self
			.video
			.as_ref()
			.map(|v| i64::from(v.kare_zamani(v.frame)))
			.unwrap_or(0);
		(bas, son, mevcut.clamp(bas, son), rate, toplam)
	}

	/// P4.8.2: aegisub menü çubuğu — mevcut işlevlere bağlı öğeler gerçek,
	/// ABI hat-alanları (v13) bekleyenler pasif gösterilir
	fn menu_cubugu(&mut self, ui: &mut egui::Ui) {
		let tema = tema_simdi(ui);
		egui::MenuBar::new().ui(ui, |mb| {
			mb.menu_button("File", |m| {
				if m.button("Yeni altyazı").clicked() {
					self.request_new_document();
					m.close();
				}
				if m.button(format!("{} Altyazı aç…", ikon::OPEN)).clicked() {
					self.tarayici_ac(SecimTur::Altyazi);
				}
				if m.button(format!("{} Video aç…", ikon::VIDEO)).clicked() {
					self.tarayici_ac(SecimTur::Video);
				}
				if m.button(format!("{} Ses aç…", ikon::AUDIO)).clicked() {
					self.tarayici_ac(SecimTur::Ses);
				}
				m.separator();
				if m
					.button(format!("{} Dosya özellikleri…", ikon::INFO))
					.clicked()
				{
					self.ozellik_win = true;
				}
				m.separator();
				if m.button(format!("{} Yeniden aç", ikon::ARROW_BACK)).clicked() {
					if let Some(p) = self.path.clone() {
						if let Err(e) = self.belge_ac(&p) {
							self.note = format!("açılamadı: {e}");
						}
					}
				}
				#[cfg(target_os = "android")]
				if m.button("SRT dışa aktar…").clicked() {
					self.kayit_bicimi("srt", true);
					m.close();
				}
				if m.button("Yazı tipi ekle…").clicked() {
					self.tarayici_ac(SecimTur::Font);
					m.close();
				}
				if m.button("Farklı kaydet…").clicked() { self.kayit_yeri_sec(); }
				if m.button(format!("{} Kaydet", ikon::SAVE)).clicked() {
					self.kaydet();
				}
			});
			mb.menu_button("Edit", |m| {
				if m.button(format!("{} Satır ekle", ikon::ADD)).clicked() {
					self.satir_ekle_tetik(false);
				}
				if m.button(format!("{} Satır çoğalt", ikon::COPY)).clicked() {
					self.satir_ekle_tetik(true);
				}
				if m.button(format!("{} Satır sil", ikon::DELETE)).clicked() {
					self.satir_sil_tetik();
				}
				m.separator();
				if m.button(format!("{} Geri", ikon::UNDO)).clicked() {
					self.geri_al();
				}
				if m.button(format!("{} İleri (Redo)", ikon::REDO)).clicked() {
					self.ileri_al();
				}
				m.separator();
				if m.button(format!("{} Ara-Bul…", ikon::SEARCH)).clicked() {
					self.ara_win = true;
				}
				if m.button(format!("{} Zaman kaydır…", ikon::LAYERS)).clicked() {
					self.shift_win = true;
				}
			});
			mb.menu_button("Subtitle", |m| {
				if m
					.button(format!("{} Stil Yöneticisi…", ikon::PALETTE))
					.clicked()
				{
					self.stil_win = true;
				}
				m.separator();
				if m
					.button(format!(
						"{} Seçili satırı yorum yap/çıkar",
						ikon::TEXT
					))
					.clicked()
				{
					if let (Some(h), Some(i)) = (self.handle, self.secili) {
						let Some(sat) = self.rows.get(i) else {
							self.note = "satır seç".into();
							return;
						};
						let st = sat.stil.clone();
						let oy = sat.oyuncu.clone();
						let et = sat.etki.clone();
						let yeni = !sat.yorum;
						if !self.an_tut(None) { return; }
						match meta_set(
							h,
							i,
							Some(&st),
							Some(&oy),
							Some(&et),
							Some(yeni),
							None,
						) {
							Ok(()) => {
								self.yerles();
								self.note = if yeni {
									"satır yorum yapıldı".into()
								} else {
									"yorum kaldırıldı".into()
								};
							}
							Err(e) => {
								self.note = format!("yorum: {e}");
							}
						}
					}
				}
				if m
					.button(format!("{} Satırı sil", ikon::DELETE))
					.clicked()
				{
					self.satir_sil_tetik();
				}
			});
			mb.menu_button("Timing", |m| {
				if m
					.button(format!("{} Zaman kaydır…", ikon::LAYERS))
					.clicked()
				{
					self.shift_win = true;
				}
				m.separator();
				if m
					.button(format!(
						"{} Başlangıcı kareye oturt",
						ikon::SKIP_PREV
					))
					.clicked()
				{
					self.kareye_oturt(true, false);
				}
				if m
					.button(format!("{} Bitişi kareye oturt", ikon::SKIP_NEXT))
					.clicked()
				{
					self.kareye_oturt(false, true);
				}
			});
			mb.menu_button("Video", |m| {
				if m.button("Video aç…").clicked() {
					self.tarayici_ac(SecimTur::Video);
					m.close();
				}
				m.separator();
				if m.button(format!("{} Oynat / Duraklat", ikon::PLAY)).clicked() {
					if let Some(v) = self.video.as_mut() {
						v.oynat = !v.oynat;
						v.son_tik = None;
					}
				}
				if m.button(format!("{} Önceki kare", ikon::SKIP_PREV)).clicked() {
					if let Some(v) = self.video.as_mut() {
						if v.frame > 0 {
							v.frame -= 1;
						}
					}
				}
				if m.button(format!("{} Sonraki kare", ikon::SKIP_NEXT)).clicked() {
					if let Some(v) = self.video.as_mut() {
						if v.frame + 1 < v.kareler {
							v.frame += 1;
						}
					}
				}
			});
			mb.menu_button("Audio", |m| {
				if m.button("Ses aç…").clicked() {
					self.tarayici_ac(SecimTur::Ses);
					m.close();
				}
				m.separator();
				if m.button(format!("{} Oynat (baştan)", ikon::PLAY)).clicked() {
					if let Some(a) = self.audio.as_ref() {
						let adet =
							unsafe { velnosub_abi_audio_len64(a.handle) };
						let rc = unsafe {
							velnosub_abi_audio_play(a.handle, 0, adet)
						};
						self.note = if rc == 0 {
							"ses: oynatılıyor".into()
						} else {
							format!("ses oynatma hatası: {}", last_error())
						};
					}
				}
				if m.button(format!("{} Durdur", ikon::STOP)).clicked() {
					if let Some(a) = self.audio.as_ref() {
						unsafe { velnosub_abi_audio_stop(a.handle) };
						self.note = "ses: durdu".into();
					}
				}
			});
			mb.menu_button("Automation", |m| {
				if m.button("Otomasyon yöneticisi…").clicked() {
					self.lua_window = true;
					m.close();
				}
				if m.button("Ana harf").clicked() {
					self.makro_uygula();
					m.close();
				}
			});
			mb.menu_button("View", |m| {
				if m.button(format!("{} Mobil görünüm", ikon::TEXT)).clicked() {
					self.duzen_sec(tema, Duzen::Mobil);
				}
				if m.button(format!("{} Masaüstü görünüm", ikon::FULL)).clicked() {
					self.duzen_sec(tema, Duzen::Masaustu);
				}
				m.horizontal(|m| {
					if m.button("−").clicked() { self.zoom = (self.zoom - 0.1).max(0.6); }
					if m.button(format!("{:.1}×", self.zoom)).clicked() { self.zoom = 1.0; }
					if m.button("+").clicked() { self.zoom = (self.zoom + 0.1).min(1.6); }
				});
				m.separator();
				if m.button(format!("{} Stil Yöneticisi…", ikon::PALETTE)).clicked() {
					self.stil_win = true;
				}
				if m.button(format!("{} Ayarlar…", ikon::SETTINGS)).clicked() {
					self.ayar_win = true;
				}
			});
			mb.menu_button("Help", |m| {
				m.label("Velnosub · Aegisub tabanlı altyazı düzenleyici");
				if m.button("Komut paleti…").clicked() {
					self.command_query = Some(String::new());
					m.close();
				}
			});
		});
	}

	/// Edit the selected row's text, timing, and ASS metadata.
	fn duzen_kutusu(&mut self, ui: &mut egui::Ui, mobil: bool) {
		let Some(i) = self.secili else {
			ui.label(RichText::new("satır seç (grid'e dokun)").weak());
			return;
		};
		let Some(sat) = self.rows.get(i) else {
			return;
		};
		let mut metin = sat.text.clone();
		let mut bas = sat.bas.clone();
		let mut son = sat.son.clone();
		let stil = sat.stil.clone();
		let mut oyuncu = sat.oyuncu.clone();
		let mut etki = sat.etki.clone();
		let yorum_sabit = sat.yorum;
		let mut yorum = yorum_sabit;
		let mut katman = sat.katman;
		let mut kenarlar = sat.kenarlar;
		let stil_kata: Vec<String> =
			self.stiller.iter().map(|s| s.name.clone()).collect();
		let mut yazi: Vec<Yazi> = Vec::new();
		let mut hatalar: Vec<String> = Vec::new();
		let hedef = if mobil { 44.0 } else { 0.0 };
		ui.vertical(|ev| {
			ev.horizontal_wrapped(|h| {
				// Yorum onaykutusu — aegisub edit kutusu başlığı; mobilde
				// dar onaykutusu darlana (kutu mesafesi dilimi)
				if h.checkbox(&mut yorum, "Yorum").changed() && yorum != yorum_sabit {
					yazi.push(Yazi::Yorum(i, yorum));
				}
				// Preserve layers already stored in the ASS file.
				h.label(RichText::new("katman").weak());
				if h
					.add(
						egui::DragValue::new(&mut katman)
							.range(0..=i32::MAX)
							.clamp_existing_to_range(false)
							.speed(0.1),
					)
					.changed()
				{
					yazi.push(Yazi::KatmanVal(i, katman));
				}
				let mb = ass_ms(&bas).unwrap_or(0);
				let ms = ass_ms(&son).unwrap_or(0);
				let sure = (ms - mb).max(0);
				let harf = metin.chars().count();
				let cps = if sure > 0 {
					harf as f32 / (sure as f32 / 1000.0)
				} else {
					0.0
				};
				h.label(RichText::new(format!("süre {sure} ms")).weak());
				h.label(RichText::new(format!("karakter {harf}")).weak());
				h.label(RichText::new(format!("CPS {cps:.1}")).weak());
			});
			ev.horizontal_wrapped(|h| {
				// Stil seçici — çekirdek stilleri; boş stil listesine
				// dokunulmaz (dosyanın kendi stil okuması load_styles)
				let sec_stil = stil.clone();
				egui::ComboBox::from_id_salt(format!("ekstil{i}"))
					.selected_text(&stil)
					.width(if mobil { 140.0 } else { 110.0 })
					.show_ui(h, |c| {
						for ad in &stil_kata {
							if c
								.selectable_label(sec_stil == *ad, ad.clone())
								.clicked()
							{
								yazi.push(Yazi::StilVal(i, ad.clone()));
							}
						}
					});
				h.label(RichText::new("oyuncu").weak());
				let w_oy = if mobil { h.available_width() * 0.5 } else { 110.0 };
				if TextEdit::singleline(&mut oyuncu)
					.desired_width(w_oy)
					.show(h)
					.response
					.changed()
				{
					let v = oyuncu.clone();
					yazi.push(Yazi::OyuncuVal(i, v));
				}
				h.label(RichText::new("etki").weak());
				if TextEdit::singleline(&mut etki)
					.desired_width(88.0)
					.show(h)
					.response
					.changed()
				{
					yazi.push(Yazi::EtkiVal(i, etki.clone()));
				}
			});
			ev.horizontal_wrapped(|h| {
				h.label(RichText::new("başlangıç").weak());
				if TextEdit::singleline(&mut bas)
					.desired_width(if mobil { 110.0 } else { 88.0 })
					.font(egui::TextStyle::Monospace)
					.show(h)
					.response
					.changed()
				{
					match ass_ms(&bas) {
						Ok(v) => yazi.push(Yazi::BasVal(i, v)),
						Err(e) => hatalar.push(e),
					}
				}
				h.label(RichText::new("bitiş").weak());
				if TextEdit::singleline(&mut son)
					.desired_width(if mobil { 110.0 } else { 88.0 })
					.font(egui::TextStyle::Monospace)
					.show(h)
					.response
					.changed()
				{
					match ass_ms(&son) {
						Ok(v) => yazi.push(Yazi::SonVal(i, v)),
						Err(e) => hatalar.push(e),
					}
				}
			});
			// P5.4 gece: metin kutusu ÖNCE — imleci bu çerçeveden alır;
			// tag şeridi ALTA (başparmak kolu: antislop-layoutmobile)
			let out = if mobil {
				TextEdit::multiline(&mut metin)
					.desired_width(ev.available_width().min(10_000.0))
					.desired_rows(4)
					.show(ev)
			} else {
				// P6.2: düzen kutusu SABİT bant — uzun metin iç-alanda kayar
				// (aegisub edit-pane kuralı; pencere kaydırması yok)
				egui::ScrollArea::vertical()
					.max_height(80.0)
					.auto_shrink([false, true])
					.show(ev, |se| {
						TextEdit::multiline(&mut metin)
							.desired_width(se.available_width().min(620.0))
							.desired_rows(2)
							.show(se)
					})
					.inner
			};
			let imlec: usize = out
				.cursor_range
				.map(|cr| cr.primary.index.min(cr.secondary.index).0)
				.unwrap_or(usize::MAX);
			if out.response.changed() {
				let v = metin.clone();
				yazi.push(Yazi::MetinVal(i, v));
			}
			// tag şeridi — imleç konumuna ekler; mobilde 44×44 hedef bandı
			// + sarılma (yatay taşma yok: horizontal_wrapped)
			ev.horizontal_wrapped(|h| {
				for (ad, tag) in TAGS {
					let deg = if mobil {
						h.add_sized(
							[hedef, hedef],
							egui::Button::new(RichText::new(ad).strong()),
						)
					} else {
						h.button(ad)
					};
					if deg.clicked() {
						let konum = imlec.min(metin.chars().count());
						let bayt = metin
							.char_indices()
							.nth(konum)
							.map(|(b, _)| b)
							.unwrap_or(metin.len());
						let (ilk, geri) = metin.split_at(bayt);
						metin = format!("{ilk}{tag}{geri}");
						yazi.push(Yazi::MetinVal(i, metin.clone()));
					}
				}
			});
			ev.horizontal_wrapped(|row| {
				for (label, margin) in ["Sol", "Sağ", "Dikey"].into_iter().zip(&mut kenarlar) {
					row.label(label);
					row.add(egui::DragValue::new(margin)
						.range(0..=i32::MAX).clamp_existing_to_range(false));
				}
			});
			if kenarlar != sat.kenarlar {
				yazi.push(Yazi::Kenarlar(i, kenarlar));
			}
		});
		for e in hatalar {
			self.note = format!("saat hatası: {e}");
		}
		self.yazlari_uygula(yazi);
	}
}

	// ---------- P5.4 gece: tag'i imleç konumuna ekleme yardımcısı ---------
	// TextEditOutput.cursor_range bize dizge-uzayında CCursor verir (karakter
	// sayısı); bayt sınırına char_indices ile inip split atarız (multi-byte
	// Türkçe metin güvenli).

const TAGS: [(&str, &str); 8] = [
	("B", "{\\b1}"),
	("I", "{\\i1}"),
	("U", "{\\u1}"),
	("S", "{\\s1}"),
	("\\N", "\\N"),
	("an1", "{\\an1}"),
	("an2", "{\\an2}"),
	("an3", "{\\an3}"),
];

impl GridApp {
	fn masaustu_cerceve(&mut self, ui: &mut egui::Ui) {
		// Keep the desktop's panel order at every width. The compact tabs
		// remain available through View and retain the same document state.
		let base = *self.ppp_base.get_or_insert_with(|| ui.ctx().pixels_per_point());
		ui.ctx().set_pixels_per_point(base * self.zoom);
		if self.secili.is_none() && !self.rows.is_empty() {
			self.secili = Some(0);
		}

		Panel::top("classic-menu").show(ui, |top| {
			ScrollArea::horizontal().id_salt("classic-menu-scroll").show(top, |menu| {
				if cfg!(target_os = "android") {
					menu.spacing_mut().interact_size.y = 29.0;
				}
				self.menu_cubugu(menu);
			});
			top.separator();
			ScrollArea::horizontal().id_salt("classic-toolbar").show(top, |top| {
				top.horizontal(|bar| {
					let actions: [(&str, &str, fn(&mut Self)); 11] = [
						(ikon::ADD, "Yeni altyazı", Self::request_new_document),
						(ikon::OPEN, "Altyazı aç", |app| app.tarayici_ac(SecimTur::Altyazi)),
						(ikon::SAVE, "Kaydet", Self::kaydet),
						(ikon::UNDO, "Geri al", Self::geri_al),
						(ikon::REDO, "İleri al", Self::ileri_al),
						(ikon::SEARCH, "Bul ve değiştir", |app| app.ara_win = true),
						(ikon::VIDEO, "Video aç", |app| app.tarayici_ac(SecimTur::Video)),
						(ikon::AUDIO, "Ses aç", |app| app.tarayici_ac(SecimTur::Ses)),
						(ikon::PALETTE, "Stil yöneticisi", |app| app.stil_win = true),
						(ikon::AUTOMATION, "Otomasyon yöneticisi", |app| app.lua_window = true),
						(ikon::SETTINGS, "Ayarlar", |app| app.ayar_win = true),
					];
					for (icon, label, action) in actions {
						let size = if cfg!(target_os = "android") { 32.0 } else { 26.0 };
						if bar.add_sized([size, size], egui::Button::new(RichText::new(icon).size(18.0)))
							.on_hover_text(label).clicked()
						{
							action(self);
						}
					}
				});
			});
		});

		Panel::bottom("classic-status").show(ui, |status| {
			status.horizontal(|row| {
				let name = self.path.as_deref().and_then(|path| std::path::Path::new(path).file_name())
					.and_then(|name| name.to_str()).unwrap_or("Yeni altyazı");
				let dirty = if self.belge_degisti() { "*" } else { "" };
				row.label(format!("{dirty}{name}"));
				row.separator();
				row.label(format!("{} satır", self.rows.len()));
				if !self.note.is_empty() && self.note != name {
					row.separator();
					row.add(egui::Label::new(&self.note).truncate()).on_hover_text(&self.note);
				}
			});
		});

		let height = ui.available_height();
		let max_workspace = layout::workbench_max_height(height);
		let default_workspace = (height * 0.55).min(max_workspace);
		let ratio = self.video.as_ref().filter(|v| v.w > 0 && v.hgt > 0)
			.map(|v| v.preview_ratio.unwrap_or(v.w as f32 / v.hgt as f32));
		Panel::top("classic-workbench")
			.resizable(true)
			.default_size(default_workspace)
			.size_range(0.0..=max_workspace)
			.frame(egui::Frame::NONE)
			.show(ui, |workbench| {
				// The grid and status keep their space when an editor panel scrolls.
				let space = workbench.available_rect_before_wrap();
				let b = layout::bolmeler_ust(space.width(), space.height(), ratio);
				let (rect, _) = workbench.allocate_exact_size(space.size(), egui::Sense::hover());
				let divider = egui::pos2(rect.left() + b.video_gen, rect.top());
				let right_size = (b.sag_gen - 4.0).max(0.0);
				let video = egui::Rect::from_min_size(rect.min, egui::vec2((b.video_gen - 4.0).max(0.0), b.ust_yuk));
				let audio = egui::Rect::from_min_size(divider, egui::vec2(right_size, b.ses_yuk));
				let edit = egui::Rect::from_min_size(
					egui::pos2(divider.x, audio.bottom()), egui::vec2(right_size, b.duzen_yuk));
				self.classic_pane(workbench, "classic-video", video, |app, ui| {
					if app.video.is_some() {
						app.video_icerik(ui, false);
					} else if ui.button("Video aç…").clicked() {
						app.tarayici_ac(SecimTur::Video);
					}
				});
				self.classic_pane(workbench, "classic-audio", audio, |app, ui| {
					if app.audio.is_some() {
						app.ses_icerik(ui, false, b.ses_yuk < 120.0);
					} else if ui.button("Ses aç…").clicked() {
						app.tarayici_ac(SecimTur::Ses);
					}
				});
				self.classic_pane(workbench, "classic-editor", edit, |app, ui| app.duzen_kutusu(ui, false));
			});
		CentralPanel::default().show(ui, |grid| self.grid_icerik(grid, false));
		if self.dialog.is_some() {
			egui::Window::new("Makro formu").show(ui.ctx(), |form| self.form_icerik(form, false));
		}
		self.diyaloglar(ui);
	}

	fn classic_pane(
		&mut self, parent: &mut egui::Ui, id: &'static str, rect: egui::Rect,
		content: impl FnOnce(&mut Self, &mut egui::Ui),
	) {
		if rect.width() <= 0.0 || rect.height() <= 0.0 {
			return;
		}
		let mut pane = parent.new_child(egui::UiBuilder::new().id_salt(id)
			.max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
		pane.set_clip_rect(rect.intersect(parent.clip_rect()));
		ScrollArea::vertical().id_salt(id).auto_shrink(false).show(&mut pane, |ui| content(self, ui));
	}

}

impl GridApp {
	/// Form düğme gönderimi: tüm alan değerleri ABI'ye yazılır, sonra düğme
	/// seçimi kaydedilir (masaüstü ve mobil ortak).
	fn form_gonder(&mut self, bi: usize, ad: &str) {
		let Some(fd) = self.dialog.as_mut() else { return };
		let h = fd.handle;
		let mut yaz = 0usize;
		for (i, a2) in fd.alanlar.iter().enumerate() {
			if a2.sinif != "label" {
				let c = CString::new(a2.deger.clone())
					.map(|c| unsafe { velnosub_abi_dialog_set(h, i as c_int, c.as_ptr()) })
					.unwrap_or(-1);
				if c == 0 {
					yaz += 1;
				}
			}
		}
		let secim = unsafe { velnosub_abi_dialog_show(h, bi as c_int) };
		self.note = format!(
			"form: düğme {} ({ad}) — {yaz}/{evet} alan tip yanı: {secim}",
			bi + 1,
			evet = fd.alanlar.iter().filter(|x| x.sinif != "label").count(),
		);
	}
}

impl Drop for GridApp {
	fn drop(&mut self) {
		if let Some(form) = self.macro_form.take() {
			unsafe { velnosub_abi_macro_abort(form.token) };
		}
		#[cfg(target_os = "android")]
		{
			unsafe { velnosub_abi_document_set_wake(None) };
			*DOCUMENT_UI.lock().unwrap() = None;
			if let Some(snapshot) = self.android_saved_snapshot.take() {
				Self::remove_saf_stage(&snapshot);
			}
		}
		Self::gecmisi_temizle(&mut self.undo_s);
		Self::gecmisi_temizle(&mut self.redo_s);
		if let Some(h) = self.handle {
			unsafe { velnosub_abi_close(h) };
		}
		if let Some(v) = self.video.take() {
			unsafe { velnosub_abi_video_close(v.handle) };
		}
		if let Some(a) = self.audio.take() {
			unsafe { velnosub_abi_audio_close(a.handle) };
		}
	}
}

impl eframe::App for GridApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		#[cfg(target_os = "android")]
		if self.handle.is_some() && !self.android_ready {
			self.android_ready = true;
			crate::not_yaz("A4 editor ready");
		}
		self.ses_yukleme_sonucu(ui);
		self.video_yukleme_sonucu(ui);
		#[cfg(target_os = "android")]
		{
			{
				let mut context = DOCUMENT_UI.lock().unwrap();
				if context.is_none() {
					*context = Some(ui.ctx().clone());
					unsafe { velnosub_abi_document_set_wake(Some(wake_document_ui)) };
				}
			}
			self.android_belge_sonucu(ui.ctx());
			let close_requested = unsafe { velnosub_abi_document_take_close() } != 0;
			// On older Android versions the native input queue can consume Back
			// before the Activity receives onBackPressed.
			let legacy_back = android_activity::AndroidApp::sdk_version() < 33
				&& ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::BrowserBack));
			if self.android_belge.is_some() {
				return;
			}
			if close_requested || legacy_back {
				if let Some(form) = self.macro_form.take() {
					self.abort_macro(form);
					return;
				}
			}
			if (close_requested || legacy_back) && self.bekleyen_belge.is_none() {
				self.bekleyen_belge = Some(PendingDocumentAction::Close);
				if !self.belge_degisti() {
					self.finish_pending_document();
					return;
				}
			}
		}
		if self.macro_form.is_some() {
			self.macro_dialog(ui.ctx());
			self.translation_window(ui.ctx());
			self.spelling_window(ui.ctx());
			return;
		}
		self.belge_pencereleri(ui.ctx());
		self.lua_yonet(ui.ctx());
		if self.macro_form.is_some() {
			ui.ctx().request_repaint();
			return;
		}
		if self.macro_form.is_none() && self.translation.is_none() &&
			ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
			self.command_query = Some(String::new());
		}
		self.komut_paleti(ui.ctx());
		// Android uses the classic editor by default at every orientation.
		// On desktop, the automatic setting adapts to the available space.
		let etkin = match self.duzen {
			Duzen::Otomatik if cfg!(target_os = "android") => Duzen::Masaustu,
			Duzen::Otomatik => {
				// Zoom'lu noktaları iki eksende de yerel (taban ppp)
				// boyuta indir: 640–800 pt yerel yatay ekran, 0.8× zoom
				// yüzünden yanlışlıkla Aegisub'a geçmesin. ppp_base yoksa
				// (henüz masaüstüne girilmediyse) boyut değişmez.
				let ppp = ui.ctx().pixels_per_point();
				let taban = self.ppp_base.unwrap_or(ppp);
				match layout::sec_taban(
					ui.available_width(),
					ui.available_height(),
					ppp,
					taban,
				) {
					Yerlesim::Aegisub => Duzen::Masaustu,
					Yerlesim::Mobil => Duzen::Mobil,
				}
			}
			d => d,
		};
		// ölçek tabana döner: Mobil'e masaüstü zoom'u sızmaz; Masaustu'ya
		// her girişte ppp_base yeniden alınır (reset sonrası taban = taban)
		if etkin != Duzen::Masaustu {
			if let Some(b) = self.ppp_base.take() {
				ui.ctx().set_pixels_per_point(b);
			}
		}
		if self.bekleyen_belge.is_none() && self.command_query.is_none()
			&& self.translation.is_none() && self.macro_form.is_none() {
			const COMMAND: egui::Modifiers = egui::Modifiers::COMMAND;
			const SHIFT_COMMAND: egui::Modifiers = egui::Modifiers {
				shift: true,
				..COMMAND
			};
			let key = |modifiers, key| ui.input_mut(|input| input.consume_key(modifiers, key));
			if key(SHIFT_COMMAND, egui::Key::S) {
				self.kayit_yeri_sec();
			} else if key(COMMAND, egui::Key::S) || key(egui::Modifiers::NONE, egui::Key::F2) {
				self.kaydet();
			} else if key(COMMAND, egui::Key::O) {
				self.tarayici_ac(SecimTur::Altyazi);
			} else if key(COMMAND, egui::Key::N) {
				self.request_new_document();
			} else if key(COMMAND, egui::Key::H) {
				self.ara_win = true;
			} else if key(COMMAND, egui::Key::I) {
				self.shift_win = true;
			} else if ui.ctx().memory(|memory| memory.focused().is_none()) {
				if key(SHIFT_COMMAND, egui::Key::Z) || key(COMMAND, egui::Key::Y) {
					self.ileri_al();
				} else if key(COMMAND, egui::Key::Z) {
					self.geri_al();
				} else if key(COMMAND, egui::Key::A) {
					self.multi_select = true;
					self.marked_rows = (0..self.rows.len()).collect();
				} else if key(COMMAND, egui::Key::D) {
					self.satir_ekle_tetik(true);
				} else if key(egui::Modifiers::NONE, egui::Key::Delete) {
					self.satir_sil_tetik();
				}
			}
		}
		match etkin {
			Duzen::Secim => {
				CentralPanel::default().show_inside(ui, |su| {
					self.secim_ekrani(su);
				});
			}
			Duzen::Mobil => self.mobil_cerceve(ui),
			Duzen::Masaustu => self.masaustu_cerceve(ui),
			Duzen::Otomatik => {}
		}
		if self.ses_win && self.audio.is_some() {
			let mut acik = true;
			let ekran = ui.max_rect().size();
			egui::Window::new("Ses zamanlama")
				.open(&mut acik)
				.resizable(true)
				.default_size(egui::vec2(
					(ekran.x * 0.9).clamp(340.0, 1100.0),
					(ekran.y * 0.85).clamp(260.0, 700.0),
				))
				.min_width(340.0)
				.show(ui.ctx(), |p| {
					self.ses_icerik(p, false, false);
				});
			// "Küçült" içeride `ses_win`i kapatır; X ise `acik`ı.
			self.ses_win = acik && self.ses_win;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn belge_gecmisi_ve_basarisiz_acma() {
		let root = std::env::var_os("VELNOSUB_TEST_ROOT")
			.map(std::path::PathBuf::from)
			.unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
		std::env::set_var("VELNOSUB_CONFIG_JSON", root.join("src/libresrc/default_config.json"));
		let source = root.join("tools/cross/apk-ui/seed/karaskel-dene.ass");
		let source = source.to_str().unwrap();
		let mut app = GridApp::new(Some(source.into()), None);
		assert_eq!(app.belge_bicimi(), "ass");
		assert!(app.handle.is_some(), "{}", app.note);
		for i in 0..30 {
			assert!(app.an_tut(None), "{}", app.note);
			edit_line(app.handle.unwrap(), 0, &format!("satır {i}")).unwrap();
		}
		app.yerles();
		assert!(app.load("bu-dosya-yok.ass").is_err());
		assert_eq!(app.rows[0].text, "satır 29");
		for i in (5..29).rev() {
			app.geri_al();
			assert_eq!(app.rows[0].text, format!("satır {i}"));
		}
		for i in 6..30 {
			app.ileri_al();
			assert_eq!(app.rows[0].text, format!("satır {i}"));
		}
		assert_eq!(app.path.as_deref(), Some(source));
		app.secili = Some(0);
		let count = app.rows.len();
		let original_margins = app.rows[0].kenarlar;
		let margins = [12, 34, 56];
		app.yazlari_uygula(vec![Yazi::Kenarlar(0, margins)]);
		assert_eq!(app.rows[0].kenarlar, margins);
		app.satir_ekle_tetik(true);
		assert_eq!(app.rows.len(), count + 1);
		assert_eq!(app.rows[0].text, app.rows[1].text);
		assert_eq!(app.rows[1].kenarlar, margins);
		app.geri_al();
		assert_eq!(app.rows.len(), count);
		assert_eq!(app.rows[0].kenarlar, margins);
		app.geri_al();
		assert_eq!(app.rows[0].kenarlar, original_margins);
		app.ileri_al();
		assert_eq!(app.rows[0].kenarlar, margins);
		let cik = std::env::temp_dir().join(format!("velnosub-test-{}", std::process::id()));
		std::fs::create_dir(&cik).unwrap();
		let invalid = cik.join("invalid.srt");
		std::fs::write(&invalid, "1\nnot a timestamp\nBroken\n").unwrap();
		assert!(app.load(invalid.to_str().unwrap()).is_err());
		assert_eq!(app.path.as_deref(), Some(source));
		assert_eq!(app.rows[0].text, "satır 29");
		let ass = cik.join("duzenlenen.ass");
		assert!(!app.save_to(cik.join("absent").join("broken.ass").to_str().unwrap()));
		assert!(app.belge_degisti());
		assert!(app.save_to(ass.to_str().unwrap()), "{}", app.note);
		assert!(!app.belge_degisti());
		app.yazlari_uygula(vec![Yazi::MetinVal(0, "sonraki düzenleme".into())]);
		assert!(app.belge_degisti());
		app.geri_al();
		assert!(!app.belge_degisti());
		app.ileri_al();
		assert!(app.belge_degisti());
		app.geri_al();
		app.yazlari_uygula(vec![Yazi::MetinVal(0, "yeni düzenleme".into())]);
		app.ileri_al();
		assert_eq!(app.rows[0].text, "yeni düzenleme");
		assert!(app.belge_degisti());
		app.load(ass.to_str().unwrap()).unwrap();
		assert_eq!(app.rows[0].text, "satır 29");
		assert_eq!(app.rows[0].kenarlar, margins);
		let srt = cik.join("ornek.srt");
		std::fs::write(&srt, "1\n00:00:00,500 --> 00:00:01,000\nİlk satır\n\n").unwrap();
		app.load(srt.to_str().unwrap()).unwrap();
		assert_eq!(app.belge_bicimi(), "srt");
		assert_eq!(app.rows[0].text, "İlk satır");
		assert!(app.an_tut(None));
		edit_line(app.handle.unwrap(), 0, "SRT düzenlendi").unwrap();
		let srt_cik = cik.join("duzenlenen.srt");
		assert!(app.save_to(srt_cik.to_str().unwrap()), "{}", app.note);
		app.load(srt_cik.to_str().unwrap()).unwrap();
		assert_eq!(app.rows[0].text, "SRT düzenlendi");
		app.load(source).unwrap();
		let original: Vec<_> = app.rows.iter().map(|row| row.text.clone()).collect();
		app.multi_select = true;
		app.marked_rows.extend([0, 2]);
		app.satir_ekle_tetik(true);
		assert_eq!(app.rows.len(), original.len() + 2);
		assert_eq!(app.rows[1].text, original[0]);
		assert_eq!(app.rows[4].text, original[2]);
		app.geri_al();
		assert_eq!(app.rows.len(), original.len());
		assert!(!app.belge_degisti());
		assert!(!app.apply_row_batch(vec![0, 2], "test", |handle, row| {
			if row == 0 { return Err("hata".into()); }
			satir_sil(handle, row)
		}));
		assert_eq!(app.rows.len(), original.len());
		assert!(!app.belge_degisti());
		app.multi_select = true;
		app.marked_rows.extend([0, 2]);
		app.secili = Some(0);
		let script = CString::new(r#"
aegisub.register_macro("Read", "", function(subs, selected, active)
	assert(#selected == 2 and active == selected[1])
	assert(aegisub.decode_path("?script"):find("seed", 1, true))
	local path = assert(aegisub.dialog.open("Fixture"))
	local file = assert(io.open(path, "r"))
	file:close()
	for _, index in ipairs(selected) do
		local line = subs[index]
		line.text = "Read ok"
		subs[index] = line
	end
end)
"#).unwrap();
		app.makro_calistir(&script, "Read", Some(source));
		let mut form = app.macro_form.take().unwrap_or_else(|| panic!("file request: {}", app.note));
		form.dialog.alanlar.iter_mut().find(|field| field.ad == "__velnosub_open_file")
			.unwrap().deger = source.into();
		app.submit_macro_form(form, 0);
		assert_eq!(app.rows[0].text, "Read ok", "{}", app.note);
		assert_eq!(app.rows[2].text, "Read ok");
		assert_eq!(app.rows[1].text, original[1]);
		app.geri_al();
		assert_eq!(app.rows[0].text, original[0]);
		assert_eq!(app.rows[2].text, original[2]);
		#[cfg(target_os = "android")]
		{
			let output = app.stage_macro_export("test", "untitled.txt").unwrap();
			std::fs::write(&output, "temp").unwrap();
			assert!(app.stage_macro_export("test", "other.txt").is_err());
			app.macro_export = None;
			assert!(!std::path::Path::new(&output).exists());
		}
		std::fs::remove_dir_all(cik).unwrap();
	}

	/// Ayar döngüsü: dosya yoksa alanlar None kalır ve uygulama varsayılan
	/// Otomatik/koyu ile açılır (seçim sorulmaz), yazım sonra birebir okunur;
	/// bilinmeyen içerik yine varsayılana düşer. Ekransız haberci — derleme
	/// ortamında GUI penceresi açılamaz; çizim kanıtı cihaz/APK bağlamına
	/// kalmıştır.
	#[test]
	fn ayar_dongusu() {
		let yol = std::env::temp_dir()
			.join(format!("velnosub-ui-test{}.json", std::process::id()));
		std::env::set_var("VELNOSUB_UI_JSON", &yol);
		let _ = std::fs::remove_file(&yol);
		let ay = ayar_yukle();
		assert!(
			ay.duzen.is_none() && ay.tema.is_none(),
			"dosya yokken varsayılan Otomatik kullanılır"
		);
		ayar_yaz("mobil", "koyu").expect("ilk yazım");
		let ay = ayar_yukle();
		assert_eq!(ay.duzen.as_deref(), Some("mobil"));
		assert_eq!(ay.tema.as_deref(), Some("koyu"));
		ayar_yaz("masaustu", "acik").expect("düzen+tema yazımı");
		let ay = ayar_yukle();
		assert_eq!(ay.duzen.as_deref(), Some("masaustu"));
		assert_eq!(ay.tema.as_deref(), Some("acik"));
		ayar_yaz("otomatik", "koyu").expect("otomatik yazımı");
		let ay = ayar_yukle();
		assert_eq!(ay.duzen.as_deref(), Some("otomatik"));
		assert_eq!(ay.tema.as_deref(), Some("koyu"));
		std::fs::write(&yol, "{}").unwrap();
		let ay = ayar_yukle();
		assert!(
			ay.duzen.is_none() && ay.tema.is_none(),
			"bilinmeyen içerikte varsayılan Otomatik kullanılır"
		);
		let _ = std::fs::remove_file(&yol);
	}

	/// Denetim 2 — kontrast ölçümü: egui varsayılan soluk metni koyu
	/// zeminde AA'nın (4.5:1) altında kalır; stil_tamir sonrası her iki
	/// temada soluk metin ve hata renginin ölçüleri raporlanır.
	#[test]
	fn kontrast_tamiri_aa_uyumlu() {
		// koyu tema: tamir ÖNCESİ gerçek değer (beklenti: 4.5 altı → ihlal)
		let koyu = egui::Style::default();
		let zemin = koyu.visuals.window_fill();
		let once = kontrast(koyu.visuals.weak_text_color(), zemin);
		assert!(once < 4.5, "koyu temada varsayılan soluk metin ihlal olmalıydı ({once:.2})");
		// tamir sonrası: >= 4.5 (metin 4.5x gövde boyundan küçük → AA normal metin)
		let mut koyu_t = egui::Style::default();
		stil_tamir(&mut koyu_t, Tema::Koyu);
		let sonra = kontrast(koyu_t.visuals.weak_text_color(), zemin);
		assert!(sonra >= 4.5);
		let zaman_gri = kontrast(egui::Color32::from_gray(170), zemin);
		assert!(zaman_gri >= 4.5);
		let hata = kontrast(egui::Color32::from_rgb(220, 120, 90), zemin);
		assert!(hata >= 4.5);
		// açık tema: tamir devrede olmalı (varsayılan kalabilir ama garanti sabit)
		let mut acik = egui::Style { visuals: egui::Visuals::light(), ..Default::default() };
		let acik_zemin = acik.visuals.window_fill();
		stil_tamir(&mut acik, Tema::Acik);
		let acik_sonra = kontrast(acik.visuals.weak_text_color(), acik_zemin);
		assert!(acik_sonra >= 4.5);
		println!(
			"kontrast ölçüleri: koyu-önce={once:.2} (ihlal) koyu-sonra={sonra:.2} zaman-gri={zaman_gri:.2} hata={hata:.2} açık-tamir={acik_sonra:.2}"
		);
	}

	#[test]
	fn typeset_sayi_ve_aci() {
		assert_eq!(sayilar_al("(10,20.5)"), vec![10.0, 20.5]);
		assert_eq!(sayilar_al("m 0 0 l 10 10"), vec![0.0, 0.0, 10.0, 10.0]);
		assert_eq!(aci_derece(370.0), 10.0);
		assert_eq!(aci_derece(-190.0), 170.0);
		assert_eq!(aci_derece(180.0), 180.0);
	}

	#[test]
	fn typeset_alfa_siniri() {
		let (w, h) = (5usize, 4usize);
		let mut d = vec![0u8; w * h * 4];
		for (x, y) in [(1usize, 1usize), (3usize, 2usize)] {
			let i = (y * w + x) * 4;
			d[i + 3] = 255;
		}
		let bb = alfa_bbox(&d, w, h).expect("alfa sınırı");
		assert_eq!(bb.min, egui::pos2(1.0, 1.0));
		assert_eq!(bb.max, egui::pos2(4.0, 3.0));
		assert!(alfa_bbox(&vec![0u8; w * h * 4], w, h).is_none());
	}

	#[test]
	fn typeset_hiza_ankrajlari() {
		let bb = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 50.0));
		assert_eq!(hiza_ankraj(bb, 2), egui::pos2(60.0, 70.0));
		assert_eq!(hiza_ankraj(bb, 5), egui::pos2(60.0, 45.0));
		assert_eq!(hiza_ankraj(bb, 7), egui::pos2(10.0, 20.0));
		assert_eq!(hiza_ankraj(bb, 9), egui::pos2(110.0, 20.0));
	}

	#[test]
	fn typeset_donusumu() {
		let p = egui::pos2(20.0, 20.0);
		let pivot = egui::pos2(10.0, 10.0);
		let q = ts_donusum(p, pivot, 2.0, 0.0, egui::vec2(3.0, 4.0));
		assert!((q.x - 33.0).abs() < 0.01 && (q.y - 34.0).abs() < 0.01);
		let r = ts_donusum(
			egui::pos2(20.0, 10.0),
			pivot,
			1.0,
			std::f32::consts::FRAC_PI_2,
			egui::Vec2::ZERO,
		);
		assert!((r.x - 10.0).abs() < 0.01 && (r.y - 20.0).abs() < 0.01);
	}

	#[test]
	fn typeset_clip_dikdortgen_gidis_donus() {
		let m = "{\\pos(10,20)\\clip(110,220,10,20)}A";
		let (r, ic) = clip_dik_oku(m).unwrap().unwrap();
		assert_eq!(r, (10.0, 20.0, 110.0, 220.0));
		assert!(!ic);
		let m = clip_dik_yaz(m, (0.0, 1.0, 100.0, 101.0), true);
		let (r, ic) = clip_dik_oku(&m).unwrap().unwrap();
		assert_eq!(r, (0.0, 1.0, 100.0, 101.0));
		assert!(ic);
		assert!(m.contains("\\iclip(0.0,1.0,100.0,101.0)"));
		assert!(!m.contains("\\clip("));
		assert!(clip_dik_yaz("{\\clip(m 0 0 l 10 0 10 10)}A", (0.0, 0.0, 1.0, 1.0), false)
			.contains("\\clip(0.0,0.0,1.0,1.0)"));
	}

	#[test]
	fn typeset_clip_vektor_gidis_donus() {
		let m = "{\\pos(10,20)}A";
		let pts = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 50.0)];
		let m = clip_vektor_yaz(m, &pts, false);
		let (okunan, ic) = clip_vektor_oku(&m).unwrap();
		assert_eq!(okunan, pts);
		assert!(!ic);
		let m = clip_vektor_yaz(&m, &pts, true);
		let (okunan, ic) = clip_vektor_oku(&m).unwrap();
		assert_eq!(okunan, pts);
		assert!(ic);
		assert!(!m.contains("\\clip(m"));
	}

	#[test]
	fn typeset_xy_ve_kenar_mesafesi() {
		let m = xy_yaz("{\\pos(10,20)}A", 370.0, -190.0);
		assert!(m.contains("\\frx10.0"));
		assert!(m.contains("\\fry170.0"));
		let d = kenar_mesafe(
			egui::pos2(0.0, 10.0),
			egui::pos2(-10.0, 0.0),
			egui::pos2(10.0, 0.0),
		);
		assert!((d - 10.0).abs() < 0.01);
	}
}
