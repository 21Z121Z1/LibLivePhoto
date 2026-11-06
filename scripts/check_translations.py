#!/usr/bin/env python3
"""Check paired reader documentation, language links and unchanged code examples."""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
EXCLUDED = {'AGENTS.md', 'SKILL.md'}


def code_blocks(text):
    blocks, active, current = [], None, []
    for line in text.splitlines():
        match = re.match(r'^ {0,3}(`{3,}|~{3,})(.*)$', line)
        if active is None and match:
            active = match.group(1)
            current = [line]
        elif active is not None:
            current.append(line)
            if match and match.group(1)[0] == active[0] and len(match.group(1)) >= len(active) and not match.group(2).strip():
                blocks.append('\n'.join(current))
                active, current = None, []
    if active is not None:
        raise ValueError('unclosed code fence')
    return blocks


def check_pair(english, chinese):
    errors = []
    if not chinese.is_file():
        return ['missing Chinese pair: ' + str(chinese)]
    en, zh = english.read_text(), chinese.read_text()
    if f']({chinese.name})' not in en:
        errors.append('missing Chinese language link: ' + str(english))
    if f']({english.name})' not in zh:
        errors.append('missing English language link: ' + str(chinese))
    if len(re.findall(r'[\u4e00-\u9fff]', zh)) < 20:
        errors.append('Chinese document has too little translated prose: ' + str(chinese))
    if code_blocks(en) != code_blocks(zh):
        errors.append('code examples differ between languages: ' + str(english))
    return errors


def main():
    files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', '*.md'], cwd=ROOT, text=True).splitlines()
    sources = [ROOT/p for p in sorted(set(files)) if not p.endswith('.zh-CN.md') and Path(p).name not in EXCLUDED]
    errors = []
    for source in sources:
        errors.extend(check_pair(source, source.with_name(source.stem + '.zh-CN.md')))
    if errors:
        raise SystemExit('\n'.join(errors))
    print(f'checked {len(sources)} English/Chinese documentation pairs and their code examples')


if __name__ == '__main__':
    main()
