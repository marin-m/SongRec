#!/bin/bash

set -ex # Exit on error, print commands

cd "$(dirname "$0")"

# Dependencies:
# sudo apt install gettext sed

# Extract the translation strings present in the Glade file

./update_pot.sh

# Update .po templates based on .pot

for locale in $(cat translations/po/LINGUAS); do
    msgmerge --no-fuzzy-matching \
        --update po/${locale}.po \
        songrec.pot
    
    # Keep binary ".mo" files synced with the ".po" files,
    # as needed, if a tool like "poedit" didn't already
    # do it automatically

    mkdir -p locale/${locale}/LC_MESSAGES/
    msgfmt po/${locale}.po \
        -o locale/${locale}/LC_MESSAGES/songrec.mo
done
