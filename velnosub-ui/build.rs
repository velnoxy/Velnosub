// P4.2: ana crate'e de libvelnosub-abi bağlama hattı. Android hedefinde
// mobil-abi kolu (arm64) kütüphanesi aranır; diğer hedeflerde host-check dizini.
fn main() {
	let target = std::env::var("TARGET").unwrap_or_default();
	let dir = std::env::var("VELNOSUB_ABI_DIR").unwrap_or_else(|_| {
		if target.contains("android") {
			if target.contains("x86_64") {
				"/root/velsub/aegisub-upstream/build-android-x86_64/tools/cross".into()
			} else {
				"/root/velsub/aegisub-upstream/build-android-arm64/tools/cross".into()
			}
		} else {
			"/root/velsub/aegisub-upstream/build-host-check/tools/cross".into()
		}
	});
	println!("cargo:rustc-link-lib=dylib=velnosub-abi");
	println!("cargo:rustc-link-search=native={dir}");
}
