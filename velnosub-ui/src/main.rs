//! velnosub — minimal eframe/egui bootstrap slice (Android aarch64).

use eframe::egui;
use egui::{CentralPanel, RichText, ViewportBuilder};

fn main() {
    let viewport = ViewportBuilder::default()
        .with_inner_size([440.0, 320.0])
        .with_min_inner_size([300.0, 240.0])
        .with_title("velnosub");
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "velnosub",
        options,
        Box::new(|_cc| Ok(Box::new(VelApp::default()))),
    ) {
        eprintln!("eframe error: {e}");
    }
}

#[derive(Default)]
struct VelApp;

impl eframe::App for VelApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ui, |ui| {
            ui.add_space(ui.available_height() * 0.35);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("velnosub").strong().size(48.0));
                ui.add_space(12.0);
                ui.label(format!("version {}", env!("CARGO_PKG_VERSION")));
            });
        });
    }
}
