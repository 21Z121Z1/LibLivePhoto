#!/usr/bin/env python3
"""Real CLI and independent packet/timeline/index checks for the round3 subset."""
import hashlib,json,platform,struct,subprocess,tempfile
from fractions import Fraction
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def boxes(data):
    at=0
    while at<len(data):
        size,kind=struct.unpack_from('>I4s',data,at);header=8
        if size==1:size=struct.unpack_from('>Q',data,at+8)[0];header=16
        if size==0:size=len(data)-at
        assert size>=header and at+size<=len(data)
        yield at,at+size,kind,data[at+header:at+size]
        at+=size
    assert at==len(data)
def box(kind,payload):return struct.pack('>I4s',len(payload)+8,kind)+payload
def stz2(data):
    out=bytearray();tables=0
    for start,end,kind,payload in boxes(data):
        if kind==b'stsz':
            flags,constant,count=struct.unpack_from('>III',payload);assert flags==0
            assert count<=100000
            sizes=[constant]*count if constant else list(struct.unpack_from('>'+str(count)+'I',payload,12))
            assert all(v<=65535 for v in sizes)
            out.extend(box(b'stz2',bytes(7)+bytes([16])+struct.pack('>I',count)+b''.join(struct.pack('>H',v) for v in sizes)));tables+=1
        elif kind in [b'moov',b'trak',b'mdia',b'minf',b'stbl']:
            changed,n=stz2(payload);out.extend(box(kind,changed));tables+=n
        else:out.extend(data[start:end])
    return bytes(out),tables
def packet_facts(path):
    raw=path.read_bytes();mdats=[(a+8,b) for a,b,k,p in boxes(raw) if k==b'mdat']
    r=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-show_packets','-show_data_hash','sha256','-of','json',str(path)],text=True))
    scales={s['index']:Fraction(s['time_base']) for s in r['streams']}
    result=[]
    for p in r['packets']:
        at=int(p['pos']);n=int(p['size']);digest=p['data_hash'].split(':',1)[1].lower()
        assert any(at>=lo and at+n<=hi for lo,hi in mdats)
        assert hashlib.sha256(raw[at:at+n]).hexdigest()==digest,('sample target',p)
        scale=scales[p['stream_index']]
        result.append((p['stream_index'],str(int(p['dts'])*scale),str(int(p['pts'])*scale),str(int(p['duration'])*scale),n,digest))
    return result,r
