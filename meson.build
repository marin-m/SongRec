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

i18n = import('i18n')
gnome = import('gnome')

dependency('gtk4', version: '>= 4.14')
dependency('libadwaita-1', version: '>= 1.8')
dependency('libsoup-3.0', version: '>= 3.4')

if host_machine.system() == 'linux'
    dependency('libpipewire-0.3')
    dependency('alsa')
    dependency('dbus-1')
    dependency('libpulse')
endif

bindir = get_option('prefix') / get_option('bindir')
etcdir = get_option('prefix') / get_option('sysconfdir')
usrdir = get_option('prefix') / get_option('datadir')

meson.add_dist_script('build-aux/meson-dist-vendor.sh')

subdir('translations')

# XX call script for regenerating .po + .pot + .mo files
# XX add code for generating internationalized `.metainfo.xml` file
# XX add code for generating internationalized `.desktop` file
# XX add code for generating compressed `.1.gz` manpage files

executable(
    # XX add code for generating and installing the
    # `songrec` Rust binary
)
