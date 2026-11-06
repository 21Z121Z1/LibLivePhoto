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

from prepare_release import WORKFLOWS, require_new_release, select_runs, verify_identity, verify_sdk

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


if __name__ == '__main__':
    unittest.main()
