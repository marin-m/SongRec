# Flatpak package

This directory contains a script that will build a Flatpak package for `SongRec`. The entry point is `build.sh`.

## Build instructions

Build dependencies:

```
sudo apt install flatpak-builder blueprint-compiler flatpak build-essential \
    libpulse-dev libgtk-4-dev libsoup-3.0-dev libadwaita-1-dev libdbus-1-dev \
    libasound2-dev appstream git cargo
```

Then, run:

```
./build.sh
```
