#!/usr/bin/python3
#-*- encoding: Utf-8 -*-
from numpy import array as nparray, sin, pi, arange, concatenate
from os.path import dirname, realpath
from argparse import ArgumentParser
from sounddevice import play, wait
from pydub import AudioSegment
from json import dumps

UTILS_DIR = realpath(dirname(__file__))

ROOT_DIR = realpath(UTILS_DIR + '/..')
FINGERPRINTING_DIR = realpath(ROOT_DIR + '/fingerprinting')

import sys
sys.path.append(FINGERPRINTING_DIR)

from signature_format import DecodedMessage

"""
    Sample usage: ./fingerprint_to_sound.py 'data:audio/vnd.shazam.sig;base64,gCX+ynzKnegoBQAAAJwRlAAAAAAAAAAAAAAAAAAAABgAAAAAAAAAAACGAQAAAHwAAAAAQCgFAABAAANgeAAAACdVdK4MAT15RAgN3XWPCjsqeFUPHjR5JQ4VQXh5CR6/dF0OS3h2zg0MvHaHD1FneFgMEtdmRQ4PZ2z4DhF2dZIMNHZwTQ834XZIDAOkaYwQBqpqLA8wVHKtDCWfczEPNGF90AwZXnmqCRPEejgJBY18xA8nGnDzDEEAA2ByAQAAGVlxRRkJinLDGgazd8QQAbN0aCsBOm5iJQoXbXwWBQRsBRMDyHY6Hh9fc88QBt1qaSgIdXeLIQUGdgcmBRxwsRYBRnU8Hgo7anIsBBJxpSQCNXT5FR9ja1MdAkt2iBIK+3QwJgZTdWwZDNFq0xUMG3OqIQNOeDMtJzhv6R4AaG4wKwVDd84XAb50UScDi24/HgllbRgZDz5tliwFWXIDFQB+cQkjITltLRQDpHYcGAEIc8QnAQluqB8SIHMFGRE3bmocBiVxuCoIw3HOFgCZdD4tIoB0ER4VSWzWJQ/CbYQcAYhyvCoKKnv/GAksbc4eJ/d9PSUBg3nrGAFxcSstLeJwkSYKF3QrHgAyb/olDzRvOCAAB3k2LRPhcYIsASlz/RIA0G9MJBgWcB0cAc5wjyUGCnNhGQSVd/EmA9B0uhoIl3IYLQE4cFImFZJ1RR4GonRPKAHBeIgUFWlsEB8II4AKEQiSdpAhFAtpLB4BZmSFKQAAQgADYIsBAAAMA3IWPAWTcoo2DiRwiE0BwnDpMgZecVNLAfpxkUMAVnQJSALmb88zCqd03jcABHLvTgZDb7w/ANJw+08T+3ISPhKGcH9DD6VzyzIA9XKaRTV3bnIvATRysjwC5XG7OAmNcT8yBEBwhD8PL3R/MwDNcBttEMRy7DImfHvBMwb2dL87Dl91Jm5O/XTIZADKcwVrAQJzEl4Oa3RdVgEIeck9ATR6+UkGrnbvVAMPdRpQAQV0ZTgB+3bHMRSEeMU6Ez5+ckoAM324VgUFfYg8BZ90r0Qf7H8zMgPWdXE2Aeh7kU8BfHgAQwHxe6dLAHl4SlgIEX9rPg6denRGE+l5+0oCr4KWMQWxebRLDax/cDwB93mESRN5e0BMAq6Cbj8JMoR7PgBbenNXA858zkoI5H36SA43et5FE/h50TERoHcbTwH4eNg0AdZ3TToL1HeaOACEeNQ8APqBNEEBqXc/TgZ8ebBNATeDvUBbKWuJPAHhYTBiBbdtDUsBkmW0XgICdL44AB1r/FQSeWu7OgBDAANghgEAAAv0aDh4ACtneoYBcGbUjwA0Zy6dALtoQqca4mfRcgHUZhSGAepqEKQKZmp6cADXZwabAeRlqpEFLmq3cVYWbv5wFHZof3cAimZphwANZjOeAU9rQKcIv2WscSX0aMCcABRq0J8APWZVrQFzaoCFAKNsh5gB6myJdgzCaQJ7DwJwWosAJWqQjwCnb5KSAPptCZwB4G56fwZ9bhmVDp9rU3cWBW3DrgcgazCbBkFt3ZIT+Wm5qA2zaXJ+ALVpF5YA8mnBmQFKaz+KAGZs65ESWmxvhACIafeqAbZqQHEAA20ffADaao+kAc5q+XQAHGpGkMLkYkylDMlrBHIWUmRUdgMxZWWPA8tr2a4J8mY2eQD4ai2bBAdq0ZITDGhLhgBUZl2eAFhng6gCfGLIgQtKZ4xwGc1j5ZsBK2QBeQFQanV0AfRjsaEBeGItiQEDZeF8DS9kDHoSe2UsdwFtZU5+CmFpLngOZGQ4hw0rXOeOAJ1gmagB4F4DmwEJYQKACANd6J4VuFJtrAAA'
"""


if __name__ == '__main__':

    args = ArgumentParser(description = 'Convert a data-URI Shazam fingerprint ' +
        'into readable hearable tones, played back instantly (or written to a ' +
        'file, if a path is provided). Not particularly useful, but gives the ' +
        'simplest output that will trick Shazam into  recognizing a non-song.')
    
    args.add_argument('fingerprint', help = 'The data-URI Shazam fingerprint ' +
        'to convert into hearable sound.')
    
    args.add_argument('output_file', nargs = '?', help = 'File path of the ' +
        '.WAV or .MP3 file to write tones to, or nothing to play back the ' +
        'sound instantly.')
    
    args = args.parse_args()
    
    message = DecodedMessage.decode_from_uri(args.fingerprint)
    
    peak_duration = 0.128 # In seconds
    sound_duration = message.number_samples / message.sample_rate_hz
    
    output_sample_rate = 44100
    output_array = nparray([0] * int(sound_duration * output_sample_rate))
    
    for frequency_band, sound_peaks in sorted(message.frequency_band_to_sound_peaks.items()):
        
        for sound_peak in sound_peaks:
            
            if sound_peak.get_seconds() + peak_duration <= sound_duration and sound_peak.get_frequency_hz() > 400:
                
                start_offset = int(sound_peak.get_seconds() * output_sample_rate)
                end_offset = int(start_offset + peak_duration * output_sample_rate)
                
                peak_samples = sound_peak.get_amplitude_pcm() * sin(2 * pi * sound_peak.get_frequency_hz() * (arange(end_offset - start_offset) / output_sample_rate))
                
                output_array = concatenate([output_array[:start_offset], output_array[start_offset:end_offset] + peak_samples, output_array[end_offset:]])
    
    if not args.output_file:
    
        play(output_array / (2 ** 15), samplerate = output_sample_rate)
        wait()
    
    else:
        
        AudioSegment(
            data = output_array.astype('int16'),
            sample_width = 2,
            frame_rate = output_sample_rate,
            channels = 1
        ).export(args.output_file)
    
