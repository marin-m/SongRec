project(
    'songrec',
    'rust',
    version: run_command(
        find_program(
            meson.project_source_root() /
            'packaging' /
            'meson-helper' /
            'meson-cargo-manifest.py'
        ),
        'package',
        'version',
        check: true,
    ).stdout(),
    meson_version: '>=1.1',
)

share = get_option('prefix') / get_option('datadir')
# etc = get_option('prefix') / get_option('sysconfdir')

##################
## DEPENDENCIES ##
##################

dependency('gtk4', version: '>= 4.14')
dependency('libadwaita-1', version: '>= 1.8')
dependency('libsoup-3.0', version: '>= 3.4')

if host_machine.system() == 'linux'
    dependency('libpipewire-0.3')
    dependency('alsa')
    dependency('dbus-1')
    dependency('libpulse')
endif

############
## SOURCE ##
############

subdir('src')

###############
## RESOURCES ##
###############

subdir('packaging')
install_subdir('translations/locale', install_dir: share)

# meson.add_dist_script('meson-helper/meson-dist-vendor.sh')

