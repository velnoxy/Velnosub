# Velsub Faz 2 (P2.2) — mobil CLI duman testi; velsub_core_so (kök meson,
# velsub_mobile kolu) burada bağlanır. GUI hedeflerini etkilemez.

velsub_cli = executable('velsub-cli', 'velsub-cli.cpp',
    link_with: velsub_core_so,
    include_directories: [libaegisub_inc, include_directories('..')],
    cpp_args: ['-DAGI_CORE'],
    install: true)
