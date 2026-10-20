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

/// P4.2 uygulama durumu: tek açık tutamaç + satır önbelleği.
pub struct GridApp {
	handle: Option<c_int>,
	rows: Vec<Row>,
	path: Option<String>,
	save_path: Option<String>,
	note: String,
	video: Option<VideoSection>,
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
		};
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
		Ok(())
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
}

impl eframe::App for GridApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		// P4.3: video şeridi — üst panel (sanallaştırılmış grid'in üstünde)
		let mut v_hata: Option<String> = None;
		if let Some(v) = self.video.as_mut() {
			Panel::top("vpanel").show(ui, |ui| {
				if v.dok.is_none() {
					if let Err(e) = v.cizim_cek(ui.ctx()) {
						v_hata = Some(e);
					}
				}
				ui.horizontal(|ui| {
					if ui.button("◀ geri").clicked() && v.frame > 0 {
						v.frame -= 1;
						if let Err(e) = v.cizim_cek(ui.ctx()) {
							v_hata = Some(e);
						}
					}
					if ui.button("ileri ▶").clicked() && v.frame + 1 < v.kareler {
						v.frame += 1;
						if let Err(e) = v.cizim_cek(ui.ctx()) {
							v_hata = Some(e);
						}
					}
					ui.label(RichText::new(format!(
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
			});
		}
		if let Some(e) = v_hata {
			self.note = format!("video doku hatası: {e}");
		}

		Panel::bottom("velnosub_durum").show(ui, |ui| {
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
		});

		CentralPanel::default().show_inside(ui, |ui| {
			if self.rows.is_empty() {
				ui.centered_and_justified(|ui| {
					ui.label(RichText::new("dosya açılmadı").weak().size(18.0));
				});
				return;
			}
			// Düzenlenip Commit tetiklenecek satırlar; ödünç çatışmasını
			// çözmek için çerçeve sonunda işlenir (tek çerçevede tek satır).
			let mut duzenenler: Vec<usize> = Vec::new();
			let satir_h =
				ui.text_style_height(&egui::TextStyle::Body) * 2.4;
			ScrollArea::vertical()
				.auto_shrink(false)
				.show_rows(ui, satir_h, self.rows.len(), |ui, aralık| {
					for i in aralık {
						let Some(row) = self.rows.get_mut(i) else { continue };
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
				});
			for i in duzenenler {
				if let (Some(row), Some(h)) = (self.rows.get(i), self.handle) {
					match edit_line(h, i, &row.text.clone()) {
						Ok(()) => self.note = format!("satır {} işlendi", i + 1),
						Err(e) => self.note = format!("işleme hatası: {e}"),
					}
				}
			}
		});
	}
}
