#!/bin/bash
cd "$(dirname "$0")/../.."
sudo snap remove --purge songrec
rm -f songrec_*_amd64.snap
snapcraft pack && \
    sudo snap install --dangerous songrec_*_amd64.snap && \
    sleep 5 &&
    sudo snap connect songrec:audio-record :audio-record && \
    songrec
