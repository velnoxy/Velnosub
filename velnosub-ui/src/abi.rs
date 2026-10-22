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
	pub const SUBTITLES: &str = "\u{e048}";
	pub const VIDEO: &str = "\u{e04b}";
	pub const AUDIO: &str = "\u{e3a1}";
	pub const HELP: &str = "\u{e8fd}";
	pub const INFO: &str = "\u{e88e}";
	pub const MOVIE: &str = "\u{e02c}";
	pub const EQ: &str = "\u{e1de}";
	pub const TEXT: &str = "\u{e262}";
	pub const BOLD: &str = "\u{e238}";
	pub const ITALIC: &str = "\u{e23f}";
	pub const UNDERLINE: &str = "\u{e249}";
	pub const STRIKE: &str = "\u{e23d}";
	pub const ZOOM_IN: &str = "\u{e8ff}";
	pub const ZOOM_OUT: &str = "\u{e900}";
	pub const FULL: &str = "\u{e5d0}";
	pub const FAST_FWD: &str = "\u{e01f}";
	pub const FAST_REW: &str = "\u{e020}";
	pub const LAYERS: &str = "\u{e53b}";
	pub const TUNE: &str = "\u{e429}";
	pub const EXTENSION: &str = "\u{e87b}";
	pub const TRANSLATE: &str = "\u{e8e2}";
	pub const VISIBILITY: &str = "\u{e8f4}";
	pub const ARROW_BACK: &str = "\u{e5c4}";
	pub const ARROW_FWD: &str = "\u{e5c8}";
	pub const CHECK: &str = "\u{e5ca}";
}

extern "C" {
	fn velnosub_abi_init(config_json_path: *const c_char) -> c_int;
	fn velnosub_abi_open(in_path: *const c_char) -> c_int;
	fn velnosub_abi_event_count(h: c_int) -> c_int;
	fn velnosub_abi_style_count(h: c_int) -> c_int;
	fn velnosub_abi_line_text(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_line_times(
		h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int,
	) -> c_int;
	fn velnosub_abi_edit_line(h: c_int, idx: c_int, text: *const c_char) -> c_int;
	fn velnosub_abi_line_meta(h: c_int, idx: c_int, buf: *mut c_char, buflen: c_int) -> c_int;
	fn velnosub_abi_line_meta_set(h: c_int, idx: c_int, stil: *const c_char,
		aktor: *const c_char, etki: *const c_char, yorum: c_int, katman: c_int,
	) -> c_int;
	fn velnosub_abi_line_times_set(h: c_int, idx: c_int, ms_bas: i64, ms_son: i64) -> c_int;
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
	fn velnosub_abi_save(h: c_int, out_path: *const c_char) -> c_int;
	fn velnosub_abi_close(h: c_int);
	fn velnosub_abi_last_error() -> *const c_char;
	fn velnosub_abi_video_open(path: *const c_char) -> c_int;
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
	fn velnosub_abi_audio_open(
		path: *const c_char, rate: c_int, ch: c_int,
	) -> c_int;
	fn velnosub_abi_audio_file_open(path: *const c_char) -> c_int;
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
	frame: i32,
	info: String,
	dok: Option<egui::TextureHandle>,
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

/// P4.4: açılı ses kolu + dalga formu peak kovaları (sabit 512 kova).
pub struct AudioSection {
	handle: c_int,
	rate: c_int,
	ch: c_int,
	toplam: i64,
	minv: Vec<i16>,
	maxv: Vec<i16>,
	info: String,
	ses_durum: String,
	oynatma_denendi: bool,
}

fn audio_peaks_cek(h: c_int, toplam: i64) -> Result<(Vec<i16>, Vec<i16>), String> {
	let k: c_int = 512;
	let mut minv = vec![0i16; k as usize];
	let mut maxv = vec![0i16; k as usize];
	let rc = unsafe {
		velnosub_abi_audio_peaks(h, 0, toplam, k, minv.as_mut_ptr(), maxv.as_mut_ptr())
	};
	if rc != 0 {
		return Err(last_error());
	}
	Ok((minv, maxv))
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
	let dh = unsafe { velnosub_abi_dialog_open(c.as_ptr()) };
	if dh < 0 {
		return Err(last_error());
	}
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
	Altyazi,
	Video,
	Ses,
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
			SecimTur::Altyazi => &["ass", "ssa", "srt", "txt"],
			SecimTur::Video => &["mkv", "mp4", "webm", "avi", "mov", "m4v", "ts"],
			SecimTur::Ses => &["mp3", "m4a", "aac", "flac", "opus", "ogg", "wav", "mka", "wv"],
		}
	}
}

