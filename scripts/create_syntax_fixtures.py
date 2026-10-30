#!/usr/bin/env python3
"""Owned grammar vectors and real silent AAC/PCE in MOV; no vendor media/code."""
import argparse,hashlib,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def icc():
    p=bytearray(168);p[:4]=(168).to_bytes(4,'big');p[8:12]=bytes([4,0x40,0,0]);p[12:24]=b'mntrRGB XYZ '
    p[36:40]=b'acsp';p[128:132]=(2).to_bytes(4,'big')
    for i,sig in enumerate([b'prv1',b'prv2']):
        at=132+i*12;p[at:at+12]=sig+(156).to_bytes(4,'big')+(11).to_bytes(4,'big')
    p[156:160]=b'unkn';p[164:167]=b'raw';return bytes(p)
def main():
    a=argparse.ArgumentParser();a.add_argument('--out',type=Path,required=True);a.add_argument('--ffmpeg',default='ffmpeg');args=a.parse_args()
    args.out.mkdir(parents=True,exist_ok=False)
    (args.out/'icc-structure-v4.4.icc').write_bytes(icc())
    unknown=bytearray(icc());unknown[9]=0x30;(args.out/'icc-unsupported-v4.3.icc').write_bytes(unknown)
    video=ROOT/'web/public/demo/motion.mp4';output=args.out/'aac-pce-wave.mov'
    command=[args.ffmpeg,'-hide_banner','-v','error','-n','-i',str(video),'-f','lavfi','-i','anullsrc=r=48000:cl=stereo',
             '-map','0:v:0','-map','1:a:0','-t','0.2','-c:v','copy','-c:a','aac','-aac_pce','1','-flags:a','+bitexact',
             '-fflags','+bitexact','-map_metadata','-1','-f','mov',str(output)]
    subprocess.run(command,check=True,timeout=60)
    from test_syntax_contracts import stz2,boxes
    source=output.read_bytes();moov=next(a for a,b,k,p in boxes(source) if k==b'moov')
    assert all(b<=moov for a,b,k,p in boxes(source) if k==b'mdat')
    indexed,tables=stz2(source);assert tables==2
    (args.out/'aac-pce-stz2.mov').write_bytes(indexed)
    print(json.dumps({'scope':'Owned neutral video copied without re-encoding plus newly encoded silence; ICC envelope vectors have opaque private tags, not full colour profiles.',
        'command':command,'ffmpeg_version':subprocess.check_output([args.ffmpeg,'-version'],text=True).splitlines()[:3],
        'source_video_sha256':hashlib.sha256(video.read_bytes()).hexdigest(),
        'files':[{'file':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(args.out.iterdir())]},indent=2))
if __name__=='__main__':main()
