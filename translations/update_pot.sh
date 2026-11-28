#!/bin/bash

set -ex # Exit on error, print commands

cd "$(dirname "$0")/.."

# Dependencies:
# sudo apt install gettext sed

# Extract the translation strings present in the Glade file

# Regenerate the base ".pot" (translation template) file

xgettext --c++ -kgettext -kpgettext --from-code utf-8 -o translations/songrec.pot.new \
    src/*.rs src/core/audio_controllers/*.rs src/core/*.rs \
    src/core/fingerprinting/*.rs src/gui/*.rs src/gui/*/*.rs src/plugins/*.rs \
    src/utils/*.rs \

xgettext --join-existing -L desktop -o translations/songrec.pot.new \
    -k -kComment -kKeywords \
    packaging/freedesktop/re.fossplant.songrec.desktop

xgettext --join-existing --from-code=UTF-8 --add-comments \
    --keyword=_ --keyword=C_:1c,2 -o translations/songrec.pot.new \
    src/gui/*.blp

xgettext --join-existing --its translations/songrec.its -o translations/songrec.pot.new \
    packaging/freedesktop/re.fossplant.songrec.metainfo.xml

newPotDate=`grep "POT-Creation-Date:" translations/songrec.pot.new | sed 's/.\{3\}$//'`
oldPotDate=`grep "POT-Creation-Date:" translations/songrec.pot | sed 's/.\{3\}$//'`
sed -i 's/'"${newPotDate}"'/'"${oldPotDate}"'/' "translations/songrec.pot.new"
set +x
changes=`diff "translations/songrec.pot" "translations/songrec.pot.new" || true`
if [ ! -z "$changes" ]; then
    set -x
    sed -i 's/'"${oldPotDate}"'/'"${newPotDate}"'/' "translations/songrec.pot.new"
    mv "translations/songrec.pot.new" "translations/songrec.pot"
else
    set -x
    rm "translations/songrec.pot.new"
fi
