//! velnosub — eframe/egui bootstrap + P4.2 altyazı grid'i (Android aarch64).
//!
//! Açılış: VELNOSUB_OPEN ortam değişkeni bir altyazı yolunu gösterirse grid
//! o dosyayla açılır; VELNOSUB_SAVE Kaydet tuşunun hedefidir.
//! (Cihazda gerçek dosya seçici sonraki dilimlere kalır.)

mod abi;

use eframe::egui;
use egui::ViewportBuilder;

fn main() {
	let viewport = ViewportBuilder::default()
		.with_inner_size([440.0, 320.0])
		.with_min_inner_size([300.0, 240.0])
		.with_title("velnosub");
	let options = eframe::NativeOptions {
		viewport,
		..Default::default()
	};

	let open = std::env::var("VELNOSUB_OPEN").ok();
	let save = std::env::var("VELNOSUB_SAVE").ok();
	if let Err(e) = eframe::run_native(
		"velnosub",
		options,
		Box::new(move |_cc| Ok(Box::new(abi::GridApp::new(open, save)))),
	) {
		eprintln!("eframe error: {e}");
	}
}
