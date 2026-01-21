#!/bin/bash

# Make errors fatal, print commands
set -ex

# Move to the application's root
cd "$(dirname "$0")/../.."

# Store offline sources for dependencies (required to build on Flathub)
mkdir -p .cargo
cargo vendor --locked vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > .cargo/config.toml

# Install the required Flatpak runtime and SDK
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install flathub --user org.freedesktop.Sdk//23.08 -y
flatpak install flathub --user org.freedesktop.Platform//23.08 -y
flatpak install flathub --user org.freedesktop.Sdk.Extension.rust-stable//23.08 -y

# Build the Flathub package
rm -rf target/ # Don't copy all the planet into the Flatpak build dir
rm -rf repo/
flatpak-builder --install repo packaging/flatpak/re.fossplant.songrec.json --user -y
