#!/usr/bin/env python3
"""Build a local host SDK archive, retaining each dependency's own notices.

This produces an artifact; it does not create a release or publish a package.
"""
import argparse,hashlib,json,os,platform,subprocess,sys,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(args):subprocess.run(list(map(str,args)),cwd=ROOT,check=True)
def source_identity():
    return {'implementation_sha':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
            'working_tree_clean':not bool(subprocess.check_output(['git','status','--porcelain','--untracked-files=normal'],cwd=ROOT,text=True).strip()),
            'tracked_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD','--binary'],cwd=ROOT)).hexdigest()}
def dependency_notices():
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=ROOT,text=True))
    files={};notices=[]
    for package in metadata['packages']:
        if package['source'] is None:continue
        directory=Path(package['manifest_path']).parent
        candidates={path for path in directory.iterdir() if path.is_file() and path.name.upper().startswith(('LICENSE','COPYING','NOTICE','COPYRIGHT'))}
        if package.get('license_file'):
            candidates.add(directory/package['license_file'])
        supplement=ROOT/'native/notices'/(package['name']+'-'+package['version'])
        if supplement.is_dir():candidates.update(p for p in supplement.iterdir() if p.is_file())
        if not candidates:raise RuntimeError('dependency notices missing: '+package['name']+' '+package['version'])
        prefix='notices/'+package['name']+'-'+package['version']+'/'
        for path in sorted(candidates):files[prefix+path.name]=path
        if package['name']=='cesu8':
            # This historical crate ships COPYRIGHT-RUST.txt and per-file
            # copyright/license headers, rather than LICENSE-* files.
            files[prefix+'README.md']=directory/'README.md'
            files[prefix+'source-lib.rs']=directory/'src/lib.rs'
        notices.append({'name':package['name'],'version':package['version'],'license':package['license'],
                        'repository':package['repository'],'files':[prefix+path.name for path in sorted(candidates)],
                        'scope':'locked dependency inventory; includes build/target-specific crates, not a claim every crate is linked on this host'})
    return files,notices
def main():
    p=argparse.ArgumentParser();p.add_argument('--compiler-java-home');a=p.parse_args()
    run(['cargo','build','--locked','--release','-p','liblivephoto-native','-p','liblivephoto-cli'])
    run([sys.executable,ROOT/'scripts/package_java.py',*(['--compiler-java-home',a.compiler_java_home] if a.compiler_java_home else [])])
    target=ROOT/'target/release'
    os_name=platform.system().lower();arch=platform.machine().lower()
    lib={'windows':'livephoto_native.dll','darwin':'liblivephoto_native.dylib'}.get(os_name,'liblivephoto_native.so')
    static='livephoto_native.lib' if os_name=='windows' else 'liblivephoto_native.a'
    cli='llp.exe' if os_name=='windows' else 'llp'
    files,notices=dependency_notices()
    files.update({f'lib/{lib}':target/lib,f'lib/{static}':target/static,f'bin/{cli}':target/cli,
           'java/liblivephoto-java.jar':ROOT/'java/build/liblivephoto-java.jar',
           'include/liblivephoto.h':ROOT/'native/include/liblivephoto.h',
           'LICENSE':ROOT/'LICENSE','CLI.md':ROOT/'CLI.md','java/README.md':ROOT/'java/README.md'})
    out=ROOT/'dist';out.mkdir(exist_ok=True)
    name=f'liblivephoto-{os_name}-{arch}-sdk.zip';archive=out/name
    receipt={**source_identity(),
             'host':os_name,'arch':arch,'files':{key:hashlib.sha256(path.read_bytes()).hexdigest() for key,path in files.items()},
             'dependencies':notices,'published':False}
    with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,strict_timestamps=False) as zipped:
        for key,path in files.items():zipped.write(path,key)
        zipped.writestr('manifest.json',json.dumps(receipt,indent=2)+'\n')
    digest=hashlib.sha256(archive.read_bytes()).hexdigest()
    (out/(name+'.sha256')).write_text(digest+'  '+name+'\n')
    (out/(name+'.receipt.json')).write_text(json.dumps({'implementation_sha':receipt['implementation_sha'],'archive':name,'sha256':digest,'files':len(files),'dependency_notices':len(notices)},indent=2)+'\n')
    print(archive)
if __name__=='__main__':main()
