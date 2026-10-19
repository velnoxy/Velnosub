// P4.1: bağlamanın nerede aranacağını ortamla söyle (varsayılan: host-check
// dizinindeki mobil koldan libvelnosub-abi.so).
fn main() {
	let dir = std::env::var("VELNOSUB_ABI_DIR").unwrap_or_else(|_| {
		"/root/velsub/aegisub-upstream/build-host-check/tools/cross".into()
	});
	println!("cargo:rustc-link-lib=dylib=velnosub-abi");
	println!("cargo:rustc-link-search=native={dir}");
}
