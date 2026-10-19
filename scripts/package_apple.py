#!/usr/bin/env python3
"""Create a local static XCFramework; no signing, release or publishing."""
import hashlib,json,shutil,subprocess
from pathlib import Path
from package_native import dependency_notices, source_identity
ROOT=Path(__file__).resolve().parents[1]
TARGETS=['aarch64-apple-ios','aarch64-apple-ios-sim','x86_64-apple-ios']
def run(args):subprocess.run(list(map(str,args)),cwd=ROOT,check=True)
def main():
    out=ROOT/'dist/apple';out.mkdir(parents=True,exist_ok=True)
    for target in TARGETS:run(['cargo','build','--locked','--release','-p','liblivephoto-native','--target',target])
    simulator=out/'liblivephoto_simulator.a'
    run(['xcrun','lipo','-create',ROOT/'target/aarch64-apple-ios-sim/release/liblivephoto_native.a',ROOT/'target/x86_64-apple-ios/release/liblivephoto_native.a','-output',simulator])
    framework=out/'LibLivePhotoCore.xcframework'
    if framework.exists():raise RuntimeError('refusing to overwrite an existing XCFramework: '+str(framework))
    run(['xcodebuild','-create-xcframework','-library',ROOT/'target/aarch64-apple-ios/release/liblivephoto_native.a','-headers',ROOT/'native/include','-library',simulator,'-headers',ROOT/'native/include','-output',framework])
    license_files,licenses=dependency_notices()
    for name,path in license_files.items():
        dest=framework/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(path,dest)
    shutil.copyfile(ROOT/'LICENSE',framework/'LICENSE')
    (framework/'dependency-notices.json').write_text(json.dumps(licenses,indent=2)+'\n')
    receipt={**source_identity(),
             'targets':TARGETS,'xcode':subprocess.check_output(['xcodebuild','-version'],text=True).strip(),
             'files':{str(p.relative_to(framework)):hashlib.sha256(p.read_bytes()).hexdigest() for p in framework.rglob('*') if p.is_file()},
             'signing':'static library; host signs its final application','device_runtime_acceptance':False}
    (out/'xcframework-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(framework)
if __name__=='__main__':main()
