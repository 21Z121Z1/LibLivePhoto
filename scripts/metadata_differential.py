#!/usr/bin/env python3
"""Provider controls/metadata-only/raw-block differential with owned test media."""
import argparse,hashlib,json,shlex,struct,subprocess,tempfile
from pathlib import Path
from package_native import source_identity
ROOT=Path(__file__).resolve().parents[1];SHA='12fb3cb8d1649a5621c55dcf99e416d32e4c4ab3'
def run(args,**kw):
 r=subprocess.run(list(map(str,args)),text=True,capture_output=True,**kw)
 if r.returncode:raise RuntimeError(r.stderr or r.stdout)
 return r.stdout
def jpeg(xml,tail=b''):
 data=(ROOT/'web/public/demo/cover.jpg').read_bytes();p=b'http://ns.adobe.com/xap/1.0/\0'+xml.encode();return data[:2]+b'\xff\xe1'+struct.pack('>H',len(p)+2)+p+data[2:]+tail
def xmp(properties):return '<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description '+properties+'/></rdf:RDF></x:xmpmeta>'
def sef(movie,version=107):
 name=b'MotionPhoto_Data';record=struct.pack('<HHI',0,0x0a30,len(name))+name+movie
 directory=b'SEFH'+struct.pack('<IIHHII',version,1,0,0x0a30,len(record),len(record));return record+directory+struct.pack('<I',len(directory))+b'SEFT'
def tlv(payload,tag):return payload+struct.pack('<I',len(payload))+tag.encode().ljust(20,b'\0')
def heic_xmp(xml):
 # Only the project's known own HEIC fixture is used; no general writer claim.
 data=(ROOT/'web/public/acceptance/still.heic').read_bytes()
 def box(tag,p):return struct.pack('>I',len(p)+8)+tag+p
 def boxes(start,end):
  result={}
  while start<end:
   size=int.from_bytes(data[start:start+4],'big');assert size>=8 and start+size<=end
   result[data[start+4:start+8]]=(start,start+size);start+=size
  assert start==end;return result
 top=boxes(0,len(data));m=top[b'meta'];children=boxes(m[0]+12,m[1]);ftyp=data[slice(*top[b'ftyp'])];coded=data[top[b'mdat'][0]+8:top[b'mdat'][1]];x=xml.encode()
 old_iinf=data[slice(*children[b'iinf'])];infe=box(b'infe',b'\2\0\0\0'+struct.pack('>HH',2,0)+b'mimeopcamera\0application/rdf+xml\0\0')
 iinf=box(b'iinf',bytes(4)+struct.pack('>H',2)+old_iinf[14:]+infe)
 def meta(offset):
  entries=b''.join(struct.pack('>HHHII',index,0,1,start,length) for index,start,length in [(1,offset,len(coded)),(2,offset+len(coded),len(x))])
  iloc=box(b'iloc',bytes(4)+b'\x44\0'+struct.pack('>H',2)+entries)
  iref=box(b'iref',bytes(4)+box(b'cdsc',struct.pack('>HHH',2,1,1)))
  return box(b'meta',bytes(4)+data[slice(*children[b'hdlr'])]+data[slice(*children[b'pitm'])]+iloc+iinf+iref+data[slice(*children[b'iprp'])])
 offset=len(ftyp)+len(meta(0))+8
 return ftyp+meta(offset)+box(b'mdat',coded+x)
def semantic_checks(reference,actual,malformed_offline=False):
 checked=[]
 for block in reference['blocks']:
  semantic=block['semantic']
  if semantic is None:continue
  name=semantic['provider'];matches=[p for p in actual['decoded'] if p['provider']==name]
  if name in ['legacy_micro_video','oplus_olive'] and not block['declaresMotion']:continue
  if not matches and malformed_offline and name=='honor_live_photo':
   assert any('honor_live_photo' in diagnostic for diagnostic in actual['diagnostics'])
   continue
  assert matches,(name,'reference provider absent from Rust projection')
  if name=='legacy_micro_video':
   camera=matches[0]['value']['camera'];assert int(camera['MicroVideo'])==semantic['flag'];assert int(camera['MicroVideoVersion'])==semantic['version'];assert camera['MicroVideoOffset']==semantic['offset'];assert camera['MicroVideoPresentationTimestampUs']==semantic['timeUs']
  elif name=='oplus_olive':
   values=[p['value'] for p in matches if p['value'].get('source')=='xmp-declaration']
   if values:
    value=values[0];assert value['version']==semantic['version'];assert value['declaredVideoLength']==semantic['videoLength'];assert value['photoTime']['value']==semantic['timeUs']
  elif name=='samsung_sef':
   value=matches[0]['value'];assert value['version']==semantic['version'];assert len(value['records'])==semantic['entries'];assert sorted(r['name'] for r in value['records'])==sorted(semantic['keys']);assert (value['motionPhotoDataMagic'] or '')==semantic['magic']
  elif name=='honor_live_photo':
   value=matches[0]['value'];assert (value['sceneMode'] or 0)==semantic['sceneMode'];assert sorted(r['tag'] for r in value['records'])==sorted(semantic['tags'])
   if value['photoTime'] is not None:assert value['photoTime']['value']==semantic['timeUs']
  if name in ['samsung_sef','honor_live_photo'] and int(semantic['videoLength']):
   assert any(sum(int(e['end'])-int(e['start']) for e in p['videoExtents'])==int(semantic['videoLength']) for p in matches)
  checked.append(name)
 return checked
