#!/usr/bin/env python3
"""Run Swift->static Rust in a newly created, owned iOS simulator."""
import json,platform,shutil,subprocess,uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(args,**kwargs):return subprocess.check_output(list(map(str,args)),cwd=ROOT,text=True,**kwargs)
def main():
    out=ROOT/'dist/apple/simulator-acceptance';out.mkdir(parents=True,exist_ok=True)
    arch='arm64' if platform.machine()=='arm64' else 'x86_64';rust='aarch64-apple-ios-sim' if arch=='arm64' else 'x86_64-apple-ios'
    sdk=run(['xcrun','--sdk','iphonesimulator','--show-sdk-path']).strip()
    # Swift allows top-level executable statements in main.swift.
    source=out/'main.swift';shutil.copyfile(ROOT/'apple/Acceptance/BridgeSimulator.swift',source)
    binary=out/'native-bridge'
    run(['xcrun','swiftc','-sdk',sdk,'-target',arch+'-apple-ios16.0-simulator',ROOT/'apple/Sources/LibLivePhotoApple/Contracts.swift',ROOT/'apple/Sources/LibLivePhotoApple/RustCoreBridge.swift',source,
         '-Xlinker','-force_load','-Xlinker',ROOT/'target'/rust/'release/liblivephoto_native.a','-Xlinker','-export_dynamic','-o',binary])
    runtimes=json.loads(run(['xcrun','simctl','list','runtimes','--json']))['runtimes']
    available=[r for r in runtimes if r.get('isAvailable') and 'SimRuntime.iOS-' in r['identifier']]
    if not available:raise RuntimeError('installed Xcode has no available iOS Simulator runtime; SDK compile alone is not runtime acceptance')
    runtime=sorted(available,key=lambda r:tuple(map(int,r['version'].split('.'))))[-1]
    types=json.loads(run(['xcrun','simctl','list','devicetypes','--json']))['devicetypes']
    phones=[t for t in types if t['name'].startswith('iPhone')]
    device=run(['xcrun','simctl','create','LLP-'+str(uuid.uuid4()),phones[-1]['identifier'],runtime['identifier']]).strip()
    try:
        run(['xcrun','simctl','boot',device]);run(['xcrun','simctl','bootstatus',device,'-b'],timeout=300)
        text=run(['xcrun','simctl','spawn',device,binary,ROOT/'web/public/demo/motion-photo.jpg',ROOT/'web/public/demo/motion.mp4',ROOT/'fixtures/bindings/agtm-video.mp4',ROOT/'fixtures/bindings/agtm-raw.bin'],timeout=120)
        (out/'runtime.log').write_text(text);print(text)
        if 'IOS_SIMULATOR_SWIFT_RUST passed' not in text:raise RuntimeError('native bridge process did not pass')
        receipt={'implementation_sha':run(['git','rev-parse','HEAD']).strip(),'runtime':runtime['identifier'],'architecture':arch,'result':'passed','scope':'actual Swift plus statically linked Rust C ABI in owned iOS Simulator','photos_library_import_edit':False,'physical_playback_hdr':False}
        (out/'runtime-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    finally:
        subprocess.run(['xcrun','simctl','shutdown',device],check=False)
        subprocess.run(['xcrun','simctl','delete',device],check=False)
if __name__=='__main__':main()
