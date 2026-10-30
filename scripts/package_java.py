#!/usr/bin/env python3
"""Build a Java-8 API jar with a full JDK. No publishing or dependency manager."""
import argparse,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 p=argparse.ArgumentParser();p.add_argument('--compiler-java-home');a=p.parse_args();build=ROOT/'java/build';classes=build/'library-classes';classes.mkdir(parents=True,exist_ok=True);java=['java']+(['-Djava.home='+a.compiler_java_home]if a.compiler_java_home else[])
 subprocess.run([*java,'com.sun.tools.javac.Main','--release','8','-d',str(classes),*[str(p)for p in sorted((ROOT/'java/src/main/java').rglob('*.java'))]],check=True)
 jar=build/'liblivephoto-java.jar';subprocess.run([*java,'sun.tools.jar.Main','--create','--file',str(jar),'-C',str(classes),'.'],check=True);print(jar)
if __name__=='__main__':main()
