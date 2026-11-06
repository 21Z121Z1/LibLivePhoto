#!/usr/bin/env python3
"""Dry-run publishing, then test only the files carried by the .crate archives.

Requires Cargo 1.90+ for workspace publishing and Python 3.11+. No upload occurs.
Before the first registry release, local dependencies are resolved through patches
to the extracted archives, never through the repository's crates/ directory.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from package_native import source_identity

ROOT = Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), check=True, **kwargs)


def check_fixtures():
    count = 0
    for name in ('liblivephoto-interface', 'liblivephoto-bridge'):
        if (ROOT / 'crates' / name / 'tests/common/mod.rs').read_bytes() != (ROOT / 'crates/liblivephoto/tests/common/mod.rs').read_bytes():
            raise RuntimeError('packaged test helper differs from core test helper: ' + name)
    for package in (ROOT / 'crates').iterdir():
        data = package / 'tests/data'
        for path in data.rglob('*'):
            if not path.is_file():
                continue
            source = ROOT / path.relative_to(data)
            # public-samples.json already belongs to this crate, not the mirror.
            if path.parent == data:
                continue
            if not source.is_file() or path.read_bytes() != source.read_bytes():
                raise RuntimeError('package fixture differs from its source: ' + str(path))
            count += 1
    print(f'package fixtures: {count} byte-identical files', flush=True)


def unpack(archive, destination):
    with tarfile.open(archive) as tar:
        for member in tar.getmembers():
            path = Path(member.name)
            if path.is_absolute() or '..' in path.parts or not (member.isfile() or member.isdir()):
                raise RuntimeError('unsafe crate archive member: ' + member.name)
        tar.extractall(destination, filter='data')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--allow-dirty', action='store_true', help='local edits only; CI requires a clean checkout')
    parser.add_argument('--archives', type=Path, help='verify existing archives instead of packaging again')
    args = parser.parse_args()
    identity = source_identity()
    if not args.allow_dirty and not identity['working_tree_clean']:
        raise RuntimeError('crate verification requires a clean checkout; use --allow-dirty only for local development')
    check_fixtures()
    if args.archives is None:
        run(['cargo', 'publish', '--workspace', '--dry-run', '--locked',
             *(['--allow-dirty'] if args.allow_dirty else [])], cwd=ROOT)
    package_dir = args.archives or ROOT / 'target/package'
    workspace = tomllib.loads((ROOT / 'Cargo.toml').read_text())
    packages = [tomllib.loads((ROOT / member / 'Cargo.toml').read_text())['package']
                for member in workspace['workspace']['members']]
    with tempfile.TemporaryDirectory(prefix='liblivephoto-crates-') as tmp:
        directory = Path(tmp)
        members, patches, hashes, vcs = [], [], {}, {}
        for package in packages:
            name, version = package['name'], package['version']
            member = f'{name}-{version}'
            archive = package_dir / (member + '.crate')
            unpack(archive, directory)
            vcs[archive.name] = json.loads((directory / member / '.cargo_vcs_info.json').read_text())['git']
            if not args.allow_dirty and (vcs[archive.name]['sha1'] != identity['implementation_sha'] or vcs[archive.name].get('dirty')):
                raise RuntimeError('archive VCS identity differs from the clean candidate: ' + member)
            metadata = tomllib.loads((directory / member / 'Cargo.toml').read_text())
            if metadata['package'].get('rust-version') != '1.88':
                raise RuntimeError('missing supported Rust version in ' + member)
            if not metadata['package'].get('description'):
                raise RuntimeError('missing package description in ' + member)
            for readme in ('README.md', 'README.zh-CN.md'):
                if (directory / member / readme).read_bytes() != (ROOT / 'crates' / name / readme).read_bytes():
                    raise RuntimeError('missing or changed package README: ' + member + '/' + readme)
            if (directory / member / 'LICENSE').read_bytes() != (ROOT / 'LICENSE').read_bytes():
                raise RuntimeError('missing or changed package license in ' + member)
            members.append(member)
            patches.append(f'{name} = {{ path = {json.dumps(member)} }}')
            hashes[archive.name] = hashlib.sha256(archive.read_bytes()).hexdigest()
        (directory / 'Cargo.toml').write_text(
            '[workspace]\nresolver = "2"\nmembers = ' + json.dumps(members) +
            '\n\n[patch.crates-io]\n' + '\n'.join(patches) + '\n')
        shutil.copyfile(ROOT / 'Cargo.lock', directory / 'Cargo.lock')
        run(['cargo', 'test', '--locked', '--workspace', '--all-targets', '--all-features'], cwd=directory)
        run(['cargo', 'test', '--locked', '--workspace', '--doc', '--all-features'], cwd=directory)
        run(['cargo', 'doc', '--locked', '--workspace', '--all-features', '--no-deps'],
            cwd=directory, env={**os.environ, 'RUSTDOCFLAGS': '-D warnings'})
        run([sys.executable, ROOT / 'scripts/external_consumer.py', '--packages', directory], cwd=directory)
        cli_package = next(p for p in packages if p['name'] == 'liblivephoto-cli')
        run(['cargo', 'install', '--locked', '--path', directory / f"liblivephoto-cli-{cli_package['version']}",
             '--root', directory / 'installed', '--offline'], cwd=directory)
        executable = directory / 'installed/bin' / ('llp.exe' if os.name == 'nt' else 'llp')
        version = json.loads(subprocess.check_output([executable, 'version'], text=True))
        if (not version.get('success') or version.get('metadata', {}).get('version') != cli_package['version']
                or version.get('metadata', {}).get('protocol') != 'LLP1'):
            raise RuntimeError('installed CLI returned the wrong version')
        if source_identity() != identity:
            raise RuntimeError('source identity changed while verifying crate archives')
        receipt = {**identity, 'archives': hashes, 'archive_vcs': vcs, 'tests': 'passed', 'rustdoc': 'passed',
                   'external_consumer': 'passed', 'installed_cli': 'passed',
                   'dependency_resolution': 'patches to extracted .crate files; registry upload/download not exercised'}
        out = ROOT / 'evidence/local/crate-packages'
        out.mkdir(parents=True, exist_ok=True)
        (out / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()
