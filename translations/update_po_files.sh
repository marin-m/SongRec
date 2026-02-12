#!/bin/bash

set -ex # Exit on error, print commands

cd "$(dirname "$0")"

# See here for the source of most commands:
# https://stackoverflow.com/questions/55981602/internationalize-python-project-using-pygtk3-and-glade

# Dependencies:
# sudo apt install intltool gettext poedit

# Extract the translation strings present in the Glade file

intltool-extract --type="gettext/glade" ../src/gui/interface.ui
intltool-extract --type="gettext/glade" ../src/gui/favorites_interface.ui

mv ../src/gui/interface.ui.h .
mv ../src/gui/favorites_interface.ui.h .

# Regenerate the base ".pot" (translation template) file

xgettext -kgettext -kN_ --c++ --from-code utf-8  -o songrec.pot ../src/*.rs ../src/audio_controllers/*.rs ../src/core/*.rs ../src/fingerprinting/*.rs ../src/gui/*.rs ../src/utils/*.rs interface.ui.h favorites_interface.ui.h

for locale in fr_FR nl it pl es ja ca de_DE ko_KR sk_SK ru pt_BR cs_CZ; do
    msgmerge --no-fuzzy-matching --update locale/${locale}/LC_MESSAGES/songrec.po songrec.pot
done

# Keep binary ".mo" files synched with the ".po" files,
# as needed, if a tool like "poedit" didn't already
# do it automatically

for locale in fr_FR nl it pl es ja ca de_DE ko_KR sk_SK pt_BR cs_CZ; do
    msgfmt locale/${locale}/LC_MESSAGES/songrec.po -o locale/${locale}/LC_MESSAGES/songrec.mo
done
