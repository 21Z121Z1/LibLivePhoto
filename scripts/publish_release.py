#!/usr/bin/env python3
"""Publish the user-approved first release after exact-main artifact preparation."""
import json
import os
from pathlib import Path
import subprocess
import sys

from package_native import source_identity
from prepare_release import ROOT, digest, gh_json, require_new_release

VERSION = '0.1.0'


def validate_candidate(identity, manifest, repo, sha, ref):
    if repo != '21Z121Z1/LibLivePhoto' or ref != 'refs/heads/main':
        raise RuntimeError('this one-time publisher is restricted to LibLivePhoto main')
    for candidate in (identity, manifest):
        if candidate.get('version') != VERSION or candidate.get('implementation_sha') != sha:
            raise RuntimeError('publication source/version does not match the prepared candidate')
    if not identity['working_tree_clean']:
        raise RuntimeError('publication requires a clean source checkout')


def main():
    repo, sha = os.environ['GITHUB_REPOSITORY'], os.environ['GITHUB_SHA']
    out = ROOT / 'dist/release' / VERSION
    assets = out / 'assets'
    manifest_path = assets / 'release-manifest.json'
    manifest = json.loads(manifest_path.read_text())
    validate_candidate(source_identity(), manifest, repo, sha, os.environ['GITHUB_REF'])
    if gh_json('api', f'repos/{repo}/commits/main')['sha'] != sha:
        raise RuntimeError('main moved after preparation; do not publish a stale candidate')
    require_new_release(repo, VERSION)
    for name, checksum in manifest['assets'].items():
        if Path(name).name != name or digest(assets / name) != checksum:
            raise RuntimeError('prepared asset changed: ' + name)

    receipt = {'version': VERSION, 'implementation_sha': sha, 'status': 'not_attempted',
               'reason': 'CARGO_REGISTRY_TOKEN is not configured'}
    if os.environ.get('CARGO_REGISTRY_TOKEN'):
        receipt['status'], receipt['reason'] = 'upload_in_progress', 'registry credential available'
        receipt_path = out / 'registry-verification.json'
        receipt_path.write_text(json.dumps(receipt, indent=2) + '\n')
        # Cargo uploads the same clean, locked workspace used to create the CI archives.
        # Cargo's dependency ordering/index wait handles the first publication.
        subprocess.run(['cargo', 'publish', '--workspace', '--locked'], cwd=ROOT, check=True, timeout=1500)
        subprocess.run([sys.executable, ROOT / 'scripts/verify_registry.py', '--version', VERSION,
                        '--manifest', manifest_path, '--receipt', receipt_path], check=True, timeout=1800)
        receipt = json.loads(receipt_path.read_text())
    status_path = assets / 'registry-status.json'
    status_path.write_text(json.dumps(receipt, indent=2) + '\n')
    checksum = digest(status_path)
    (assets / 'registry-status.json.sha256').write_text(checksum + '  registry-status.json\n')
    manifest['assets']['registry-status.json'] = checksum
    manifest['registry'] = receipt
    manifest_path.write_text(json.dumps(manifest, indent=2) + '\n')
    (assets / 'release-manifest.json.sha256').write_text(digest(manifest_path) + '  release-manifest.json\n')
    if receipt['status'] == 'published_and_verified':
        status = 'crates.io: all six crates published; registry-only Rust consumer and CLI installation passed.\n\ncrates.io：六个 crate 已发布；纯注册表 Rust 消费程序与 CLI 安装验证通过。'
    else:
        status = ('crates.io upload was not attempted because the repository has no registry credential. '
                  'Use the GitHub source/SDK assets or the Git tag installation in RELEASING.md.\n\n'
                  '仓库未配置注册表凭证，本次未尝试 crates.io 上传。可使用 GitHub 源码/SDK，'
                  '或按 RELEASING.zh-CN.md 从 Git 标签安装。')
    notes = out / 'notes.md'
    notes.write_text(notes.read_text() + '\n\n## Distribution status / 分发状态\n\n' + status + '\n')

    # Check again immediately before creating the immutable public version.
    if gh_json('api', f'repos/{repo}/commits/main')['sha'] != sha:
        raise RuntimeError('main moved before release creation')
    require_new_release(repo, VERSION)
    subprocess.run(['gh', 'release', 'create', 'v' + VERSION, '--repo', repo, '--target', sha,
                    '--title', 'v' + VERSION, '--draft', '--notes-file', str(notes),
                    *map(str, sorted(assets.iterdir()))], check=True, timeout=900)
    # Verify the uploaded bytes while the release is still a draft.
    uploaded = out / 'uploaded-assets'
    subprocess.run(['gh', 'release', 'download', 'v' + VERSION, '--repo', repo,
                    '--dir', str(uploaded)], check=True, timeout=900)
    if {p.name for p in assets.iterdir()} != {p.name for p in uploaded.iterdir()}:
        raise RuntimeError('uploaded release asset inventory mismatch')
    for asset in assets.iterdir():
        if digest(asset) != digest(uploaded / asset.name):
            raise RuntimeError('uploaded release asset hash mismatch: ' + asset.name)
    subprocess.run(['gh', 'release', 'edit', 'v' + VERSION, '--repo', repo,
                    '--draft=false', '--latest'], check=True, timeout=90)
    release = gh_json('api', f'repos/{repo}/releases/tags/v{VERSION}')
    if release['draft'] or release['prerelease'] or release['tag_name'] != 'v' + VERSION:
        raise RuntimeError('release did not become an ordinary published version')
    summary = (f'Published / 已发布 [{VERSION}]({release["html_url"]})\n\n'
               f'Source / 源码: `{sha}`\n\n' + status + '\n\n'
               'All uploaded assets were downloaded again and SHA-256 verified.\n\n'
               '已重新下载全部上传产物并验证 SHA-256。\n')
    with open(os.environ['GITHUB_STEP_SUMMARY'], 'a') as output:
        output.write(summary)
    print(summary)


if __name__ == '__main__':
    main()
