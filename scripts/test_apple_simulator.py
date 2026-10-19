#!/usr/bin/env python3
"""Run Swift->static Rust in a newly created, owned iOS simulator."""
import argparse,json,os,platform,re,shutil,subprocess,uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(args,**kwargs):
    kwargs.setdefault('timeout',180)
    # Hosted runners can export a macOS SDKROOT, including when xcrun is asked
    # for another SDK. Keep SDK selection explicit in this child process only.
    kwargs.setdefault('env',{key:value for key,value in os.environ.items() if key!='SDKROOT'})
    return subprocess.check_output(list(map(str,args)),cwd=ROOT,text=True,**kwargs)
def tail(path,limit=16000):
    with path.open('rb') as source:
        source.seek(max(0,path.stat().st_size-limit))
        return source.read(limit).decode('utf-8',errors='replace')
def logged(args,path,timeout):
    print('+ '+' '.join(map(str,args)),flush=True)
    try:
        with path.open('w') as output:
            subprocess.run(list(map(str,args)),cwd=ROOT,stdout=output,stderr=subprocess.STDOUT,check=True,timeout=timeout,
                           env={key:value for key,value in os.environ.items() if key!='SDKROOT'})
    finally:
        print(tail(path),flush=True)
def owned_state(device):
    devices=json.loads(run(['xcrun','simctl','list','devices','--json'],timeout=30))['devices']
    return next((item for group in devices.values() for item in group if item.get('udid')==device),None)
def cleanup(device,path):
    deleted=False
    with path.open('w') as output:
        for action in ['shutdown','delete']:
            try:
                result=subprocess.run(['xcrun','simctl',action,device],stdout=output,stderr=subprocess.STDOUT,timeout=30,check=False)
                if action=='delete':deleted=result.returncode==0
            except (subprocess.SubprocessError,OSError) as error:
                output.write(str(error)+'\n')
    print(tail(path,4000),flush=True)
    return deleted
def diagnostics(device,out):
    path=out/'CoreSimulator.log'
    try:
        with path.open('w') as output:
            subprocess.run(['/usr/bin/log','show','--last','8m','--style','compact','--predicate',
                            'process == "CoreSimulatorService" AND eventMessage CONTAINS "'+device+'"'],
                           stdout=output,stderr=subprocess.STDOUT,timeout=20,check=False)
    except (subprocess.SubprocessError,OSError) as error:
        with path.open('a') as output:output.write(str(error)+'\n')
    print(tail(path,6000),flush=True)
