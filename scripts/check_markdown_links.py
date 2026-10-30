#!/usr/bin/env python3
"""Check repo-relative Markdown links and heading anchors.

External URLs are ignored. Fenced code, inline code, and YAML front matter are
not scanned. Heading anchors follow GitHub slug rules, including numeric
suffixes for duplicate headings. Fragments shaped like GitHub line anchors
(L10 or L10-L20) are accepted without a heading match.
"""
import re
import sys
import tempfile
import unicodedata
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
MARKDOWN = {'.md', '.markdown', '.mdx'}
SCHEME = re.compile(r'^[A-Za-z][A-Za-z0-9+.-]*:')
FENCE = re.compile(r'^( {0,3})(`{3,}|~{3,})(.*)$')
ATX = re.compile(r' {0,3}#{1,6}[ \t]+(.*)$')
SETEXT = re.compile(r' {0,3}(=+|-+)[ \t]*$')
REF_DEF = re.compile(r' {0,3}\[[^\]]+\]:[ \t]*(<[^>\n]+>|\S+)')
LINE_ANCHOR = re.compile(r'^L\d+(?:-L\d+)?$')
INLINE_CODE = re.compile(r'(?<!`)(`+)(?!`)(.*?)\1(?!`)')


def slug(value):
    """GitHub-style slug: keep letters, marks, numbers, underscores, and hyphens."""
    if not isinstance(value, str):
        return ''
    kept = []
    for char in value.lower():
        if char in ' -_' or unicodedata.category(char)[0] in 'LMN' or unicodedata.category(char) == 'Pc':
            kept.append(char)
    return ''.join(kept).replace(' ', '-')


def unique_slug(occurrences, value):
    result = slug(value)
    original = result
    while result in occurrences:
        occurrences[original] += 1
        result = f'{original}-{occurrences[original]}'
    occurrences[result] = 0
    return result


def renderish(text):
    text = re.sub(r'<[^>\n]+>', '', text)
    text = re.sub(r'!\[([^\]]*)\]\([^)]*\)', r'\1', text)
    text = re.sub(r'\[([^\]]*)\]\([^)]*\)', r'\1', text)
    text = re.sub(r'`([^`]+)`', r'\1', text)
    text = text.replace('**', '').replace('__', '')
    text = re.sub(r'(?<!\w)_([^_\n]+)_(?!\w)', r'\1', text)
    text = re.sub(r'(?<!\w)\*([^*\n]+)\*(?!\w)', r'\1', text)
    return text.strip()


def mask_inline_code(line):
    return INLINE_CODE.sub(lambda match: ' ' * len(match.group(0)), line)


def unescape_destination(value):
    out = []
    i = 0
    while i < len(value):
        if value[i] == '\\' and i + 1 < len(value):
            out.append(value[i + 1])
            i += 2
            continue
        out.append(value[i])
        i += 1
    return ''.join(out)


def parse_destination(line, start):
    i = start
    while i < len(line) and line[i] in ' \t':
        i += 1
    if i >= len(line):
        return None
    if line[i] == '<':
        end = line.find('>', i + 1)
        if end < 0:
            return None
        dest = line[i:end + 1]
        j = end + 1
    else:
        depth = 0
        j = i
        while j < len(line):
            char = line[j]
            if char == '\\' and j + 1 < len(line):
                j += 2
                continue
            if char == '(':
                depth += 1
            elif char == ')':
                if depth == 0:
                    break
                depth -= 1
            elif char in ' \t' and depth == 0:
                break
            j += 1
        if j == i:
            return None
        dest = line[i:j]
    while j < len(line) and line[j] in ' \t':
        j += 1
    if j < len(line) and line[j] in '"\'':
        quote = line[j]
        close = line.find(quote, j + 1)
        if close < 0:
            return None
        j = close + 1
        while j < len(line) and line[j] in ' \t':
            j += 1
    if j >= len(line) or line[j] != ')':
        return None
    return dest, j


def inline_destinations(line):
    i = 0
    while True:
        found = line.find('](', i)
        if found < 0:
            return
        if found > 0 and line[found - 1] == '\\':
            i = found + 2
            continue
        parsed = parse_destination(line, found + 2)
        if parsed is None:
            i = found + 2
            continue
        dest, end = parsed
        yield dest
        i = end + 1


def content_lines(text):
    lines = text.splitlines()
    start = 0
    if lines and lines[0].strip() == '---':
        for index in range(1, len(lines)):
            if lines[index].strip() == '---':
                start = index + 1
                break
    in_fence = None
    for index in range(start, len(lines)):
        line = lines[index]
        fence = FENCE.match(line)
        if fence:
            marker = fence.group(2)
            info = fence.group(3)
            if in_fence is None:
                in_fence = (marker[0], len(marker))
                continue
            char, length = in_fence
            if marker[0] == char and len(marker) >= length and info.strip() == '':
                in_fence = None
                continue
        if in_fence is not None:
            continue
        yield index + 1, mask_inline_code(line)


def classify(dest):
    dest = dest.strip()
    if dest.startswith('<') and dest.endswith('>') and len(dest) >= 2:
        dest = dest[1:-1].strip()
    dest = unescape_destination(dest).strip()
    if not dest or SCHEME.match(dest) or dest.startswith('//'):
        return None
    path, fragment = dest, None
    if '#' in dest:
        path, fragment = dest.split('#', 1)
        fragment = unquote(fragment)
    path = unquote(path.split('?', 1)[0])
    return path, fragment


