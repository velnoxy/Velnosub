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
		Box::new(move |cc| {
			// kayıtlı tema başlangıçta uygulanır (koyu varsayılan);
			// kontrast tamiri egui'nin soluk metnini AA uyumlu yapar
			let ayar = abi::ayar_yukle();
			let tema = match ayar.tema.as_deref() {
				Some("acik") => abi::Tema::Acik,
				_ => abi::Tema::Koyu,
			};
			let visuals = match tema {
				abi::Tema::Koyu => egui::Visuals::dark(),
				abi::Tema::Acik => egui::Visuals::light(),
			};
			cc.egui_ctx.all_styles_mut(|st| {
				st.visuals = visuals.clone();
				abi::stil_tamir(st, tema);
			});
			Ok(Box::new(abi::GridApp::new(open, save)))
		}),
	) {
		eprintln!("eframe error: {e}");
	}
}
