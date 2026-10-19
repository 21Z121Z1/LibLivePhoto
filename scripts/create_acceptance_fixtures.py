#!/usr/bin/env python3
"""Author owned cross-platform vectors. Never overwrite any existing fixture.

Goldens establish transport parity, not an independent format oracle. The CLI
suite separately verifies every encoded packet, index and rational time with
ffprobe, while Rust tests check raw timing tables and stricter Rust policies.
"""
import argparse
import hashlib
import json
import platform
import subprocess
from pathlib import Path
from syntax_contracts import ROOT, FIXTURES, NativeCore
from test_syntax_contracts import box, boxes


def rewrite(data, callback):
    output = bytearray()
    for start, end, kind, payload in boxes(data):
        changed = callback(kind, payload)
        if changed is not None:
            output.extend(changed)
        elif kind in (b'moov', b'trak', b'mdia', b'minf', b'stbl', b'wave'):
            output.extend(box(kind, rewrite(payload, callback)))
        elif kind == b'stsd':
            def entry(k, p):
                if k != b'mp4a': return None
                version = int.from_bytes(p[8:10], 'big')
                assert version in [0, 1]
                header = 44 if version == 1 else 28
                return box(k, p[:header]+rewrite(p[header:], callback))
            output.extend(box(kind, payload[:8]+rewrite(payload[8:], entry)))
        else:
            output.extend(data[start:end])
    return bytes(output)


def descriptor(tag, payload):
    n, length = len(payload), []
    while True:
        length.insert(0, (n & 127) | (128 if length else 0))
        n >>= 7
        if not n:
            return bytes([tag, *length])+payload


