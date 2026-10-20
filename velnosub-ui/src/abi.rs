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

struct Row {
	times: String,
	text: String,
}

/// P4.2 uygulama durumu: tek açık tutamaç + satır önbelleği.
pub struct GridApp {
	handle: Option<c_int>,
	rows: Vec<Row>,
	path: Option<String>,
	save_path: Option<String>,
	note: String,
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
		};
		if let Some(p) = open {
			if let Err(e) = app.load(&p) {
				app.note = format!("açılamadı: {e}");
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
}

impl eframe::App for GridApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
