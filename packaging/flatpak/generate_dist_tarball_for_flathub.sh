#!/bin/bash

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <github_tag>"
    exit 1
fi

# Make errors fatal, print commands
set -ex

cd "$(dirname "$0")"

REPO_DIR="$(git rev-parse --show-toplevel)"

rm -rf /tmp/dist_dir /tmp/songrec_tarball_"$1"_for_flathub_build.tar.gz

cp -a "${REPO_DIR}" /tmp/dist_dir

cd /tmp/dist_dir

rm -rf target/ vendor/ .flatpak-builder packaging/flatpak/.flatpak-builder repo .cargo


# Fetch dependency sources to be bundled with the applicaiton
mkdir -p .cargo
cargo vendor --locked vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > .cargo/config.toml

rm -rf .git packaging/ffmpeg/linux/

tar zcvf ../songrec_tarball_"$1"_for_flathub_build.tar.gz .
