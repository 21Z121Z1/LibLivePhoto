#!/usr/bin/env python3
"""Owned neutral media + independent framing. Never overwrite existing fixtures.

Synthetic contracts are not original-device/Gallery acceptance or a native writer.
Expected facts are authored independently, not copied from a core result.
"""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sef(records):
    data = bytearray(b'\xff\xd8\xff\xd9')
    entries = []
    for kind, name, payload in records:
        name = name.encode()
        raw = b'\0\0'+kind.to_bytes(2, 'little')+len(name).to_bytes(4, 'little')+name+payload
        entries.append((len(data), len(raw), kind))
        data.extend(raw)
    start = len(data)
    data.extend(b'SEFH'+(107).to_bytes(4, 'little')+len(entries).to_bytes(4, 'little'))
    for at, size, kind in entries:
        data.extend(b'\0\0'+kind.to_bytes(2, 'little')+(start-at).to_bytes(4, 'little')+size.to_bytes(4, 'little'))
    data.extend((len(data)-start).to_bytes(4, 'little')+b'SEFT')
    return bytes(data)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    directory = parser.parse_args().out
    directory.mkdir(parents=True, exist_ok=False)
    data = {}

    def save(name, value):
        with (directory/name).open('xb') as output:
            output.write(value)
        data[name] = value

    video = (ROOT/'web/public/demo/motion.mp4').read_bytes()
    save('motion.mp4', video)
    other = [(0x0a33, 'MotionPhoto_AutoPlay', video), (0x1234, 'Unknown', b'opaque\0unknown')]
    primary = (0x0a30, 'MotionPhoto_Data', video)
    save('samsung-normal.jpg', sef(other+[primary]))
    save('samsung-no-primary.jpg', sef(other))
    save('samsung-ambiguous.jpg', sef(other+[primary, primary]))
    save('samsung-bad-pointer.jpg', sef(other+[(0x0a30, 'MotionPhoto_Data', b'mpv2'+b'\xff'*4+(10).to_bytes(4, 'big'))]))
    vivo = (ROOT/'fixtures/bindings/vivo-stream-motion.jpg').read_bytes()
    save('vivo-alias.jpg', vivo)
    old = b'synthetic-camera-id-00000000'
    assert vivo.count(old) == 2
    save('vivo-id-conflict.jpg', vivo.replace(old, b'synthetic-camera-id-00000001', 1))
    # Owned fixture: update only its APP1 segment's length and known XML attribute.
    end = vivo.index(b'Camera:MotionPhoto="1"')
    legacy = b' Camera:MicroVideo="1" Camera:MicroVideoOffset="1"'
    changed = bytearray(vivo[:end]+legacy+vivo[end:])
    pos = 2
    while pos < end:
        assert vivo[pos] == 255
        size = int.from_bytes(vivo[pos+2:pos+4], 'big')
        if pos+2+size > end:
            assert vivo[pos+1] == 225
            changed[pos+2:pos+4] = (size+len(legacy)).to_bytes(2, 'big')
            break
        pos += size+2
    else:
        raise AssertionError('owned XMP APP1 required')
    save('vivo-xmp-span-conflict.jpg', bytes(changed))
    bad = bytearray(vivo)
    at = bad.index(b'streamcount')
    bad[at] ^= 1
    save('vivo-bad-stream.jpg', bytes(bad))
    at = vivo.index(b'ftyp')-4
    save('vivo-movie.mp4', vivo[at:])
    cases = []

    def case(name, filename, facts, action='metadata', success=True, files=None):
        request = {'action': action}
        if action == 'extract': request.update(still=False, video=True, xmp=False)
        cases.append({'id': name, 'primary': filename, 'request': request, 'success': success,
                      'expect': [{'path': ['metadata']+p, 'value': v} for p, v in facts], 'files': files or []})

    def fact(index, path):
        return ['decoded', index, 'value']+path

    for name, index, primaries, videos, verified in [
        ('normal', 1, [2], [0, 2], True),
        ('no-primary', 1, [], [0], False),
        ('ambiguous', 0, [2, 3], [0, 2, 3], False),
        ('bad-pointer', 0, [2], [0], False),
    ]:
        facts = [(['verifiedMotion'], verified), (['decoded', index, 'provider'], 'samsung_video_inventory'),
                 (fact(index, ['declaredPrimaryRecords']), primaries)]
        facts += [(fact(index, ['videos', i, 'recordIndex']), n) for i, n in enumerate(videos)]
        facts += [(fact(index, ['videos', i, 'role']), 'uninterpreted') for i in range(len(videos))]
        if name == 'bad-pointer': facts.append((fact(index, ['diagnostics', 0, 'recordIndex']), 2))
        case('samsung-'+name, 'samsung-'+name+'.jpg', facts)
        if not verified:
            case('samsung-'+name+'-parse-rejected', 'samsung-'+name+'.jpg', [], 'parse', False)
    for name in ['alias', 'id-conflict', 'xmp-span-conflict']:
        facts = [(['verifiedMotion'], True), (['decoded', 0, 'provider'], 'vivo_media'),
                 (fact(0, ['declarations', 'footers', 0, 'origins']), ['TerminalSource', 'MotionVideoUuid']),
                 (fact(0, ['declarations', 'identifierComparisons']), 1),
                 (fact(0, ['declarations', 'pairingVerified']), False),
                 (fact(0, ['declarations', 'imageTimeUnit']), 'unverified'),
                 (fact(0, ['declarations', 'xmpSpans', 0, 'status']), 'Matched')]
        if name == 'alias': facts.append((fact(0, ['declarations', 'issues']), []))
        elif name == 'id-conflict': facts.append((fact(0, ['declarations', 'issues', 0, 'code']), 'footer-internal-identifiers-differ'))
        else:
            facts += [(fact(0, ['declarations', 'xmpSpans', 1, 'status']), 'Mismatch'),
                      (fact(0, ['declarations', 'issues', 0, 'code']), 'xmp-media-span')]
        case('vivo-'+name, 'vivo-'+name+'.jpg', facts)
    case('vivo-bad-stream', 'vivo-bad-stream.jpg', [(['verifiedMotion'], True), (['decoded', 0, 'provider'], 'google')])
    case('samsung-primary-original-bytes', 'samsung-normal.jpg', [], 'extract', files=[{'mime': 'video/mp4', 'fixture': 'motion.mp4'}])
    cases[-1]['request']['video'] = True
    case('vivo-conflicting-id-original-bytes', 'vivo-id-conflict.jpg', [], 'extract', files=[{'mime': 'video/mp4', 'fixture': 'vivo-movie.mp4'}])
    # The movie above differs only in the JSON declaration; encoded packets and
    # all opaque data remain untouched. Expected original movie includes that ID.
    conflicted = data['vivo-id-conflict.jpg']
    save('vivo-conflicting-movie.mp4', conflicted[conflicted.index(b'ftyp')-4:])
    cases[-1]['files'][0]['fixture'] = 'vivo-conflicting-movie.mp4'
    cases[-1]['request']['video'] = True
    assert len(cases) == 13
    manifest = {'scope': 'owned framing/byte contracts; no native Gallery acceptance', 'cases': cases,
                'sha256': {name: hashlib.sha256(value).hexdigest() for name, value in data.items()}}
    with (directory/'cases.json').open('x', encoding='utf-8', newline='\n') as output:
        json.dump(manifest, output, indent=2)
        output.write('\n')
    print(directory, len(cases), hashlib.sha256((directory/'cases.json').read_bytes()).hexdigest())


if __name__ == '__main__':
    main()
