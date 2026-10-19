#!/usr/bin/env python3
"""Build an AAR and real instrumentation APK with an installed SDK/NDK.

No Gradle downloads, SDK license acceptance, publishing or device permission
changes are performed here. Missing toolchains fail with their exact path.
"""
import argparse, hashlib, json, os, re, shutil, subprocess, zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ABIS = {
    'arm64-v8a': ('aarch64-linux-android', 'aarch64-linux-android26-clang'),
    'x86_64': ('x86_64-linux-android', 'x86_64-linux-android26-clang'),
}

def run(args, **kwargs):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), check=True, cwd=ROOT, **kwargs)

def newest(paths):
    paths = list(paths)
    if not paths:
        raise RuntimeError('no installed toolchain matches the required path')
    return sorted(paths, key=lambda p: tuple(int(n) if n.isdigit() else -1 for n in p.name.replace('-', '.').split('.')))[-1]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--sdk', type=Path, default=os.environ.get('ANDROID_SDK_ROOT') or os.environ.get('ANDROID_HOME'))
    parser.add_argument('--ndk', type=Path, default=os.environ.get('ANDROID_NDK_HOME'))
    parser.add_argument('--abi', action='append', choices=list(ABIS))
    parser.add_argument('--compiler-java-home')
    args = parser.parse_args()
    if args.sdk is None or not args.sdk.is_dir():
        raise RuntimeError('installed Android SDK required: set ANDROID_SDK_ROOT or --sdk (no SDK found)')
    sdk = args.sdk.resolve()
    platform = newest(p for p in (sdk/'platforms').glob('android-*') if re.fullmatch(r'android-\d+', p.name) and (p/'android.jar').is_file() and (p/'framework.aidl').is_file())
    tools = newest(p for p in (sdk/'build-tools').glob('*') if (p/'aapt2').is_file() and (p/'aidl').is_file())
    ndk = args.ndk.resolve() if args.ndk else newest((sdk/'ndk').glob('*'))
    prebuilt = newest((ndk/'toolchains/llvm/prebuilt').glob('linux-*'))
    abis = args.abi or list(ABIS)
    build = ROOT/'android/build'
    for path in ['generated', 'classes', 'test-classes', 'dex', 'jni']:
        (build/path).mkdir(parents=True, exist_ok=True)
    for source in sorted((ROOT/'android/src/main/aidl').rglob('*.aidl')):
        run([tools/'aidl', '-p'+str(platform/'framework.aidl'), '-I'+str(ROOT/'android/src/main/aidl'),
             '-o'+str(build/'generated'), source])
    java = ['java'] + (['-Djava.home='+args.compiler_java_home] if args.compiler_java_home else [])
    sources = sorted((ROOT/'java/src/main/java').rglob('*.java')) + sorted((ROOT/'android/src/main/java').rglob('*.java')) + sorted((build/'generated').rglob('*.java'))
    # Android's core lambda compile stubs are required by javac; D8 desugars
    # those call sites. They are build inputs, never APK runtime classes.
    lambda_stubs = tools/'core-lambda-stubs.jar'
    if not lambda_stubs.is_file():
        raise RuntimeError('missing SDK core lambda compile stubs: '+str(lambda_stubs))
    bootclasspath = os.pathsep.join(map(str, [platform/'android.jar', lambda_stubs]))
    run([*java, 'com.sun.tools.javac.Main', '-source', '8', '-target', '8', '-bootclasspath', bootclasspath,
         '-d', build/'classes', *sources])
    run([*java, 'sun.tools.jar.Main', '--create', '--file', build/'classes.jar', '-C', build/'classes', '.'])
    for abi in abis:
        target, compiler = ABIS[abi]
        clang = prebuilt/'bin'/compiler
        if not clang.is_file():
            raise RuntimeError('missing NDK linker: '+str(clang))
        env = os.environ.copy()
        env['CARGO_TARGET_'+target.upper().replace('-', '_')+'_LINKER'] = str(clang)
        env['CARGO_TARGET_'+target.upper().replace('-', '_')+'_RUSTFLAGS'] = '-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384'
        run(['cargo', 'build', '--locked', '--release', '-p', 'liblivephoto-native', '--target', target], env=env)
        dest = build/'jni'/abi
        dest.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT/'target'/target/'release/liblivephoto_native.so', dest/'liblivephoto_native.so')
    aar = build/'liblivephoto-android.aar'
    with zipfile.ZipFile(aar, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.write(ROOT/'android/src/main/AndroidManifest.xml', 'AndroidManifest.xml')
        archive.write(build/'classes.jar', 'classes.jar')
        archive.writestr('R.txt', '')
        archive.writestr('proguard.txt', '-keep class org.liblivephoto.NativeEngine { native <methods>; }\n')
        archive.write(ROOT/'LICENSE', 'META-INF/LICENSE')
        archive.write(ROOT/'evidence/native-dependencies.json', 'META-INF/native-dependencies.json')
        for abi in abis:
            archive.write(build/'jni'/abi/'liblivephoto_native.so', 'jni/'+abi+'/liblivephoto_native.so')
        for source in sorted((ROOT/'android/src/main/aidl').rglob('*.aidl')):
            archive.write(source, 'aidl/'+str(source.relative_to(ROOT/'android/src/main/aidl')))
    run([*java, 'com.sun.tools.javac.Main', '-source', '8', '-target', '8', '-bootclasspath', bootclasspath,
         '-classpath', build/'classes', '-d', build/'test-classes', *sorted((ROOT/'android/src/test/java').rglob('*.java'))])
    run([*java, 'sun.tools.jar.Main', '--create', '--file', build/'test-classes.jar', '-C', build/'test-classes', '.'])
    run([tools/'d8', '--lib', platform/'android.jar', '--min-api', '26', '--output', build/'dex', build/'classes.jar', build/'test-classes.jar'])
    unsigned = build/'acceptance-unsigned.apk'
    run([tools/'aapt2', 'link', '-o', unsigned, '--manifest', ROOT/'android/src/test/AndroidManifest.xml',
         '-I', platform/'android.jar', '--min-sdk-version', '26', '--target-sdk-version', '35', '-A', ROOT/'web/public/demo'])
    with zipfile.ZipFile(unsigned, 'a', zipfile.ZIP_DEFLATED) as archive:
        archive.write(build/'dex/classes.dex', 'classes.dex')
        for abi in abis:
            archive.write(build/'jni'/abi/'liblivephoto_native.so', 'lib/'+abi+'/liblivephoto_native.so')
    aligned = build/'acceptance-aligned.apk'
    run([tools/'zipalign', '-f', '-p', '4', unsigned, aligned])
    # This key is only for the generated local test APK; it is never an artifact.
    key = build/'local-test.keystore'
    if not key.exists():
        run(['keytool', '-genkeypair', '-keystore', key, '-storepass', 'local-acceptance-only', '-keypass', 'local-acceptance-only',
             '-alias', 'test', '-keyalg', 'RSA', '-keysize', '2048', '-validity', '30', '-dname', 'CN=LibLivePhoto local generated tests'])
    apk = build/'liblivephoto-acceptance.apk'
    run([tools/'apksigner', 'sign', '--ks', key, '--ks-pass', 'pass:local-acceptance-only', '--key-pass', 'pass:local-acceptance-only', '--out', apk, aligned])
    run([tools/'apksigner', 'verify', apk])
    receipt = {'implementation_sha': subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
               'sdk_platform': platform.name, 'build_tools': tools.name, 'ndk': ndk.name, 'abis': abis,
               'artifacts': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [aar, apk]},
               'runtime_acceptance': 'not implied by building; run scripts/test_android.py on an authorized device/emulator'}
    (build/'build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt,indent=2))

if __name__ == '__main__':
    main()
