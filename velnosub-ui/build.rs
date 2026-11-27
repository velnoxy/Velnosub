fn main() {
	let target = std::env::var("TARGET").expect("Cargo target");
	let mut document_io = cc::Build::new();
	document_io.cpp(true).file("src/document_io.cpp");
	if target.contains("android") {
		let linker = format!("CARGO_TARGET_{}_LINKER", target.to_ascii_uppercase().replace('-', "_"));
		println!("cargo:rerun-if-env-changed={linker}");
		if let Some(compiler) = std::env::var_os(linker) {
			let archiver = std::path::Path::new(&compiler).with_file_name("llvm-ar");
			document_io.compiler(compiler);
			if archiver.is_file() {
				document_io.archiver(archiver);
			}
		}
		document_io.cpp_link_stdlib("c++_shared");
	}
	println!("cargo:rerun-if-changed=src/document_io.cpp");
	document_io.compile("velnosub-document-io");
	let build = if target.starts_with("x86_64") && target.contains("android") {
		"build-android-x86_64"
	} else if target.contains("android") {
		"build-android-arm64"
	} else {
		"build-host-check"
	};
	let dir = std::env::var_os("VELNOSUB_ABI_DIR")
		.map(std::path::PathBuf::from)
		.unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("..").join(build).join("tools/cross"));
	println!("cargo:rerun-if-env-changed=VELNOSUB_ABI_DIR");
	println!("cargo:rustc-link-lib=dylib=velnosub-abi");
	println!("cargo:rustc-link-search=native={}", dir.display());
}
