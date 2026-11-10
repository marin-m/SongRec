# Windows compilation

This files gives commands intended to compile this application on Windows using MSYS/UCRTW64.

The following references have been used:
* https://devinsights.iblogger.org/msys2-environment-differences/
* https://geopjr.dev/blog/compiling-crystal-gtk4-apps-for-windows
* https://stackoverflow.com/questions/31492799/cross-compile-a-rust-application-from-linux-to-windows
* https://gtk-rs.org/docs-src/tutorial/cross
* https://github.com/qarmin/Instrukcje-i-Tutoriale/blob/master/GtkRsCross.md
* https://stackoverflow.com/questions/45444811/how-to-compiling-c-gtk3-program-in-linux-mint-for-windows

In PowerShell, with MSYS2 already installed:

```powershell
winget install --id Git.Git -e --source winget
# (restart PowerShell)
cd C:\msys64\home\*
git clone https://github.com/marin-m/SongRec.git SongRec-main

# Then, follow instructions for MSYS2 below
```

In MSYS2 UCRT64:

```bash
pacman -Sy mingw-w64-ucrt-x86_64-git mingw-w64-ucrt-x86_64-rust mingw-w64-ucrt-x86_64-blueprint-compiler mingw-w64-ucrt-x86_64-upx mingw-w64-ucrt-x86_64-7zip unzip mingw-w64-ucrt-x86_64-gettext-runtime mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-libadwaita mingw-w64-ucrt-x86_64-adwaita-icon-theme mingw-w64-ucrt-x86_64-glib2 mingw-w64-ucrt-x86_64-gtk4 mingw-w64-ucrt-x86_64-pkgconf mingw-w64-ucrt-x86_64-dbus mingw-w64-ucrt-x86_64-openssl mingw-w64-ucrt-x86_64-libsoup3 mingw-w64-ucrt-x86_64-nsis

# Clone repo if you didn't do it outside of MSYS2 already
git clone https://github.com/marin-m/SongRec.git SongRec-main
cd SongRec-main

./packaging/windows/build_msys2.sh # Build standalone binary + archive for installer

./dist/SongRec-standalone.exe # Test standalone binary

./packaging/windows/build_nsis.sh # Build installer

./dist/SongRec-installer.exe # Test installer
```

The Windows Store MSIX bundle can then be updated.
