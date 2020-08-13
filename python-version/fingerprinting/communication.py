#!/usr/bin/python3
#-*- encoding: Utf-8 -*-
from uuid import uuid5, getnode, NAMESPACE_DNS, NAMESPACE_URL
from random import seed, random, choice
from pytz import all_timezones
from requests import get, post
from base64 import b64encode
from locale import getlocale
from json import dumps
from time import time

from signature_format import DecodedMessage
from user_agent import USER_AGENTS

locale = (getlocale()[0] or 'en_US').split('.')[0]

first_uuid = str(uuid5(NAMESPACE_DNS, str(getnode()))).upper()
second_uuid = str(uuid5(NAMESPACE_URL, str(getnode())))

def recognize_song_from_signature(signature : DecodedMessage) -> dict:

    # Même si Macron ne veut pas, nous on est là

    fuzz = random() * 15.3 - 7.65

    seed(getnode())

    return post('https://amp.shazam.com/discovery/v5/fr/FR/android/-/tag/' + first_uuid + '/' + second_uuid, params = {
        'sync': 'true',
        'webv3': 'true',
        'sampling': 'true',
        'connected': '',
        'shazamapiversion': 'v3',
        'sharehub': 'true',
        'video': 'v3'
    }, headers = {
        'Content-Type': 'application/json',
        'User-Agent': choice(USER_AGENTS),
        'Content-Language': locale
    }, json = {
        "geolocation": {
            "altitude": random() * 400 + 100 + fuzz,
            "latitude": random() * 180 - 90 + fuzz,
            "longitude": random() * 360 - 180 + fuzz
        },
        "signature": {
            "samplems": int(signature.number_samples / signature.sample_rate_hz * 1000),
            "timestamp": int(time() * 1000),
            "uri": signature.encode_to_uri()
        },
        "timestamp": int(time() * 1000),
        "timezone": choice([timezone for timezone in all_timezones if 'Europe/' in timezone])
    }).json()


if __name__ == '__main__':
    
    print(dumps(recognize_song_from_signature(DecodedMessage.decode_from_uri('data:audio/vnd.shazam.sig;base64,gCX+ynzKnegoBQAAAJwRlAAAAAAAAAAAAAAAAAAAABgAAAAAAAAAAACGAQAAAHwAAAAAQCgFAABAAANgeAAAACdVdK4MAT15RAgN3XWPCjsqeFUPHjR5JQ4VQXh5CR6/dF0OS3h2zg0MvHaHD1FneFgMEtdmRQ4PZ2z4DhF2dZIMNHZwTQ834XZIDAOkaYwQBqpqLA8wVHKtDCWfczEPNGF90AwZXnmqCRPEejgJBY18xA8nGnDzDEEAA2ByAQAAGVlxRRkJinLDGgazd8QQAbN0aCsBOm5iJQoXbXwWBQRsBRMDyHY6Hh9fc88QBt1qaSgIdXeLIQUGdgcmBRxwsRYBRnU8Hgo7anIsBBJxpSQCNXT5FR9ja1MdAkt2iBIK+3QwJgZTdWwZDNFq0xUMG3OqIQNOeDMtJzhv6R4AaG4wKwVDd84XAb50UScDi24/HgllbRgZDz5tliwFWXIDFQB+cQkjITltLRQDpHYcGAEIc8QnAQluqB8SIHMFGRE3bmocBiVxuCoIw3HOFgCZdD4tIoB0ER4VSWzWJQ/CbYQcAYhyvCoKKnv/GAksbc4eJ/d9PSUBg3nrGAFxcSstLeJwkSYKF3QrHgAyb/olDzRvOCAAB3k2LRPhcYIsASlz/RIA0G9MJBgWcB0cAc5wjyUGCnNhGQSVd/EmA9B0uhoIl3IYLQE4cFImFZJ1RR4GonRPKAHBeIgUFWlsEB8II4AKEQiSdpAhFAtpLB4BZmSFKQAAQgADYIsBAAAMA3IWPAWTcoo2DiRwiE0BwnDpMgZecVNLAfpxkUMAVnQJSALmb88zCqd03jcABHLvTgZDb7w/ANJw+08T+3ISPhKGcH9DD6VzyzIA9XKaRTV3bnIvATRysjwC5XG7OAmNcT8yBEBwhD8PL3R/MwDNcBttEMRy7DImfHvBMwb2dL87Dl91Jm5O/XTIZADKcwVrAQJzEl4Oa3RdVgEIeck9ATR6+UkGrnbvVAMPdRpQAQV0ZTgB+3bHMRSEeMU6Ez5+ckoAM324VgUFfYg8BZ90r0Qf7H8zMgPWdXE2Aeh7kU8BfHgAQwHxe6dLAHl4SlgIEX9rPg6denRGE+l5+0oCr4KWMQWxebRLDax/cDwB93mESRN5e0BMAq6Cbj8JMoR7PgBbenNXA858zkoI5H36SA43et5FE/h50TERoHcbTwH4eNg0AdZ3TToL1HeaOACEeNQ8APqBNEEBqXc/TgZ8ebBNATeDvUBbKWuJPAHhYTBiBbdtDUsBkmW0XgICdL44AB1r/FQSeWu7OgBDAANghgEAAAv0aDh4ACtneoYBcGbUjwA0Zy6dALtoQqca4mfRcgHUZhSGAepqEKQKZmp6cADXZwabAeRlqpEFLmq3cVYWbv5wFHZof3cAimZphwANZjOeAU9rQKcIv2WscSX0aMCcABRq0J8APWZVrQFzaoCFAKNsh5gB6myJdgzCaQJ7DwJwWosAJWqQjwCnb5KSAPptCZwB4G56fwZ9bhmVDp9rU3cWBW3DrgcgazCbBkFt3ZIT+Wm5qA2zaXJ+ALVpF5YA8mnBmQFKaz+KAGZs65ESWmxvhACIafeqAbZqQHEAA20ffADaao+kAc5q+XQAHGpGkMLkYkylDMlrBHIWUmRUdgMxZWWPA8tr2a4J8mY2eQD4ai2bBAdq0ZITDGhLhgBUZl2eAFhng6gCfGLIgQtKZ4xwGc1j5ZsBK2QBeQFQanV0AfRjsaEBeGItiQEDZeF8DS9kDHoSe2UsdwFtZU5+CmFpLngOZGQ4hw0rXOeOAJ1gmagB4F4DmwEJYQKACANd6J4VuFJtrAAA')), indent = 4, ensure_ascii = False))
