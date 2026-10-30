#!/usr/bin/env python3
"""Same-input decoding of existing descriptor bytes, no metadata-track authoring."""
import argparse,hashlib,json,shlex,struct,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];SHA='12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3'
def run(a,**kw):
 r=subprocess.run(list(map(str,a)),text=True,capture_output=True,**kw)
 if r.returncode:raise RuntimeError(r.stderr)
 return r.stdout
# Synthetic independent descriptor test vectors, not a Moments creation API.
def descriptor(tag,p,width=1):
 n=len(p);digits=[0]*width
 for i in range(width-1,-1,-1):digits[i]=n&127;n>>=7
 assert n==0
 return bytes([tag])+bytes(v|(128 if i<width-1 else 0) for i,v in enumerate(digits))+p
def frame(score,time):return descriptor(0xc3,struct.pack('>fq',score,time))
def main():
 p=argparse.ArgumentParser();p.add_argument('--reference-source',type=Path,required=True);p.add_argument('--reference-build',type=Path,required=True);a=p.parse_args();source=a.reference_source.resolve();assert run(['git','-C',source,'rev-parse','HEAD']).strip()==SHA;out=Path(tempfile.mkdtemp(prefix='score-diff-',dir=ROOT/'evidence/local'));build=a.reference_build.resolve()/'lib/core';link=shlex.split((build/'CMakeFiles/motion_photo_tests.dir/link.txt').read_text());ref=out/'reference';run(['c++','-std=c++17','-I'+str(source/'lib/core/motion_photo/includes'),'-I'+str(source/'lib/core/third_party/image_io/includes'),ROOT/'scripts/video_metadata_reference_probe.cc','-o',ref,*link[link.index('-o')+2:]],cwd=build);run(['cargo','build','--locked','-p','liblivephoto','--example','video_metadata_probe'],cwd=ROOT);rows=[]
 for width in [1,2,4]:
  for flags in [0,0x80,0xff]:
   scores=descriptor(0xc1,struct.pack('>I',7)+frame(.8125,812500)+descriptor(0xc2,struct.pack('>I',3)+frame(0.,0)+frame(.25,500000)+frame(1.,1000000),width),width);payload=descriptor(0xc0,scores+descriptor(0xc4,descriptor(0xc5,bytes([flags]))+descriptor(0xc5,bytes([flags^128]))),width);path=out/f'w{width}-flags{flags}.bin';path.write_bytes(payload);expected=json.loads(run([ref,path]));actual=json.loads(run([ROOT/'target/debug/examples/video_metadata_probe',path]));assert actual==expected,(expected,actual);rows.append({'name':path.name,'sha256':hashlib.sha256(payload).hexdigest(),'decoded_fields_equal':True})
 receipt={'reference_sha':SHA,'implementation_sha':run(['git','-C',ROOT,'rev-parse','HEAD']).strip(),'cases':rows,'differences':['Rust retains absent fields as None and unknown descriptors/raw flag bits; it does not create default score/timestamp objects.','Semantic validation is explicit, including score range, ordering and exact photo-time consistency.','No Moments construction API is implemented.']};(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(out/'receipt.json')
if __name__=='__main__':main()