def main():
    subprocess.run(['cargo','build','--locked','-p','liblivephoto-cli'],cwd=ROOT,check=True)
    cli=ROOT/'target/debug'/('llp.exe' if platform.system()=='Windows' else 'llp')
    (ROOT/'evidence/local').mkdir(parents=True,exist_ok=True)
    out=Path(tempfile.mkdtemp(prefix='syntax-contracts-',dir=ROOT/'evidence/local'))
    print('evidence: '+str(out),flush=True)
    calls=[]
    def run(*args,success):
        r=subprocess.run([str(cli),*map(str,args)],cwd=ROOT,text=True,capture_output=True,timeout=40)
        value=json.loads(r.stdout);assert value['success']==success,(args,r.returncode,r.stdout,r.stderr)
        assert (r.returncode==0)==success,(args,r.returncode)
        calls.append({'args':list(map(str,args)),'exit_code':r.returncode,'success':value['success']});return value
    profiles=ROOT/'fixtures/syntax';valid=profiles/'icc-structure-v4.4.icc';unknown=profiles/'icc-unsupported-v4.3.icc'
    r=run('conformance',valid,'--options-json','{"profile":"icc-structure-v4.4"}',success=True)
    assert r['metadata']['profile']=='icc.1-2022-v4.4-structure'
    r=run('conformance',unknown,'--options-json','{"profile":"icc-structure-v4.4"}',success=False)
    assert r['metadata']['issues'][0]['kind']=='Unsupported'
    wrapped=out/'icc-range.bin';wrapped.write_bytes(bytes(13)+valid.read_bytes()+bytes(4))
    run('conformance',wrapped,'--range','13:168','--options-json','{"profile":"icc-structure-v4.4"}',success=True)
    bad=out/'truncated.icc';bad.write_bytes(valid.read_bytes()[:-1])
    run('conformance',bad,'--options-json','{"profile":"icc-structure-v4.4"}',success=False)
    movie=profiles/'aac-pce-wave.mov';source=movie.read_bytes();original_hash=hashlib.sha256(source).hexdigest()
    r=run('parse',ROOT/'web/public/demo/cover.jpg','--parts','--movie',movie,success=True)
    audio=next(d['audio']['aac'] for d in r['metadata']['sampleDescriptions'] if d['audio'])
    assert audio['channelConfiguration']==0 and audio['channels']==2 and audio['sampleRate']==48000
    assert audio['programConfig']['front']==[{'isPair':True,'tag':0}]
    assert audio['programConfig']['speakerPositionsInferred'] is False
    assert audio['programConfig']['renderingVerified'] is False
    malformed=bytearray(source);assert malformed.count(b'esds')==1
    malformed[malformed.index(b'esds')+4]=1
    bad_audio=out/'unsupported-esds.mov';bad_audio.write_bytes(malformed)
    failed=run('conformance',bad_audio,'--tracks','1','--options-json','{"profile":"google-video-container-1.0"}',success=False)
    assert 'esds version/flags' in json.dumps(failed)
    top=list(boxes(source));moov=next(a for a,b,k,p in top if k==b'moov')
    assert all(b<=moov for a,b,k,p in top if k==b'mdat'),'authored conversion requires mdat before moov'
    indexed,tables=stz2(source);assert tables==2
    assert indexed==(profiles/'aac-pce-stz2.mov').read_bytes()
    input_file=out/'stz2.mov';input_file.write_bytes(indexed)
    before,probe=packet_facts(input_file);legacy,_=packet_facts(movie);assert before==legacy
    audio_probe=next(s for s in probe['streams'] if s['codec_type']=='audio')
    assert audio_probe['channels']==2 and audio_probe['sample_rate']=='48000'
    outputs=[]
    for compact in [False,True]:
        directory=out/('compact' if compact else 'preserve')
        args=['optimize',input_file,'--out',directory]+(['--compact'] if compact else [])
        r=run(*args,success=True)
        files=list(directory.glob('movie.*'));assert len(files)==1
        after,_=packet_facts(files[0]);assert after==before
        outputs.append({'compact':compact,'samples':len(after),'file_sha256':hashlib.sha256(files[0].read_bytes()).hexdigest(),'each_sample_hash_dts_pts_duration_and_index_target_verified':True})
    for kind in [b'sidx',b'moof',b'mfra',b'uuid']:
        unsupported=out/(kind.decode()+'.mov');unsupported.write_bytes(indexed+box(kind,b''))
        run('optimize',unsupported,success=False)
    assert hashlib.sha256(movie.read_bytes()).hexdigest()==original_hash
    from syntax_contracts import CliCore,run_cases
    acceptance=run_cases(CliCore(cli),'cli-syntax')
    vendor_acceptance=run_cases(CliCore(cli),'vendor-cli',ROOT/'fixtures/vendor')
    selection_acceptance=run_cases(CliCore(cli),'vendor-selection-cli',ROOT/'fixtures/vendor-selection')
    # Golden bytes only establish platform transport parity. Independently
    # demux each real-media golden and validate every indexed sample extent,
    # encoded hash, rational DTS/PTS/duration, including actual unused media.
    def stream(facts,index):return [p[1:] for p in facts if p[0]==index]
    primary,_=packet_facts(profiles/'motion.mp4')
    packet_checks=[]
    for kind,source in [('stz2','aac-pce-stz2-unused.mov'),('legacy','aac-direct.mp4')]:
        before,_=packet_facts(profiles/source)
        for action in ['preserve','compact','remove-audio','import-audio']:
            path=profiles/('golden-'+kind+'-'+action+'-0.bin')
            after,_=packet_facts(path)
            expected_video=stream(primary,0) if action=='import-audio' else stream(before,0)
            assert stream(after,0)==expected_video,(kind,action,'video')
            assert stream(after,1)==([] if action=='remove-audio' else stream(before,1)),(kind,action,'audio')
            packet_checks.append({'kind':kind,'operation':action,'samples':len(after),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'packets':after})
    unused=b'LLP round4 unreferenced media '+bytes(range(73))
    preserved=(profiles/'golden-stz2-preserve-0.bin').read_bytes();compacted=(profiles/'golden-stz2-compact-0.bin').read_bytes()
    assert unused in preserved and unused not in compacted and len(preserved)-len(compacted)==len(unused)==103
    receipt={'implementation_sha':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'tracked_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD'],cwd=ROOT)).hexdigest(),
        'scope':'CLI ICC envelope and AAC-LC/PCE/wave; two-track real owned MOV converted to stz2. ffprobe demux DTS/PTS include its edit handling; raw stts/ctts preservation is also checked by Rust tests. No colour/audio rendering or device acceptance.',
        'source_sha256':original_hash,'stz2_tables':tables,'packets':legacy,'outputs':outputs,'calls':calls,
        'acceptance':acceptance,'vendor_acceptance':vendor_acceptance,'selection_acceptance':selection_acceptance,'independent_packet_checks':packet_checks,'actual_discarded_unreferenced_media_bytes':len(unused),
        'ffprobe_version':subprocess.check_output(['ffprobe','-version'],text=True).splitlines()[0]}
    (out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print('INDEPENDENT_PACKET_CONTRACTS_PASSED '+json.dumps({'implementation_sha':receipt['implementation_sha'],'outputs':len(packet_checks),'packets':sum(len(row['packets']) for row in packet_checks),'actual_discarded_media_bytes':len(unused),'ffprobe_version':receipt['ffprobe_version']}),flush=True)
    print('passed; receipt: '+str(out/'receipt.json'))
if __name__=='__main__':main()
