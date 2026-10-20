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

struct Row {
	times: String,
	text: String,
}

/// P4.3: açılı video kolu ve önizleme dokusu durumu.
pub struct VideoSection {
	handle: c_int,
	w: usize,
	hgt: usize,
	kareler: i32,
	frame: i32,
	info: String,
	dok: Option<egui::TextureHandle>,
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
pub struct GridApp {
	handle: Option<c_int>,
	rows: Vec<Row>,
	path: Option<String>,
	save_path: Option<String>,
	note: String,
	video: Option<VideoSection>,
	audio: Option<AudioSection>,
	stiller: Vec<StyleRow>,
	dialog: Option<DialogSection>,
	// P4.6 dilim 2: arayüz düzeni — ilk girişte seçim ekranı (`Secim`),
	// seçim ayar dosyasına yazılır (`velnosub-ui.json`; VELNOSUB_UI_JSON
	// ortam değişkeniyle yol geçilebilir), Ayarlar bölümünden değiştirilir
	duzen: Duzen,
	aktif: Sekme,
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

/// Dosyadaki `"ui"` değerini okur; dosya yoksa ya da biçim bilinmiyorsa None.
fn ayar_yukle() -> Option<String> {
	let icerik = std::fs::read_to_string(ayar_yolu()).ok()?;
	if icerik.contains("\"mobil\"") {
		Some("mobil".into())
	} else if icerik.contains("\"masaustu\"") {
		Some("masaustu".into())
	} else {
		None
	}
}

fn ayar_yaz(duzen: &str) -> std::io::Result<()> {
	std::fs::write(ayar_yolu(), format!("{{ \"ui\": \"{duzen}\" }}"))
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
			stiller: Vec::new(),
			dialog: None,
			duzen: Duzen::Secim,
			aktif: Sekme::Altyazi,
		};
		match ayar_yukle().as_deref() {
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
		let mut rows = Vec::with_capacity(ev as usize);
		for i in 0..ev {
			let times = line_times(h, i)
				.unwrap_or_else(|e| format!("(zaman hatası: {e})"));
			let text = line_text(h, i).unwrap_or_else(|e| format!("(hata: {e})"));
			rows.push(Row { times, text });
		}
		self.handle = Some(h);
		self.rows = rows;
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
			kareler,
			frame: 0,
			info: format!("fps:{fps:.3} {bilgi}"),
			dok: None,
		});
		self.note = format!("video açıldı: {w}x{hgt}, {kareler} kare");
		Ok(())
	}

	/// P4.4: ses kolu aç; tüm aralığın peak kovalarını tek geçişte çek.
	fn load_audio(&mut self, ender: &str) -> Result<(), String> {
		if let Some(a) = self.audio.take() {
			unsafe { velnosub_abi_audio_close(a.handle) };
		}
		let yolu = CString::new(ender.to_string()).map_err(|e| e.to_string())?;
		let h = unsafe { velnosub_abi_audio_open(yolu.as_ptr(), 48000, 2) };
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

	fn video_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		// P4.3: video şeridi — sanallaştırılmış grid'in üstünde; hata üstüne note
		let mut v_hata: Option<String> = None;
		if let Some(v) = self.video.as_mut() {
			if v.dok.is_none() {
				if let Err(e) = v.cizim_cek(ui.ctx()) {
					v_hata = Some(e);
				}
			}
			let mut geri = egui::Button::new("◀ geri");
			let mut ileri = egui::Button::new("ileri ▶");
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
				su.label(RichText::new(format!(
					"kare {}/{}",
					v.frame + 1, v.kareler
				)).weak());
			});
			if let Some(sac) = &v.dok {
				ui.image((
					sac.id(),
					egui::vec2(v.w as f32, v.hgt as f32),
				));
			}
			ui.label(RichText::new(&v.info).weak());
			ui.add_space(4.0);
		}
		if let Some(e) = v_hata {
			self.note = format!("video doku hatası: {e}");
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

	fn grid_icerik(&mut self, ui: &mut egui::Ui, mobil: bool) {
		if self.rows.is_empty() {
			ui.centered_and_justified(|ui| {
				ui.label(RichText::new("dosya açılmadı").weak().size(18.0));
			});
			return;
		}
		// Düzenlenip Commit tetiklenecek satırlar; ödünç çatışmasını
		// çözmek için çerçeve sonunda işlenir (tek çerçevede tek satır).
		let mut duzenenler: Vec<usize> = Vec::new();
		// mobil: satır dokunma yüksekliği en az 44pt (3.2x gövde yüksekliği)
		let satir_h = ui.text_style_height(&egui::TextStyle::Body)
			* if mobil { 3.2 } else { 2.4 };
		ScrollArea::vertical()
			.auto_shrink(false)
			.show_rows(ui, satir_h, self.rows.len(), |ui, aralık| {
				for i in aralık {
					let Some(row) = self.rows.get_mut(i) else { continue };
					if mobil {
						// telefon genişliğinde reflow: zaman üstte, metin tam genişlik
						ui.vertical(|ui| {
							ui.label(
								RichText::new(format!("{}", &row.times))
									.weak()
									.color(Color32::GRAY),
							);
							let w = ui.available_width();
							let resp = TextEdit::singleline(&mut row.text)
								.desired_width(w)
								.show(ui)
								.response;
							if resp.changed() {
								duzenenler.push(i);
							}
						});
					} else {
						ui.horizontal(|ui| {
							ui.label(
								RichText::new(format!("{}  ", &row.times))
									.weak()
									.color(Color32::GRAY),
							);
							let w = ui.available_width() * 0.62;
							let resp = TextEdit::singleline(&mut row.text)
								.desired_width(w)
								.show(ui)
								.response;
							if resp.changed() {
								duzenenler.push(i);
							}
						});
					}
				}
			});
		for i in duzenenler {
			if let (Some(row), Some(h)) = (self.rows.get(i), self.handle) {
				match edit_line(h, i, &row.text.clone()) {
					Ok(()) => self.note = format!("satır {} işlendi", i + 1),
					Err(e) => self.note = format!("işleme hatası: {e}"),
				}
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
			self.duzen_sec(Duzen::Mobil);
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
			self.duzen_sec(Duzen::Masaustu);
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

	fn duzen_sec(&mut self, duzen: Duzen) {
		let kayit = match duzen {
			Duzen::Mobil => "mobil",
			Duzen::Masaustu => "masaustu",
			Duzen::Secim => return,
		};
		match ayar_yaz(kayit) {
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
		ui.add_space(8.0);
		ui.label(RichText::new("Arayüz düzeni").strong());
		ui.add_space(4.0);
		let gen = ui.available_width().min(360.0);
		if ui.add(
			egui::Button::new(RichText::new("Mobil UI").size(18.0))
				.min_size(egui::vec2(gen, 44.0))
				.selected(self.duzen == Duzen::Mobil),
		).clicked() {
			self.duzen_sec(Duzen::Mobil);
			return;
		}
		if ui.add(
			egui::Button::new(RichText::new("Masaüstü UI").size(18.0))
				.min_size(egui::vec2(gen, 44.0))
				.selected(self.duzen == Duzen::Masaustu),
		).clicked() {
			self.duzen_sec(Duzen::Masaustu);
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
			// taşma olmaz (satır yatay kaydırma yok)
			let adet = etiketler.len().max(1) as f32;
			let gen = ((su.available_width() - 8.0) / adet).max(48.0);
			su.horizontal(|k| {
				for (s, ad) in &etiketler {
					if k.add(
						egui::Button::new(RichText::new(*ad))
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
	}

	/// Masaüstü çerçeve: mevcut P4.2–P4.6 şerit akışı (üst paneller +
	/// alt durum çubuğu + grid), gövdeler mobil ile aynı.
	fn masaustu_cerceve(&mut self, ui: &mut egui::Ui) {
		if self.video.is_some() {
			Panel::top("vpanel").show(ui, |su| {
				self.video_icerik(su, false);
			});
		}
		if self.audio.is_some() {
			Panel::top("wpanel").show(ui, |su| {
				self.ses_icerik(su, false);
			});
		}
		if !self.stiller.is_empty() {
			Panel::top("spanel").show(ui, |su| {
				self.stil_icerik(su, false);
			});
		}
		if self.dialog.is_some() {
			Panel::top("dpanel").show(ui, |su| {
				self.form_icerik(su, false);
			});
		}
		Panel::bottom("velnosub_durum").show(ui, |su| {
			self.durum_icerik(su, false);
		});
		CentralPanel::default().show_inside(ui, |su| {
			self.grid_icerik(su, false);
		});
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

	/// İlk giriş kuralı ve seçim kaydı: dosya yoksa `None` (seçim ekranı),
	/// yazımdan sonra değer birebir okunur. Ekransız haberci (derleme ortamı
	/// GUI penceresi açamaz; düzen çizimi cihaz/APK bağlamına kaldı).
	#[test]
	fn ayar_yoksa_secim_kaydi_birebir() {
		let yol = std::env::temp_dir()
			.join(format!("velnosub-ui-test{}.json", std::process::id()));
		std::env::set_var("VELNOSUB_UI_JSON", &yol);
		let _ = std::fs::remove_file(&yol);
		assert_eq!(ayar_yukle(), None, "dosya yokken secim ekranı beklenirdi");
		ayar_yaz("mobil").expect("mobil ayarı yazılamadı");
		assert_eq!(ayar_yukle().as_deref(), Some("mobil"));
		ayar_yaz("masaustu").expect("masaustu yazılamadı");
		assert_eq!(ayar_yukle().as_deref(), Some("masaustu"));
		// bilinmeyen içerik (eskimiş dosya) seçimsizdir; tekrar sorar
		std::fs::write(&yol, "{}").unwrap();
		assert_eq!(ayar_yukle(), None);
		let _ = std::fs::remove_file(&yol);
	}
}
