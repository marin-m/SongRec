#!/bin/bash

# Build a NSIS installer from a self-extractible .EXE
# generated using the instructions in README.md

# Requirements:
# sudo apt install nsis

set -ex

cd "$(dirname "$0")"

makensis songrec.nsi
