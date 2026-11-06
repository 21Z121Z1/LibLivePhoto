#!/usr/bin/env python3
"""Prepare versioned assets from successful exact-main CI; never publish them."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import time
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ('verify.yml', 'apple-web.yml', 'android.yml', 'docs.yml')


def gh_json(*args):
    return json.loads(subprocess.check_output(['gh', *map(str, args)], text=True, timeout=90))


class CiPending(RuntimeError):
    """Exact-main jobs have not all finished yet; a bounded wait is allowed."""


def require_new_release(repo, version):
    for endpoint in (f'git/ref/tags/v{version}', f'releases/tags/v{version}'):
        result = subprocess.run(['gh', 'api', f'repos/{repo}/{endpoint}'], capture_output=True, text=True, timeout=90)
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
    for workflow, run in selected.items():
        if run['status'] == 'completed' and run['conclusion'] != 'success':
            raise RuntimeError(f'{workflow} latest run is not successful: {run["id"]}')
    if set(selected) != set(WORKFLOWS):
        raise CiPending('missing exact-main workflows: ' + str(set(WORKFLOWS) - set(selected)))
    if any(run['status'] != 'completed' for run in selected.values()):
        raise CiPending('exact-main workflows are still running')
    return selected


def wait_for_ci(repo, sha, timeout):
    deadline = time.monotonic() + timeout
    while True:
        response = gh_json('api', f'repos/{repo}/actions/runs?head_sha={sha}&event=push&branch=main&per_page=100')
        if response['total_count'] > 100:
            raise RuntimeError('more than 100 candidate runs; inspect CI before preparing a release')
        try:
            return select_runs(response['workflow_runs'], sha)
        except CiPending as error:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(str(error)) from error
            print(f'Waiting for exact-main CI: {error}; {int(remaining)} seconds remain', flush=True)
            time.sleep(min(15, remaining))


def release_notes(repo, version, sha):
    notes = [f'Source / 源码: `{sha}`']
    for language, suffix in [('English', ''), ('简体中文', '.zh-CN')]:
        path = ROOT / 'release-notes' / f'v{version}{suffix}.md'
        # A release body is not located inside release-notes/. Keep its links valid.
        def absolute_link(match):
            target = match.group(1)
            if re.match(r'[A-Za-z][A-Za-z0-9+.-]*:', target) or target.startswith('#'):
                return match.group(0)
            relative, separator, anchor = target.partition('#')
            resolved = (path.parent / relative).resolve().relative_to(ROOT)
            return f'](https://github.com/{repo}/blob/v{version}/{resolved.as_posix()}{separator}{anchor})'
        body = re.sub(r'\]\(([^\s)]+)\)', absolute_link, path.read_text())
        notes.append(f'## {language}\n\n{body}')
    return '\n\n'.join(notes)


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
    parser.add_argument('--wait-seconds', type=int, default=0, help='bounded wait for the exact-main workflows')
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}', args.sha) or not re.fullmatch(r'\d+\.\d+\.\d+', args.version):
        raise RuntimeError('full lowercase commit SHA and ordinary semantic version required')
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', args.repo):
        raise RuntimeError('owner/repository required')
    if not 0 <= args.wait_seconds <= 3600:
        raise RuntimeError('CI wait must be between 0 and 3600 seconds')
    actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    if actual != args.sha or gh_json('api', f'repos/{args.repo}/commits/main')['sha'] != args.sha:
        raise RuntimeError('release candidate must be the current main head')
    require_new_release(args.repo, args.version)
    for path in (ROOT / 'crates').glob('*/Cargo.toml'):
        if tomllib.loads(path.read_text())['package']['version'] != args.version:
            raise RuntimeError('crate version mismatch: ' + str(path))
    runs = wait_for_ci(args.repo, args.sha, args.wait_seconds)
    if gh_json('api', f'repos/{args.repo}/commits/main')['sha'] != args.sha:
        raise RuntimeError('main changed while waiting for CI')
    for workflow, run in runs.items():
        jobs = gh_json('api', f'repos/{args.repo}/actions/runs/{run["id"]}/jobs?per_page=100')
        if jobs['total_count'] > 100 or not jobs['jobs'] or any(j['conclusion'] != 'success' for j in jobs['jobs']):
            raise RuntimeError('CI job failed, was skipped, or was not inspected: ' + workflow)
    out = ROOT / 'dist/release' / args.version
    out.mkdir(parents=True, exist_ok=False)
    assets = out / 'assets'
    assets.mkdir()

    def download(workflow, artifact):
        run = runs[workflow]
        directory = out / 'ci' / artifact
        subprocess.run(['gh', 'run', 'download', str(run['id']), '--repo', args.repo,
                        '--name', artifact, '--dir', str(directory)], check=True, timeout=600)
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
    if set(receipt['files']) != {p.relative_to(framework).as_posix() for p in framework.rglob('*') if p.is_file()}:
        raise RuntimeError('XCFramework file inventory does not match')
    if not {'README.md', 'README.zh-CN.md'} <= set(receipt['files']):
        raise RuntimeError('XCFramework bilingual documentation is missing')
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
    (out / 'notes.md').write_text(release_notes(args.repo, args.version, args.sha))
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    main()
