#!/usr/bin/env python3
"""Build and actually run the maintained host JVM/JNI client. No uploads."""
import argparse,json,os,platform,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(a):subprocess.run(list(map(str,a)),cwd=ROOT,check=True)
def main():
 p=argparse.ArgumentParser();p.add_argument('--compiler-java-home');a=p.parse_args();run(['cargo','build','--locked','-p','liblivephoto-native']);out=ROOT/'java/build/classes';out.mkdir(parents=True,exist_ok=True)
 sources=sorted((ROOT/'java/src/main/java').rglob('*.java'))+sorted((ROOT/'java/src/test/java').rglob('*.java'));java=['java']+(['-Djava.home='+a.compiler_java_home]if a.compiler_java_home else[])
 run([*java,'com.sun.tools.javac.Main','--release','8','-d',out,*sources]);name={'Darwin':'liblivephoto_native.dylib','Windows':'livephoto_native.dll'}.get(platform.system(),'liblivephoto_native.so');library=ROOT/'target/debug'/name
 run(['java','-Dliblivephoto.native.path='+str(library),'-cp',out,'org.liblivephoto.NativeSmoke',ROOT/'web/public/demo']);print(json.dumps({'library':str(library),'java_release':8,'smoke':'passed'}))
if __name__=='__main__':main()
