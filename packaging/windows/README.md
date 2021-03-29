# Windows cross-compilation

This files gives commands that seem to work in order to cross-compile this application for Windows under Linux.

The following references have been used:
* https://stackoverflow.com/questions/31492799/cross-compile-a-rust-application-from-linux-to-windows
* https://gtk-rs.org/docs-src/tutorial/cross
* https://github.com/qarmin/Instrukcje-i-Tutoriale/blob/master/GtkRsCross.md
* https://stackoverflow.com/questions/45444811/how-to-compiling-c-gtk3-program-in-linux-mint-for-windows

Note: It seems that the currently produced Windows build is mostly working, but decoding a MP3 file produces a crash, for example. Also, these are some command line warnings from the application about missing mime type resources, GSetting folder, etc. Even though the core functionality is working.

```console-session
sudo apt install mingw-w64-tools rpm2cpio binutils-mingw-w64-x86-64 wget zip libz-mingw-w64-dev win-iconv-mingw-w64-dev

sudo mkdir /opt/gtkwin
cd /opt/gtkwin
sudo chown -R $USER:$USER .
rm -rf /opt/gtkwin/*
for pkg in mingw64-gtk3-3.24.23-1 mingw64-pango-1.44.7-3 mingw64-cairo-1.16.0-4 mingw64-harfbuzz-2.6.8-2 mingw64-glib2-2.64.3-2 mingw64-gdk-pixbuf-2.40.0-3 mingw64-atk-2.36.0-3 mingw64-harfbuzz-2.6.8-2 mingw64-libpng-1.6.37-4 mingw64-pixman-0.40.0-2 mingw64-pcre-8.43-4 mingw64-gettext-0.20.2-3 mingw64-libffi-3.1-10 mingw64-libepoxy-1.5.4-3 mingw64-fribidi-1.0.10-2 mingw64-libjpeg-turbo-2.0.5-2 mingw64-libtiff-4.0.9-7 mingw64-freetype-2.10.2-2 mingw64-fontconfig-2.13.1-4 mingw64-expat-2.2.8-3 mingw64-bzip2-1.0.8-3; do
    wget https://download-ib01.fedoraproject.org/pub/fedora/linux/releases/33/Everything/x86_64/os/Packages/m/${pkg}.fc33.noarch.rpm
    rpm2cpio ${pkg}.fc33.noarch.rpm | cpio -idmv
done

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

cd ~/rust-shazam/
export PKG_CONFIG_ALLOW_CROSS=1
export RUSTFLAGS="-L /opt/gtkwin/usr/x86_64-w64-mingw32/usr/lib"
export MINGW_PREFIX=/opt/gtkwin/usr/x86_64-w64-mingw32
export PKG_CONFIG_PATH=/opt/gtkwin/usr/x86_64-w64-mingw32/sys-root/mingw/lib/pkgconfig
cargo build --target x86_64-pc-windows-gnu --release

cd ~/rust-shazam/
GTK_APP=/tmp/windows_release
GTK_LIBRARY=/opt/gtkwin/usr/x86_64-w64-mingw32/sys-root/mingw
mkdir $GTK_APP
cp target/x86_64-pc-windows-gnu/release/songrec.exe $GTK_APP
cp $GTK_LIBRARY/bin/*.dll $GTK_APP
mkdir -p $GTK_APP/share/glib-2.0/schemas
mkdir $GTK_APP/share/icons
cp $GTK_LIBRARY/share/glib-2.0/schemas/* $GTK_APP/share/glib-2.0/schemas
for theme in Adwaita hicolor gnome Tango; do cp -r /usr/share/icons/${theme} $GTK_APP/share/icons/; done
cp /usr/x86_64-w64-mingw32/lib/*.dll /usr/x86_64-w64-mingw32/bin/*.dll /usr/lib/gcc/x86_64-w64-mingw32/9.3-win32/*.dll $GTK_APP
mkdir $GTK_APP/lib
cp -r $GTK_LIBRARY/lib/gdk-pixbuf-2.0 $GTK_APP/lib

cd $GTK_APP
RUST_BACKTRACE=full wine songrec.exe

cp -r * ~/win32/windows_release/

zip -r /tmp/windows_release.zip $GTK_APP
```
