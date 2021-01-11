project('oku',
        'rust',
        license: 'AGPL3.0+',
        version: '0.1.0')
dependency('gio-2.0')
dependency('glib-2.0')
dependency('gtk+-3.0')
dependency('pango')
dependency('webkit2gtk-4.0')
dependency('webkit2gtk-web-extension-4.0')
subdir('data')
subdir('src')
datadir = get_option('prefix') / get_option('datadir')

application_id = 'com.github.madebyemil.oku'
# Read more https://mesonbuild.com/Reference-manual.html#install_data

install_data(
    '@0@.desktop'.format(application_id),
    install_dir: datadir / 'applications'
)

install_data(
    '@0@.metainfo.xml'.format(application_id),
    install_dir: datadir / 'metainfo'
)

install_data(
    'hicolor',
    install_dir: datadir / 'icons'
)

install_data(
    'README',
    install_dir: datadir / 'doc' / 'oku'
)

install_data(
    'COPYING',
    install_dir: datadir / 'doc' / 'oku'
)

sources = [
    'main.rs',
]

cargo_script = find_program(join_paths(meson.source_root(), 'build-aux/cargo.sh'))
cargo_release = custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    cargo_script,
    meson.build_root(),
    meson.source_root(),
    '@OUTPUT@',
    get_option('buildtype'),
    meson.project_name(),
  ]
)

meson.add_dist_script(
  'build-aux/dist-vendor.sh',
  meson.build_root() / 'meson-dist' / meson.project_name() + '-' + version,
  meson.source_root()
)