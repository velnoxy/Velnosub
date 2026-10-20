# velnosub — aegisub-core (Faz 1)
#
# Kaynak listesi (aegisub_core_src) src/meson.build içinde tanımlı; burada yalnızca
# hedef tanımlanır. wx'siz derlenir: bu hedefteki hiçbir dosya `#include <wx/…>`
# kullanamaz.

aegisub_core_deps = deps_core
aegisub_core_deps += dependency('icu-uc', version: '>= 4.8.1.1')
aegisub_core_deps += dependency('icu-i18n', version: '>= 4.8.1.1')

if conf.has('WITH_HUNSPELL')
    aegisub_core_deps += dependency('hunspell')
endif

if conf.has('WITH_FFMS2')
    aegisub_core_deps += dependency('ffms2', version: '>= 2.22')
endif

# P3.2: velnosub ffmpeg video sağlayıcı — sağlayıcı TU derlenirken .a kitaplık
# sembolleri de çekirdek arşivine taşınır (meson duruk transferi etmez) ve
# makro çekirdek derleme argümanıyla beslenir (acconf.pch yok).
aegisub_core_args = ['-DAGI_CORE']
if conf.has('WITH_VELNOSUB_FFMPEG')
    aegisub_core_deps += dependency('libavformat')
    aegisub_core_deps += dependency('libswscale')
    aegisub_core_deps += dependency('libavcodec')
    aegisub_core_deps += dependency('libavutil')
    aegisub_core_args += ['-DWITH_VELNOSUB_FFMPEG=1']
endif

# P1.4: subtitles_provider_libass ve MatroskaParser yolu.
# P2.2 (Faz 2): cross'ta sistem libass yoksa wrap fallback devreye girer
# (libass + freetype + harfbuzz + fribidi alt-projeleri).
aegisub_core_deps += dependency('libass', version: '>= 0.9.7',
                                fallback: ['libass', 'libass_dep'])

aegisub_core = static_library('aegisub-core', aegisub_core_src, acconf, version_h,
    include_directories: [libaegisub_inc, deps_inc, version_inc,
                          include_directories('..')],
    dependencies: aegisub_core_deps,
    cpp_args: aegisub_core_args,
    install: false)
