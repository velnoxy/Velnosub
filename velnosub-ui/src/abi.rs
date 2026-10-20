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
		if let Some(s) = std::env::var("VELNOSUB_AUDIO").ok() {
			if let Err(e) = app.load_audio(&s) {
				app.note = format!("ses açılamadı: {e}");
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

		// P4.4: ses şeridi — oynatma tuşları + dalga formu (peak kovalarından)
		if let Some(a) = self.audio.as_mut() {
			Panel::top("wpanel").show(ui, |ui| {
				ui.add_space(4.0);
				ui.horizontal(|ui| {
					if ui.button("▶ oynat").clicked() {
						let adet = a.toplam.min((a.rate as i64) * 5);
						let rc = unsafe { velnosub_abi_audio_play(a.handle, 0, adet) };
						a.oynatma_denendi = true;
						a.ses_durum = if rc == 0 {
							"oynatma istendi".into()
						} else {
							format!("oynatma olmadı: {}", last_error())
						};
					}
					if ui.button("⏹ kes").clicked() {
						unsafe { velnosub_abi_audio_stop(a.handle) };
						a.ses_durum = "kesildi".into();
					}
					ui.label(RichText::new(&a.info).weak());
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
			});
		}

		// P4.5: stiller şeridi — ad/font özeti + canlı boyut düzenleme
		if !self.stiller.is_empty() {
			Panel::top("spanel").show(ui, |ui| {
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
						ui.label(RichText::new(format!(
							"{} — {} · p{} s{} o{} k{} · bi{} hiza{} ken[{}] kod{} ölç[{}] {}",
							r.name, r.font, r.primary, r.secondary, r.outline,
							r.shadow, r.flags, r.alignment, r.margins,
							r.encoding, r.scale, r.edge
						))
						.weak());
					});
				}
				ui.add_space(4.0);
			});
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
