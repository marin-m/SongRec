#!/bin/bash

set -ex # Exit on error, print commands

cd "$(dirname "$0")"

# See here for the source of most commands:
# https://stackoverflow.com/questions/55981602/internationalize-python-project-using-pygtk3-and-glade

# Dependencies:
# sudo apt install intltool gettext poedit

# Extract the translation strings present in the Glade file

intltool-extract --type="gettext/glade" ../src/gui/interface.glade
intltool-extract --type="gettext/glade" ../src/gui/favorites_interface.glade

mv ../src/gui/interface.glade.h .
mv ../src/gui/favorites_interface.glade.h .

# Regenerate the base ".pot" (translation template) file

xgettext -kgettext -kN_ --c++ --from-code utf-8  -o songrec.pot ../src/*.rs ../src/audio_controllers/*.rs ../src/core/*.rs ../src/fingerprinting/*.rs ../src/gui/*.rs ../src/utils/*.rs interface.glade.h favorites_interface.glade.h

for locale in fr_FR nl it pl es ja ca de_DE ko_KR; do
    msgmerge --no-fuzzy-matching --update ${locale}/LC_MESSAGES/songrec.po songrec.pot
done

# Keep binary ".mo" files synched with the ".po" files,
# as needed, if a tool like "poedit" didn't already
# do it automatically

for locale in fr_FR nl it pl es ja ca de_DE ko_KR; do
    msgfmt ${locale}/LC_MESSAGES/songrec.po -o ${locale}/LC_MESSAGES/songrec.mo
done
