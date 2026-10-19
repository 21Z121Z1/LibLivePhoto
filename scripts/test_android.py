#!/usr/bin/env python3
"""Install an owned acceptance APK after bounded, read-only readiness probes."""
import argparse
import datetime
import json
import hashlib
import re
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = 'org.liblivephoto.android.acceptance'
SCOPES = {'core': 20, 'metadata': 11, 'lifecycle': 16, 'concurrency': 18, 'syntax': 50}
SYNTAX_MANIFEST = ROOT/'fixtures/syntax/cases.json'
SYNTAX_CASE_IDS = [case['id'] for case in json.loads(SYNTAX_MANIFEST.read_text())['cases']]
SYNTAX_MANIFEST_SHA256 = hashlib.sha256(SYNTAX_MANIFEST.read_bytes()).hexdigest()


def string(value):
    return value.decode('utf-8', errors='replace') if isinstance(value, bytes) else value or ''


class Adb:
    def __init__(self, prefix, out):
        self.prefix, self.out = prefix, out
        self.sequence = 0

    def command(self, tail, timeout=20):
        started = time.monotonic()
        self.sequence += 1
        result = {'command': tail, 'returncode': None, 'timed_out': False,
                  'stdout': '', 'stderr': '', 'sequence': self.sequence,
                  'observed_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='milliseconds')}
        try:
            completed = subprocess.run([*self.prefix, *tail], text=True,
                                       capture_output=True, timeout=timeout)
            result.update(returncode=completed.returncode, stdout=completed.stdout,
                          stderr=completed.stderr)
        except subprocess.TimeoutExpired as error:
            result.update(timed_out=True, stdout=string(error.stdout), stderr=string(error.stderr))
        except OSError as error:
            result['stderr'] = str(error)
        result['elapsed_seconds'] = round(time.monotonic() - started, 3)
        recorded = {**result, 'stdout': result['stdout'][-24000:], 'stderr': result['stderr'][-24000:]}
        with (self.out / 'adb-commands.jsonl').open('a') as output:
            output.write(json.dumps(recorded) + '\n')
        if result['returncode'] != 0 or result['timed_out']:
            print('ANDROID_HARNESS_ADB_FAILURE ' + json.dumps(recorded), flush=True)
        return result

    def require(self, tail, timeout=60):
        result = self.command(tail, timeout)
        if result['returncode'] != 0 or result['timed_out']:
            raise RuntimeError('ADB command failed: ' + json.dumps(result)[-24000:])
        return result['stdout']


def package_listing(result):
    """Only a successful, clean, syntactically valid listing can prove absence."""
    if result['returncode'] != 0 or result['timed_out'] or result['stderr'].strip():
        return None
    packages = []
    for line in result['stdout'].splitlines():
        if not re.fullmatch(r'package:[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*', line.strip()):
            return None
        packages.append(line.strip()[8:])
    return packages


def wait_ready(adb, timeout=120, max_attempts=40, clock=time.monotonic, sleep=time.sleep):
    deadline = clock() + timeout
    attempts = []
    for _ in range(max_attempts):
        attempt = []
        ready = True
        probes = [(['shell', 'getprop', 'sys.boot_completed'], lambda s: s.strip() == '1'),
                  (['shell', 'service', 'check', 'package'], lambda s: bool(re.fullmatch(r'Service package: found\s*', s))),
                  (['shell', 'service', 'check', 'activity'], lambda s: bool(re.fullmatch(r'Service activity: found\s*', s)))]
        for tail, accepts in probes:
            remaining = deadline - clock()
            if remaining <= 0:
                ready = False
                break
            result = adb.command(tail, timeout=min(20, remaining))
            attempt.append(result)
            if result['returncode'] != 0 or result['timed_out'] or not accepts(result['stdout']):
                ready = False
        if ready and clock() < deadline:
            result = adb.command(['shell', 'pm', 'list', 'packages', PACKAGE],
                                 timeout=min(20, deadline - clock()))
            attempt.append(result)
            packages = package_listing(result)
            if packages is not None:
                attempts.append(attempt)
                if PACKAGE in packages:
                    raise RuntimeError('test package already installed; refusing to overwrite: ' + PACKAGE)
                return attempts
        attempts.append(attempt)
        remaining = deadline - clock()
        if remaining <= 0:
            break
        sleep(min(3, remaining))
    raise RuntimeError('Android package/activity services did not become safely queryable '
                       'within the bounded readiness budget; see adb-commands.jsonl (cause undetermined)')


def verify_scope(scope, text):
    marker = r'ANDROID_SERVICE_ACCEPTANCE passed; scope=' + re.escape(scope) + r'; assertions=(\d+)'
    counts = re.findall(marker, text)
    codes = re.findall(r'^INSTRUMENTATION_CODE:\s*(-?\d+)\s*$', text, re.MULTILINE)
    if counts != [str(SCOPES[scope])] or codes != ['-1']:
        raise RuntimeError('actual Android acceptance failed in ' + scope + '; expected ' +
                           str(SCOPES[scope]) + ' assertions and INSTRUMENTATION_CODE: -1')
    if scope == 'syntax':
        ids = re.findall(r'^INSTRUMENTATION_STATUS: syntaxCase=([^\r\n]+)', text, re.MULTILINE)
        digests = re.findall(r'^INSTRUMENTATION_STATUS: syntaxManifestSha256=([^\r\n]+)', text, re.MULTILINE)
        if ids != SYNTAX_CASE_IDS or digests != [SYNTAX_MANIFEST_SHA256]*42:
            raise RuntimeError('actual Android syntax acceptance requires all42 individual cases in manifest order and the exact manifest SHA256')


def diagnostic_snapshot(adb, filename, label, command, timeout):
    result = adb.command(command, timeout=timeout)
    text = result['stdout'][-24000:] + '\nSTDERR:\n' + result['stderr'][-4000:]
    (adb.out / filename).write_text(text)
    print(label + '\n' + text, flush=True)


def system_state(adb, label):
    states = []
    for tail in [['shell', 'getprop', 'sys.boot_completed'],
                 ['shell', 'service', 'check', 'package'],
                 ['shell', 'service', 'check', 'activity']]:
        states.append(adb.command(tail, timeout=10))
    print('ANDROID_HARNESS_SERVICE_DIAGNOSTIC ' + json.dumps({'phase': label, 'states': states})[-24000:], flush=True)


def scope_diagnostics(adb, scope):
    system_state(adb, 'after-' + scope)
    diagnostic_snapshot(adb, scope + '-service-lifecycle.log', 'ANDROID_HARNESS_SCOPE_LIFECYCLE ' + scope,
                        ['logcat', '-d', '-b', 'main', '-b', 'system', '-b', 'crash', '-t', '2000', '-s',
                         'LibLivePhotoBinding:I', 'LibLivePhotoServiceLifecycle:I', 'LibLivePhotoAcceptance:I',
                         'ActivityManager:I', 'AndroidRuntime:E', 'libc:F', '*:S'], 20)
    diagnostic_snapshot(adb, scope + '-bound-services.log', 'ANDROID_HARNESS_BOUND_SERVICES ' + scope,
                        ['shell', 'dumpsys', '-t', '8', 'activity', 'services', PACKAGE], 10)
    diagnostic_snapshot(adb, scope + '-process-exit-info.log', 'ANDROID_HARNESS_SCOPE_EXIT_INFO ' + scope,
                        ['shell', 'dumpsys', '-t', '8', 'activity', 'exit-info', PACKAGE], 10)


def diagnostics(adb):
    system_state(adb, 'final')
    diagnostic_snapshot(adb, 'logcat.log', 'ANDROID_HARNESS_GUEST_LOGCAT',
                        ['logcat', '-d', '-t', '200', '-s', 'LibLivePhotoAcceptance:*',
                         'AndroidRuntime:E', 'libc:F'], 20)
    # The newest main-buffer rows can contain none of the selected tags after
    # instrumentation reports "Process crashed". Preserve the dedicated crash
    # buffer and this package's exit history before uninstall removes evidence.
    diagnostic_snapshot(adb, 'logcat-crash.log', 'ANDROID_HARNESS_CRASH_LOGCAT',
                        ['logcat', '-b', 'crash', '-d', '-t', '200', '*:V'], 20)
    diagnostic_snapshot(adb, 'process-exit-info.log', 'ANDROID_HARNESS_EXIT_INFO',
                        ['shell', 'dumpsys', '-t', '8', 'activity', 'exit-info', PACKAGE], 10)


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument('--adb', default='adb')
    parser.add_argument('--serial')
    args = parser.parse_args(argv)
    build = ROOT / 'android/build'
    build.mkdir(parents=True, exist_ok=True)
    out = Path(tempfile.mkdtemp(prefix='acceptance-', dir=build))
    print('Android acceptance evidence: ' + str(out), flush=True)
    adb = Adb([args.adb] + (['-s', args.serial] if args.serial else []), out)
    receipt = {'result': 'not_executed', 'completed_scopes': [], 'assertions': 0,
               'gallery_acceptance': False, 'readiness_deadline_seconds': 120,
               'readiness_max_attempts': 40, 'per_scope_deadline_seconds': 240,
               'per_operation_deadline_seconds': 40, 'installation_confirmed': False,
               'bind_deadline_seconds': 20,
               'ready_future_wait_deadline_seconds': 20,
               'ready_future_wait_starts_after_bind_returns': True,
               'synchronous_bind_bounded_by_scope_deadline': True,
               'installation_outcome_uncertain': False, 'owned_package_removed': False}
    path = out / 'runtime-receipt.json'
    path.write_text(json.dumps(receipt, indent=2) + '\n')
    failure = None
    try:
        receipt['implementation_sha'] = subprocess.check_output(
            ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True, timeout=10).strip()
        print(adb.require(['devices']), flush=True)
        adb.require(['get-state'])
        attempts = wait_ready(adb)
        receipt['readiness_attempts'] = len(attempts)
        receipt['package_absence_confirmed'] = True
        install = adb.command(['install', '--no-streaming', str(build / 'liblivephoto-acceptance.apk')], timeout=120)
        if install['returncode'] != 0 or install['timed_out'] or not re.search(r'^Success\s*$', install['stdout'], re.MULTILINE):
            receipt['installation_outcome_uncertain'] = True
            raise RuntimeError('APK installation did not confirm success: ' + json.dumps(install)[-24000:])
        receipt['installation_confirmed'] = True
        transcripts = []
        for scope, count in SCOPES.items():
            receipt['active_scope'] = scope
            system_state(adb, 'before-' + scope)
            print('Actual installed service acceptance scope: ' + scope, flush=True)
            result = adb.command(['shell', 'am', 'instrument', '-w', '-r', '-e', 'scope', scope,
                                  PACKAGE + '/org.liblivephoto.android.ServiceAcceptance'], timeout=240)
            text = result['stdout']
            (out / ('instrumentation-' + scope + '.log')).write_text(
                text + '\nSTDERR:\n' + result['stderr'])
            transcripts.append(text)
            (out / 'instrumentation.log').write_text('\n'.join(transcripts))
            print(text, flush=True)
            scope_diagnostics(adb, scope)
            if result['returncode'] != 0 or result['timed_out']:
                raise RuntimeError('ADB instrumentation failed or exceeded its deadline in ' + scope)
            verify_scope(scope, text)
            receipt['completed_scopes'].append(scope)
            receipt['assertions'] += count
        for name, property_name in [('device', 'ro.product.model'), ('sdk', 'ro.build.version.sdk'),
                                    ('abi', 'ro.product.cpu.abi'), ('hardware', 'ro.hardware')]:
            receipt[name] = adb.require(['shell', 'getprop', property_name]).strip()
        receipt['scope'] = 'installed APK, four remote services, isolated parser, actual Rust JNI, FD ranges/ownership, byte-exact extract/build, diagnostics/cancel'
    except BaseException as error:
        failure = error
        receipt['error'] = str(error)
    finally:
        diagnostics(adb)
        if receipt['installation_confirmed']:
            cleanup = adb.command(['uninstall', PACKAGE], timeout=60)
            receipt['owned_package_removed'] = cleanup['returncode'] == 0 and not cleanup['timed_out'] and cleanup['stdout'].strip() == 'Success'
            if not receipt['owned_package_removed']:
                receipt['cleanup_error'] = 'owned acceptance package removal did not confirm success'
                if failure is None:
                    failure = RuntimeError(receipt['cleanup_error'])
        receipt['result'] = 'passed' if failure is None and receipt['assertions'] == sum(SCOPES.values()) and receipt['owned_package_removed'] else 'failed'
        path.write_text(json.dumps(receipt, indent=2) + '\n')
        print('ANDROID_HARNESS_RUNTIME_RECEIPT ' + json.dumps(receipt), flush=True)
    if failure is not None:
        raise failure


if __name__ == '__main__':
    main()