def main(argv=None):
    parser=argparse.ArgumentParser();parser.add_argument('--device-type');parser.add_argument('--runtime');args=parser.parse_args(argv)
    out=ROOT/'dist/apple/simulator-acceptance';out.mkdir(parents=True,exist_ok=True)
    # A valid new invocation must not leave an earlier success receipt current
    # when SDK/runtime discovery fails before a device can even be created.
    (out/'runtime-receipt.json').write_text(json.dumps({'result':'not_executed','scope':'preparing owned iOS Simulator acceptance'},indent=2)+'\n')
    arch='arm64' if platform.machine()=='arm64' else 'x86_64';rust='aarch64-apple-ios-sim' if arch=='arm64' else 'x86_64-apple-ios'
    sdk=run(['xcrun','--sdk','iphonesimulator','--show-sdk-path']).strip()
    if not Path(sdk).name.lower().startswith('iphonesimulator'):
        raise RuntimeError('xcrun did not select an iPhoneSimulator SDK: '+sdk)
    # Swift allows top-level executable statements in main.swift.
    source=out/'main.swift';shutil.copyfile(ROOT/'apple/Acceptance/BridgeSimulator.swift',source)
    binary=out/'native-bridge'
    run(['xcrun','swiftc','-sdk',sdk,'-target',arch+'-apple-ios16.0-simulator',ROOT/'apple/Sources/LibLivePhotoApple/Contracts.swift',ROOT/'apple/Sources/LibLivePhotoApple/RustCoreBridge.swift',source,
         '-Xlinker','-force_load','-Xlinker',ROOT/'target'/rust/'release/liblivephoto_native.a','-Xlinker','-export_dynamic','-o',binary],env={**os.environ,'SDKROOT':sdk})
    runtimes=json.loads(run(['xcrun','simctl','list','runtimes','--json']))['runtimes']
    available=[r for r in runtimes if r.get('isAvailable') and 'SimRuntime.iOS-' in r['identifier'] and tuple(map(int,r['version'].split('.')))>=(16,0)]
    if not available:raise RuntimeError('installed Xcode has no available iOS Simulator runtime; SDK compile alone is not runtime acceptance')
    if args.runtime:
        runtime=next((r for r in available if r['identifier']==args.runtime),None)
        if runtime is None:raise RuntimeError('requested runtime is unavailable or below iOS16 deployment target')
    else:
        # Exercise the deployment floor on an already installed runtime. The
        # latest hosted iOS26.5 image stalled in CoreLocation data migration on
        # two device types; no runtime is downloaded or migration disabled here.
        runtime=min(available,key=lambda r:tuple(map(int,r['version'].split('.'))))
    print('Compiler SDK and installed compatible runtimes:',sdk,[(r['identifier'],r['version']) for r in available],flush=True)
    types=json.loads(run(['xcrun','simctl','list','devicetypes','--json']))['devicetypes']
    version=[int(n) for n in runtime['version'].split('.')];version=(version+[0,0])[:3]
    encoded_version=(version[0]<<16)|(version[1]<<8)|version[2]
    phones=[t for t in types if t['name'].startswith('iPhone') and t.get('minRuntimeVersion',0)<=encoded_version<=t.get('maxRuntimeVersion',0xffffffff)]
    if not phones:raise RuntimeError('no installed iPhone device type supports '+runtime['identifier'])
    if args.device_type:
        selected=next((t for t in phones if t['identifier']==args.device_type),None)
        if selected is None:raise RuntimeError('requested device type is unavailable or incompatible with '+runtime['identifier'])
    else:
        # Two cold boots of iPhone 17e stalled on the hosted runner. Try a
        # different compatible configuration without changing runtime/API scope.
        candidates=[t for t in phones if 'SE' in t['name']] or phones
        selected=max(candidates,key=lambda t:(t.get('minRuntimeVersion',0),tuple(map(int,re.findall(r'\d+',t['name'])))))
    print('Owned simulator type/runtime:',selected['identifier'],runtime['identifier'],flush=True)
    device=run(['xcrun','simctl','create','LLP-'+str(uuid.uuid4()),selected['identifier'],runtime['identifier']]).strip()
    receipt={'implementation_sha':run(['git','rev-parse','HEAD']).strip(),'runtime':runtime['identifier'],'architecture':arch,
             'device':device,'device_type':selected['identifier'],'result':'failed','scope':'actual Swift plus statically linked Rust C ABI in owned iOS Simulator',
             'bootstatus_completed':False,'runtime_process_passed':False,'springboard_ui_acceptance':False,
             'photos_library_import_edit':False,'physical_playback_hdr':False,
             'runtime_available':runtime.get('isAvailable'),'runtime_version':runtime['version'],
             'compiler_sdk':sdk,'available_ios_runtimes':[{'identifier':r['identifier'],'version':r['version']} for r in available],
             'compatible_device_types':[t['identifier'] for t in phones]}
    try:
        run(['xcrun','simctl','boot',device],timeout=30)
        try:
            logged(['xcrun','simctl','bootstatus',device,'-b'],out/'boot.log',300)
            receipt['bootstatus_completed']=True
        except subprocess.TimeoutExpired:
            # A monitor timeout alone does not prove that the runtime cannot
            # execute a process. Only recover if this owned device is actually
            # Booted; the binary must still run and pass every original assertion.
            # This command-line C ABI test establishes no UI readiness claim.
            try:state=owned_state(device)
            finally:diagnostics(device,out)
            receipt['boot_device_state']=state
            (out/'boot-state.json').write_text(json.dumps(state,indent=2)+'\n')
            if state is None or state.get('state')!='Booted':
                raise RuntimeError('bootstatus timed out and owned simulator is not Booted')
            print('Bootstatus monitor timed out; owned device is Booted. Attempting actual runtime process with its unchanged 120-second deadline.',flush=True)
        logged(['xcrun','simctl','spawn',device,binary,ROOT/'web/public/demo/motion-photo.jpg',ROOT/'web/public/demo/motion.mp4',ROOT/'fixtures/bindings/agtm-video.mp4',ROOT/'fixtures/bindings/agtm-raw.bin'],out/'runtime.log',120)
        text=tail(out/'runtime.log')
        if 'IOS_SIMULATOR_SWIFT_RUST passed' not in text or 'IOS_SIMULATOR_RUNTIME:' not in text:raise RuntimeError('native bridge process did not pass its runtime identity and assertions')
        receipt.update(result='passed',runtime_process_passed=True)
    except BaseException as error:
        receipt['error']=str(error)
        raise
    finally:
        receipt['owned_device_deleted']=cleanup(device,out/'cleanup.log')
        cleanup_failed=receipt['result']=='passed' and not receipt['owned_device_deleted']
        if cleanup_failed:receipt.update(result='failed',error='owned simulator deletion failed')
        (out/'runtime-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
        print(json.dumps(receipt,indent=2),flush=True)
        if cleanup_failed:raise RuntimeError('owned simulator deletion failed; see cleanup.log')
if __name__=='__main__':main()
