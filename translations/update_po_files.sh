#!/bin/bash

set -ex # Exit on error, print commands

cd "$(dirname "$0")/.."

# See here for the source of most commands:
# https://stackoverflow.com/questions/55981602/internationalize-python-project-using-pygtk3-and-glade

# Dependencies:
# sudo apt install intltool gettext poedit

# Extract the translation strings present in the Glade file

intltool-extract --type="gettext/glade" src/gui/interface.ui

mv src/gui/interface.ui.h translations/

# Regenerate the base ".pot" (translation template) file

xgettext -kgettext -kN_ --c++ --from-code utf-8  -o translations/songrec.pot src/*.rs src/core/audio_controllers/*.rs src/core/*.rs src/core/fingerprinting/*.rs src/gui/*.rs src/gui/*/*.rs src/plugins/*.rs src/utils/*.rs translations/interface.ui.h

for locale in translations/locale/*; do
    msgmerge --no-fuzzy-matching --update ${locale}/LC_MESSAGES/songrec.po translations/songrec.pot
done

# Keep binary ".mo" files synched with the ".po" files,
# as needed, if a tool like "poedit" didn't already
# do it automatically

for locale in translations/locale/*; do
    msgfmt ${locale}/LC_MESSAGES/songrec.po -o ${locale}/LC_MESSAGES/songrec.mo
done
