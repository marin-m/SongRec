#!/bin/bash

# Make errors fatal, print commands
set -ex

# Move to the application's root
cd "$(dirname "$0")/../.."

# Store offline sources for dependencies (required to build on Flathub)
mkdir -p .cargo
cargo vendor --locked vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > .cargo/config

# Install the required Flatpak runtime and SDK
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install flathub --user org.freedesktop.Sdk//20.08 -y
flatpak install flathub --user org.freedesktop.Platform//20.08 -y
flatpak install flathub --user org.freedesktop.Sdk.Extension.rust-stable//20.08 -y

# Build the Flathub package
rm -rf target/ # Don't copy all the planet into the Flatpak build dir
rm -rf repo/
flatpak-builder --install repo packaging/flatpak/com.github.marinm.songrec.json --user -y
