#!/usr/bin/env python3
"""Harness fault injection only; these tests never claim Android execution."""
import contextlib
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import test_android as harness


def result(stdout='', stderr='', code=0, timeout=False):
    return {'returncode': code, 'stdout': stdout, 'stderr': stderr, 'timed_out': timeout}


class Readiness(unittest.TestCase):
    def readiness(self, listings, service_ready=True, limit=4):
        calls = []
        now = [0]

        class Fake:
            def command(self, tail, timeout):
                calls.append(tail)
                if tail[1:3] == ['pm', 'list']:
                    return listings.pop(0) if len(listings) > 1 else listings[0]
                if tail[1] == 'getprop':
                    return result('1\n')
                return result('Service ' + tail[-1] + (': found\n' if service_ready else ': not found\n'))

        def sleep(seconds):
            now[0] += seconds

        error = None
        attempts = None
        try:
            attempts = harness.wait_ready(Fake(), timeout=10, max_attempts=limit,
                                          clock=lambda: now[0], sleep=sleep)
        except RuntimeError as caught:
            error = caught
        return calls, now[0], attempts, error

    def test_exit_224_cannot_prove_absence_but_later_clean_success_can(self):
        calls, _, attempts, error = self.readiness([result('partial\n', 'Broken pipe', 224), result()])
        self.assertIsNone(error)
        self.assertEqual(len(attempts), 2)
        self.assertEqual(attempts[0][-1]['stderr'], 'Broken pipe')
        self.assertFalse(any(call[0] == 'install' for call in calls))

    def test_permanent_failure_is_bounded_and_never_installs(self):
        calls, elapsed, attempts, error = self.readiness([result(code=224)], limit=3)
        self.assertIsNone(attempts)
        self.assertRegex(str(error), 'bounded readiness')
        self.assertLessEqual(elapsed, 10)
        self.assertEqual(sum(call[1:3] == ['pm', 'list'] for call in calls), 3)
        self.assertFalse(any(call[0] == 'install' for call in calls))

    def test_boot_completed_does_not_substitute_for_system_services(self):
        calls, _, _, error = self.readiness([result()], service_ready=False)
        self.assertIsInstance(error, RuntimeError)
        self.assertFalse(any(call[1:3] == ['pm', 'list'] for call in calls))

    def test_only_clean_package_listing_can_prove_absence(self):
        for response in [result('Error: package service unavailable\n'), result(stderr='error'),
                         result(timeout=True), result(code=224), result('\n')]:
            self.assertIsNone(harness.package_listing(response))
        self.assertEqual(harness.package_listing(result()), [])
        self.assertEqual(harness.package_listing(result('package:org.example.other\n')), ['org.example.other'])

    def test_preexisting_package_is_refused_without_retry_or_mutation(self):
        calls, _, _, error = self.readiness([result('package:' + harness.PACKAGE + '\n')])
        self.assertRegex(str(error), 'refusing to overwrite')
        self.assertEqual(len(calls), 4)

    def test_timeout_and_exit_code_preserve_both_output_streams(self):
        with tempfile.TemporaryDirectory() as directory:
            adb = harness.Adb(['unit-adb'], Path(directory))
            responses = [subprocess.CompletedProcess([], 224, 'pm partial', 'Broken pipe'),
                         subprocess.TimeoutExpired([], 1, output=b'boot partial', stderr=b'timeout stderr')]
            with patch.object(harness.subprocess, 'run', side_effect=responses):
                with contextlib.redirect_stdout(io.StringIO()) as visible:
                    adb.command(['shell', 'pm'], timeout=1)
                    adb.command(['shell', 'getprop'], timeout=1)
                self.assertIn('ANDROID_HARNESS_ADB_FAILURE', visible.getvalue())
                self.assertIn('Broken pipe', visible.getvalue())
                self.assertIn('timeout stderr', visible.getvalue())
            lines = [json.loads(s) for s in (Path(directory) / 'adb-commands.jsonl').read_text().splitlines()]
            self.assertEqual(lines[0]['returncode'], 224)
            self.assertEqual(lines[0]['stdout'], 'pm partial')
            self.assertEqual(lines[0]['stderr'], 'Broken pipe')
            self.assertTrue(lines[1]['timed_out'])
            self.assertEqual(lines[1]['stderr'], 'timeout stderr')


