#!/usr/bin/env python3
"""Additional CC0 media from the project's own scene and generated sine wave.

Only new paths are created. HEIC uses actual x265-coded pixels and hvcC from a
one-frame MP4; no private camera samples or fabricated coded data are used.
"""
import hashlib, json, struct, subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'web/public/acceptance'
def run(args):return subprocess.check_output(list(map(str,args)),text=True)
def u16(n):return struct.pack('>H',n)
def u32(n):return struct.pack('>I',n)
def box(tag,p):return u32(len(p)+8)+tag+p
def main():
    OUT.mkdir(parents=True,exist_ok=True)
    names=['audio.mp4','hevc.mp4','unsupported.mov','still-source.mp4','still.heic','provenance.json']
    if any((OUT/n).exists() for n in names):raise RuntimeError('refusing to overwrite existing generated files')
    ff=['ffmpeg','-hide_banner','-loglevel','error','-n']
    run([*ff,'-i',ROOT/'web/public/demo/motion.mp4','-f','lavfi','-i','sine=frequency=440:sample_rate=48000:duration=3','-map','0:v:0','-map','1:a:0','-c:v','copy','-c:a','aac','-ac','2','-t','3','-movflags','+faststart',OUT/'audio.mp4'])
    enc=['-c:v','libx265','-preset','ultrafast','-x265-params','pools=1:frame-threads=1:log-level=error','-tag:v','hvc1','-pix_fmt','yuv420p']
    run([*ff,'-i',ROOT/'web/public/demo/motion.mp4','-an',*enc,'-movflags','+faststart',OUT/'hevc.mp4'])
    run([*ff,'-i',ROOT/'web/public/demo/cover.jpg','-frames:v','1',*enc,OUT/'still-source.mp4'])
    run([*ff,'-i',ROOT/'web/public/demo/cover.jpg','-frames:v','1','-c:v','jpeg2000','-tag:v','mjp2','-f','mov',OUT/'unsupported.mov'])
    encoded=(OUT/'still-source.mp4').read_bytes()
    info=json.loads(run(['ffprobe','-v','error','-show_packets','-of','json',OUT/'still-source.mp4']))
    assert len(info['packets'])==1
    packet=info['packets'][0];sample=encoded[int(packet['pos']):int(packet['pos'])+int(packet['size'])]
    at=encoded.index(b'hvcC')-4;size=int.from_bytes(encoded[at:at+4],'big');hvcc=encoded[at:at+size]
    assert len(hvcc)==size and size>30
    ftyp=box(b'ftyp',b'heic'+u32(0)+b'heicmif1miaf')
    hdlr=box(b'hdlr',bytes(8)+b'pict'+bytes(12)+b'LibLivePhoto own scene\0')
    pitm=box(b'pitm',bytes(4)+u16(1))
    infe=box(b'infe',b'\2\0\0\0'+u16(1)+u16(0)+b'hvc1'+b'own scene\0')
    iinf=box(b'iinf',bytes(4)+u16(1)+infe)
    ipco=box(b'ipco',box(b'ispe',bytes(4)+u32(720)+u32(480))+hvcc+box(b'pixi',bytes(4)+b'\3\10\10\10'))
    ipma=box(b'ipma',bytes(4)+u32(1)+u16(1)+b'\3\x81\x82\3')
    iprp=box(b'iprp',ipco+ipma)
    def meta(offset):
        iloc=box(b'iloc',bytes(4)+b'\x44\0'+u16(1)+u16(1)+u16(0)+u16(1)+u32(offset)+u32(len(sample)))
        return box(b'meta',bytes(4)+hdlr+pitm+iloc+iinf+iprp)
    offset=len(ftyp)+len(meta(0))+8
    with (OUT/'still.heic').open('xb') as output:output.write(ftyp+meta(offset)+box(b'mdat',sample))
    # Decode the actual generated HEIC with the installed independent decoder.
    run(['magick',OUT/'still.heic','-format','%wx%h','info:'])
    provenance={'license':'CC0-1.0','creator':'LibLivePhoto project; existing own geometric scene plus generated 440 Hz sine',
                'sourceScene':'../demo/provenance.json','generator':'scripts/generate_acceptance_media.py',
                'privateMedia':False,'pixelAndAudioReencoding':'explicit generation of additional fixtures only; no user input is transcoded',
                'buildTools':'FFmpeg with x264/x265/AAC and ImageMagick/libheif decoder; not linked into library or runtime',
                'files':{p.name:{'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(OUT.iterdir()) if p.suffix in ['.mp4','.mov','.heic']}}
    with (OUT/'provenance.json').open('x') as output:json.dump(provenance,output,indent=2);output.write('\n')
    print(OUT/'provenance.json')
if __name__=='__main__':main()
