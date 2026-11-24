project(
    'songrec',
    'rust',
    version: run_command(
        find_program(
            meson.project_source_root() /
            'packaging/build-aux/meson-cargo-manifest.py'
        ),
        'package',
        'version',
        check: true,
    ).stdout(),
    meson_version: '>=0.57',
)

cargo_bin = find_program('cargo')

target_dir = meson.project_build_root() / 'target'
manifest = meson.project_source_root() / 'Cargo.toml'

bindir = get_option('prefix') / get_option('bindir')
etcdir = get_option('prefix') / get_option('sysconfdir')
usrdir = get_option('prefix') / get_option('datadir')

cargo_env = {
  'CARGO_HOME': meson.project_build_root() / 'cargo',
  'LOCALEDIR': get_option('prefix') / get_option('localedir'),
  'RUST_BACKTRACE': 'full'
}

cargo_options = [
  '--target-dir', target_dir,
  '--release',
  '--manifest-path', manifest,
]

i18n = import('i18n')
gnome = import('gnome')

# Cf. https://docs.flatpak.org/en/latest/flatpak-devel.html#buildsystem-handling

i18n.merge_file(
    input: 'packaging/freedesktop/re.fossplant.songrec.desktop',
    output: 're.fossplant.songrec.desktop',
    type: 'desktop',
    po_dir: 'translations/po',
    install: true,
    install_dir: usrdir / 'applications',
)

appstream_file = i18n.merge_file(
    input: 'packaging/freedesktop/re.fossplant.songrec.metainfo.xml',
    output: 're.fossplant.songrec.metainfo.xml',
    type: 'xml',
    po_dir: 'translations/po',
    install: true,
    install_dir: usrdir / 'metainfo',
)

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
    test(
        'Validate metainfo file',
        appstreamcli,
        args: ['validate', '--no-net', '--explain', appstream_file],
        workdir: meson.current_build_dir()
    )
endif

install_man(
    'packaging/manpages-autogen/songrec.1.gz',
    'packaging/manpages-autogen/songrec-audio-file-to-fingerprint.1.gz',
    'packaging/manpages-autogen/songrec-audio-file-to-recognized-song.1.gz',
    'packaging/manpages-autogen/songrec-fingerprint-to-recognized-song.1.gz',
    'packaging/manpages-autogen/songrec-gui.1.gz',
    'packaging/manpages-autogen/songrec-gui-norecording.1.gz',
    'packaging/manpages-autogen/songrec-listen.1.gz',
    'packaging/manpages-autogen/songrec-microphone-to-recognized-song.1.gz',
    'packaging/manpages-autogen/songrec-recognize.1.gz',
    install_dir: usrdir / 'share' / 'man' / 'man1'
)

dependency('gtk4', version: '>= 4.14')
dependency('libadwaita-1', version: '>= 1.8')
dependency('libsoup-3.0', version: '>= 3.4')

if host_machine.system() == 'linux'
    dependency('libpipewire-0.3')
    dependency('alsa')
    dependency('dbus-1')
    dependency('libpulse')
endif

# meson.add_dist_script('build-aux/meson-dist-vendor.sh')

# subdir('translations')

# XX call script for regenerating .po + .pot + .mo files
# XX add code for generating internationalized `.metainfo.xml` file
# XX add code for generating internationalized `.desktop` file
# XX add code for generating compressed `.1.gz` manpage files

cargo_build = custom_target(
    'cargo-build',
    build_by_default: true,
    build_always_stale: true,
    output: 'release',
    console: true,
    env: cargo_env,
    command: [
        cargo_bin,
        'build',
        cargo_options,
    ],
)

custom_target(
    'cp-binary',
    depends: cargo_build,
    build_by_default: true,
    build_always_stale: true,
    install: true,
    install_dir: bindir,
    output: meson.project_name(),
    command: [
        'cp',
        target_dir / 'release' / meson.project_name(),
        '@OUTPUT@',
    ],
)

test(
  'cargo-test',
  cargo_bin,
  args: [
    'test',
    cargo_options,
  ],
  protocol: 'rust',
  env: cargo_env,
  is_parallel: false,
  timeout: 1800,
)


# executable(
    # XX add code for generating and installing the
    # `songrec` Rust binary
# )
