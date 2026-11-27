project('lpeg', 'c', version: '1.1.0', default_options: ['c_std=c99'])

lua = dependency('luajit')
lpeg = static_library('velnosub-lpeg',
    'lpcap.c', 'lpcode.c', 'lpcset.c', 'lpprint.c', 'lptree.c', 'lpvm.c',
    dependencies: lua,
    pic: true,
)
lpeg_dep = declare_dependency(link_with: lpeg)
