#!/usr/bin/env python3
"""Verify extracted release artifacts, not target/debug library paths."""
import argparse,ctypes,hashlib,json,platform,subprocess,tempfile,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
    p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--compiler-java-home');a=p.parse_args()
    # Own archive; verify paths and manifest hashes before extraction.
    with tempfile.TemporaryDirectory(prefix='liblivephoto-sdk-') as temp:
        directory=Path(temp)
        with zipfile.ZipFile(a.archive) as zipped:
            for name in zipped.namelist():
                path=Path(name)
                if path.is_absolute() or '..' in path.parts:raise RuntimeError('unsafe archive path')
            zipped.extractall(directory)
        manifest=json.loads((directory/'manifest.json').read_text())
        for name,digest in manifest['files'].items():assert hashlib.sha256((directory/name).read_bytes()).hexdigest()==digest,name
        windows=platform.system()=='Windows';cli=directory/'bin'/('llp.exe' if windows else 'llp')
        if not windows:cli.chmod(0o700)
        version=json.loads(subprocess.check_output([str(cli),'version'],text=True));assert version['success']
        lib=next((directory/'lib').glob('*.dll' if windows else '*.dylib' if platform.system()=='Darwin' else '*.so'))
        native=ctypes.CDLL(str(lib));native.llp_api_version.restype=ctypes.c_uint32;assert native.llp_api_version()==1
        if windows:
            import _ctypes
            _ctypes.FreeLibrary(native._handle)
            native=None
        # Compile a real external consumer against only the packaged API jar.
        source=directory/'PackagedConsumer.java';source.write_text('''import org.liblivephoto.*;
public class PackagedConsumer { public static void main(String[] args) throws Exception {
  NativeEngine engine=new NativeEngine();if(!engine.version().success)throw new AssertionError("version");
  NativeResult value=engine.parse(Input.path(java.nio.file.Paths.get(args[0])));
  if(!value.success||!value.headerJson.contains("812500"))throw new AssertionError(value.headerJson);
  System.out.println("PACKAGED_JAVA_RUST_CONSUMER passed"); }}\n''')
        jar=directory/'java/liblivephoto-java.jar';java=['java']+(['-Djava.home='+a.compiler_java_home] if a.compiler_java_home else [])
        subprocess.run([*java,'com.sun.tools.javac.Main','--release','8','-classpath',str(jar),'-d',str(directory),str(source)],check=True)
        subprocess.run(['java','-Dliblivephoto.native.path='+str(lib),'-classpath',str(directory)+(';' if windows else ':')+str(jar),'PackagedConsumer',str(ROOT/'web/public/demo/motion-photo.jpg')],check=True)
        print(json.dumps({'implementation_sha':manifest['implementation_sha'],'archive':a.archive.name,'manifest_hashes':'passed','release_cli':'passed','release_c_abi':'passed','external_java_jar_consumer':'passed'}))
if __name__=='__main__':main()
