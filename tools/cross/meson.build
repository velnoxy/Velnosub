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

# P5.0: LuaJIT zemin dumanı. luajit.pc (build-luajit.sh) bulunursa hedef
# tanımlanır; bulunmazsa bilinçli olarak atlanır (desktop'a sızma yok).
luajit_dep = dependency('luajit', required: false)
if luajit_dep.found()
    velnosub_automation = executable('velnosub-automation', 'velnosub-automation.cpp',
        link_with: velnosub_core_so,
        include_directories: [libaegisub_inc, deps_inc, version_inc, velnosub_src_inc],
        dependencies: [boost_dep, luajit_dep],
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
