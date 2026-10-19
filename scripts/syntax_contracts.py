#!/usr/bin/env python3
"""Shared acceptance vectors through actual LLP1 entry points, never a mock core."""
import ctypes
import hashlib
import json
import platform
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'fixtures/syntax'


def decode(data):
    at = 4
    assert data[:4] == b'LLP1'

    def integer(n):
        nonlocal at
        value = int.from_bytes(data[at:at+n], 'big')
        assert at+n <= len(data)
        at += n
        return value

    def field():
        nonlocal at
        n = integer(4)
        assert n <= len(data)-at
        value = data[at:at+n]
        at += n
        return value

    result = json.loads(field())
    files = []
    for _ in range(integer(4)):
        mime, name = field().decode(), field().decode()
        resource = integer(8)
        files.append({'mime': mime, 'name': name, 'resourceId': resource, 'bytes': field()})
    assert at == len(data)
    result['files'] = files
    return result


class NativeCore:
    def __init__(self, path):
        self.library = library = ctypes.CDLL(str(path))
        pointer, size = ctypes.c_void_p, ctypes.c_size_t
        library.llp_process.argtypes = [pointer, size, pointer, size, pointer, size]
        library.llp_process.restype = pointer
        library.llp_response_len.argtypes = [pointer]
        library.llp_response_len.restype = size
        library.llp_response_data.argtypes = [pointer]
        library.llp_response_data.restype = pointer
        library.llp_response_free.argtypes = [pointer]

    def __call__(self, request, primary, movie=b''):
        request = json.dumps(request).encode()
        buffers = [ctypes.create_string_buffer(v) for v in [request, primary, movie]]
        response = self.library.llp_process(buffers[0], len(request), buffers[1], len(primary), buffers[2], len(movie))
        assert response, 'native returned null'
        try:
            return decode(ctypes.string_at(self.library.llp_response_data(response), self.library.llp_response_len(response)))
        finally:
            self.library.llp_response_free(response)


class CliCore:
    def __init__(self, path):
        self.path = path

    def __call__(self, request, primary, movie=b''):
        with tempfile.TemporaryDirectory(prefix='llp-cli-syntax-') as temp:
            directory = Path(temp)
            (directory/'primary').write_bytes(primary)
            (directory/'movie').write_bytes(movie)
            command = [str(self.path), request['action'], str(directory/'primary'), '--options-json', json.dumps(request), '--out', str(directory/'out')]
            if movie:
                command += ['--movie', str(directory/'movie')]
            result = subprocess.run(command, capture_output=True, text=True, timeout=40)
            value = json.loads(result.stdout)
            assert (result.returncode == 0) == value['success'], (request, result.returncode, result.stderr)
            for file in value['files']:
                file['bytes'] = (directory/'out'/file['name']).read_bytes()
            return value


class CompiledCCore:
    def __init__(self, directory, include, library):
        self.directory, self.library = directory, library
        self.executable = directory/('c-consumer.exe' if platform.system() == 'Windows' else 'c-consumer')
        compiler = shutil.which('clang') or shutil.which('cc') or shutil.which('gcc')
        if compiler is None:
            raise RuntimeError('real C compiler required for packaged SDK acceptance')
        source = directory/'consumer.c'
        shutil.copyfile(ROOT/'native/acceptance/consumer.c', source)
        command = [compiler, '-std=c11', '-Wall', '-Wextra', '-Werror', '-I', str(include), str(source), '-o', str(self.executable)]
        if platform.system() == 'Linux':
            command.append('-ldl')
        subprocess.run(command, check=True)
        print('PACKAGED_C_CONSUMER_COMPILED ' + json.dumps({'compiler': compiler, 'header': str(include), 'library': str(library)}), flush=True)

    def __call__(self, request, primary, movie=b''):
        with tempfile.TemporaryDirectory(prefix='llp-c-input-', dir=self.directory) as temp:
            directory = Path(temp)
            (directory/'request').write_text(json.dumps(request), encoding='utf-8')
            (directory/'primary').write_bytes(primary)
            (directory/'movie').write_bytes(movie)
            subprocess.run([str(self.executable), str(self.library), str(directory/'request'), str(directory/'primary'), str(directory/'movie'), str(directory/'response')], check=True, timeout=40)
            return decode((directory/'response').read_bytes())


def at_path(value, path):
    for component in path:
        value = value[component]
    return value


def run_cases(execute, label, directory=FIXTURES):
    manifest = json.loads((directory / 'cases.json').read_text())
    for name, digest in manifest['sha256'].items():
        assert hashlib.sha256((directory / name).read_bytes()).hexdigest() == digest, name
    previous, receipts = {}, []

    def data(source):
        if source is None:
            return b''
        if isinstance(source, str):
            return (directory / source).read_bytes()
        return previous[source['case']]['files'][source.get('file', 0)]['bytes']

    for case in manifest['cases']:
        actual = execute(case['request'], data(case['primary']), data(case.get('movie')))
        assert actual['success'] == case['success'], (case['id'], actual['metadata'])
        for fact in case.get('expect', []):
            assert at_path(actual, fact['path']) == fact['value'], (case['id'], fact, actual['metadata'])
        assert len(actual['files']) == len(case.get('files', [])), (case['id'], 'file count')
        for actual_file, expected_file in zip(actual['files'], case.get('files', [])):
            assert actual_file['mime'] == expected_file['mime'], (case['id'], 'mime')
            assert actual_file['bytes'] == data(expected_file['fixture']), (case['id'], 'media bytes')
        previous[case['id']] = actual
        receipts.append({'id': case['id'], 'fields': len(case.get('expect', [])), 'files': len(case.get('files', []))})
        print(f"SYNTAX_CONTRACT_CASE {label} {case['id']} fields={receipts[-1]['fields']} files={receipts[-1]['files']}", flush=True)
    result = {'entry': label, 'cases': len(receipts), 'checks': receipts,
              'manifest_sha256': hashlib.sha256((directory / 'cases.json').read_bytes()).hexdigest()}
    print('SYNTAX_CONTRACTS_PASSED ' + json.dumps(result), flush=True)
    return result
