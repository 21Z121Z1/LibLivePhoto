#!/usr/bin/env python3
"""Reject stale/dirty sources, later CI failures and corrupted release assets."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess
import zipfile

from prepare_release import WORKFLOWS, release_notes, require_new_release, select_runs, verify_identity, verify_sdk, wait_for_ci
from publish_release import validate_candidate

SHA = 'a' * 40


class ReleaseChecks(unittest.TestCase):
    def runs(self):
        return [{'path': '.github/workflows/' + name, 'id': i, 'run_number': 1,
                 'head_sha': SHA, 'head_branch': 'main', 'event': 'push',
                 'status': 'completed', 'conclusion': 'success'}
                for i, name in enumerate(WORKFLOWS)]

    def test_latest_failure_and_missing_exact_main_run_block_preparation(self):
        runs = self.runs()
        self.assertEqual(len(select_runs(runs, SHA)), 4)
        later = {**runs[0], 'run_number': 2, 'conclusion': 'failure'}
        with self.assertRaises(RuntimeError):
            select_runs(runs + [later], SHA)
        runs[0]['head_sha'] = 'b' * 40
        with self.assertRaises(RuntimeError):
            select_runs(runs, SHA)

    def test_wrong_sha_version_and_dirty_artifacts_are_rejected(self):
        receipt = {'implementation_sha': SHA, 'version': '0.1.0', 'working_tree_clean': True}
        verify_identity(receipt, SHA, '0.1.0')
        for change in ({'implementation_sha': 'b' * 40}, {'version': '0.2.0'},
                       {'working_tree_clean': False}, {'working_tree_clean': None}):
            with self.assertRaises(RuntimeError):
                verify_identity({**receipt, **change}, SHA, '0.1.0')

    def test_existing_tags_and_uncertain_remote_errors_block_preparation(self):
        missing = subprocess.CompletedProcess([], 1, '', 'gh: Not Found (HTTP 404)')
        with patch('prepare_release.subprocess.run', return_value=missing):
            require_new_release('owner/repo', '0.1.0')
        for result in (subprocess.CompletedProcess([], 0, '{}', ''),
                       subprocess.CompletedProcess([], 1, '', 'HTTP 500')):
            with patch('prepare_release.subprocess.run', return_value=result):
                with self.assertRaises(RuntimeError):
                    require_new_release('owner/repo', '0.1.0')

    def test_tampered_sdk_bytes_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'sdk.zip'
            manifest = {'implementation_sha': SHA, 'version': '0.1.0', 'working_tree_clean': True,
                        'files': {'bin/llp': hashlib.sha256(b'original').hexdigest()}}
            for value in (b'original', b'changed'):
                with zipfile.ZipFile(path, 'w') as archive:
                    archive.writestr('manifest.json', json.dumps(manifest))
                    archive.writestr('bin/llp', value)
                if value == b'original':
                    verify_sdk(path, SHA, '0.1.0')
                else:
                    with self.assertRaises(RuntimeError):
                        verify_sdk(path, SHA, '0.1.0')

    def test_wait_only_retries_pending_ci_and_respects_deadline(self):
        runs = self.runs()
        pending = [{**r, 'status': 'in_progress', 'conclusion': None} for r in runs]
        responses = [{'total_count': 4, 'workflow_runs': r} for r in [pending, runs]]
        with patch('prepare_release.gh_json', side_effect=responses), patch('prepare_release.time.sleep') as sleep:
            self.assertEqual(len(wait_for_ci('owner/repo', SHA, 30)), 4)
            sleep.assert_called_once()
        failed = [{**runs[0], 'conclusion': 'failure'}]
        with patch('prepare_release.gh_json', return_value={'total_count': 1, 'workflow_runs': failed}), patch('prepare_release.time.sleep') as sleep:
            with self.assertRaises(RuntimeError):
                wait_for_ci('owner/repo', SHA, 30)
            sleep.assert_not_called()
        with patch('prepare_release.gh_json', return_value={'total_count': 0, 'workflow_runs': []}):
            with self.assertRaises(TimeoutError):
                wait_for_ci('owner/repo', SHA, 0)

    def test_first_publisher_refuses_stale_dirty_wrong_repo_and_version(self):
        source = {'version': '0.1.0', 'implementation_sha': SHA, 'working_tree_clean': True}
        validate_candidate(source, source, '21Z121Z1/LibLivePhoto', SHA, 'refs/heads/main')
        for change in ({'version': '0.2.0'}, {'implementation_sha': 'b'*40}, {'working_tree_clean': False}):
            with self.assertRaises(RuntimeError):
                validate_candidate({**source, **change}, source, '21Z121Z1/LibLivePhoto', SHA, 'refs/heads/main')
        for repo, ref in [('owner/repo', 'refs/heads/main'), ('21Z121Z1/LibLivePhoto', 'refs/heads/feature')]:
            with self.assertRaises(RuntimeError):
                validate_candidate(source, source, repo, SHA, ref)

    def test_release_notes_include_both_languages_and_tag_scoped_links(self):
        body = release_notes('21Z121Z1/LibLivePhoto', '0.1.0', SHA)
        for value in ['## English', '## 简体中文', 'First ordinary release', '首个普通版本',
                      '/blob/v0.1.0/COMPATIBILITY.md#device-verification',
                      '/blob/v0.1.0/COMPATIBILITY.zh-CN.md#实机验证']:
            self.assertIn(value, body)
        self.assertNotIn('](../', body)


if __name__ == '__main__':
    unittest.main()
