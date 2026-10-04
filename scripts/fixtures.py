#!/usr/bin/env python3
"""Fetch or verify exact public source fixtures. Never upload fixture contents."""
import argparse
import hashlib
import json
from pathlib import Path
import urllib.request
import urllib.parse

ROOT = Path(__file__).resolve().parent.parent

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT / "fixtures/cache")
    parser.add_argument("--fetch", action="store_true")
    parser.add_argument("--manifest",type=Path,default=ROOT / "fixtures/manifest.json")
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text())
    for row in manifest["fixtures"]:
        path = args.root / row["path"]
        if not path.exists() and args.fetch:
            prefix="fixtures/" if manifest["repository"]=="21Z121Z1/XDRemux" else ""
            relative=urllib.parse.quote(prefix+row["path"],safe="/")
            url = f'https://raw.githubusercontent.com/{manifest["repository"]}/{manifest["revision"]}/{relative}'
            with urllib.request.urlopen(url, timeout=60) as response:
                data = response.read(128 * 1024 * 1024 + 1)
            if len(data)>128*1024*1024:raise ValueError("fixture exceeds byte budget")
            if hashlib.sha256(data).hexdigest() != row["sha256"]:
                raise ValueError(f'fixture identity mismatch: {row["path"]}')
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        if hashlib.sha256(path.read_bytes()).hexdigest() != row["sha256"]:
            raise ValueError(f'fixture identity mismatch: {row["path"]}')
    print(f'verified {len(manifest["fixtures"])} public fixture identities')

if __name__ == "__main__":
    main()
