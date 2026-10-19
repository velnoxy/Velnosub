# velnosub Faz 2 (P2.2/P2.3) — mobil CLI dumanları; velnosub_core_so (kök meson,
# velnosub_mobile kolu) burada bağlanır. GUI hedeflerini etkilemez.

velnosub_cli = executable('velnosub-cli', 'velnosub-cli.cpp',
    link_with: velnosub_core_so,
    include_directories: [libaegisub_inc, deps_inc, version_inc, velnosub_src_inc],
    dependencies: [boost_dep],
    cpp_args: ['-DAGI_CORE'],
    install: true)

# P2.3: libass kare render dumanı (ass_add_font + ass_render_frame; PPM çıktı).
velnosub_ass_render = executable('velnosub-ass-render', 'velnosub-ass-render.cpp',
    link_with: velnosub_core_so,
    include_directories: [libaegisub_inc, deps_inc, version_inc, velnosub_src_inc],
    dependencies: [boost_dep, velnosub_libass_dep],
    cpp_args: ['-DAGI_CORE'],
    install: true)

# P5.0: LuaJIT zemin dumanı. Bağlantı yolu derlemesine göre çözülür:
# native (host-check) → luajit-host; cross → deps prefix (arm64-v8a/x86_64).
# pkg-config KULLANILMIYOR: meson 1.1+ harici PKG_CONFIG_PATH'i yutmuyor,
# native-file pkg_config_libdir de bu konakta yeterli olmadı (p41-host3 kaydı);
# baş-üstü yolu burada ele veriyoruz. bulunamazsa hedef bilinçli atlanır.
if not meson.is_cross_build()
    luajit_prefix = '/root/velsub/ndk-deps/luajit-host'
else
    if target_machine.cpu_family() == 'aarch64'
        luajit_prefix = '/root/velsub/ndk-deps/arm64-v8a'
    else
        luajit_prefix = '/root/velsub/ndk-deps/x86_64'
    endif
endif
luajit_inc = luajit_prefix / 'include/luajit-2.1'
luajit_lib = cc.find_library('luajit-5.1',
    dirs: [luajit_prefix / 'lib'], required: false)
if cxx.has_header('lua.hpp', args: ['-I' + luajit_inc]) \
   and luajit_lib.found()
    velnosub_automation = executable('velnosub-automation', 'velnosub-automation.cpp',
        link_with: velnosub_core_so,
        include_directories: [libaegisub_inc, deps_inc, version_inc, velnosub_src_inc,
                              include_directories(luajit_inc)],
        dependencies: [boost_dep, luajit_lib],
        cpp_args: ['-DAGI_CORE'],
        install: true)
endif

# P4.1: C ABI şeridi — Rust/egui (velnosub-ui) tarafının bağlanacağı
# extern "C" yüzey; akış P2.2 CLI'ın koşuma geçmiş tek kanalıdır.
velnosub_abi = shared_library('velnosub-abi', 'velnosub-abi.cpp',
    link_with: velnosub_core_so,
    include_directories: [libaegisub_inc, deps_inc, version_inc, velnosub_src_inc],
    dependencies: [boost_dep],
    cpp_args: ['-DAGI_CORE'],
    install: true)
