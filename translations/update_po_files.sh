#!/bin/bash

set -ex # Exit on error, print commands

cd "$(dirname "$0")/.."

# See here for the source of most commands:
# https://stackoverflow.com/questions/55981602/internationalize-python-project-using-pygtk3-and-glade

# Dependencies:
# sudo apt install gettext sed poedit

# Extract the translation strings present in the Glade file

# Regenerate the base ".pot" (translation template) file

xgettext --c++ -kgettext --from-code utf-8 -o translations/songrec.pot \
    src/*.rs src/core/audio_controllers/*.rs src/core/*.rs \
    src/core/fingerprinting/*.rs src/gui/*.rs src/gui/*/*.rs src/plugins/*.rs \
    src/utils/*.rs

xgettext --join-existing -L desktop -o translations/songrec.pot \
    packaging/freedesktop/re.fossplant.songrec.desktop.in

xgettext --join-existing -L glade -o translations/songrec.pot \
    src/gui/interface.ui

xgettext --join-existing --its translations/songrec.its -o translations/songrec.pot \
    packaging/freedesktop/re.fossplant.songrec.metainfo.xml.in

for locale in translations/locale/*; do
    msgmerge --no-fuzzy-matching --update ${locale}/LC_MESSAGES/songrec.po translations/songrec.pot
done

# Keep binary ".mo" files synched with the ".po" files,
# as needed, if a tool like "poedit" didn't already
# do it automatically

cp packaging/freedesktop/re.fossplant.songrec.metainfo.xml.in \
    packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml

cp packaging/freedesktop/re.fossplant.songrec.desktop.in \
    packaging/rootfs/usr/share/applications/re.fossplant.songrec.desktop

for locale in translations/locale/*; do
    msgfmt ${locale}/LC_MESSAGES/songrec.po -o ${locale}/LC_MESSAGES/songrec.mo

    msgfmt --xml ${locale}/LC_MESSAGES/songrec.po -l "$(basename ${locale})" \
        --template packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml \
        -o packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml.new

    mv packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml.new \
        packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml

    msgfmt --desktop ${locale}/LC_MESSAGES/songrec.po -l "$(basename ${locale})" \
        --template packaging/rootfs/usr/share/applications/re.fossplant.songrec.desktop \
        -o packaging/rootfs/usr/share/applications/re.fossplant.songrec.desktop.new

    mv packaging/rootfs/usr/share/applications/re.fossplant.songrec.desktop.new \
        packaging/rootfs/usr/share/applications/re.fossplant.songrec.desktop
done

sed -z 's/  <name xml:[^\n]*\n//g' \
    packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml > \
    packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml.new

mv packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml.new \
    packaging/rootfs/usr/share/metainfo/re.fossplant.songrec.metainfo.xml
