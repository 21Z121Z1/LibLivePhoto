#!/usr/bin/env python3
"""Pinned same-input reference checks; only project-generated media are created."""
import argparse,hashlib,json,shlex,subprocess,tempfile
from pathlib import Path
from google_differential import packets
ROOT=Path(__file__).resolve().parents[1];SHA='12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3'
def run(a,**kw):return subprocess.run(list(map(str,a)),check=True,capture_output=True,text=True,**kw).stdout
def app1(xml):
 jpeg=(ROOT/'web/public/demo/cover.jpg').read_bytes();p=b'http://ns.adobe.com/xap/1.0/\0'+xml.encode();return jpeg[:2]+b'\xff\xe1'+(len(p)+2).to_bytes(2,'big')+p+jpeg[2:]
def main():
 p=argparse.ArgumentParser();p.add_argument('--reference-source',type=Path,required=True);p.add_argument('--reference-build',type=Path,required=True);a=p.parse_args();s=a.reference_source.resolve();assert run(['git','-C',s,'rev-parse','HEAD']).strip()==SHA;run(['git','-C',s,'diff','--exit-code'])
 out=Path(tempfile.mkdtemp(prefix='oplus-remux-',dir=ROOT/'evidence/local'));build=a.reference_build.resolve()/'lib/core';link=shlex.split((build/'CMakeFiles/motion_photo_tests.dir/link.txt').read_text());ref=out/'reference';run(['c++','-std=c++17','-I'+str(s),ROOT/'scripts/google_reference_probe.cc','-o',ref,*link[link.index('-o')+2:]],cwd=build)
 for name in ['core_probe','remux_probe']:run(['cargo','build','--locked','-p','liblivephoto','--example',name],cwd=ROOT)
 movie=(ROOT/'web/public/demo/motion.mp4').read_bytes();rows=[]
 base='xmlns:OpCamera="http://ns.oplus.com/photo/1.0/camera/"';rdf='xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"'
 def xml(props):
  props=props.replace('o:','OpCamera:')
  return f'<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF {rdf}><rdf:Description {base} {props}/></rdf:RDF></x:xmpmeta>'
 for name,still,tail in [
  ('attributes',app1(xml(f'o:OLivePhotoVersion="2" o:VideoLength="{len(movie)}" o:PrimaryPresentationTimestampUs="812500"')),movie),
  ('checksum',app1(xml('o:OLivePhotoVersion="2"')),movie+b'a'*60+b'jxrs'+(2).to_bytes(4,'little')),
  ('short',app1(xml(f'o:OLivePhotoVersion="2" o:VideoLength="{len(movie)}"')),movie+b'jxrs'+(2).to_bytes(4,'little')),
 ]:
  folder=out/name;folder.mkdir();path=folder/'input.jpg';path.write_bytes(still+tail);log=run([ref,path,folder]);reference=json.JSONDecoder().raw_decode(log.split('REFERENCE_JSON\t')[1])[0];actual=json.loads(run([ROOT/'target/debug/examples/core_probe',path,folder]));assert actual['recognized'] and actual['video_length']==len(movie),(name,actual);assert reference['file']['recognized'],(name,reference);assert reference['file']['video_length']==len(movie)-(60 if name=='short' else 0),(name,reference);assert (folder/'rust-video.bin').read_bytes()==movie;assert ((folder/'reference-file-video.bin').read_bytes()==movie)==(name!='short')
  rows.append({'name':name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'reference':reference,'implementation':actual,'file_video_bytes_equal':(folder/'reference-file-video.bin').read_bytes()==movie,'reference_short_footer_overstrip_60_bytes':name=='short','memory_extraction_bytes_equal':(folder/'reference-video.bin').read_bytes()==movie if (folder/'reference-video.bin').exists()else None})
 for included in [False,True]:
  name='directory-inclusive'if included else'directory-exclusive';folder=out/name;folder.mkdir();directory=json.dumps([{'name':'live.subVideo','offset':len(movie)+2,'length':len(movie)},{'name':'capture.mode','offset':2,'length':2}],separators=(',',':')).encode()+b'\0';tail=movie+movie+b'CM'+directory+b'jxrs'+(len(directory)+(8 if included else 0)).to_bytes(4,'little');path=folder/'input.jpg';path.write_bytes(app1(xml('o:OLivePhotoVersion="2"'))+tail);log=run([ref,path,folder]);reference=json.JSONDecoder().raw_decode(log.split('REFERENCE_JSON\t')[1])[0];actual=json.loads(run([ROOT/'target/debug/examples/core_probe',path,folder]));assert reference['file']['video_length']==len(movie) and actual['video_length']==len(movie),(reference,actual);assert (folder/'rust-video.bin').read_bytes()==movie;assert (folder/'reference-file-video.bin').read_bytes()==movie;rows.append({'name':name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'reference':reference,'implementation':actual,'primary_and_auxiliary_not_confused':True})
 remux=[]
 for name in ['motion.mp4','apple-paired.mov']:
  src=ROOT/'web/public/demo'/name;dst=out/('optimized-'+name);report=json.loads(run([ROOT/'target/debug/examples/remux_probe','optimize',src,dst]));assert packets(src)==packets(dst);remux.append({'name':name,'input_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'output_sha256':hashlib.sha256(dst.read_bytes()).hexdigest(),'encoded_packets_and_exact_timing_equal':True,'report':report})
 # Import the project's H.264 track with its B-frame composition offsets.
 src=ROOT/'web/public/demo/motion.mp4';dst=out/'imported.mp4';report=json.loads(run([ROOT/'target/debug/examples/remux_probe','import',src,dst,src,'1']));original=packets(src);after=packets(dst);assert after==original+original;remux.append({'name':'same-input-two-video-import','encoded_packets_and_exact_timing_equal':True,'report':report})
 receipt={'reference_sha':SHA,'implementation_sha':run(['git','-C',ROOT,'rev-parse','HEAD']).strip(),'working_tree_diff_sha256':hashlib.sha256(run(['git','-C',ROOT,'diff','HEAD']).encode()).hexdigest(),'oplus':rows,'remux':remux,'limits':['Google memory extractor may include JXRS footer; file extraction is compared separately.','No native vendor Gallery acceptance.','hvc1 structural test is separate from this real H.264 packet run.']};(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(out/'receipt.json')
if __name__=='__main__':main()