def read_descriptor(data, at=0):
    tag, size = data[at], 0
    at += 1
    for _ in range(4):
        v = data[at]; at += 1
        size = size*128+(v & 127)
        if v < 128:
            return tag, data[at:at+size], at+size
    raise ValueError('descriptor length')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    out = args.out

    def save(name, data):
        with (out/name).open('xb') as stream:
            stream.write(data)
        return name

    for name in ['icc-structure-v4.4.icc', 'icc-unsupported-v4.3.icc', 'aac-pce-wave.mov', 'aac-pce-stz2.mov']:
        save(name, (FIXTURES/name).read_bytes())
    save('cover.jpg', (ROOT/'web/public/demo/cover.jpg').read_bytes())
    save('motion.mp4', (ROOT/'web/public/demo/motion.mp4').read_bytes())
    from test_syntax_contracts import stz2
    agtm = (ROOT/'fixtures/bindings/agtm-video.mp4').read_bytes()
    agtm_indexed, _ = stz2(agtm)
    old_mdats = [(a+8,b) for a,b,k,p in boxes(agtm) if k == b'mdat']
    new_mdats = [(a+8,b) for a,b,k,p in boxes(agtm_indexed) if k == b'mdat']
    def relocate(k,p):
        if k not in [b'stco',b'co64']: return None
        width = 4 if k == b'stco' else 8
        values = []
        for i in range(8,len(p),width):
            old = int.from_bytes(p[i:i+width],'big')
            index = next(j for j,(lo,hi) in enumerate(old_mdats) if lo <= old < hi)
            values.append((old-old_mdats[index][0]+new_mdats[index][0]).to_bytes(width,'big'))
        return box(k,p[:8]+b''.join(values))
    agtm_indexed = rewrite(agtm_indexed,relocate)
    save('agtm-stz2.mp4', agtm_indexed)
    save('agtm-raw.bin', (ROOT/'fixtures/bindings/agtm-raw.bin').read_bytes())
    icc = (out/'icc-structure-v4.4.icc').read_bytes()
    save('icc-truncated.icc', icc[:-1])
    bad = bytearray(icc); bad[128:132] = (4097).to_bytes(4, 'big')
    save('icc-budget.icc', bad)
    normal = out/'aac-direct.mp4'
    subprocess.run(['ffmpeg', '-hide_banner', '-v', 'error', '-n', '-i', str(ROOT/'web/public/demo/motion.mp4'), '-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo', '-map', '0:v:0', '-map', '1:a:0', '-t', '0.2', '-c:v', 'copy', '-c:a', 'aac', '-flags:a', '+bitexact', '-fflags', '+bitexact', '-map_metadata', '-1', '-f', 'mp4', str(normal)], check=True, timeout=60)
    direct = normal.read_bytes()
    assert all(b <= next(a for a, b, k, p in boxes(direct) if k == b'moov') for a, b, k, p in boxes(direct) if k == b'mdat')

    def change_esds(mode):
        def change(kind, payload):
            if kind != b'esds': return None
            if mode == 'version': return box(kind, bytes([1])+payload[1:])
            tag, es, _ = read_descriptor(payload, 4); assert tag == 3 and es[:3] == b'\0\2\0'
            tag, config, at = read_descriptor(es, 3); assert tag == 4
            decoder = descriptor(4, config)
            if mode == 'duplicate': es = es[:3]+decoder+decoder+es[at:]
            elif mode == 'extension': es = es+descriptor(7, b'opaque')
            elif mode == 'pce-truncated':
                tag, asc, _ = read_descriptor(config, 13); assert tag == 5 and len(asc) > 2
                es = es[:3]+descriptor(4, config[:13]+descriptor(5, asc[:2]))+es[at:]
            return box(kind, payload[:4]+descriptor(3, es))
        return change

    for mode in ['version', 'duplicate', 'extension']:
        save('aac-direct-'+mode+'.mp4', rewrite(direct, change_esds(mode)))
    pce = (out/'aac-pce-wave.mov').read_bytes()
    save('aac-pce-truncated.mov', rewrite(pce, change_esds('pce-truncated')))
    indexed = (out/'aac-pce-stz2.mov').read_bytes()
    unused = b'LLP round4 unreferenced media '+bytes(range(73))
    mdats = [a for a,b,k,p in boxes(indexed) if k == b'mdat']; assert len(mdats) == 1
    save('aac-pce-stz2-unused.mov', rewrite(indexed, lambda k,p: box(k,p+unused) if k == b'mdat' else None))
    def mixed(k,p):
        if k != b'stz2': return None
        assert p[7] == 16
        sizes = [int.from_bytes(p[i:i+2], 'big') for i in range(12, len(p), 2)]
        return box(k,p)+box(b'stsz', bytes(8)+len(sizes).to_bytes(4,'big')+b''.join(n.to_bytes(4,'big') for n in sizes))
    save('aac-mixed-sizes.mov', rewrite(indexed, mixed))
    library_name = {'Darwin':'liblivephoto_native.dylib', 'Windows':'livephoto_native.dll'}.get(platform.system(), 'liblivephoto_native.so')
    core = NativeCore(ROOT/'target/debug'/library_name)
    cases = []
    def fact(path, value): return {'path':path.split('.') if isinstance(path,str) else path, 'value':value}
    def case(id, request, primary, success=True, expect=None, movie=None, golden=False):
        c = {'id': id, 'request': request, 'primary':primary, 'success':success, 'expect':expect or []}
        if movie: c['movie'] = movie
        if golden:
            r = core(request, (out/primary).read_bytes(), (out/movie).read_bytes() if movie else b'')
            assert r['success'], (id, r['metadata'])
            c['files'] = []
            for i, file in enumerate(r['files']):
                name = save('golden-'+id+'-'+str(i)+'.bin', file['bytes'])
                c['files'].append({'mime': file['mime'], 'fixture': name})
        cases.append(c)
        return c
    profile = {'action':'conformance','profile':'icc-structure-v4.4'}
    case('icc-valid',profile,'icc-structure-v4.4.icc',expect=[fact('metadata.profile','icc.1-2022-v4.4-structure'),fact('metadata.checkedRequiredRulesSatisfied',True)])
    case('icc-unsupported',profile,'icc-unsupported-v4.3.icc',False,[fact(['metadata','issues',0,'kind'],'Unsupported')])
    case('icc-truncated',profile,'icc-truncated.icc',False,[fact('metadata.error.code','metadataMismatch')])
    case('icc-budget',profile,'icc-budget.icc',False,[fact('metadata.error.code','inputLimit')])
    parse = {'action':'parse','inputKind':'parts','time':{'value':'1','timescale':10}}
    audio = ['metadata','sampleDescriptions',1,'audio','aac']
    pce_facts = [fact(audio+['channelConfiguration'],0),fact(audio+['channels'],2),fact(audio+['sampleRate'],48000),fact(audio+['programConfig','front'],[{'isPair':True,'tag':0}]),fact(audio+['programConfig','speakerPositionsInferred'],False),fact(audio+['programConfig','renderingVerified'],False),fact('metadata.photoTime',{'value':'1','timescale':10})]
    pce_facts += [fact(['metadata','tracks',0,'frameTiming','frames',1,'time'],{'value':'128000','timescale':2048000}),fact(['metadata','tracks',0,'frameTiming','frames',5,'time'],{'value':'768000','timescale':2048000})]
    for id, movie in [('pce-wave','aac-pce-wave.mov'),('pce-stz2','aac-pce-stz2.mov'),('pce-stz2-unused','aac-pce-stz2-unused.mov')]:
        case(id,parse,'cover.jpg',expect=pce_facts,movie=movie)
    case('direct-aac',parse,'cover.jpg',expect=[fact(audio+['channelConfiguration'],2),fact(audio+['channels'],2),fact(audio+['sampleRate'],48000),fact(audio+['programConfig'],None),fact('metadata.photoTime',{'value':'1','timescale':10})],movie='aac-direct.mp4')
    conformance = {'action':'conformance','profile':'google-video-container-1.0','trackIds':[1]}
    for mode, code in [('version','unsupportedLayout'),('duplicate','metadataMismatch'),('extension','unsupportedLayout')]:
        expect = [fact('metadata.error.code',code)] if mode == 'duplicate' else [fact(['metadata','issues',0,'kind'],'Unsupported')]
        case('direct-'+mode,conformance,'aac-direct-'+mode+'.mp4',False,expect)
        diagnostic = {'version':'esds version/flags','duplicate':'declared duplicate ES DecoderConfigDescriptor differs from the encoded container','extension':'unsupported ES descriptor extension'}[mode]
        case('direct-'+mode+'-inspect',parse,'cover.jpg',True,[fact('metadata.sampleDescriptions',[]),fact('metadata.diagnostics',['sample descriptions unavailable: '+diagnostic]),fact('metadata.photoTime',{'value':'1','timescale':10})],movie='aac-direct-'+mode+'.mp4')
    case('pce-truncated',conformance,'aac-pce-truncated.mov',False,[fact('metadata.error.code','invalidInput')])
    case('mixed-sizes',{'action':'optimize'},'aac-mixed-sizes.mov',False,[fact('metadata.error.code','metadataMismatch')])
    for id, source in [('stz2','aac-pce-stz2-unused.mov'),('legacy','aac-direct.mp4')]:
        for compact in [False,True]:
            discarded = len(unused) if id == 'stz2' and compact else 0
            retained = len(unused) if id == 'stz2' and not compact else 0
            c = case(id+('-compact' if compact else '-preserve'),{'action':'optimize','compact':compact},source,expect=[fact('metadata.discardedMediaBytes',str(discarded)),fact('metadata.retainedUnknownMediaBytes',str(retained)),fact('metadata.encodedSamplesUnchanged',True)],golden=True)
            case(c['id']+'-parse',parse,'cover.jpg',expect=pce_facts if id=='stz2' else [fact(audio+['channels'],2),fact(audio+['programConfig'],None),fact('metadata.photoTime',{'value':'1','timescale':10})],movie={'case':c['id']})
        case(id+'-remove-audio',{'action':'remove-tracks','trackIds':[2],'compact':True},source,expect=[fact('metadata.removedTrackIds',[2]),fact('metadata.encodedSamplesUnchanged',True)],golden=True)
        case(id+'-import-audio',{'action':'import-tracks','trackIds':[2]},'motion.mp4',movie=source,expect=[fact('metadata.encodedSamplesUnchanged',True)],golden=True)
    agtm_case = case('agtm-stz2',{'action':'agtm','videoInput':True,'time':{'value':'1','timescale':1}},'agtm-stz2.mp4')
    agtm_case['files'] = [{'mime':'application/octet-stream','fixture':'agtm-raw.bin'}]
    # Default write policy preserves even uninterpretable descriptors. Strict
    # capabilities rejection/Unmapped is independently checked by the Rust API.
    for id, source in [('pce','aac-pce-wave.mov'),('direct','aac-direct.mp4'),('version','aac-direct-version.mp4'),('duplicate','aac-direct-duplicate.mp4'),('extension','aac-direct-extension.mp4')]:
        build = case(id+'-build',{'action':'build','inputKind':'parts','target':'google-jpeg','time':{'value':'1','timescale':10}},'cover.jpg',movie=source,golden=True)
        case(id+'-extract',{'action':'extract','still':False,'video':True,'xmp':False},{'case':build['id']})['files'] = [{'mime':'video/quicktime' if id=='pce' else 'video/mp4','fixture':source}]
        # Reopening malformed AAC descriptions is expected to fail inspection,
        # while extraction above remains byte-exact and does not infer support.
        if id in ['pce','direct']:
            case(id+'-built-time',{'action':'parse'},{'case':build['id']},expect=[fact('metadata.photoTime',{'value':'100000','timescale':1000000})])
    case('inexact-third',{'action':'build','inputKind':'parts','target':'google-jpeg','time':{'value':'1','timescale':3}},'cover.jpg',False,movie='aac-pce-wave.mov',expect=[fact('metadata.canExecute',False),fact('metadata.blockers',['Android XMP time requires an exact integer microsecond value'])])
    manifest={'version':1,'scope':'Owned syntax/transport parity; ICC selected structure only, AAC declarations only, no native Gallery acceptance or rendering certification.',
              'unused_media_bytes':len(unused),'cases':cases,
              'sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.iterdir())}}
    save('cases.json',(json.dumps(manifest,indent=2)+'\n').encode())
    print(json.dumps({'cases':len(cases),'directory':str(out),'unused_media_bytes':len(unused)},indent=2))
if __name__ == '__main__': main()
