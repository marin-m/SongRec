#!/bin/bash

# Make errors fatal, print commands
set -ex

# Move to the application's root
cd "$(dirname "$0")/../.."

# Store offline sources for dependencies (required to build on Flathub)
# mkdir -p .cargo
# cargo vendor --locked vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > .cargo/config.toml

# Install the required Flatpak runtime and SDK
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install flathub org.gnome.Sdk//51 --user -y
flatpak install flathub org.gnome.Platform//51 --user -y
flatpak install flathub org.freedesktop.Sdk.Extension.rust-stable//25.08 --user -y
flatpak install flathub org.freedesktop.Sdk.Extension.llvm22//25.08 --user -y

# Build the Flathub package
rm -rf target/ # Don't copy all the planet into the Flatpak build dir
rm -rf repo/
PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig/ flatpak-builder --install repo packaging/flatpak/re.fossplant.songrec.json --user -y
