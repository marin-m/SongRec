#!/bin/bash

set -ex

cd "$(dirname "$0")/../.."

SRC="$(pwd)"

mkdir -p dist/
wget --no-verbose -nc http://www.angusj.com/resourcehacker/resource_hacker.zip -O dist/resource_hacker.zip
unzip -d dist/resource_hacker dist/resource_hacker.zip

export GETTEXT_SYSTEM=1
cargo build --release --no-default-features -F gui,ffmpeg

wget --no-verbose -nc https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full.7z -O dist/ffmpeg-release-full.7z
7z -y x dist/ffmpeg-release-full.7z -odist/ -i'r!*ffmpeg.exe'

cd "${SRC}"
GTK_APP=dist/windows_release
GTK_LIBRARY=/ucrt64
mkdir -p $GTK_APP
rm -rf $GTK_APP/*
cp -r translations $GTK_APP
cp target/release/songrec.exe $GTK_APP

cd $GTK_APP
cp $GTK_LIBRARY/bin/*.dll .
mkdir -p share/glib-2.0/schemas
mkdir -p share/locale
mkdir -p share/icons
cp $GTK_LIBRARY/share/glib-2.0/schemas/* share/glib-2.0/schemas
# cp -r $GTK_LIBRARY/share/locale/* share/locale
glib-compile-schemas share/glib-2.0/schemas/
cp -r $GTK_LIBRARY/share/icons/Adwaita share/icons/
rm -rf share/icons/Adwaita/{256x256,512x512,96x96}
cp $GTK_LIBRARY/bin/gdbus.exe $GTK_LIBRARY/bin/gspawn*.exe .
mkdir lib libexec
cp "${SRC}"/dist/ffmpeg-*-full_build/bin/ffmpeg.exe .
upx --force ffmpeg.exe libgtk-4-1.dll
cp -r $GTK_LIBRARY/lib/gdk-pixbuf-2.0 $GTK_LIBRARY/lib/gio lib
cp -r $GTK_LIBRARY/libexec/glib-* libexec
rm -f libLLVM* rustc_driver* std-*.dll
"${SRC}"/dist/resource_hacker/ResourceHacker.exe -open songrec.exe -save songrec.exe -action addoverwrite -res "${SRC}"/packaging/windows/songrec.ico -mask ICONGROUP,MAINICON,0 -log CONSOLE

# RUST_BACKTRACE=full songrec.exe

# Create a self-extracting and executing 7-Zip based
# archive (see https://stackoverflow.com/questions/27904532/how-do-i-make-a-self-extract-and-running-installer
# + https://github.com/phillipp/SevenZipSharp/blob/master/SevenZip/sfx/Configs.xml)

rm -rf "${SRC}"/dist/songrec-files.7z
7z a -m0=Copy "${SRC}"/dist/songrec-files.7z * # -m0=Copy = Do not compress (for self-extraction performance)

cd "${SRC}"/dist
# From http://www.angusj.com/resourcehacker/: use this to add a custom .ICO file to the 7-Zip stub
resource_hacker/ResourceHacker.exe -open ../packaging/windows/7zxSD_LZMA2_x64.sfx -save SongRec-standalone.exe -action delete -mask ,101, -log CONSOLE
resource_hacker/ResourceHacker.exe -open SongRec-standalone.exe -save SongRec-standalone.exe -action addoverwrite -res ../packaging/windows/songrec.ico -mask ICONGROUP,MAINICON,0 -log CONSOLE

cat << EOF >> SongRec-standalone.exe
;!@Install@!UTF-8!
ExecuteFile="songrec.exe"
GUIMode="2"
;!@InstallEnd@!
EOF
cat songrec-files.7z >> SongRec-standalone.exe
