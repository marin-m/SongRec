#!/usr/bin/python3
#-*- encoding: Utf-8 -*-
from numpy import array as nparray, sin, pi, arange, concatenate
from os.path import dirname, realpath
from argparse import ArgumentParser
from pydub import AudioSegment
from sys import stderr
from json import dumps

UTILS_DIR = realpath(dirname(__file__))

ROOT_DIR = realpath(UTILS_DIR + '/..')
FINGERPRINTING_DIR = realpath(ROOT_DIR + '/fingerprinting')

import sys
sys.path.append(FINGERPRINTING_DIR)

from communication import recognize_song_from_signature
from algorithm import SignatureGenerator

"""
    Sample usage: ./audio_file_to_fingerprint.py ../tests/stupeflip.wav
"""


if __name__ == '__main__':

    args = ArgumentParser(description = 'Generate a Shazam fingerprint from a ' +
        "sound file, perform song recognition towards Shazam's servers and " +
        'print obtained information to the standard output.')
    
    args.add_argument('input_file', help = 'The .WAV or .MP3 file to recognize.')
    
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
        signature_generator.samples_processed += 16000 * (int(audio.duration_seconds / 2) - 6)
    
    results = '(Not enough data)'
    
    while True:
        
        signature = signature_generator.get_next_signature()
        
        if not signature:
            print(dumps(results, indent = 4, ensure_ascii = False))
            break
        
        results = recognize_song_from_signature(signature)
        
        if results['matches']:
            print(dumps(results, indent = 4, ensure_ascii = False))
            break
        
        else:
            
            stderr.write(('[ Note: No matching songs for the first %g seconds, ' +
                'typing to recognize more input... ]\n') % (signature_generator.samples_processed / 16000))
            stderr.flush()
            
            # signature_generator.MAX_TIME_SECONDS = 12 # min(12, signature_generator.MAX_TIME_SECONDS + 3) # DEBUG
    
    
    
    
