#!/usr/bin/env python3
"""Actual C ABI calls through the produced native library; no mocked response."""
import ctypes,json,platform,struct,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 subprocess.run(['cargo','build','--locked','-p','liblivephoto-native'],cwd=ROOT,check=True);name={'Darwin':'liblivephoto_native.dylib','Windows':'livephoto_native.dll'}.get(platform.system(),'liblivephoto_native.so');library=ctypes.CDLL(str(ROOT/'target/debug'/name));pointer=ctypes.c_void_p;size=ctypes.c_size_t
 library.llp_process.argtypes=[pointer,size,pointer,size,pointer,size];library.llp_process.restype=pointer;library.llp_response_len.argtypes=[pointer];library.llp_response_len.restype=size;library.llp_response_data.argtypes=[pointer];library.llp_response_data.restype=pointer;library.llp_response_free.argtypes=[pointer]
 def execute(request,source):
  request=json.dumps(request).encode();r=ctypes.create_string_buffer(request);s=ctypes.create_string_buffer(source);response=library.llp_process(r,len(request),s,len(source),None,0)
  try:
   data=ctypes.string_at(library.llp_response_data(response),library.llp_response_len(response));assert data[:4]==b'LLP1';length=struct.unpack('>I',data[4:8])[0];return json.loads(data[8:8+length])
  finally:library.llp_response_free(response)
 assert execute({'action':'version'},b'')['success'];result=execute({'action':'parse'},(ROOT/'web/public/demo/motion-photo.jpg').read_bytes());assert result['success'];assert result['metadata']['photoTime']=={'value':'812500','timescale':1000000};assert not execute({'action':'parse'},b'invalid')['success'];print('C_ABI_SMOKE passed: actual version, motion parse, exact time, error and response ownership')
if __name__=='__main__':main()
