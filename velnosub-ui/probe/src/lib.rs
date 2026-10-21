//! velnosub-probe — P4.8.1 teşhis dili: YALNIZ eframe + NativeActivity.
//! abi/core/asset/filesDir YÖNETMEZ. Bu pencere açılmadıysa sorun
//! eframe/glue/EGL katmanındadır; açıldıysa sorun velnosub-ui'nin ek
//! yüklerindedir (libvelnosub-abi + core zinciri / asset kopyası).

pub mod temel {
	use eframe::egui;

	pub struct ProbeApp {
		pub tık: u64,
	}

	impl Default for ProbeApp {
		fn default() -> Self {
			Self { tık: 0 }
		}
	}

	impl eframe::App for ProbeApp {
		// eframe 0.37: App::ui içine CentralPanel zaten sarılır (show_inside
		// kullanan abi.rs'deki deprecated uyarının azdiri bu)
		fn ui(&mut self, ui: &mut egui::Ui, _ç: &mut eframe::Frame) {
			ui.heading("velnosub PROBE");
			ui.label("pencere açıldı — eframe + NativeActivity ÇALIŞIYOR");
			if ui.button("dokun (+1)").clicked() {
				self.tık += 1;
			}
			ui.label(format!("okçamlar: {}", self.tık));
		}
	}
}

/// Ortak koşum (host masorder + Android aynı kapı). open/save pros yok.
pub fn calistir(verbose: bool) -> Result<(), String> {
	let viewport = eframe::egui::ViewportBuilder::default()
		.with_inner_size([300.0, 240.0])
		.with_title("velnosub PROBE");
	let options = eframe::NativeOptions {
		viewport,
		..Default::default()
	};
	eframe::run_native(
		"velnosub-probe",
		options,
		Box::new(move |_cc| Ok(Box::new(temel::ProbeApp::default()))),
	)
	.map_err(|e| {
		let s = format!("{e}");
		if verbose {
			eprintln!("probe run_native HATA: {s}");
		}
		s
	})
}

#[cfg(target_os = "android")]
static BAS: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
extern "C" {
	fn __android_log_print(prio: i32, tag: *const std::os::raw::c_char, fmt: *const std::os::raw::c_char, ...) -> i32;
}

#[cfg(target_os = "android")]
unsafe fn künye(not: &str) {
	let tag = b"velnosub-probe\0".as_ptr().cast::<std::os::raw::c_char>();
	let fmt = b"%s\0".as_ptr().cast::<std::os::raw::c_char>();
	let mesaj = std::ffi::CString::new(not).unwrap_or_default();
	let _ = __android_log_print(4, tag, fmt, mesaj.as_ptr());
}

#[cfg(target_os = "android")]
fn not_yaz(not: &str) {
	unsafe { künye(not) };
}

// init_array: lib dlopen anındaki işaret — yoksa çökme linker katmanında
#[cfg(target_os = "android")]
#[used]
#[link_section = ".init_array"]
static YUKLENDI: extern "C" fn() = yuklendi_isareti;

#[cfg(target_os = "android")]
extern "C" fn yuklendi_isareti() {
	unsafe { künye("L0 libvelnosub_probe.so yuklendi (init_array)") };
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
	let _ = BAS.set(std::time::Instant::now());
	not_yaz("A0 android_main girdi");
	not_yaz(&format!("A1 sdk={}", android_activity::AndroidApp::sdk_version()));

	let viewport = eframe::egui::ViewportBuilder::default()
		.with_inner_size([300.0, 240.0])
		.with_title("velnosub PROBE");
	let mut options = eframe::NativeOptions {
		viewport,
		..Default::default()
	};
	// winit event loop'u AndroidApp'sız kurulamaz
	options.event_loop_builder = Some(Box::new(move |builder| {
		use winit::platform::android::EventLoopBuilderExtAndroid;
		builder.with_android_app(app);
	}));
	not_yaz("A2 options kuruldu — run_native çağrısı");

	match calistir(true) {
		Ok(()) => not_yaz("A3 run_native kapandı"),
		Err(e) => not_yaz(&format!("A3 run_native HATA: {e}")),
	}
}

#[cfg(not(target_os = "android"))]
fn main() {
	let _ = calistir(true);
}
