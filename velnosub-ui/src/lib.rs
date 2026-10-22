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
		#[cfg(target_os = "android")]
		not_yaz(&format!("A3 run_native HATA: {e}"));
		eprintln!("eframe error: {e}");
	}
	#[cfg(target_os = "android")]
	not_yaz("A3 run_native kapandı");
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
static EXT_YOL: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
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

// breadcrumb: logd + filesDir + dış filesDir (termux'a "tüm dosyalar" izni
// verildiyse proot /mnt/sdcard/Android/data/com.velnoxy.velnosubui/files
// içinden doğrudan okunur — logcat erişilemese bile kanıt dışarı çıkar)
#[cfg(target_os = "android")]
fn not_yaz(not: &str) {
	use std::io::Write;
	unsafe { android_log(not) };
	let ms = BAS.get().map(|b| b.elapsed().as_millis()).unwrap_or(0);
	let satir = format!("[{ms}] {not}\n");
	for yol in [BOOT_YOL.get(), EXT_YOL.get()].into_iter().flatten() {
		let sonuc = std::fs::OpenOptions::new()
			.create(true)
			.append(true)
			.open(yol)
			.and_then(|mut f| f.write_all(satir.as_bytes()));
		if let Err(e) = sonuc {
			eprintln!("velnosub breadcrumb yazılamadı {}: {e}", yol.display());
		}
	}
}

// .init_array: lib dlopen anında (android_main'den önce) tetiklenir —
// "L0 lib yüklendi" işareti düşmezse çökme linker/DT_NEEDED katmanındadır
#[cfg(target_os = "android")]
#[used]
#[link_section = ".init_array"]
static LIB_YUKLENDI_ISARETI: extern "C" fn() = lib_yuklendi_isareti;

#[cfg(target_os = "android")]
extern "C" fn lib_yuklendi_isareti() {
	unsafe { android_log("L0 libvelnosub_ui.so yuklendi (init_array)") };
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
	if let Some(dis) = app.external_data_path() {
		// dış filesDir: /storage/emulated/0/Android/data/<pkg>/files —
		// termux "tüm dosyalar" izniyle proot üzerinde OKUNUR
		let _ = EXT_YOL.set(dis.join("velnosub-boot.txt"));
		// P4.8.2 N3: video/ses rish ile buraya konur; abi.rs menüsü bu
		// dizini tarar (uzanti_listele)
		std::env::set_var("VELNOSUB_EXTDIR", &dis);
	}
	// P5.5: kutu-sesi decode önbelleği (abi: audio_file_open → user-ses.pcm)
	std::env::set_var("VELNOSUB_CACHE", &baz);
	not_yaz("A0 android_main girdi");
	not_yaz(&format!(
		"A1 filesDir={} sdk={}",
		baz.display(),
		android_activity::AndroidApp::sdk_version()
	));
	// filesDir'e karaskel-dene.ass + DejaVuSans.ttf kopyala
	// (asset_manager: lib.rs:1030; ndk-0.9.0 asset.rs:47/225)
	not_yaz("A1b asset kopyası başlıyor");
	// asset yolları APK içinde assets/velnosub/ altında (aapt list kanıtı)
	varlik_yaz(&app, "velnosub/karaskel-dene.ass", &baz.join("karaskel-dene.ass"));
	not_yaz("A1c karaskel-dene.ass tamam");
	varlik_yaz(&app, "velnosub/DejaVuSans.ttf", &baz.join("DejaVuSans.ttf"));
	not_yaz("A1d DejaVuSans.ttf tamam");
	varlik_yaz(&app, "velnosub/default_config.json", &baz.join("default_config.json"));
	not_yaz("A1e default_config.json tamam");
	// abi.rs CWD-fallbacklerini abs yola bağla (Android'de cwd=/ = read-only);
	// fonts/config/ayar kayıt dosyaları filesDir'den okunur/yazılır
	std::env::set_var("VELNOSUB_CONFIG_JSON", baz.join("default_config.json"));
	std::env::set_var("VELNOSUB_UI_JSON", baz.join("velnosub-ui.json"));
	std::env::set_var("VELNOSUB_FONT", baz.join("DejaVuSans.ttf"));
	std::env::set_var("VELNOSUB_SAVE", baz.join("kayit.ass"));
	not_yaz("A1f VE env yolları set edildi");
	// eğri dosyalar: open=sabit deneme dosyası, save=Kayıt hedefi
	let open = Some(baz.join("karaskel-dene.ass").display().to_string());
	let save = Some(baz.join("kayit.ass").display().to_string());

	let viewport = ViewportBuilder::default()
		.with_inner_size([440.0, 320.0])
		.with_min_inner_size([300.0, 240.0])
		.with_title("velnosub");
	// eframe 0.37: android_app zorunlu alan — event-loop'u eframe kendisi
	// builder.with_android_app(native_options.android_app) ile kurar
	// (run.rs:47-56); bizim event_loop_builder hook'u artık çift uygulama
	// yapardı — kaldırıldı, alan doğrudan geçilir
	let mut options = eframe::NativeOptions {
		viewport,
		android_app: Some(app),
		..Default::default()
	};
	not_yaz("A2 event-loop kurulum tamam — run_native çağrılıyor");
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
			not_yaz(&format!("V0 asset manager'da yok: {ad}"));
			eprintln!("velnosub: asset yok: {ad}");
			return;
		}
	};
	if let Err(e) = sonuc {
		not_yaz(&format!(
			"V1 asset kopyası olmaz {ad} → {}: {e}",
			hedef.display()
		));
		eprintln!(
			"velnosub: asset kopyası olmaz {ad} → {}: {e}",
			hedef.display()
		);
	}
}
