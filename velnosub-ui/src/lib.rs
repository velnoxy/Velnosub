//! velnosub — arayüz kütüphanesi (P4.8 dilim 2).
//!
//! İki giriş: host masaüstünde `calistir_env` (bin), Android'de
//! `android_main` (cdylib, NativeActivity). Uygulama kurulumu (tema +
//! GridApp + run_native) ortak `kos` içinde; iki koşum da aynı kapıyı
//! kullanır, davranış birebir.

pub mod abi;

use eframe::egui;
use egui::ViewportBuilder;

/// VELNOSUB_OPEN / VELNOSUB_SAVE ortam değişkenlerinden koşum — host bin.
pub fn calistir_env() {
	let open = std::env::var("VELNOSUB_OPEN").ok();
	let save = std::env::var("VELNOSUB_SAVE").ok();
	calistir(open, save);
}

/// Açıkça yollu koşum: open altyazı yolu, save Kayıt hedefi.
pub fn calistir(open: Option<String>, save: Option<String>) {
	let viewport = ViewportBuilder::default()
		.with_inner_size([440.0, 320.0])
		.with_min_inner_size([300.0, 240.0])
		.with_title("velnosub");
	let options = eframe::NativeOptions {
		viewport,
		..Default::default()
	};
	kos(open, save, options);
}

/// Ortak tapa: run_native + AppCreation kurulumu (tema: ayar_yukle →
/// stil_tamir; GridApp). Host ve Android aynı yolu kullanır.
fn kos(
	open: Option<String>,
	save: Option<String>,
	options: eframe::NativeOptions,
) {
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

// Android girişi: ANativeActivity_onCreate glue'u android-activity'nin
// kendisinden gelir (native-activity feature, eframe zinciri üzerinden
// etkin); burada yalnız android_main sağlanır.
//
// P4.8.1 teşhis: filesDir/velnosub-boot.txt'e adım-adım breadcrumb —
// logcat erişilemezse de çökme noktası tek bakışta görülür.
#[cfg(target_os = "android")]
static BOOT_YOL: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
#[cfg(target_os = "android")]
static BAS: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

// liblog.so platform kütüphanesi: MAIN buffer'a yazar — çökme sonrası
// `logcat -d -s velnosub` ile okunur (crash buffer bu ortamda okunamıyor)
#[cfg(target_os = "android")]
extern "C" {
	fn __android_log_print(prio: i32, tag: *const std::os::raw::c_char, fmt: *const std::os::raw::c_char, ...) -> i32;
}

#[cfg(target_os = "android")]
unsafe fn android_log(not: &str) {
	let tag = b"velnosub\0".as_ptr().cast::<std::os::raw::c_char>();
	let fmt = b"%s\0".as_ptr().cast::<std::os::raw::c_char>();
	let mesaj = std::ffi::CString::new(not).unwrap_or_default();
	let _ = __android_log_print(4, tag, fmt, mesaj.as_ptr());
}

#[cfg(target_os = "android")]
fn not_yaz(not: &str) {
	use std::io::Write;
	unsafe { android_log(not) };
	let yol = match BOOT_YOL.get() {
		Some(y) => y,
		None => {
			eprintln!("velnosub-not-yaz (yol yok): {not}");
			return;
		}
	};
	let ms = BAS.get().map(|b| b.elapsed().as_millis()).unwrap_or(0);
	let satir = format!("[{ms}] {not}");
	let sonuc = std::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(yol)
		.and_then(|mut f| writeln!(f, "{satir}"));
	if let Err(e) = sonuc {
		eprintln!("velnosub breadcrumb yazılamadı: {e}");
	}
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
	// --- alpha kapsam (ruling 5): asset kopyası + tek örnek açılış ---
	// internal_data_path: android-activity-0.6.1/src/lib.rs:1272
	let _ = BAS.set(std::time::Instant::now());
	let baz = match app.internal_data_path() {
		Some(y) => y,
		None => {
			let mesaj = "internal_data_path yok — koşum atlandı";
			eprintln!("velnosub: {mesaj}");
			return;
		}
	};
	if BOOT_YOL.set(baz.join("velnosub-boot.txt")).is_ok() {
		std::panic::set_hook(Box::new(|info| {
			let nerede = info
				.location()
				.map(|l| format!("{}:{}", l.file(), l.line()))
				.unwrap_or_else(|| "?".to_string());
			not_yaz(&format!("PANIC {nerede}: {info}"));
		}));
	}
	not_yaz("A0 android_main girdi");
	not_yaz(&format!("A1 filesDir={}", baz.display()));
	// filesDir'e karaskel-dene.ass + DejaVuSans.ttf kopyala
	// (asset_manager: lib.rs:1030; ndk-0.9.0 asset.rs:47/225)
	varlik_yaz(&app, "karaskel-dene.ass", &baz.join("karaskel-dene.ass"));
	varlik_yaz(&app, "DejaVuSans.ttf", &baz.join("DejaVuSans.ttf"));
	// eğri dosyalar: open=sabit deneme dosyası, save=Kayıt hedefi
	let open = Some(baz.join("karaskel-dene.ass").display().to_string());
	let save = Some(baz.join("kayit.ass").display().to_string());

	let viewport = ViewportBuilder::default()
		.with_inner_size([440.0, 320.0])
		.with_min_inner_size([300.0, 240.0])
		.with_title("velnosub");
	let mut options = eframe::NativeOptions {
		viewport,
		..Default::default()
	};
	// winit'in event loop'u AndroidApp'sız kurulamaz; trait:
	// winit::platform::android::EventLoopBuilderExtAndroid
	// (with_android_app: winit-0.30.13/src/platform/android.rs:128)
	options.event_loop_builder = Some(Box::new(move |builder| {
		use winit::platform::android::EventLoopBuilderExtAndroid;
		builder.with_android_app(app);
	}));
	kos(open, save, options);
}

/// APK varlığı asset manager'dan okur filesDir'e yazar (mevcutsa üstüne).
#[cfg(target_os = "android")]
fn varlik_yaz(app: &android_activity::AndroidApp, ad: &str, hedef: &std::path::Path) {
	let ad_c = match std::ffi::CString::new(ad) {
		Ok(y) => y,
		Err(_) => return,
	};
	let sonuc = match app.asset_manager().open(ad_c.as_c_str()) {
		Some(mut ust) => ust
			.buffer()
			.and_then(|dizi| std::fs::write(hedef, dizi)),
		None => {
			eprintln!("velnosub: asset yok: {ad}");
			return;
		}
	};
	if let Err(e) = sonuc {
		eprintln!(
			"velnosub: asset kopyası olmaz {ad} → {}: {e}",
			hedef.display()
		);
	}
}