class RuntimeGates(unittest.TestCase):
    def test_empty_filtered_log_still_captures_bounded_crash_and_package_exit_history(self):
        with tempfile.TemporaryDirectory() as directory:
            calls = []

            class Adb:
                out = Path(directory)

                def command(self, tail, timeout):
                    calls.append((tail, timeout))
                    if tail[:3] == ['logcat', '-b', 'crash']:
                        return result('x' * 30000 + 'synthetic native crash')
                    if 'exit-info' in tail:
                        return result('synthetic exit reason', 'unsupported-or-timeout', timeout=True)
                    return result()

            with contextlib.redirect_stdout(io.StringIO()) as output:
                harness.diagnostics(Adb())
            self.assertIn('ANDROID_HARNESS_CRASH_LOGCAT', output.getvalue())
            self.assertIn('synthetic native crash', output.getvalue())
            self.assertIn('unsupported-or-timeout', output.getvalue())
            self.assertLess(len((Path(directory) / 'logcat-crash.log').read_text()), 24100)
            self.assertIn('STDERR:', (Path(directory) / 'process-exit-info.log').read_text())
            self.assertIn((['shell', 'dumpsys', '-t', '8', 'activity', 'exit-info', harness.PACKAGE], 10), calls)
            self.assertTrue(all(command[0] in ['shell', 'logcat'] and timeout <= 20 for command, timeout in calls))

    def test_terminal_instrumentation_failure_cannot_be_masked_by_a_pass_marker(self):
        for suffix in ['INSTRUMENTATION_CODE: 0\n', '', 'INSTRUMENTATION_CODE: -1\nINSTRUMENTATION_CODE: 0\n']:
            with self.assertRaises(RuntimeError):
                harness.verify_scope('core', 'ANDROID_SERVICE_ACCEPTANCE passed; scope=core; assertions=20\n' + suffix)

    def scenario(self, readiness_failure=False, cleanup_failure=False, bad_scope=None, install_uncertain=False, crash_scope=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            old = root / 'android/build/acceptance-old'
            old.mkdir(parents=True)
            (old / 'runtime-receipt.json').write_text('{"result":"passed","old":true}')
            calls = []

            def command(adb, tail, timeout=20):
                calls.append(tail)
                if tail[0] == 'install':
                    return result(timeout=True) if install_uncertain else result('Success\n')
                if tail[0] == 'uninstall':
                    return result(code=1) if cleanup_failure else result('Success\n')
                if tail[1:3] == ['am', 'instrument']:
                    scope = tail[7]
                    if scope == crash_scope:
                        return result('INSTRUMENTATION_RESULT: shortMsg=Process crashed.\nINSTRUMENTATION_CODE: 0\n')
                    count = harness.SCOPES[scope] - int(scope == bad_scope)
                    return result('ANDROID_SERVICE_ACCEPTANCE passed; scope=' + scope +
                                  '; assertions=' + str(count) + '\nINSTRUMENTATION_CODE: -1\n')
                return result('unit-fixture\n')

            error = None
            with patch.object(harness, 'ROOT', root), patch.object(harness.Adb, 'command', command), \
                    patch.object(harness.subprocess, 'check_output', return_value='unit-sha\n'), \
                    patch.object(harness, 'wait_ready', side_effect=RuntimeError('readiness failed') if readiness_failure else None, return_value=[[]]):
                try:
                    # Synthetic acceptance markers must never look like actual
                    # installed-service successes in the CI log.
                    with contextlib.redirect_stdout(io.StringIO()):
                        harness.main([])
                except RuntimeError as caught:
                    error = caught
            paths = list((root / 'android/build').glob('acceptance-*/runtime-receipt.json'))
            self.assertEqual(len(paths), 2)
            receipt = json.loads(next(p for p in paths if p.parent != old).read_text())
            self.assertEqual(json.loads((old / 'runtime-receipt.json').read_text())['old'], True)
            return receipt, calls, error

    def test_all_65_assertions_and_cleanup_are_required(self):
        receipt, _, error = self.scenario()
        self.assertIsNone(error)
        self.assertEqual(receipt['result'], 'passed')
        self.assertEqual(receipt['completed_scopes'], list(harness.SCOPES))
        self.assertEqual(receipt['assertions'], 65)
        self.assertTrue(receipt['owned_package_removed'])

    def test_preinstall_failure_has_receipt_and_diagnostics_without_mutation(self):
        receipt, calls, error = self.scenario(readiness_failure=True)
        self.assertIsInstance(error, RuntimeError)
        self.assertEqual(receipt['result'], 'failed')
        self.assertEqual(receipt['assertions'], 0)
        self.assertTrue(any(call[0] == 'logcat' for call in calls))
        self.assertFalse(any(call[0] in ['install', 'uninstall'] for call in calls))

    def test_partial_assertion_count_cannot_produce_success(self):
        receipt, _, error = self.scenario(bad_scope='metadata')
        self.assertIsInstance(error, RuntimeError)
        self.assertEqual(receipt['completed_scopes'], ['core'])
        self.assertEqual(receipt['result'], 'failed')

    def test_scope_process_death_keeps_service_and_exit_evidence_before_cleanup(self):
        receipt, calls, error = self.scenario(crash_scope='metadata')
        self.assertIsInstance(error, RuntimeError)
        self.assertEqual(receipt['completed_scopes'], ['core'])
        self.assertEqual(receipt['assertions'], 20)
        self.assertEqual(receipt['active_scope'], 'metadata')
        self.assertEqual(receipt['bind_deadline_seconds'], 20)
        self.assertEqual(receipt['result'], 'failed')
        uninstall = next(i for i, call in enumerate(calls) if call[0] == 'uninstall')
        exits = [i for i, call in enumerate(calls) if 'exit-info' in call]
        services = [i for i, call in enumerate(calls) if 'services' in call]
        self.assertGreaterEqual(len(exits), 3)
        self.assertEqual(len(services), 2)
        self.assertTrue(all(i < uninstall for i in exits + services))

    def test_cleanup_failure_does_not_leave_a_passed_receipt(self):
        receipt, _, error = self.scenario(cleanup_failure=True)
        self.assertIsInstance(error, RuntimeError)
        self.assertEqual(receipt['assertions'], 65)
        self.assertEqual(receipt['result'], 'failed')

    def test_uncertain_install_does_not_uninstall_an_unconfirmed_package(self):
        receipt, calls, error = self.scenario(install_uncertain=True)
        self.assertIsInstance(error, RuntimeError)
        self.assertTrue(receipt['installation_outcome_uncertain'])
        self.assertFalse(any(call[0] == 'uninstall' for call in calls))


if __name__ == '__main__':
    unittest.main()
