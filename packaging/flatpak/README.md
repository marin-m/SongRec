# Flatpak package

This directory contains a script that will build a Flatpak package for `SongRec`. The entry point is `build.sh`.

## Build instructions

In order to get a recent Flatpak and Flatpak-Builder version with Ubuntu Bionic (Flatpak-Builder 0.x won't correctly parse our Flatpak manifest), be sure to run:

```
sudo add-apt-repository --yes --update ppa:alexlarsson/flatpak
sudo apt-get install flatpak-builder flatpak
```
Also install other build dependencies:

```bash
sudo apt install build-essential libasound2-dev libgtk-3-dev libssl-dev git cargo
```

Then, run:

```
./build.sh
```
