# Windows cross-compilation

This files gives commands that seem to work in order to cross-compile this application for Windows under Linux.

The following references have been used:
* https://stackoverflow.com/questions/31492799/cross-compile-a-rust-application-from-linux-to-windows
* https://gtk-rs.org/docs-src/tutorial/cross
* https://github.com/qarmin/Instrukcje-i-Tutoriale/blob/master/GtkRsCross.md
* https://stackoverflow.com/questions/45444811/how-to-compiling-c-gtk3-program-in-linux-mint-for-windows

```console-session
sudo apt install mingw-w64-tools rpm2cpio binutils-mingw-w64-x86-64 wget zip libz-mingw-w64-dev win-iconv-mingw-w64-dev libgtk-3-dev p7zip-full wine64 upx-ucl

sudo mkdir /opt/gtkwin
cd /opt/gtkwin
sudo chown -R $USER:$USER .
rm -rf /opt/gtkwin/*
for pkg in mingw64-gtk3-3.24.23-1 mingw64-pango-1.44.7-3 mingw64-cairo-1.16.0-4 mingw64-harfbuzz-2.6.8-2 mingw64-glib2-2.64.3-2 mingw64-gdk-pixbuf-2.40.0-3 mingw64-atk-2.36.0-3 mingw64-harfbuzz-2.6.8-2 mingw64-libpng-1.6.37-4 mingw64-pixman-0.40.0-2 mingw64-pcre-8.43-4 mingw64-libffi-3.1-10 mingw64-libepoxy-1.5.4-3 mingw64-fribidi-1.0.10-2 mingw64-libjpeg-turbo-2.0.5-2 mingw64-libtiff-4.0.9-7 mingw64-freetype-2.10.2-2 mingw64-fontconfig-2.13.1-4 mingw64-expat-2.2.8-3 mingw64-librsvg2-2.40.19-8 mingw64-gtk-update-icon-cache-3.24.23-1 mingw64-adwaita-icon-theme-3.36.1-2 mingw64-hicolor-icon-theme-0.17-3 mingw64-bzip2-1.0.8-3; do
    wget https://download-ib01.fedoraproject.org/pub/fedora/linux/releases/33/Everything/x86_64/os/Packages/m/${pkg}.fc33.noarch.rpm
    rpm2cpio ${pkg}.fc33.noarch.rpm | cpio -idmv
done

for pkg in mingw64-gettext-0.21-2; do
    wget https://download-ib01.fedoraproject.org/pub/fedora/linux/releases/34/Everything/x86_64/os/Packages/m/${pkg}.fc34.noarch.rpm
    rpm2cpio ${pkg}.fc34.noarch.rpm | cpio -idmv
done

cd /opt/gtkwin
wget -nc http://www.angusj.com/resourcehacker/resource_hacker.zip
7z x -y -oresource_hacker/ resource_hacker.zip

cd /opt/gtkwin/usr/x86_64-w64-mingw32/sys-root/mingw/
find -name '*.pc' | while read pc; do sed -e "s@^prefix=.*@prefix=$PWD@" -i "$pc"; done

sudo apt remote --purge rustc cargo

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # Type "1"
# Login and reconnect to add Rust to the $PATH, or run:
source $HOME/.cargo/env

# If you already installed Rust, then update it:
rustup update

rustup target add x86_64-pc-windows-gnu
rustup toolchain install stable-x86_64-pc-windows-gnu

echo "[target.x86_64-pc-windows-gnu]" > ~/.cargo/config
echo "linker = \"x86_64-w64-mingw32-gcc\"" >> ~/.cargo/config
echo "ar = \"x86_64-w64-mingw32-gcc-ar\"" >> ~/.cargo/config

wget -nc https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full.7z -O /opt/gtkwin/ffmpeg-release-full.7z
7z -y x /opt/gtkwin/ffmpeg-release-full.7z -o/opt/gtkwin -i'r!*ffmpeg.exe'

cd ~/rust-shazam/
export PKG_CONFIG_ALLOW_CROSS=1
export GETTEXT_SYSTEM=1
export RUSTFLAGS="-L /opt/gtkwin/usr/x86_64-w64-mingw32/sys-root/mingw/lib"
export MINGW_PREFIX=/opt/gtkwin/usr/x86_64-w64-mingw32/sys-root/mingw
export PKG_CONFIG_PATH=/opt/gtkwin/usr/x86_64-w64-mingw32/sys-root/mingw/lib/pkgconfig
cargo build --target x86_64-pc-windows-gnu --release --no-default-features -F gui,ffmpeg

cd ~/rust-shazam/
GTK_APP=/tmp/windows_release
GTK_LIBRARY=/opt/gtkwin/usr/x86_64-w64-mingw32/sys-root/mingw
mkdir $GTK_APP
cp -r translations $GTK_APP
cp target/x86_64-pc-windows-gnu/release/songrec.exe $GTK_APP
cp $GTK_LIBRARY/bin/*.dll $GTK_APP
mkdir -p $GTK_APP/share/glib-2.0/schemas
mkdir $GTK_APP/share/icons
cp $GTK_LIBRARY/share/glib-2.0/schemas/* $GTK_APP/share/glib-2.0/schemas
glib-compile-schemas $GTK_APP/share/glib-2.0/schemas/
cp -r $GTK_LIBRARY/share/icons/Adwaita $GTK_APP/share/icons/
rm -rf $GTK_APP/share/icons/Adwaita/{256x256,512x512,96x96}
cp $GTK_LIBRARY/bin/gdbus.exe $GTK_LIBRARY/bin/gspawn*.exe $GTK_APP
cp /usr/x86_64-w64-mingw32/lib/*.dll /usr/x86_64-w64-mingw32/bin/*.dll /usr/lib/gcc/x86_64-w64-mingw32/*-win32/*.dll $GTK_APP
mkdir $GTK_APP/lib
cp /opt/gtkwin/ffmpeg-*-full_build/bin/ffmpeg.exe $GTK_APP/
upx --force $GTK_APP/songrec.exe $GTK_APP/ffmpeg.exe $GTK_APP/libgtk-3-0.dll
cp -r $GTK_LIBRARY/lib/gdk-pixbuf-2.0 $GTK_APP/lib

cd $GTK_APP
cp -r * ~/win32/windows_release/

# RUST_BACKTRACE=full wine songrec.exe

# Create a self-extracting and executing 7-Zip based
# archive (see https://stackoverflow.com/questions/27904532/how-do-i-make-a-self-extract-and-running-installer
# + https://github.com/phillipp/SevenZipSharp/blob/master/SevenZip/sfx/Configs.xml)

rm -rf /tmp/songrec-files.7z
7z -m0=Copy a /tmp/songrec-files.7z * # -m0=Copy = Do not compress (for self-extraction performance)
# From http://www.angusj.com/resourcehacker/: use this to add a custom .ICO file to the 7-Zip stub
wine /opt/gtkwin/resource_hacker/ResourceHacker.exe -open ~/rust-shazam/packaging/windows/7zxSD_LZMA2_x64.sfx -save /tmp/SongRec-standalone.exe -action delete -mask ,101, -log CONSOLE
wine /opt/gtkwin/resource_hacker/ResourceHacker.exe -open /tmp/SongRec-standalone.exe -save /tmp/SongRec-standalone.exe -action addoverwrite -res ~/rust-shazam/packaging/windows/songrec.ico -mask ICONGROUP,MAINICON,0 -log CONSOLE
cat << EOF >> /tmp/SongRec-standalone.exe
;!@Install@!UTF-8!
ExecuteFile="songrec.exe"
GUIMode="2"
;!@InstallEnd@!
EOF
cat /tmp/songrec-files.7z >> /tmp/SongRec-standalone.exe

cp /tmp/SongRec-standalone.exe ~/win32/ # Copy the generated executable to my VirtualBox shared folder

# wine /tmp/SongRec-standalone.exe

# zip -r /tmp/windows_release.zip $GTK_APP
```
