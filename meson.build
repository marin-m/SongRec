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

gnome = import('gnome')

##################
## DEPENDENCIES ##
##################

dependency('gtk4', version: '>= 4.14')
dependency('libadwaita-1', version: '>= 1.8')
dependency('libsoup-3.0', version: '>= 3.4')

if host_machine.system() == 'linux'
    get_option('pipewire').enable_auto_if(
        dependency(
            'libpipewire-0.3',
            required: get_option('pipewire')
        ).found()
    )
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
install_subdir('translations/locale', install_dir: get_option('localedir'))

# meson.add_dist_script('meson-helper/meson-dist-vendor.sh')

gnome.post_install()