pub struct GridApp {
	handle: Option<c_int>,
	rows: Vec<Row>,
	path: Option<String>,
	save_path: Option<String>,
	note: String,
	video: Option<VideoSection>,
	audio: Option<AudioSection>,
	// P5.5: kendi dosyalarını açma — uygulamaiçi tarayıcı
	tarayici: Tarayici,
	stiller: Vec<StyleRow>,
	dialog: Option<DialogSection>,
	// P4.6 dilim 2: ASS önizleme (video üstünde saydam dokuya binen kare)
	oniz: Oniz,
	// P4.6 dilim 2: arayüz düzeni — varsayılan Masaustu (aegisub yüzeyi,
	// ölçek+ kaydırmalı); Mobil ayarlardan/seçimden değiştirilir
	// (`velnosub-ui.json`; VELNOSUB_UI_JSON ortam değişkeniyle yol geçilir)
	duzen: Duzen,
	// P4.8.1: masaüstü yüzeyi ölçeği — ppp = ppp_base × zoom; Mobil'e
	// dönüşte tabana döner
	zoom: f32,
	ppp_base: Option<f32>,
	aktif: Sekme,
	// masaüstü düzeninde Ayarlar penceresi açık mı (mobilde ayrı sekme var)
	ayar_win: bool,
	// P4.8.2: seçili satır (aegisub edit kutusu bu satıra bağlanır)
	secili: Option<usize>,
	// P5.4 gece: geri/ileri anlık görüntüleri + koalema zamanı
	undo_s: Vec<String>,
	redo_s: Vec<String>,
	son_an: Option<std::time::Instant>,
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

/// Arayüz düzeni. `Secim` yalnızca ayar dosyası yokken geçerli ilk ekrandır.
#[derive(PartialEq, Clone, Copy)]
enum Duzen {
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
/// varsayılan kullanılır: dosya yoksa düzen seçimi sorulur, tema koyu).
pub struct Ayar {
	pub duzen: Option<String>,
	pub tema: Option<String>,
}

/// `{ "anahtar": "deger" }` biçimi için sade tarayıcı: anahtarı bulup
/// ondan sonraki ilk tırnaklı parçayı döner. Yalnız bizim yazdığımız
/// `velnosub-ui.json` beklenir; yabancı içerik None döner (yeniden sor).
fn deger_bul(icerik: &str, anahtar: &str) -> Option<String> {
	let cap = format!("\"{anahtar}\"");
	let bas = icerik.find(&cap)? + cap.len();
	let kalan = &icerik[bas..];
	let i = kalan.find('"')? + 1;
	let kalan2 = &kalan[i..];
	let j = kalan2.find('"')?;
	Some(kalan2[..j].to_string())
}

/// Ayar dosyasını okur; dosya yoksa boş Ayar (hem düzen hem tema None).
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
			handle: None,
			rows: Vec::new(),
			path: None,
			save_path: save,
			note: "açık dosya yok".into(),
			video: None,
			audio: None,
			tarayici: Tarayici::yeni(),
			stiller: Vec::new(),
			dialog: None,
			duzen: Duzen::Masaustu,
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
			oniz: Oniz::default(),
		};
		match ayar_yukle().duzen.as_deref() {
			Some("mobil") => app.duzen = Duzen::Mobil,
			Some("masaustu") => app.duzen = Duzen::Masaustu,
			_ => {}
		}
		if let Some(p) = open {
			if let Err(e) = app.load(&p) {
				app.note = format!("açılamadı: {e}");
			}
		}
		if let Some(v) = std::env::var("VELNOSUB_VIDEO").ok() {
			if let Err(e) = app.load_video(&v) {
				app.note = format!("video açılamadı: {e}");
			}
		}
		if let Some(s) = std::env::var("VELNOSUB_AUDIO").ok() {
			if let Err(e) = app.load_audio(&s) {
				app.note = format!("ses açılamadı: {e}");
			}
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
			// [5]=etki [6..8]=kenarlar (UI'da henüz kullanılmıyor)
			rows.push(Row {
				bas: bir,
				son,
				text,
				stil: mp.get(3).cloned().unwrap_or_default(),
				oyuncu: mp.get(4).cloned().unwrap_or_default(),
				etki: mp.get(5).cloned().unwrap_or_default(),
				yorum: mp.get(1).map(|v| v == "1").unwrap_or(false),
				katman: mp.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
			});
		}
		self.rows = rows;
	}

	// ---------------- P5.4 gece: geri/ileri (tam doküman anları) ---------
	// Her mutasyon ÖNCESİ mevcut doküman filesDir'e snapshot edilir (kaydet
	// hattı ABI save). 2 sn koalema: ardışık klavye yazımları tek geri-adım.
	const UNO_LIMIT: usize = 24;

	fn an_cektir(&mut self, onek: &str, n: usize) -> Option<String> {
		let h = self.handle?;
		let save = std::env::var("VELNOSUB_SAVE").ok()?;
		let dizin = std::path::Path::new(&save).parent()?.to_path_buf();
		let p = dizin.join(format!("{onek}-{n}.ass"));
		let Ok(c) = CString::new(p.to_str()?.to_string()) else {
			return None;
		};
		let rc = unsafe { velnosub_abi_save(h, c.as_ptr()) };
		if rc == 0 {
			Some(p.to_string_lossy().into_owned())
		} else {
			None
		}
	}

	fn an_tut(&mut self) {
		if let Some(t) = self.son_an {
			if t.elapsed() < std::time::Duration::from_secs(2) {
				return;
			}
		}
		let n = self.undo_s.len();
		let Some(p) = self.an_cektir("undo", n % 24 + 1) else {
			return;
		};
		self.undo_s.push(p);
		if self.undo_s.len() > 24 {
			self.undo_s.remove(0);
		}
		self.son_an = Some(std::time::Instant::now());
		self.redo_s.clear();
	}

	fn geri_al(&mut self) {
		if self.undo_s.is_empty() {
			self.note = "geri alınacak işlem yok".into();
			return;
		}
		// mevcut dokümanı ileri-anına sakla (aegi: redo tarafı)
		let n = self.redo_s.len() + 1;
		if let Some(p) = self.an_cektir("redo", n % 24 + 1) {
			self.redo_s.push(p);
			if self.redo_s.len() > 24 {
				self.redo_s.remove(0);
			}
		}
		let p = self.undo_s.pop().unwrap();
		if let Err(e) = self.load(&p) {
			self.note = format!("geri alınamadı: {e}");
			return;
		}
		self.note = format!("geri alındı — kalan an: {}", self.undo_s.len());
	}

	fn ileri_al(&mut self) {
		if self.redo_s.is_empty() {
			self.note = "ileri alınacak işlem yok".into();
			return;
		}
		let n = self.undo_s.len() + 1;
		if let Some(p) = self.an_cektir("undo", n % 24 + 1) {
			self.undo_s.push(p);
			if self.undo_s.len() > 24 {
				self.undo_s.remove(0);
			}
		}
		let p = self.redo_s.pop().unwrap();
		if let Err(e) = self.load(&p) {
			self.note = format!("ileri alınamadı: {e}");
			return;
		}
		self.note = "ileri alındı".into();
	}

	// ---------------- P5.4 gece: satır ekle/sil --------------------------
	fn satir_ekle_tetik(&mut self, cogalt: bool) {
		let Some(h) = self.handle else {
			self.note = "açık dosya yok".into();
			return;
		};
		let i = self.secili.unwrap_or(0);
		let idx = (i + 1).min(self.rows.len());
		let sonuc = if cogalt {
			self.rows
				.get(i)
				.clone()
				.map(|s| {
					satir_ekle(
						h, idx,
						&s.text.clone(),
						&s.bas.clone(),
						&s.son.clone(),
						&s.stil.clone(),
					)
				})
				.unwrap_or_else(|| Err("satır yok".into()))
		} else {
			satir_ekle(h, idx, "", "", "", "")
		};
		match sonuc {
			Ok(()) => {
				self.yerles();
				self.secili = Some(idx);
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
		let Some(h) = self.handle else {
			self.note = "açık dosya yok".into();
			return;
		};
		let Some(i) = self.secili else {
			self.note = "satır seç".into();
			return;
		};
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
		let idxler: Vec<usize> = if self.shift_tum {
			(0..self.rows.len()).collect()
		} else {
			vec![self.secili.unwrap_or(0)]
		};
		let mut yazi: Vec<Yazi> = Vec::new();
		for i in idxler {
			let Some(r) = self.rows.get(i) else { continue };
			let (Ok(mb), Ok(ms)) = (ass_ms(&r.bas), ass_ms(&r.son)) else {
				continue;
			};
			yazi.push(Yazi::BasVal(i, (mb + d as i64).max(0)));
			yazi.push(Yazi::SonVal(i, (ms + d as i64).max(0)));
		}
		if yazi.is_empty() {
			self.note = "kaydırılacak saat yok".into();
			return;
		}
		self.son_an = None;
		self.yazlari_uygula(yazi);
		self.note = format!("kaydırıldı: {d} ms");
	}

	// ---------------- P5.4 gece: paylaşımlı diyalog pencereleri ---------
	// masaüstü + mobil aynı; Ayarlar penceresi de buraya taşındı
	/// P5.5: tarayıcıyı istenen türle açar
	fn tarayici_ac(&mut self, tur: SecimTur) {
		self.tarayici = Tarayici {
			acik: true,
			tur,
			yol: std::path::PathBuf::from("/storage/emulated/0"),
			secili: None,
			hata: None,
		};
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
									if let Err(e) = self.load(&yolc) {
										self.note = format!("açılamadı: {e}");
									}
								}
								SecimTur::Video => self.video_ac(&yolc),
								SecimTur::Ses => self.ses_ac(&yolc),
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
		let Some(fps) = self.video.as_ref().map(|v| v.fps) else {
			self.note = "kareye oturt: video yok".into();
			return;
		};
		let Some(i) = self.secili else {
			self.note = "kareye oturt: satır seç".into();
			return;
		};
		if fps <= 0.0 {
			self.note = "kareye oturt: fps geçersiz".into();
			return;
		}
		let Some(sat) = self.rows.get(i) else {
			return;
		};
		let mb = ass_ms(&sat.bas).unwrap_or(0);
		let ms = ass_ms(&sat.son).unwrap_or(0);
		let cevir = |x: i64| -> i64 {
			let kare = (x as f64 * fps / 1000.0).round();
			(kare * 1000.0 / fps).round() as i64
		};
		let yb = if bas { cevir(mb) } else { mb };
		let ys = if son { cevir(ms) } else { ms };
		if ys <= yb {
			self.note = "kareye oturt: geçersiz süre".into();
			return;
		}
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
		if let Some(h) = self.handle.take() {
			unsafe { velnosub_abi_close(h) };
			self.rows.clear();
		}
		let in_c = CString::new(path.to_string()).map_err(|e| e.to_string())?;
		let h = unsafe { velnosub_abi_open(in_c.as_ptr()) };
		if h < 0 {
			return Err(last_error());
		}
		let ev = unsafe { velnosub_abi_event_count(h) };
		if ev < 0 {
			let e = last_error();
			unsafe { velnosub_abi_close(h) };
			return Err(e);
		}
		let st = unsafe { velnosub_abi_style_count(h) };
		self.handle = Some(h);
		self.yerles();
		self.secili = None;
		self.undo_s.clear();
		self.redo_s.clear();
		self.path = Some(path.to_string());
		self.note = format!("{path} — {ev} satır, {st} stil");
		self.load_styles();
		Ok(())
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

	fn save_to(&mut self, path: &str) {
		let Some(h) = self.handle else {
			self.note = "kayıt: açık dosya yok".into();
			return;
		};
		let result = CString::new(path.to_string()).map_err(|e| e.to_string()).and_then(|out_c| {
			let rc = unsafe { velnosub_abi_save(h, out_c.as_ptr()) };
			if rc == 0 {
				Ok(())
			} else {
				Err(last_error())
			}
		});
		self.note = match result {
			Ok(()) => format!("kaydedildi: {path}"),
			Err(e) => format!("kayıt hatası: {e}"),
		};
	}

	/// P4.3: video kolu aç; bir kare ertelendi (ui'da doku kurulumu).
	fn load_video(&mut self, ender: &str) -> Result<(), String> {
		if let Some(v) = self.video.take() {
			unsafe { velnosub_abi_video_close(v.handle) };
		}
		let yolu = CString::new(ender.to_string()).map_err(|e| e.to_string())?;
		let h = unsafe { velnosub_abi_video_open(yolu.as_ptr()) };
		if h < 0 {
			return Err(last_error());
		}
		let (w, hgt, fps, kareler) = video_geom(h)?;
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
		self.video = Some(VideoSection {
			handle: h,
			w,
			hgt,
			fps,
			kareler,
			frame: 0,
			info: format!("fps:{fps:.3} {bilgi}"),
			dok: None,
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

	/// P4.4: ses kolu aç; tüm aralığın peak kovalarını tek geçişte çek.
	fn load_audio(&mut self, ender: &str) -> Result<(), String> {
		if let Some(a) = self.audio.take() {
			unsafe { velnosub_abi_audio_close(a.handle) };
		}
		let yolu = CString::new(ender.to_string()).map_err(|e| e.to_string())?;
		// P5.5: uzantı dağıtımı — kutu dosyalar (mp3/mkv/m4a…) çekirdek
		// ffmpeg hattıyla decode edilir (audio_file_open); ham s16 PCM
		// mevcut audio_open hattından geçer
		let uz = ender
			.rsplit_once('.')
			.map(|(_, k)| k.to_ascii_lowercase())
			.unwrap_or_default();
		const KUTULAR: &[&str] = &[
			"mp3", "m4a", "aac", "flac", "opus", "ogg", "wma", "wv",
			"wav", "mka", "mkv", "webm", "mp4", "avi", "mov", "ts",
		];
		let h = if KUTULAR.contains(&uz.as_str()) {
			unsafe { velnosub_abi_audio_file_open(yolu.as_ptr()) }
		} else {
			unsafe { velnosub_abi_audio_open(yolu.as_ptr(), 48000, 2) }
		};
		if h < 0 {
			return Err(last_error());
		}
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
			return Err(last_error());
		}
		let (minv, maxv) = audio_peaks_cek(h, toplam)?;
		self.audio = Some(AudioSection {
			handle: h,
			rate: 48000,
			ch: 2,
			toplam,
			minv,
			maxv,
			info: bilgi,
			ses_durum: "—".into(),
			oynatma_denendi: false,
		});
		self.note = format!("ses açıldı: {toplam} örnek");
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
		gen: usize, yuk: usize, hiz: f64, kare: i32,
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
		if !self.oniz.acik || hiz <= 0.0 || kare < 0 {
			return;
		}
		let ms = (kare as f64 * 1000.0 / hiz).round() as i32;
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

	fn video_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		// P4.3: video şeridi — sanallaştırılmış grid'in üstünde; hata üstüne note
		let mut v_hata: Option<String> = None;
		let mut oniz_dik: Option<egui::Rect> = None;
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
						let hk =
							((v.fps * (oyn_d0 as f64) / 1000.0).round()
								as i32)
								.clamp(0, (v.kareler - 1).max(0));
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
				su.label(RichText::new(format!(
					"kare {}/{}",
					v.frame + 1, v.kareler
				)).weak());
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
				let hedef = ((hedef_ms * v.fps / 1000.0).round() as i32)
					.clamp(0, (v.kareler - 1).max(0));
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
			if let Some(sac) = &v.dok {
				// P6.1 masaüstü: pane genişliğine sığdır (aegisub kuralı —
				// video pane taşmaz, kaydırma olmaz; mobil ham boyutta kalır)
				let boy = if mobil {
					egui::vec2(v.w as f32, v.hgt as f32)
				} else {
					let maks = ui.available_width();
					// oran küçültme — kare oranı bozulmaz; büyümez (≤1)
					let k = (maks / (v.w.max(1) as f32)).min(1.0);
					egui::vec2(v.w as f32 * k, v.hgt as f32 * k)
				};
				let yanit = ui.image((sac.id(), boy));
				oniz_dik = Some(yanit.rect);
			}
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
		// P4.6 dilim 2 önizleme: şerit çiziminden sonra şartı hazırla ve
		// video dikdörtgenine saydam dokuyu işle
		let (h, k5) = (
			self.handle,
			self.video
				.as_ref()
				.map(|v| (v.w, v.hgt, v.fps, v.frame)),
		);
		if let (Some(alt_h), Some((gen, yuk, hiz, kare))) = (h, k5) {
			self.oniz_cek(ui.ctx(), alt_h, gen, yuk, hiz, kare);
			if let (Some(dik), Some(d)) = (oniz_dik, self.oniz.dok.as_ref()) {
				if self.oniz.etki > 0 {
					ui.put(
						dik,
						egui::Image::new((
							d.id(),
							egui::vec2(gen as f32, yuk as f32),
						)),
					);
				}
			}
			if let Some(hata) = &self.oniz.hata {
				ui.label(RichText::new(hata).weak());
			}
		}
	}

	fn ses_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		// P4.4: ses şeridi — oynatma tuşları + dalga formu (peak kovalarından)
		if let Some(a) = self.audio.as_mut() {
			ui.add_space(4.0);
			let mut oynat = egui::Button::new("▶ oynat");
			let mut kes = egui::Button::new("⏹ kes");
			if mobil {
				oynat = oynat.min_size(egui::vec2(0.0, 44.0));
				kes = kes.min_size(egui::vec2(0.0, 44.0));
			}
			ui.horizontal(|su| {
				if su.add(oynat).clicked() {
					let adet = a.toplam.min((a.rate as i64) * 5);
					let rc = unsafe { velnosub_abi_audio_play(a.handle, 0, adet) };
					a.oynatma_denendi = true;
					a.ses_durum = if rc == 0 {
						"oynatma istendi".into()
					} else {
						format!("oynatma olmadı: {}", last_error())
					};
				}
				if su.add(kes).clicked() {
					unsafe { velnosub_abi_audio_stop(a.handle) };
					a.ses_durum = "kesildi".into();
				}
				su.label(RichText::new(&a.info).weak());
			});
			if a.oynatma_denendi {
				let d = unsafe { velnosub_abi_audio_state(a.handle) };
				let ad = match d {
					-1 => "oynatma yok",
					4 => "hazırlanıyor",
					0 => "oynatılıyor",
					1 => "boşalma",
					2 => "hata",
					_ => "bilinmeyen",
				};
				a.ses_durum = format!("durum {d} — {ad}");
			}
			ui.label(RichText::new(&a.ses_durum).weak());
			ui.label(RichText::new(format!("{} kanal", a.ch)).weak());

			// dalga formu: 512 kova → duvara dik çubuklar (min..max)
			let genislik = ui.available_width();
			let yuk = 72.0f32;
			let (dik, _) =
				ui.allocate_exact_size(egui::vec2(genislik, yuk), egui::Sense::hover());
			let boy = a.minv.len().max(1) as f32;
			let orta = dik.center().y;
			let yarim = yuk * 0.5 - 2.0;
			let adim = dik.width() / boy;
			let kalem = egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 180, 250));
			for k in 0..a.minv.len() {
				let x = dik.left() + adim * (k as f32 + 0.5);
				let ymin = orta - yarim * (a.maxv[k] as f32 / 32768.0);
				let ymax = orta - yarim * (a.minv[k] as f32 / 32768.0);
				ui.painter()
					.line_segment([egui::pos2(x, ymin), egui::pos2(x, ymax)], kalem);
			}
			ui.add_space(4.0);
		}
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
				if ui.button("Kaydet").clicked() {
					if let Some(p) = self.save_path.clone() {
						self.save_to(&p);
					} else {
						self.note = "kayıt yolu verilmedi (VELNOSUB_SAVE)".into();
					}
				}
				if ui.button("Yeniden aç").clicked() {
					if let Some(p) = self.path.clone() {
						if let Err(e) = self.load(&p) {
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
			// mobil: Kaydet/Yeniden aç en az 44pt, not tüm genişlikte
			ui.add_space(8.0);
			let gen = ui.available_width();
			if ui.add(
				egui::Button::new(RichText::new("Kaydet").strong())
					.min_size(egui::vec2(gen, 44.0)),
			).clicked() {
				if let Some(p) = self.save_path.clone() {
					self.save_to(&p);
				} else {
					self.note = "kayıt yolu verilmedi (VELNOSUB_SAVE)".into();
				}
			}
			ui.add_space(8.0);
			if ui.add(
				egui::Button::new("Yeniden aç")
					.min_size(egui::vec2(gen, 44.0)),
			).clicked() {
				if let Some(p) = self.path.clone() {
					if let Err(e) = self.load(&p) {
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

/// Zaman yazısı: iki ASS dizgesini ms'e çöz, çekirdeğe yaz, satırı ASS
/// kanonizasyonuna döndür (başında yazan "0:00:01,00" tarzı girdiyi düzeltir).
fn zaman_yaz(
	uyg: &mut GridApp,
	h: c_int,
	i: usize,
	bs: String,
	sn: String,
) -> Result<(), String> {
	let mb = ass_ms(&bs)?;
	let ms = if sn.is_empty() { mb } else { ass_ms(&sn)? };
	times_set(h, i, mb, ms)?;
	if let Some(r) = uyg.rows.get_mut(i) {
		r.bas = ms_ass(mb);
		r.son = ms_ass(ms);
	}
	Ok(())
}

/// P5.4 gece: grid/demeç yazımları — topladıktan sonra çerçeve sonunda
/// işlenir (ödünç çatışması yok; ABI yazısı tek noktadan)
enum Yazi {
	Metin(usize),
	Bas(usize),
	Son(usize),
	Stil(usize),
	Oyuncu(usize),
	Etki(usize),
	Yorum(usize, bool),
	// P5.4 gece: edit-kutusu doğrudan değerleri (değer elle taşınır —
	// grid hücreleri satır alanını doğrudan düzenlediğinden işaret-yeterli)
	BasVal(usize, i64),
	SonVal(usize, i64),
	MetinVal(usize, String),
	StilVal(usize, String),
	OyuncuVal(usize, String),
	EtkiVal(usize, String),
}

impl GridApp {
	fn grid_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		if self.rows.is_empty() {
			ui.centered_and_justified(|ui| {
				ui.label(RichText::new("dosya açılmadı").weak().size(18.0));
			});
			return;
		}
		// Düzenlenip Commit tetiklenecek satırlar; ödünç çatışmasını
		// çözmek için çerçeve sonunda işlenir (tek çerçevede tek satır).
		let mut yazi: Vec<Yazi> = Vec::new();
		// P4.8.2: seçim ödünçsuz — önceden kopyalanır, dokunuş çerçeve
		// sonunda secili'ye uygulanır
		let secili_var = self.secili;
		let mut satir_sec: Option<usize> = None;
		// P5.4 gece: stil adı listesi önden kopyalanır (hücre ödünçsüzlüğü)
		let stiller_ad: Vec<String> =
			self.stiller.iter().map(|s| s.name.clone()).collect();
		// mobil: satır dokunma yüksekliği en az 44pt (3.2x gövde yüksekliği)
		let satir_h = ui.text_style_height(&egui::TextStyle::Body)
			* if mobil { 3.2 } else { 2.4 };
		if !mobil {
			// P6.2: aegisub kolon başlığı — satır widget yarı genişlikleri
			//yle eş (36/78/78/88/86/80), tıklanamaz soluk şerit
			ui.horizontal(|ub| {
				ub.add_sized(
					[36.0, 16.0],
					egui::Label::new(RichText::new("#").weak()),
				);
				ub.add_sized(
					[78.0, 16.0],
					egui::Label::new(RichText::new("Başla").weak()),
				);
				ub.add_sized(
					[78.0, 16.0],
					egui::Label::new(RichText::new("Bitiş").weak()),
				);
				ub.add_sized(
					[88.0, 16.0],
					egui::Label::new(RichText::new("Stil").weak()),
				);
				ub.add_sized(
					[86.0, 16.0],
					egui::Label::new(RichText::new("Oyuncu").weak()),
				);
				ub.add_sized(
					[80.0, 16.0],
					egui::Label::new(RichText::new("Etki").weak()),
				);
				ub.label(RichText::new("Metin").weak());
			});
			ui.add_space(3.0);
		}
		ScrollArea::vertical()
			.auto_shrink(false)
			.show_rows(ui, satir_h, self.rows.len(), |ui, aralık| {
				for i in aralık {
					let Some(row) = self.rows.get_mut(i) else { continue };
					if mobil {
						// telefon genişliğinde reflow: zaman üstte, metin tam genişlik
						ui.vertical(|ui| {
							ui.label(
								RichText::new(format!("{} → {}", &row.bas, &row.son))
									.weak()
									.color(Color32::from_gray(170)),
							);
							let w = ui.available_width();
							let resp = TextEdit::singleline(&mut row.text)
								.desired_width(w)
								.show(ui)
								.response;
							if resp.changed() {
								yazi.push(Yazi::Metin(i));
							}
						});
					} else {
						// P5.4 gece aegisub kolonları: # ✐ Start End Style
						// Actor Effect Text — hepsi GERÇEK ABI alan yazımı
						ui.vertical(|su| {
							su.horizontal(|h| {
								h.add_sized(
									[36.0, 18.0],
									egui::Label::new(
										RichText::new(format!("{:>3}", i + 1)).weak(),
									),
								);
								if h
									.checkbox(&mut row.yorum, "")
									.changed()
								{
									let v = row.yorum;
									yazi.push(Yazi::Yorum(i, v));
								}
								let yorum_renk = if row.yorum {
									// P6.2 aegisub yorum stili: checkbox'lı satır
									// mavi tinta (açık/koyu temada AA uyumlu ton)
									Some(Color32::from_rgb(100, 150, 235))
								} else {
									None
								};
								if h.add_sized(
									[78.0, 18.0],
									TextEdit::singleline(&mut row.bas)
										.desired_width(76.0)
										.font(egui::TextStyle::Monospace)
										.text_color_opt(yorum_renk),
								).changed() {
									yazi.push(Yazi::Bas(i));
								}
								if h.add_sized(
									[78.0, 18.0],
									TextEdit::singleline(&mut row.son)
										.desired_width(76.0)
										.font(egui::TextStyle::Monospace)
										.text_color_opt(yorum_renk),
								).changed() {
									yazi.push(Yazi::Son(i));
								}
								let sec = row.stil.clone();
								egui::ComboBox::from_id_salt(format!("kstil{i}"))
									.selected_text(&row.stil)
									.width(88.0)
									.show_ui(h, |c| {
										for ad in &stiller_ad {
											c.selectable_label(
												*ad == sec,
												ad.clone(),
											)
											.clicked()
											.then(|| ())
											.map(|_| yazi.push(Yazi::Stil(i)));
										}
									});
								if h.add_sized(
									[86.0, 18.0],
									TextEdit::singleline(&mut row.oyuncu)
										.desired_width(84.0),
								).changed() {
									yazi.push(Yazi::Oyuncu(i));
								}
								if h.add_sized(
									[80.0, 18.0],
									TextEdit::singleline(&mut row.etki)
										.desired_width(78.0),
								).changed() {
									yazi.push(Yazi::Etki(i));
								}
								// P6.2 metin kolonu KALAN tam genişlik — kolonlar
								// satırlar halinde sabit hizalanır (aegisub h-.)
								let resp = TextEdit::singleline(&mut row.text)
									.desired_width(h.available_width())
									.text_color_opt(yorum_renk)
									.show(h)
									.response;
								if resp.changed() {
									yazi.push(Yazi::Metin(i));
								}
							});
						});
					}
				}
			});
		// P4.8.2: dokunuşla satır seçimi — edit kutusu bu satıra bağlanır
		if let Some(s) = satir_sec {
			self.secili = Some(s);
		}
		self.yazlari_uygula(yazi);
	}

	/// P5.4 gece: ABI yazım hattı — önce koalemalı anlık (undo), sonra
	/// çekirdek yazısı; zaman yazımsında ASS kanonizasyonu da burada.
	/// Grid ve edit-kutusu ortak boru.
	fn yazlari_uygula(&mut self, yazi: Vec<Yazi>) {
		if yazi.is_empty() {
			return;
		}
		self.an_tut();
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
				Yazi::Bas(i) | Yazi::Son(i) => {
					let (bs, sn) = self
						.rows
						.get(i)
						.map(|r| (r.bas.clone(), r.son.clone()))
						.unwrap_or_default();
					zaman_yaz(self, h, i, bs, sn)
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
				Yazi::Stil(i) => {
					let s = self.rows.get(i).map(|r| r.stil.clone()).unwrap_or_default();
					meta_set(h, i, Some(&s), None, None, None, None)
				}
				Yazi::StilVal(i, ref s) => {
					if let Some(r) = self.rows.get_mut(i) {
						r.stil = s.clone();
					}
					meta_set(h, i, Some(s), None, None, None, None)
				}
				Yazi::Oyuncu(i) => {
					let s = self.rows.get(i).map(|r| r.oyuncu.clone()).unwrap_or_default();
					meta_set(h, i, None, Some(&s), None, None, None)
				}
				Yazi::OyuncuVal(i, ref s) => {
					if let Some(r) = self.rows.get_mut(i) {
						r.oyuncu = s.clone();
					}
					meta_set(h, i, None, Some(s), None, None, None)
				}
				Yazi::Etki(i) => {
					let s = self.rows.get(i).map(|r| r.etki.clone()).unwrap_or_default();
					meta_set(h, i, None, None, Some(&s), None, None)
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
			};
			if let Err(e) = hata {
				self.note = format!("işleme hatası: {e}");
			}
		}
	}

	/// İlk giriş: ayar dosyası yokken gösterilen düzen seçim ekranı. Seçim
	/// dosyaya yazılır; dosyaya yazılamazsa oturum seçilmiş düzenle devam
	/// eder ve uyarı dürüstçe gösterilir.
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
		ui.add_space(8.0);
		ui.heading("Ayarlar");
		self.ayarlar_icerik(ui);
	}

	/// Ayarlar içeriği: mobil Ayarlar sekmesinde ve masaüstündeki Ayarlar
	/// penceresinde AYNI gövde kullanılır.
	fn ayarlar_icerik(&mut self, ui: &mut egui::Ui) {
		ui.add_space(8.0);
		ui.label(RichText::new("Arayüz düzeni").strong());
		ui.add_space(4.0);
		let gen = ui.available_width().min(360.0);
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
			RichText::new(
				if self.duzen == Duzen::Mobil {
					"seçili: mobil UI"
				} else {
					"seçili: masaüstü UI"
				},
			)
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
		ui.label(RichText::new(format!("ayar dosyası: {}", ayar_yolu())).weak());
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
		let mut etiketler: Vec<(Sekme, &'static str)> =
			vec![(Sekme::Altyazi, "Altyazı")];
		if self.video.is_some() {
			etiketler.push((Sekme::Video, "Video"));
		}
		if self.audio.is_some() {
			etiketler.push((Sekme::Ses, "Ses"));
		}
		if !self.stiller.is_empty() {
			etiketler.push((Sekme::Stil, "Stil"));
		}
		if self.dialog.is_some() {
			etiketler.push((Sekme::Form, "Form"));
		}
		etiketler.push((Sekme::Ayarlar, "Ayarlar"));
		if !etiketler.iter().any(|(s, _)| *s == self.aktif) {
			self.aktif = Sekme::Altyazi;
		}
		Panel::bottom("vtabs").show(ui, |su| {
			su.add_space(6.0);
			// sekme genişliği kalan alana oranlanır: en dar pencerede de
			// taşma olmaz (satır yatay kaydırma yok);
			// denetim 1: dar genişlikte etiket kırpılmasın — düğme iç
			// padding'i daralır, metin boyu sekme genişliğine uyarlanır
			// (9–15.5pt bandı)
			let adet = etiketler.len().max(1) as f32;
			let gen = ((su.available_width() - 8.0) / adet).max(44.0);
			su.horizontal(|k| {
				k.spacing_mut().button_padding = egui::vec2(4.0, 8.0);
				for (s, ad) in &etiketler {
					let metin_boyu = (gen * 0.22).clamp(9.0, 15.5);
					if k.add(
						egui::Button::new(RichText::new(*ad).size(metin_boyu))
							.min_size(egui::vec2(gen, 44.0))
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
					self.grid_icerik(su, true);
				}
				Sekme::Video => self.video_icerik(su, true),
				Sekme::Ses => self.ses_icerik(su, true),
				Sekme::Stil => self.stil_icerik(su, true),
				Sekme::Form => self.form_icerik(su, true),
				Sekme::Ayarlar => self.ayarlar_bolum(su),
			}
		});
		// P5.4 gece: mobil yüzede paylaşımlı diyaloglar (Ara-Bul / zaman
		// kaydır / Ayarlar pencere) — masaüstü ile aynı kanal
		self.diyaloglar(ui);
	}

	/// Masaüstü çerçeve: mevcut P4.2–P4.6 şerit akışı (üst paneller +
	/// alt durum çubuğu + grid), gövdeler mobil ile aynı.
	/// P4.8.2: uzantı listesi — VELNOSUB_EXTDIR'de (app dış filesDir;
	/// rish ext_data_rw ile dosya konur) eşleşen dosya yolları sıralı
	fn uzanti_listele(&self, uz: &[&str]) -> Vec<String> {
		let Ok(yol) = std::env::var("VELNOSUB_EXTDIR") else { return Vec::new(); };
		let mut sonuc = Vec::new();
		let Ok(girdiler) = std::fs::read_dir(&yol) else { return sonuc; };
		let mut kepce: Vec<_> = girdiler
			.filter_map(|r| r.ok())
			.map(|e| e.path())
			.filter(|p| {
				p.extension()
					.and_then(|x| x.to_str())
					.is_some_and(|x| uz.contains(&x.to_ascii_lowercase().as_str()))
			})
			.collect();
		kepce.sort();
		for p in kepce {
			if let Some(s) = p.to_str() {
				sonuc.push(s.to_string());
			}
		}
		sonuc
	}

	/// Video/ses dosyası yükler ve notu tazeler (menü/toolbar çağrısı)
	fn video_ac(&mut self, yol: &str) {
		if let Err(e) = self.load_video(yol) {
			self.note = format!("video açılamadı: {e}");
		}
	}

	fn ses_ac(&mut self, yol: &str) {
		if let Err(e) = self.load_audio(yol) {
			self.note = format!("ses açılamadı: {e}");
		}
	}

	/// P6.4: seçilen iç altyazı akışını SRT'ye çeker ve yükler —
	/// seçici hattı; yüklenen .srt doğrudan düzen/mevcut satır-görünümüne
	/// girer (menüdeki Aç ile aynı boru)
	fn alt_yukle(&mut self, aks: i64) {
		if aks < 0 || aks > i32::MAX as i64 {
			self.note = "akış numarası geçersiz".into();
			return;
		}
		let Some(yol) = self.video.as_ref().map(|v| v.yol.clone()) else {
			return;
		};
		let cik = {
			let ana = alt_onbellek_ana();
			if ana.is_empty() {
				"user-alt.srt".to_string()
			} else {
				format!("{ana}/user-alt.srt")
			}
		};
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
		if let Err(e) = self.load(&cik) {
			self.note = format!("altyazı yüklenemedi: {e}");
			return;
		}
		self.note = format!("iç altyazı akışı {aks}: {son} olay SRT'ye döşendi");
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
			.map(|v| (v.frame as f64 / v.fps.max(1e-6) * 1000.0) as i64)
			.unwrap_or(0);
		(bas, son, mevcut.clamp(bas, son), rate, toplam)
	}

	/// P4.8.2: aegisub menü çubuğu — mevcut işlevlere bağlı öğeler gerçek,
	/// ABI hat-alanları (v13) bekleyenler pasif gösterilir
	fn menu_cubugu(&mut self, ui: &mut egui::Ui) {
		let tema = tema_simdi(ui);
		egui::MenuBar::new().ui(ui, |mb| {
			mb.menu_button("File", |m| {
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
						if let Err(e) = self.load(&p) {
							self.note = format!("açılamadı: {e}");
						}
					}
				}
				if m.button(format!("{} Kaydet", ikon::SAVE)).clicked() {
					if let Some(p) = self.save_path.clone() {
						self.save_to(&p);
					} else {
						self.note = "kayıt yolu verilmedi (VELNOSUB_SAVE)".into();
					}
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
				let liste = self.uzanti_listele(&["mp4", "webm", "mkv"]);
				if liste.is_empty() {
					m.label(RichText::new("ext filesDir'de video yok (rish ile kopyala)").weak());
				}
				for y in liste {
					if m.button(y.clone()).clicked() {
						self.video_ac(&y);
					}
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
				let liste = self.uzanti_listele(&["mp3", "wav", "m4a", "ogg"]);
				if liste.is_empty() {
					m.label(RichText::new("ext filesDir'de ses yok (rish ile kopyala)").weak());
				}
				for y in liste {
					if m.button(y.clone()).clicked() {
						self.ses_ac(&y);
					}
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
				m.label(
					RichText::new(format!(
						"{} Lua motoru mobil derlemede kapalı",
						ikon::EXTENSION
					))
					.weak(),
				);
				m.label(RichText::new("Faz 5'te luajit mobil hedefiyle açılacak").weak());
			});
			mb.menu_button("View", |m| {
				if m.button(format!("{} Mobil görünüm", ikon::TEXT)).clicked() {
					self.duzen_sec(tema, Duzen::Mobil);
				}
				if m.button(format!("{} Masaüstü görünüm", ikon::FULL)).clicked() {
					self.duzen_sec(tema, Duzen::Masaustu);
				}
				if m.button(format!("{} Ölçek 1.0×", ikon::ZOOM_IN)).clicked() {
					self.zoom = 1.0;
				}
				m.separator();
				if m.button(format!("{} Stil Yöneticisi…", ikon::PALETTE)).clicked() {
					self.stil_win = true;
				}
				if m.button(format!("{} Ayarlar…", ikon::SETTINGS)).clicked() {
					self.ayar_win = true;
				}
			});
			mb.menu_button("Help", |m| {
				m.label(format!("{} velnosub UI — P8.1 (aegisub yüzeyi)", ikon::INFO));
				m.label("player: satır penceresi + ses/video eşleşik");
			});
		});
	}

	/// P4.8.2: aegisub subtitle-edit kutusu (grid üstü): seçili satırın
	/// metni düzenlenebilir (commit = edit_line); Style/Actor/Effect kolonu
	/// ABI hat-alanları (v13) gelmedikçe "—" gösterir; B/I/U/S ve \N düğmeleri
	/// metnin SONUNA tag ekler (imleç konumu v13'te izlenecek)
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
		let mut stil = sat.stil.clone();
		let mut oyuncu = sat.oyuncu.clone();
		let mut etki = sat.etki.clone();
		let yorum_sabit = sat.yorum;
		let mut yorum = yorum_sabit;
		let stil_kata: Vec<String> =
			self.stiller.iter().map(|s| s.name.clone()).collect();
		let mut yazi: Vec<Yazi> = Vec::new();
		let mut hatalar: Vec<String> = Vec::new();
		let hedef = if mobil { 44.0 } else { 0.0 };
		ui.vertical(|ev| {
			ev.horizontal(|h| {
				// Yorum onaykutusu — aegisub edit kutusu başlığı; mobilde
				// dar onaykutusu darlana (kutu mesafesi dilimi)
				if h.checkbox(&mut yorum, "Yorum").changed() && yorum != yorum_sabit {
					yazi.push(Yazi::Yorum(i, yorum));
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
			ev.horizontal(|h| {
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
				if !mobil {
					h.label(RichText::new("etki").weak());
					if TextEdit::singleline(&mut etki)
						.desired_width(88.0)
						.show(h)
						.response
						.changed()
					{
						let v = etki.clone();
						yazi.push(Yazi::EtkiVal(i, v));
					}
				}
			});
			ev.horizontal(|h| {
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
					.auto_shrink(false)
					.show(ev, |se| {
						TextEdit::multiline(&mut metin)
							.desired_width(se.available_width().min(620.0))
							.desired_rows(4)
							.show(se)
					})
					.inner
			};
			let mut imlec: usize = out
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
		// P4.8.1 aegisub-yüzeyi: ppp ölçek; P6.1: kaydırmasız panel yapısı;
		// default 0.8× (1220'lük ekranda ~okunur küçük), ± düğmeleriyle
		// ayarlanır; set_pixels_per_point frame'den sonra etkin (egui kuralı)
		let base = *self
			.ppp_base
			.get_or_insert_with(|| ui.ctx().pixels_per_point());
		ui.ctx()
			.set_pixels_per_point((base * self.zoom).clamp(0.35, 1.6));
		CentralPanel::default().show_inside(ui, |su| {
			// P6.1 kaydırmasız masaüstü: aegisub pencere yapısı — pencere
			// SABİT panellerden örüldü (tavan: menü+öçe+taşımegut; video;
			// ses; alt: düzen kutusu + durum; merkez: grid). Tüm pencereyi
			// saran ScrollArea::both kaldırıldı — kaydırma yalnız grid'in
			// satır-virtualizasyonunda (show_rows); aegisub kuralı.
			let tum_yuk = su.available_height();
			egui::Panel::top("masaustu-tavan").show_inside(su, |top| {
				let tema = tema_simdi(top);
				// P4.8.2: aegisub menü çubuğu en üstte
				self.menu_cubugu(top);
					top.horizontal(|h| {
						h.label("ölçek");
						if h.button("−").clicked() {
							self.zoom = (self.zoom - 0.1).max(0.35);
						}
						if h.button("+").clicked() {
							self.zoom = (self.zoom + 0.1).min(1.6);
						}
						if h.button("1.0×").clicked() {
							self.zoom = 1.0;
						}
						h.label(format!("{:.1}×", self.zoom));
						if h.button("Ayarlar").clicked() {
							self.ayar_win = true;
						}
						if h.button("Mobil görünüm").clicked() {
							self.duzen_sec(tema, Duzen::Mobil);
						}
					});
					top.add_space(4.0);
					// P4.8.2 toolbar: sık işlemler (menünün şortcutu)
					top.horizontal(|h| {
						if h.button("Kaydet").clicked() {
							if let Some(p) = self.save_path.clone() {
								self.save_to(&p);
							} else {
								self.note = "kayıt yolu verilmedi (VELNOSUB_SAVE)".into();
							}
						}
						if h.button("Yeniden aç").clicked() {
							if let Some(p) = self.path.clone() {
								if let Err(e) = self.load(&p) {
									self.note = format!("açılamadı: {e}");
								}
							}
						}
						if h.button("Aç video").clicked() {
							match self.uzanti_listele(&["mp4", "webm", "mkv"]).first() {
								Some(y) => {
									let yol = y.clone();
									self.video_ac(&yol);
								}
								None => {
									self.note = "ext filesDir'de video yok (rish ile kopyala)".into()
								}
							}
						}
						if h.button("Aç ses").clicked() {
							match self.uzanti_listele(&["mp3", "wav", "m4a", "ogg"]).first() {
								Some(y) => {
									let yol = y.clone();
									self.ses_ac(&yol);
								}
								None => {
									self.note = "ext filesDir'de ses yok (rish ile kopyala)".into()
								}
							}
						}
					});
					top.add_space(2.0);
			}); // tavan şeridi kapanır (menü+öçe+toolbar içeride)
			if self.video.is_some() {
				// Aegisub: video pane — genişliğe sığar, kalanın %44'ünü
				// aşmaz (üst panelin altına oturur; kaydırma olmaz)
				egui::Panel::top("masaustu-video")
				.size_range(egui::Rangef::new(20.0, (tum_yuk * 0.44).max(120.0)))
					.show_inside(su, |v| self.video_icerik(v, false));
			}
			if self.audio.is_some() {
				// Aegisub: dalga şeridi videonun altında SABİT
				egui::Panel::top("masaustu-ses")
				.size_range(egui::Rangef::new(20.0, (tum_yuk * 0.22).max(150.0)))
					.show_inside(su, |s| self.ses_icerik(s, false));
			}
			// İlk eklenen bottom en dış kenara oturur: durum EN ALTA
			egui::Panel::bottom("masaustu-durum")
				.show_inside(su, |d| self.durum_icerik(d, false));
			// Aegisub: düzen kutusu (subtitle edit) durumun üstünde SABİT
			egui::Panel::bottom("masaustu-duzen")
				.show_inside(su, |d| self.duzen_kutusu(d, false));
			// Merkez: stil/form şeritleri varsa grid'i sektirir; grid'in tek
			// kaydırma alanı show_rows'lu satır listesi
			egui::CentralPanel::default().show(su, |merkez| {
				if !self.stiller.is_empty() {
					self.stil_icerik(merkez, false);
				}
				if self.dialog.is_some() {
					self.form_icerik(merkez, false);
				}
				self.grid_icerik(merkez, false);
			});
		});
		self.diyaloglar(ui);
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

impl eframe::App for GridApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		// ölçek tabana döner: Mobil'e masaüstü zoom'u sızmaz; Masaustu'ya
		// her girişte ppp_base yeniden alınır (reset sonrası taban = taban)
		if self.duzen != Duzen::Masaustu {
			if let Some(b) = self.ppp_base.take() {
				ui.ctx().set_pixels_per_point(b);
				self.zoom = 0.8;
			}
		}
		match self.duzen {
			Duzen::Secim => {
				CentralPanel::default().show_inside(ui, |su| {
					self.secim_ekrani(su);
				});
			}
			Duzen::Mobil => self.mobil_cerceve(ui),
			Duzen::Masaustu => self.masaustu_cerceve(ui),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// İlk giriş kuralı ve seçim kaydı: dosya yoksa düzen seçimi sorulur
	/// (Ayar.duzen None), yazım sonra birebir okunur; bilinmeyen içerik
	/// yeniden sorar. Ekransız haberci — derleme ortamında GUI penceresi
	/// açılamaz; çizim kanıtı cihaz/APK bağlamına kalmıştır.
	#[test]
	fn ayar_dongusu() {
		let yol = std::env::temp_dir()
			.join(format!("velnosub-ui-test{}.json", std::process::id()));
		std::env::set_var("VELNOSUB_UI_JSON", &yol);
		let _ = std::fs::remove_file(&yol);
		let ay = ayar_yukle();
		assert!(ay.duzen.is_none() && ay.tema.is_none(), "dosya yokken seçim sorulmalı");
		ayar_yaz("mobil", "koyu").expect("ilk yazım");
		let ay = ayar_yukle();
		assert_eq!(ay.duzen.as_deref(), Some("mobil"));
		assert_eq!(ay.tema.as_deref(), Some("koyu"));
		ayar_yaz("masaustu", "acik").expect("düzen+tema yazımı");
		let ay = ayar_yukle();
		assert_eq!(ay.duzen.as_deref(), Some("masaustu"));
		assert_eq!(ay.tema.as_deref(), Some("acik"));
		std::fs::write(&yol, "{}").unwrap();
		let ay = ayar_yukle();
		assert!(ay.duzen.is_none() && ay.tema.is_none(), "bilinmeyen içerik yeniden sorar");
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
}
