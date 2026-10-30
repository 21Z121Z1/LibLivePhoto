#!/usr/bin/env python3
"""Run unchanged pinned XDRemux sources/contracts in an external harness.

The dependency alias targets this library's re-exported public API. This is not
the full XDRemux product build and does not edit the consumer repository or pin.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
REVISION = "0688ea7d6270df52cc5968e2c36dd016f13bd31b"
SOURCE = "crates/xdremux-motion-photo/tests/liblivephoto_interop.rs"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--xdremux-repository", type=Path, required=True)
    parser.add_argument("--check-consumer-sources", action="store_true",
                        help="also compile every original facade source/example/test with a local dependency override")
    args = parser.parse_args()
    body = subprocess.check_output(["git", "-C", str(args.xdremux_repository), "show", REVISION + ":" + SOURCE])
    fixture = Path("motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg")
    with tempfile.TemporaryDirectory(prefix="liblivephoto-xdremux-contract-") as tmp:
        root = Path(tmp)
        package = root / "crates/replay"
        (package / "tests").mkdir(parents=True)
        (package / "tests/interop.rs").write_bytes(body)
        (root / "fixtures" / fixture).parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "fixtures/cache" / fixture, root / "fixtures" / fixture)
        library = json.dumps(str(ROOT / "crates/liblivephoto"))
        (package / "Cargo.toml").write_text('[package]\nname="public-contract-replay"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\nxdremux-motion-photo={package="liblivephoto",path=' + library + '}\n')
        subprocess.run(["cargo", "test", "--manifest-path", str(package / "Cargo.toml")], check=True, cwd=package)
        if args.check_consumer_sources:
            # Copy bytes from the pinned tree, without checking out or editing
            # the consumer repository. Its original Rust sources remain exact.
            prefix = "crates/xdremux-motion-photo/"
            paths = subprocess.check_output(["git", "-C", str(args.xdremux_repository),
                    "ls-tree", "-r", "--name-only", REVISION, prefix], text=True).splitlines()
            hashes = {}
            for name in paths:
                content = subprocess.check_output(["git", "-C", str(args.xdremux_repository), "show", REVISION+":"+name])
                target = root/name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(content)
                hashes[name] = hashlib.sha256(content).hexdigest()
            facade = root/prefix
            manifest = facade/"Cargo.toml"
            original = manifest.read_text()
            dependency = 'liblivephoto = { git = "https://github.com/21Z121Z1/LibLivePhoto", rev = "794ab552596db0f66095600d23cf7937a631b9ec", features = ["xdremux-compat"] }'
            assert dependency in original, "pinned consumer dependency contract changed"
            manifest.write_text(original.replace(dependency, 'liblivephoto = { path = '+library+', features = ["xdremux-compat"] }'))
            # Only a standalone workspace scaffold and path dependency replace
            # the product workspace/pin. No product build is implied.
            (root/"Cargo.toml").write_text('[workspace]\nmembers=["crates/xdremux-motion-photo"]\nresolver="2"\n[workspace.package]\nedition="2021"\n[workspace.lints]\n')
            subprocess.run(["cargo", "check", "--offline", "--manifest-path", str(root/"Cargo.toml"), "--all-targets"], check=True, cwd=root)
            subprocess.run(["cargo", "test", "--offline", "--manifest-path", str(root/"Cargo.toml"), "-p", "xdremux-motion-photo", "--test", "liblivephoto_interop"], check=True, cwd=root)
            print("REAL_CONSUMER_SOURCES_PASSED "+json.dumps({
                "implementation_sha":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
                "consumer_sha":REVISION,"source_files":len(paths)-1,"original_files_sha256":hashes,
                "cargo_check_all_targets":"passed","unchanged_interop_test":"1 passed",
                "consumer_repository_modified":False,"full_product_build":False,
                "derived_harness_overrides":"local liblivephoto path with original xdremux-compat feature; standalone workspace edition/lints"}))
    print("unchanged XDRemux public contract passed:", REVISION, SOURCE)
    print("test source SHA-256:", hashlib.sha256(body).hexdigest())

if __name__ == "__main__":
    main()
