#!/usr/bin/env python3
"""Prepare versioned assets from successful exact-main CI; never publish them."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ('verify.yml', 'apple-web.yml', 'android.yml', 'docs.yml')


def gh_json(*args):
    return json.loads(subprocess.check_output(['gh', *map(str, args)], text=True))


def require_new_release(repo, version):
    for endpoint in (f'git/ref/tags/v{version}', f'releases/tags/v{version}'):
        result = subprocess.run(['gh', 'api', f'repos/{repo}/{endpoint}'], capture_output=True, text=True)
        if result.returncode == 0:
            raise RuntimeError('release or tag already exists: v' + version)
        if 'HTTP 404' not in result.stderr:
            raise RuntimeError('cannot establish absence of release/tag: ' + result.stderr)


def select_runs(runs, sha):
    selected = {}
    # A later failed/running attempt must not be masked by an earlier success.
    for run in sorted(runs, key=lambda r: (r['run_number'], r.get('run_attempt', 1)), reverse=True):
        workflow = Path(run['path']).name
        if (workflow in WORKFLOWS and workflow not in selected and
                run['head_sha'] == sha and run['head_branch'] == 'main' and run['event'] == 'push'):
            selected[workflow] = run
    if set(selected) != set(WORKFLOWS):
        raise RuntimeError('missing exact-main workflows: ' + str(set(WORKFLOWS) - set(selected)))
    for workflow, run in selected.items():
        if run['status'] != 'completed' or run['conclusion'] != 'success':
            raise RuntimeError(f'{workflow} latest run is not successful: {run["id"]}')
    return selected


def verify_identity(receipt, sha, version):
    if (receipt.get('implementation_sha') != sha or receipt.get('version') != version or
            receipt.get('working_tree_clean') is not True):
        raise RuntimeError('artifact source SHA, version or clean-worktree receipt does not match')


def unique(directory, pattern):
    found = list(directory.rglob(pattern))
    if len(found) != 1:
        raise RuntimeError(f'expected one {pattern}, got {len(found)} in {directory}')
    return found[0]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_sdk(path, sha, version):
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise RuntimeError('duplicate SDK archive members')
        manifest = json.loads(archive.read('manifest.json'))
        verify_identity(manifest, sha, version)
        if set(names) != set(manifest['files']) | {'manifest.json'}:
            raise RuntimeError('SDK file inventory does not match')
        for name, expected in manifest['files'].items():
            if hashlib.sha256(archive.read(name)).hexdigest() != expected:
                raise RuntimeError('SDK member hash mismatch: ' + name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sha', required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--repo', default='21Z121Z1/LibLivePhoto')
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}', args.sha) or not re.fullmatch(r'\d+\.\d+\.\d+', args.version):
        raise RuntimeError('full lowercase commit SHA and ordinary semantic version required')
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', args.repo):
        raise RuntimeError('owner/repository required')
    actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    if actual != args.sha or gh_json('api', f'repos/{args.repo}/commits/main')['sha'] != args.sha:
        raise RuntimeError('release candidate must be the current main head')
    require_new_release(args.repo, args.version)
    for path in (ROOT / 'crates').glob('*/Cargo.toml'):
        if tomllib.loads(path.read_text())['package']['version'] != args.version:
            raise RuntimeError('crate version mismatch: ' + str(path))
    response = gh_json('api', f'repos/{args.repo}/actions/runs?head_sha={args.sha}&event=push&branch=main&per_page=100')
    if response['total_count'] > 100:
        raise RuntimeError('more than 100 candidate runs; inspect CI before preparing a release')
    runs = select_runs(response['workflow_runs'], args.sha)
    out = ROOT / 'dist/release' / args.version
    out.mkdir(parents=True, exist_ok=False)
    assets = out / 'assets'
    assets.mkdir()

    def download(workflow, artifact):
        run = runs[workflow]
        jobs = gh_json('api', f'repos/{args.repo}/actions/runs/{run["id"]}/jobs?per_page=100')
        if jobs['total_count'] > 100 or not jobs['jobs'] or any(j['conclusion'] != 'success' for j in jobs['jobs']):
            raise RuntimeError('CI job failed, was skipped, or was not inspected: ' + workflow)
        directory = out / 'ci' / artifact
        subprocess.run(['gh', 'run', 'download', str(run['id']), '--repo', args.repo,
                        '--name', artifact, '--dir', str(directory)], check=True)
        return directory

    crates = download('verify.yml', 'crate-packages')
    receipt = json.loads(unique(crates, 'receipt.json').read_text())
    verify_identity(receipt, args.sha, args.version)
    archives = list(crates.rglob('*.crate'))
    expected = {f'{p.parent.name}-{args.version}.crate' for p in (ROOT / 'crates').glob('*/Cargo.toml')}
    if {a.name for a in archives} != expected or len(archives) != len(expected):
        raise RuntimeError('crate archive set does not match the workspace')
    for archive in archives:
        if digest(archive) != receipt['archives'][archive.name]:
            raise RuntimeError('crate archive hash mismatch: ' + archive.name)
        shutil.copyfile(archive, assets / archive.name)
    for platform in ('ubuntu-latest', 'macos-latest', 'windows-latest'):
        directory = download('verify.yml', 'native-sdk-' + platform)
        archive = unique(directory, '*-sdk.zip')
        verify_sdk(archive, args.sha, args.version)
        name = archive.name.replace('liblivephoto-', f'liblivephoto-{args.version}-', 1)
        shutil.copyfile(archive, assets / name)
    apple = download('apple-web.yml', 'apple-sdk-build')
    framework = unique(apple, 'LibLivePhotoCore.xcframework')
    receipt = json.loads(unique(apple, 'xcframework-receipt.json').read_text())
    verify_identity(receipt, args.sha, args.version)
    for name, expected in receipt['files'].items():
        if digest(framework / name) != expected:
            raise RuntimeError('XCFramework member hash mismatch: ' + name)
    shutil.make_archive(str(assets / f'liblivephoto-{args.version}-ios-experimental'), 'zip',
                        framework.parent, framework.name)
    android = download('android.yml', 'android-service-evidence')
    receipt = json.loads(unique(android, 'build-receipt.json').read_text())
    verify_identity(receipt, args.sha, args.version)
    aar = unique(android, 'liblivephoto-android.aar')
    if digest(aar) != receipt['artifacts'][aar.name]:
        raise RuntimeError('AAR hash mismatch')
    shutil.copyfile(aar, assets / f'liblivephoto-{args.version}-android-experimental.aar')
    hashes = {p.name: digest(p) for p in sorted(assets.iterdir())}
    for name, checksum in hashes.items():
        (assets / (name + '.sha256')).write_text(f'{checksum}  {name}\n')
    manifest = {'version': args.version, 'implementation_sha': args.sha, 'assets': hashes,
                'runs': {name: run['html_url'] for name, run in runs.items()},
                'experimental': ['C/JNI', 'Android AAR', 'Swift/XCFramework', 'WASM']}
    (assets / 'release-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    notes = (ROOT / 'CHANGELOG.md').read_text()
    (out / 'notes.md').write_text(f'Source: `{args.sha}`\n\n' + notes)
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    main()
