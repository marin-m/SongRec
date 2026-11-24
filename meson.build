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

# etc = get_option('prefix') / get_option('sysconfdir')
share = get_option('prefix') / get_option('datadir')

dependency('gtk4', version: '>= 4.14')
dependency('libadwaita-1', version: '>= 1.8')
dependency('libsoup-3.0', version: '>= 3.4')

if host_machine.system() == 'linux'
    dependency('libpipewire-0.3')
    dependency('alsa')
    dependency('dbus-1')
    dependency('libpulse')
endif

subdir('src')
subdir('packaging/freedesktop')

install_subdir('packaging/icons', install_dir : share)
install_subdir('translations/locale', install_dir : share)

install_man(
    'packaging/manpages-autogen/songrec.1',
    'packaging/manpages-autogen/songrec-audio-file-to-fingerprint.1',
    'packaging/manpages-autogen/songrec-audio-file-to-recognized-song.1',
    'packaging/manpages-autogen/songrec-fingerprint-to-recognized-song.1',
    'packaging/manpages-autogen/songrec-gui.1',
    'packaging/manpages-autogen/songrec-gui-norecording.1',
    'packaging/manpages-autogen/songrec-listen.1',
    'packaging/manpages-autogen/songrec-microphone-to-recognized-song.1',
    'packaging/manpages-autogen/songrec-recognize.1',
    install_dir: share / 'man' / 'man1'
)

# meson.add_dist_script('build-aux/meson-dist-vendor.sh')

