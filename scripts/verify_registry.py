#!/usr/bin/env python3
"""Install the actual crates.io release into an empty Cargo home and exercise it."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import urllib.request

from external_consumer import SOURCE

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', required=True)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'\d+\.\d+\.\d+', args.version):
        raise RuntimeError('ordinary semantic version required')
    release = json.loads(args.manifest.read_text())
    if release['version'] != args.version:
        raise RuntimeError('release manifest version mismatch')
    checksums = {}
    for name, expected in release['assets'].items():
        if not name.endswith('.crate'):
            continue
        crate = name.removesuffix(f'-{args.version}.crate')
        request = urllib.request.Request(f'https://crates.io/api/v1/crates/{crate}/{args.version}',
                                         headers={'User-Agent': 'LibLivePhoto-release-verification/0.1.0'})
        with urllib.request.urlopen(request, timeout=60) as response:
            uploaded = json.load(response)['version']
        if uploaded['num'] != args.version or uploaded['yanked'] or uploaded['checksum'] != expected:
            raise RuntimeError('registry version/checksum does not match tested archive: ' + crate)
        checksums[crate] = uploaded['checksum']
    if len(checksums) != 6:
        raise RuntimeError('expected all six published workspace crates')
    with tempfile.TemporaryDirectory(prefix='liblivephoto-registry-') as tmp:
        directory = Path(tmp)
        env = {**os.environ, 'CARGO_HOME': str(directory / 'cargo')}
        # Verification needs no registry credential and never reads a local patch config.
        env.pop('CARGO_REGISTRY_TOKEN', None)
        consumer = directory / 'consumer'
        (consumer / 'src').mkdir(parents=True)
        (consumer / 'Cargo.toml').write_text(
            '[package]\nname="registry-consumer"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[dependencies]\nliblivephoto="=' + args.version + '"\n')
        (consumer / 'src/main.rs').write_text(SOURCE)

        def run(command, **kwargs):
            return subprocess.run(list(map(str, command)), cwd=consumer, env=env, check=True,
                                  text=True, timeout=1200, **kwargs)

        run(['cargo', 'run', '--quiet', '--', ROOT / 'web/public/demo/motion-photo.jpg',
             ROOT / 'fixtures/syntax/aac-direct.mp4', ROOT / 'fixtures/syntax/aac-pce-wave.mov'])
        metadata = json.loads(run(['cargo', 'metadata', '--format-version', '1'], capture_output=True).stdout)
        for package in metadata['packages']:
            if package['name'].startswith('liblivephoto'):
                if package['version'] != args.version or not (package['source'] or '').startswith('registry+'):
                    raise RuntimeError('consumer resolved a non-registry LibLivePhoto dependency')
        run(['cargo', 'install', '--locked', '--version', args.version, '--root', directory / 'installed',
             'liblivephoto-cli'])
        cli = directory / 'installed/bin/llp'
        version = json.loads(run([cli, 'version'], capture_output=True).stdout)
        if (not version['success'] or version['metadata']['version'] != args.version or
                version['metadata']['protocol'] != 'LLP1'):
            raise RuntimeError('installed registry CLI returned an unexpected version')
        sample = ROOT / 'web/public/demo/motion-photo.jpg'
        for operation in ('parse', 'check'):
            result = json.loads(run([cli, operation, sample], capture_output=True).stdout)
            if not result['success']:
                raise RuntimeError('installed registry CLI failed: ' + operation)
        output = directory / 'extracted'
        run([cli, 'extract', sample, '--video', '--out', output])
        if (output / 'resource-1.mp4').read_bytes() != (ROOT / 'web/public/demo/motion.mp4').read_bytes():
            raise RuntimeError('registry CLI extraction changed the video bytes')
    receipt = {'version': args.version, 'implementation_sha': release['implementation_sha'],
               'status': 'published_and_verified', 'registry_checksums': checksums,
               'external_rust_consumer': 'passed', 'installed_cli': 'passed',
               'dependency_resolution': 'crates.io only, empty Cargo home, no path/patch dependencies'}
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()
