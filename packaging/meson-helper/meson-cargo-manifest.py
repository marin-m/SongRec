#!/usr/bin/env python3

try:
    import tomllib
except ImportError:
    # Incompatible with Python < 3.11
    from pip._vendor import tomli as tomllib
import sys

from os.path import dirname, realpath, join

SCRIPT_DIR = dirname(realpath(__file__))
PACKAGING_DIR = dirname(realpath(SCRIPT_DIR))
ROOT_DIR = dirname(realpath(PACKAGING_DIR))

with open(join(ROOT_DIR, 'Cargo.toml'), 'rb') as f:
    toml = tomllib.load(f)
    for arg in sys.argv[1:]:
        toml = toml[arg]
    print(toml, end='')