def main():
 p=argparse.ArgumentParser();p.add_argument('--reference-source',type=Path,required=True);p.add_argument('--reference-build',type=Path,required=True);p.add_argument('--protobuf-include',type=Path);a=p.parse_args();source=a.reference_source.resolve();assert run(['git','-C',source,'rev-parse','HEAD']).strip()==SHA
 out=Path(tempfile.mkdtemp(prefix='metadata-diff-',dir=ROOT/'evidence/local'));build=a.reference_build.resolve()/'lib/core';link=shlex.split((build/'CMakeFiles/motion_photo_tests.dir/link.txt').read_text());ref=out/'reference'
 includes=['-I'+str(source/'lib/core/motion_photo/includes'),'-I'+str(source/'lib/core/third_party/image_io/includes'),'-I'+str(build)]
 if a.protobuf_include:includes.append('-I'+str(a.protobuf_include))
 run(['c++','-std=c++17',*includes,ROOT/'scripts/metadata_reference_probe.cc','-o',ref,*link[link.index('-o')+2:]],cwd=build);run(['cargo','build','--locked','-p','liblivephoto-cli'],cwd=ROOT);cli=ROOT/'target/debug/llp'
 movie=(ROOT/'web/public/demo/motion.mp4').read_bytes();still=(ROOT/'web/public/demo/cover.jpg').read_bytes();legacy='xmlns:c="http://ns.google.com/photos/1.0/camera/" c:MicroVideo="1" c:MicroVideoVersion="1" c:MicroVideoOffset="'+str(len(movie))+'" c:MicroVideoPresentationTimestampUs="812500"';op='xmlns:o="http://ns.oplus.com/photo/1.0/camera/" o:OLivePhotoVersion="2" o:VideoLength="'+str(len(movie))+'" o:PrimaryPresentationTimestampUs="812500"'
 cases=[('ordinary.jpg',still,False),('google.jpg',(ROOT/'web/public/demo/motion-photo.jpg').read_bytes(),True),('ordinary.heic',(ROOT/'web/public/acceptance/still.heic').read_bytes(),False),('legacy.jpg',jpeg(xmp(legacy),movie),True),('legacy-metadata-only.jpg',jpeg(xmp(legacy)),True),('oplus.jpg',jpeg(xmp(op),movie),True),('oplus-metadata-only.jpg',jpeg(xmp(op)),True),('samsung-raw.jpg',still+sef(movie,106),True)]
 cases.extend([('legacy-canonical.jpg',jpeg(xmp(legacy.replace('c:','GCamera:').replace('xmlns:c=','xmlns:GCamera=')),movie),True),('oplus-canonical.jpg',jpeg(xmp(op.replace('o:','OpCamera:').replace('xmlns:o=','xmlns:OpCamera=')),movie),True),('oplus-metadata-only.heic',heic_xmp(xmp(op.replace('o:','OpCamera:').replace('xmlns:o=','xmlns:OpCamera='))),True)])
 op_heic=heic_xmp(xmp(op.replace('o:','OpCamera:').replace('xmlns:o=','xmlns:OpCamera=')))
 cases.append(('oplus-mpvd.heic',op_heic+struct.pack('>I',len(movie)+8)+b'mpvd'+movie,True))
 for magic in [b'mpv2',b'mpv3']:cases.append(('samsung-'+magic.decode()+'.jpg',still+sef(magic+struct.pack('>II',100,len(movie))+movie),True))
 for suffix in ['LIVE_'+str(len(movie)),'450'.ljust(20,'\0')+'LIVE_'+str(len(movie))]:cases.append(('honor-'+str(len(suffix))+'.jpg',still+movie+suffix.encode().ljust(20 if len(suffix)<20 else 40,b'\0'),True))
 offline=b'{"common":{"sceneMode":73}}'
 cases.extend([('honor-tlv.jpg',still+tlv(movie,'HiHonor_1000000')+tlv(b'0001','HiHonor_CIID'),True),('honor-metadata-only.jpg',still+tlv(struct.pack('<I',len(offline))+offline,'HiHonor_OfflineData')+tlv(b'0001','HiHonor_CIID'),False),('honor-invalid-offline-length.jpg',still+tlv(offline,'HiHonor_OfflineData')+tlv(b'0001','HiHonor_CIID'),False)])
 rows=[]
 for name,content,expected in cases:
  path=out/name;path.write_bytes(content)
  for mode in ['all','none']:
   folder=out/(name+'-'+mode);folder.mkdir();raw=folder/'reference.raw';reference=json.JSONDecoder().raw_decode(run([ref,path,raw,mode]).split('METADATA_REFERENCE_JSON\t')[1])[0]
   args=[cli,'metadata',path]+(['--disable-3p'] if mode=='none' else []);actual=json.loads(run(args))['metadata'];declarations=any(p['motionDeclaration'] for p in actual['decoded']);google=reference['declaresMotion'];want=expected and (mode=='all' or name=='google.jpg')
   (folder/'reference.json').write_text(json.dumps(reference,indent=2)+'\n');(folder/'implementation.json').write_text(json.dumps(actual,indent=2)+'\n')
   assert declarations==want,(name,mode,actual)
   if name not in ['legacy.jpg','legacy-metadata-only.jpg','oplus.jpg','oplus-metadata-only.jpg','oplus-metadata-only.heic']:assert google==want,(name,mode,reference)
   # Classifications may differ for malformed/metadata-only vendor signatures;
   # retain both facts and never equate either to validated media.
   blocks=json.loads(run([cli,'extract-metadata',path,'--out',folder/'rust',*(['--disable-3p'] if mode=='none' else [])]));raw_files=[(folder/'rust'/f['name']).read_bytes() for f in blocks['files']]
   assert len(raw_files)==len(actual['blocks'])
   for raw_file,block in zip(raw_files,actual['blocks']):assert raw_file==b''.join(content[int(e['start']):int(e['end'])] for e in block['extents'])
   raw_match=raw.read_bytes() in raw_files if reference['extracted'] else None
   if reference['extracted']:assert raw_match,(name,mode,'reference raw metadata not present among complete source blocks')
   try:checked=semantic_checks(reference,actual,name=='honor-invalid-offline-length.jpg')
   except AssertionError as error:raise AssertionError((name,mode,str(folder),str(error))) from error
   rows.append({'case':name,'mode':mode,'sha256':hashlib.sha256(content).hexdigest(),'reference':reference,'implementation':{'declaresMotion':declarations,'verifiedMotion':actual['verifiedMotion'],'blocks':len(actual['blocks']),'diagnostics':actual['diagnostics']},'declaration_equal':google==declarations,'reference_raw_block_preserved':raw_match,'provider_semantic_fields_checked':checked,'malformedOfflineLengthRejectedWithoutDroppingRawBlock':name=='honor-invalid-offline-length.jpg' and mode=='all'})
 receipt={'reference_sha':SHA,**source_identity(),'cases':rows,'limits':['Synthetic provider contracts plus real own coded scene; not an original-device corpus or native Gallery acceptance.','Raw metadata extraction emits explicit complete source blocks; proprietary blocks can include coded media.','Declarations are not validated motion assets; Rust reports these independently.','Pinned Google does not classify namespace-equivalent c:/o: aliases in these legacy/OpCamera attribute cases; canonical GCamera:/OpCamera: cases are checked independently.','Pinned HeifContainerSplitter::Split emits XMP only when mpvd exists; Rust also exposes ordinary-image metadata and explicitly distinguishes declarations from verified motion.','For malformed Honor OfflineData with a missing length prefix, Google skips four bytes and scans for sceneMode; Rust reports malformed-trailer diagnostics and retains the full raw source block. A separate correctly length-prefixed metadata-only case verifies sceneMode and tag semantics.']};(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(out/'receipt.json')
if __name__=='__main__':main()
