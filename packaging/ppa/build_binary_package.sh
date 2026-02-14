#!/bin/bash

# Print commands, exit on error
set -xe

# Go to script's directory
cd "$(dirname "$0")"

PKGVER="$(grep -Po '(?<=\().+?(?=\))' debian/changelog | head -1)"

# Create a target directory for our new source package before we build it
temp_dir="$(mktemp -d)"

function cleanup_dirs {
    rm -rf "${temp_dir}"
}

trap cleanup_dirs INT TERM

cp -ra ../../ "${temp_dir}/songrec-${PKGVER}"

cd "${temp_dir}/songrec-${PKGVER}"

rm -rf target/ vendor/ .flatpak-builder packaging/flatpak/.flatpak-builder repo

mkdir -p .cargo
cargo vendor --locked vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > .cargo/config.toml

# "dpkg-source" will destroy the ".gitignore" files from source archive anyway.
# Prevent "cargo" to check for their presence.
find vendor -name .cargo-checksum.json -exec sed -ri 's/"[^"]*?\.gitignore":"[^"]+?"[,\}]//g' '{}' \;
find vendor -name .cargo-checksum.json -exec sed -ri 's/"[^"]*?\.orig":"[^"]+?"[,\}]//g' '{}' \;
find vendor -name .cargo-checksum.json -exec sed -ri 's/"[^"]*?\.a":"[^"]+?"[,\}]//g' '{}' \;
find vendor -name .cargo-checksum.json -exec sed -ri 's/"[^"]*?\.mailmap":"[^"]+?"[,\}]//g' '{}' \;

mv packaging/ppa/debian .

sed -ri "s/\) bionic/staging) resolute/g" debian/changelog

debuild -b

mv ../*.tar* ../../ || :
mv ../*.dsc* ../../ || :
mv ../*.deb* ../../ || :
mv ../*changes* ../../ || :
mv ../*build* ../../ || :
mv ../*source* ../../ || :

cleanup_dirs

echo 'Find your package in /tmp now'