def heading_slugs(rows):
    slugs = []
    occurrences = {}
    index = 0
    while index < len(rows):
        _, line = rows[index]
        if index + 1 < len(rows):
            next_no, next_line = rows[index + 1]
            line_no = rows[index][0]
            if next_no == line_no + 1 and SETEXT.fullmatch(next_line) and line.strip() and not line.lstrip().startswith('#'):
                slugs.append(unique_slug(occurrences, renderish(line.strip())))
                index += 2
                continue
        heading = ATX.match(line)
        if heading:
            content = re.sub(r'[ \t]+#+\s*$', '', heading.group(1)).strip()
            if content:
                slugs.append(unique_slug(occurrences, renderish(content)))
        index += 1
    return slugs


def destinations(rows):
    found = []
    for line_no, line in rows:
        for dest in inline_destinations(line):
            found.append((line_no, dest))
        ref = REF_DEF.match(line)
        if ref:
            found.append((line_no, ref.group(1)))
    return found


def resolve_target(root, source, path):
    if path.startswith('/'):
        target = (root / path.lstrip('/')).resolve()
    else:
        target = (source.parent / path).resolve()
    try:
        target.relative_to(root)
    except ValueError:
        return None
    return target


def check_file(root, path, cache):
    text = path.read_text(encoding='utf-8')
    rows = list(content_lines(text))
    errors = []
    checked = 0
    for line_no, dest in destinations(rows):
        classified = classify(dest)
        if classified is None:
            continue
        rel_path, fragment = classified
        checked += 1
        if rel_path == '':
            target = path
        else:
            target = resolve_target(root, path, rel_path)
            if target is None:
                errors.append(f'{path.relative_to(root)}:{line_no}: link escapes the repository: {dest}')
                continue
            if not target.exists():
                errors.append(f'{path.relative_to(root)}:{line_no}: missing target for {dest}')
                continue
        if fragment is None or fragment == '' or LINE_ANCHOR.fullmatch(fragment):
            continue
        if target.suffix.lower() not in MARKDOWN:
            continue
        key = str(target)
        if key not in cache:
            cache[key] = set(heading_slugs(list(content_lines(target.read_text(encoding='utf-8')))))
        if fragment not in cache[key] and fragment.lower() not in cache[key]:
            errors.append(f'{path.relative_to(root)}:{line_no}: missing heading anchor #{fragment} in {target.relative_to(root)}')
    return checked, errors


def markdown_files(root):
    files = []
    for path in root.rglob('*'):
        if not path.is_file() or path.suffix.lower() not in MARKDOWN:
            continue
        if any(part.startswith('.') and part != '.agents' for part in path.relative_to(root).parts):
            continue
        if '.git' in path.parts:
            continue
        files.append(path)
    return sorted(files)


def check_repository(root):
    cache = {}
    errors = []
    checked = 0
    files = markdown_files(root)
    for path in files:
        count, file_errors = check_file(root, path, cache)
        checked += count
        errors.extend(file_errors)
    return len(files), checked, errors


def self_test():
    assert slug('Hello World') == 'hello-world'
    assert slug('C++') == 'c'
    assert slug('foo & bar') == 'foo--bar'
    assert slug('100% done') == '100-done'
    occurrences = {}
    assert unique_slug(occurrences, 'Hello') == 'hello'
    assert unique_slug(occurrences, 'Hello') == 'hello-1'
    assert unique_slug(occurrences, 'Hello') == 'hello-2'
    sample = """---
title: ignored
---
# Title
See [guide](guide.md#section) and [site](https://example.com/a.md).
```md
[skipped](missing.md)
```
[fenced lookalike](real.md)
## Section
## Section
A paragraph
---
"""
    rows = list(content_lines(sample))
    slugs = heading_slugs(rows)
    assert slugs[:3] == ['title', 'section', 'section-1'], slugs
    dests = [classify(dest) for _, dest in destinations(rows)]
    assert ('guide.md', 'section') in dests
    assert ('real.md', None) in dests
    assert all(item is None or item[0] != 'missing.md' for item in dests)
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / 'guide.md').write_text('# Section\n\n## Section\n', encoding='utf-8')
        (root / 'real.md').write_text('# Real\n', encoding='utf-8')
        (root / 'index.md').write_text(sample + '\n[line](real.md#L12)\n[gone](nope.md)\n[bad](guide.md#nope)\n', encoding='utf-8')
        _, errors = check_file(root, root / 'index.md', {})
    messages = '\n'.join(errors)
    assert 'missing.md' not in messages
    assert 'nope.md' in messages
    assert '#nope' in messages
    assert 'L12' not in messages
    assert not any('example.com' in message for message in errors)
    print('self-test passed')


def main():
    self_test()
    if '--self-test' in sys.argv:
        return
    files, checked, errors = check_repository(ROOT)
    for error in errors:
        print(error, file=sys.stderr)
    print(f'checked {checked} repo-relative links in {files} Markdown files')
    if errors:
        print(f'{len(errors)} broken repo-relative links', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
