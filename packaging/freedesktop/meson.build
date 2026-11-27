i18n = import('i18n')

share = get_option('datadir')

# Cf. https://docs.flatpak.org/en/latest/flatpak-devel.html#buildsystem-handling

i18n.merge_file(
    input: 're.fossplant.songrec.desktop',
    output: 're.fossplant.songrec.desktop',
    type: 'desktop',
    po_dir: meson.project_source_root() / 'translations/po',
    install: true,
    install_dir: share / 'applications',
)

appstream_file = i18n.merge_file(
    input: 're.fossplant.songrec.metainfo.xml',
    output: 're.fossplant.songrec.metainfo.xml',
    type: 'xml',
    po_dir: meson.project_source_root() / 'translations/po',
    install: true,
    install_dir: share / 'metainfo',
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
