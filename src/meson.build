cargo_bin = find_program('cargo')

bindir = get_option('prefix') / get_option('bindir')

cargo_env = {
  'CARGO_HOME': meson.project_build_root() / 'cargo',
  'LOCALEDIR': get_option('prefix') / get_option('localedir'),
  'RUST_BACKTRACE': 'full'
}

target_dir = meson.project_build_root() / 'target'
manifest = meson.project_source_root() / 'Cargo.toml'

options = []
if get_option('gui').enabled()
    options += ['gui']
endif
if get_option('ffmpeg').enabled()
    options += ['ffmpeg']
endif
if host_machine.system() == 'linux'
    options += ['pulse', 'mpris']
    if get_option('pipewire').enabled()
        options += ['pipewire']
    endif
endif

feature_list = ','.join(options)

cargo_options = [
  '--target-dir', target_dir,
  '--release',
  '--manifest-path', manifest,
  '--no-default-features',
  '--features', feature_list
]

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

