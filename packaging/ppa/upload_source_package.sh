#!/bin/bash

# Print commands, exit on error
set -xe

# Go to script's directory
cd "$(dirname "$0")"

ORIG_DIR="$(pwd)"

# Create a target directory for our new source package before we build it
temp_dir="$(mktemp -d)"

function cleanup_dirs {
    rm -rf "${temp_dir}"
}

trap cleanup_dirs INT TERM

for version in bionic focal jammy lunar mantic noble; do

    rm -rf ../../target/ ../../vendor/ ../../.flatpak-builder ../flatpak/.flatpak-builder ../../repo ../../.cargo

    cp -ra ../../ "${temp_dir}/songrec-0.4.2${version}"

    cd "${temp_dir}/songrec-0.4.2${version}"

    mkdir -p .cargo
    cargo vendor --locked vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > .cargo/config

    # "dpkg-source" will destroy the ".gitignore" files from source archive anyway.
    # Prevent "cargo" to check for their presence.
    find vendor -name .cargo-checksum.json -exec sed -ri 's/"[^"]*?\.gitignore":"[^"]+?"[,\}]//g' '{}' \;

    mv packaging/ppa/debian .
    
    sed -ri "s/\) bionic/${version}) ${version}/g" debian/changelog

    debuild -S -sa -k7BD68AA06BBE1BB41DB4D98E007F79B1496791FA

    rm -f /tmp/songrec*

    mv ../*.tar* ../../ || :
    mv ../*.dsc* ../../ || :
    mv ../*.deb* ../../ || :
    mv ../*changes* ../../ || :
    mv ../*build* ../../ || :
    mv ../*source* ../../ || :

    # Push to Launchpad

    dput ppa:marin-m/songrec "../../songrec_0.4.2${version}_source.changes"

    cd "${ORIG_DIR}"

    rm -rf "${temp_dir}/songrec-0.4.2${version}"

done

cleanup_dirs

echo 'Package successfully uploaded to Launchpad, find it in /tmp'
