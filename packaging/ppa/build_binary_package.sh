#!/bin/bash

# Print commands, exit on error
set -xe

# Go to script's directory
cd "$(dirname "$0")"

# Create a target directory for our new source package before we build it
temp_dir="$(mktemp -d)"

function cleanup_dirs {
    rm -rf "${temp_dir}"
}

trap cleanup_dirs INT TERM

rm -rf ../../target/ ../../vendor/ ../../.flatpak-builder ../flatpak/.flatpak-builder ../../repo

cp -ra ../../ "${temp_dir}/songrec-0.4.2"

cd "${temp_dir}/songrec-0.4.2"

mkdir -p .cargo
cargo vendor --locked vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > .cargo/config

# "dpkg-source" will destroy the ".gitignore" files from source archive anyway.
# Prevent "cargo" to check for their presence.
find vendor -name .cargo-checksum.json -exec sed -ri 's/"[^"]*?\.gitignore":"[^"]+?"[,\}]//g' '{}' \;

mv packaging/ppa/debian .

debuild -b

mv ../*.tar* ../../ || :
mv ../*.dsc* ../../ || :
mv ../*.deb* ../../ || :
mv ../*changes* ../../ || :
mv ../*build* ../../ || :
mv ../*source* ../../ || :

cleanup_dirs

echo 'Find your package in /tmp now'
