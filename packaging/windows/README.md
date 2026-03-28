# Windows compilation

This files gives commands that seem to work in order to compile this application on Windows using MSYS/MINGW64 (WIP).

The following references have been used:
* https://devinsights.iblogger.org/msys2-environment-differences/
* https://stackoverflow.com/questions/31492799/cross-compile-a-rust-application-from-linux-to-windows
* https://gtk-rs.org/docs-src/tutorial/cross
* https://github.com/qarmin/Instrukcje-i-Tutoriale/blob/master/GtkRsCross.md
* https://stackoverflow.com/questions/45444811/how-to-compiling-c-gtk3-program-in-linux-mint-for-windows

```console-session
pacman -S mingw-w64-x86_64-git mingw-w64-x86_64-rust mingw-w64-x86_64-upx mingw-w64-x86_64-7zip openssh intltool curl git unzip mingw-w64-x86_64-gettext-runtime mingw-w64-x86_64-gcc mingw-w64-x86_64-libadwaita mingw-w64-x86_64-adwaita-icon-theme mingw-w64-x86_64-glib2 mingw-w64-x86_64-gtk4 mingw-w64-x86_64-pkgconf mingw-w64-x86_64-dbus mingw-w64-x86_64-openssl mingw-w64-x86_64-libsoup3 mingw-w64-x86_64-ffmpeg

cd /tmp
wget -nc http://www.angusj.com/resourcehacker/resource_hacker.zip
unzip -d /tmp/resource_hacker resource_hacker.zip

wget -nc https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full.7z -O /tmp/ffmpeg-release-full.7z
7z -y x /tmp/ffmpeg-release-full.7z -o/tmp -i'r!*ffmpeg.exe'

cd ~/SongRec-main/
export GETTEXT_SYSTEM=1
cargo build --release --no-default-features -F gui,ffmpeg

cd ~/SongRec-main/
GTK_APP=/tmp/windows_release
GTK_LIBRARY=/mingw64
mkdir $GTK_APP
cp -r translations $GTK_APP
cp target/release/songrec.exe $GTK_APP
cp $GTK_LIBRARY/bin/*.dll $GTK_APP
mkdir -p $GTK_APP/share/glib-2.0/schemas
mkdir $GTK_APP/share/icons
cp $GTK_LIBRARY/share/glib-2.0/schemas/* $GTK_APP/share/glib-2.0/schemas
glib-compile-schemas $GTK_APP/share/glib-2.0/schemas/
cp -r $GTK_LIBRARY/share/icons/Adwaita $GTK_APP/share/icons/
rm -rf $GTK_APP/share/icons/Adwaita/{256x256,512x512,96x96}
cp $GTK_LIBRARY/bin/gdbus.exe $GTK_LIBRARY/bin/gspawn*.exe $GTK_APP
mkdir $GTK_APP/lib
cp /tmp/ffmpeg-*-full_build/bin/ffmpeg.exe $GTK_APP/
upx --force $GTK_APP/songrec.exe $GTK_APP/ffmpeg.exe $GTK_APP/libgtk-4-1.dll
cp -r $GTK_LIBRARY/lib/gdk-pixbuf-2.0 $GTK_APP/lib

cd $GTK_APP
mkdir -p ~/windows_release/
cp -r * ~/windows_release/

# RUST_BACKTRACE=full songrec.exe

# Create a self-extracting and executing 7-Zip based
# archive (see https://stackoverflow.com/questions/27904532/how-do-i-make-a-self-extract-and-running-installer
# + https://github.com/phillipp/SevenZipSharp/blob/master/SevenZip/sfx/Configs.xml)

rm -rf /tmp/songrec-files.7z
rm -f libLLVM* rustc_driver*
7z a /tmp/songrec-files.7z * # -m0=Copy = Do not compress (for self-extraction performance)
# From http://www.angusj.com/resourcehacker/: use this to add a custom .ICO file to the 7-Zip stub
/tmp/resource_hacker/ResourceHacker.exe -open ~/SongRec-main/packaging/windows/7zxSD_LZMA2_x64.sfx -save /tmp/SongRec-standalone.exe -action delete -mask ,101, -log CONSOLE
/tmp/resource_hacker/ResourceHacker.exe -open /tmp/SongRec-standalone.exe -save /tmp/SongRec-standalone.exe -action addoverwrite -res ~/SongRec-main/packaging/windows/songrec.ico -mask ICONGROUP,MAINICON,0 -log CONSOLE
cd /tmp/
cat << EOF >> SongRec-standalone.exe
;!@Install@!UTF-8!
ExecuteFile="songrec.exe"
GUIMode="2"
;!@InstallEnd@!
EOF
cat /tmp/songrec-files.7z >> /tmp/SongRec-standalone.exe

# /tmp/SongRec-standalone.exe

# zip -r /tmp/windows_release.zip $GTK_APP
```
