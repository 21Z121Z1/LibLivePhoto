#!/usr/bin/env python3
"""Create a local static XCFramework; no signing, release or publishing."""
import hashlib,json,os,shutil,subprocess
from pathlib import Path
from package_native import dependency_notices, source_identity
ROOT=Path(__file__).resolve().parents[1]
TARGETS=['aarch64-apple-ios','aarch64-apple-ios-sim','x86_64-apple-ios']
def build_env():return {key:value for key,value in os.environ.items() if key!='SDKROOT'}
def run(args,env=None):subprocess.run(list(map(str,args)),cwd=ROOT,check=True,env=build_env() if env is None else env)
def target_sdk(target):
    name='iphoneos' if target=='aarch64-apple-ios' else 'iphonesimulator'
    path=subprocess.check_output(['xcrun','--sdk',name,'--show-sdk-path'],text=True,env=build_env()).strip()
    if not Path(path).name.lower().startswith(name):raise RuntimeError('wrong SDK selected for '+target+': '+path)
    return path
def main():
    out=ROOT/'dist/apple';out.mkdir(parents=True,exist_ok=True)
    sdks={target:target_sdk(target) for target in TARGETS}
    for target in TARGETS:run(['cargo','build','--locked','--release','-p','liblivephoto-native','--target',target],env={**build_env(),'SDKROOT':sdks[target],'IPHONEOS_DEPLOYMENT_TARGET':'16.0'})
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
             'targets':TARGETS,'target_sdks':sdks,'deployment_target':'16.0','xcode':subprocess.check_output(['xcodebuild','-version'],text=True).strip(),
             'files':{str(p.relative_to(framework)):hashlib.sha256(p.read_bytes()).hexdigest() for p in framework.rglob('*') if p.is_file()},
             'signing':'static library; host signs its final application','device_runtime_acceptance':False}
    (out/'xcframework-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(framework)
if __name__=='__main__':main()
