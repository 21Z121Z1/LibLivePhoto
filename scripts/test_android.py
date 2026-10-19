#!/usr/bin/env python3
"""Install only the generated acceptance APK and run actual Binder/JNI checks."""
import argparse, json, os, subprocess, time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
    p=argparse.ArgumentParser();p.add_argument('--adb',default='adb');p.add_argument('--serial');args=p.parse_args()
    adb=[args.adb]+(['-s',args.serial] if args.serial else [])
    def run(tail,**kw):return subprocess.check_output([*adb,*tail],text=True,**kw)
    devices=run(['devices']);print(devices,flush=True)
    run(['get-state'])
    # Refuse a pre-existing installation: never replace a user's application.
    package='org.liblivephoto.android.acceptance'
    existing=run(['shell','pm','list','packages',package]).splitlines()
    if 'package:'+package in [line.strip() for line in existing]:raise RuntimeError('test package already installed; refusing to overwrite: '+package)
    apk=ROOT/'android/build/liblivephoto-acceptance.apk';out=ROOT/'android/build'
    installed=False
    try:
        print(run(['install','--no-streaming',str(apk)]));installed=True
        text=run(['shell','am','instrument','-w',package+'/org.liblivephoto.android.ServiceAcceptance'],timeout=240)
        (out/'instrumentation.log').write_text(text);print(text)
        if 'ANDROID_SERVICE_ACCEPTANCE passed' not in text or 'INSTRUMENTATION_CODE: -1' not in text:
            raise RuntimeError('actual Android service acceptance failed; see instrumentation.log and logcat')
        receipt={'implementation_sha':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                 'device':run(['shell','getprop','ro.product.model']).strip(),
                 'sdk':run(['shell','getprop','ro.build.version.sdk']).strip(),
                 'abi':run(['shell','getprop','ro.product.cpu.abi']).strip(),
                 'hardware':run(['shell','getprop','ro.hardware']).strip(),
                 'scope':'installed APK, four remote services, isolated parser, actual Rust JNI, FD ranges/ownership, byte-exact extract/build, diagnostics/cancel',
                 'gallery_acceptance':False,'result':'passed'}
        (out/'runtime-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    finally:
        (out/'logcat.log').write_text(run(['logcat','-d','-s','LibLivePhotoAcceptance:*','AndroidRuntime:E','libc:F']))
        if installed:print(run(['uninstall',package]))
if __name__=='__main__':main()
