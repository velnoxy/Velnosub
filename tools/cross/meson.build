# velnosub Faz 2 (P2.2) — mobil CLI duman testi; velnosub_core_so (kök meson,
# velnosub_mobile kolu) burada bağlanır. GUI hedeflerini etkilemez.

velnosub_cli = executable('velnosub-cli', 'velnosub-cli.cpp',
    link_with: velnosub_core_so,
    include_directories: [libaegisub_inc, deps_inc, version_inc, velnosub_src_inc],
    dependencies: [boost_dep],
    cpp_args: ['-DAGI_CORE'],
    install: true)
