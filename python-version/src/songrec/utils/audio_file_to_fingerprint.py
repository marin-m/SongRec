#!/usr/bin/env python3
# -*- encoding: Utf-8 -*-
from numpy import array as nparray, sin, pi, arange, concatenate
from os.path import dirname, realpath
from argparse import ArgumentParser
from pydub import AudioSegment

SCRIPT_DIR = dirname(realpath(__file__))
MODULE_DIR = dirname(realpath(SCRIPT_DIR))
SRC_DIR = dirname(realpath(MODULE_DIR))

import sys

sys.path.append(SRC_DIR)

from songrec.fingerprinting.signature_format import DecodedMessage
from songrec.fingerprinting.algorithm import SignatureGenerator

"""
    Sample usage: uv run ./audio_file_to_fingerprint.py ../tests/stupeflip.wav
"""


def main():
    args = ArgumentParser(
        description='Generate a Shazam fingerprint from a '
        + 'sound file, and print it to the standard output.'
    )

    args.add_argument(
        'input_file',
        help='The .WAV or .MP3 file to generate '
        + 'an audio fingerprint for.',
    )

    args = args.parse_args()

    audio = AudioSegment.from_file(args.input_file)

    audio = audio.set_sample_width(2)
    audio = audio.set_frame_rate(16000)
    audio = audio.set_channels(1)

    signature_generator = SignatureGenerator()
    signature_generator.feed_input(audio.get_array_of_samples())

    # Prefer starting at the middle at the song, and with a
    # substantial bit of music to provide.

    signature_generator.MAX_TIME_SECONDS = 12
    if audio.duration_seconds > 12 * 3:
        signature_generator.samples_processed += 16000 * (
            int(audio.duration_seconds / 2) - 6
        )

    print(signature_generator.get_next_signature().encode_to_uri())


if __name__ == '__main__':
    main()
