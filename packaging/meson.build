share = get_option('datadir')

subdir('freedesktop')

install_subdir('icons', install_dir: share)

install_man(
    'manpages-autogen/songrec.1',
    'manpages-autogen/songrec-audio-file-to-fingerprint.1',
    'manpages-autogen/songrec-audio-file-to-recognized-song.1',
    'manpages-autogen/songrec-fingerprint-to-recognized-song.1',
    'manpages-autogen/songrec-gui.1',
    'manpages-autogen/songrec-gui-norecording.1',
    'manpages-autogen/songrec-listen.1',
    'manpages-autogen/songrec-microphone-to-recognized-song.1',
    'manpages-autogen/songrec-recognize.1'
)

# meson.add_dist_script('meson-helper/meson-dist-vendor.sh')

