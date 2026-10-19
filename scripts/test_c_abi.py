#!/usr/bin/env python3
"""Actual C ABI calls through the produced native library; no mocked response."""
import ctypes,json,platform,struct,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 subprocess.run(['cargo','build','--locked','-p','liblivephoto-native'],cwd=ROOT,check=True);name={'Darwin':'liblivephoto_native.dylib','Windows':'livephoto_native.dll'}.get(platform.system(),'liblivephoto_native.so');library=ctypes.CDLL(str(ROOT/'target/debug'/name));pointer=ctypes.c_void_p;size=ctypes.c_size_t
 library.llp_process.argtypes=[pointer,size,pointer,size,pointer,size];library.llp_process.restype=pointer;library.llp_response_len.argtypes=[pointer];library.llp_response_len.restype=size;library.llp_response_data.argtypes=[pointer];library.llp_response_data.restype=pointer;library.llp_response_free.argtypes=[pointer]
 def execute(request,source,movie=b''):
  source=bytes(source);movie=bytes(movie)
  request=json.dumps(request).encode();r=ctypes.create_string_buffer(request);s=ctypes.create_string_buffer(source);m=ctypes.create_string_buffer(movie);response=library.llp_process(r,len(request),s,len(source),m,len(movie))
  try:
   data=ctypes.string_at(library.llp_response_data(response),library.llp_response_len(response));assert data[:4]==b'LLP1';length=struct.unpack('>I',data[4:8])[0];return json.loads(data[8:8+length])
  finally:library.llp_response_free(response)
 assert execute({'action':'version'},b'')['success'];result=execute({'action':'parse'},(ROOT/'web/public/demo/motion-photo.jpg').read_bytes());assert result['success'];assert result['metadata']['photoTime']=={'value':'812500','timescale':1000000};assert not execute({'action':'parse'},b'invalid')['success']
 source=(ROOT/'web/public/demo/motion.mp4').read_bytes();report=execute({'action':'conformance','profile':'google-video-container-1.0','trackIds':[1]},source);assert report['metadata']['profile']=='google-video-container-1.0-declarations';assert report['metadata']['checkedRules'];assert 'no codec decoding' in report['metadata']['scope'];assert not execute({'action':'conformance','profile':'google-video-container-1.0','trackIds':[1,1]},source)['success']
 album=execute({'action':'metadata'},(ROOT/'fixtures/bindings/camera-album.jpg').read_bytes());assert album['success'];assert not album['metadata']['verifiedMotion'];provider=next(d for d in album['metadata']['decoded'] if d['provider']=='camera_album');assert provider['value']['imageTime']=='31';assert provider['value']['pairingVerified'] is False;assert '1.234567890123456789e100' in provider['value']['rawJson']
 vivo=execute({'action':'metadata'},(ROOT/'fixtures/bindings/vivo-stream-motion.jpg').read_bytes());assert vivo['success'];meta=vivo['metadata'];d=next(d for d in meta['decoded'] if d['provider']=='vivo_media')['value'];assert meta['verifiedMotion'];assert len(d['streams'])==3;assert d['duplicateKeys'];assert not d['streamSemanticsInterpreted'];assert not d['pairingVerified'];assert meta['blocks'][d['streams'][0]['blockIndex']]['rawBytesHex']=='00ff6f7061717565'
 syntax=ROOT/'fixtures/syntax';icc=(syntax/'icc-structure-v4.4.icc').read_bytes()
 good=execute({'action':'conformance','profile':'icc-structure-v4.4'},icc);assert good['success'];assert good['metadata']['profile']=='icc.1-2022-v4.4-structure'
 unsupported=execute({'action':'conformance','profile':'icc-structure-v4.4'},(syntax/'icc-unsupported-v4.3.icc').read_bytes());assert not unsupported['success'];assert unsupported['metadata']['issues'][0]['kind']=='Unsupported'
 assert not execute({'action':'conformance','profile':'icc-structure-v4.4'},icc[:-1])['success']
 assert not execute({'action':'conformance','profile':'icc-structure-v4.4'},bytes(16*1024*1024+1))['success']
 movie=(syntax/'aac-pce-wave.mov').read_bytes();parsed=execute({'action':'parse','inputKind':'parts'},(ROOT/'web/public/demo/cover.jpg').read_bytes(),movie);assert parsed['success']
 audio=next(d['audio']['aac'] for d in parsed['metadata']['sampleDescriptions'] if d['audio']);assert audio['channelConfiguration']==0 and audio['channels']==2 and audio['sampleRate']==48000
 assert audio['programConfig']['front']==[{'isPair':True,'tag':0}] and not audio['programConfig']['speakerPositionsInferred']
 bad=bytearray(movie);assert bad.count(b'esds')==1;bad[bad.index(b'esds')+4]=1
 assert not execute({'action':'conformance','profile':'google-video-container-1.0','trackIds':[1]},bad)['success']
 from syntax_contracts import NativeCore,run_cases
 run_cases(NativeCore(ROOT/'target/debug'/name),'host-c-abi')
 print('C_ABI_SMOKE passed: actual version, motion parse, exact time, error, ownership, conformance opaque vivo bytes, ICC valid/unsupported/malformed and actual AAC-LC PCE/wave (no rendering/device certification)')
if __name__=='__main__':main()
