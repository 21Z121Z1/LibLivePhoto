#!/usr/bin/env python3
"""Run the unchanged pinned XDRemux public contract in an external harness.

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
    print("unchanged XDRemux public contract passed:", REVISION, SOURCE)
    print("test source SHA-256:", hashlib.sha256(body).hexdigest())

if __name__ == "__main__":
    main()
