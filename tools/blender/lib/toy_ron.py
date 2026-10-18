"""Minimal RON reader for tool scripts (no Blender needed): structs/tuples `( … )` with named
fields become dicts (`_name` = the struct/enum name when one precedes the parenthesis), unnamed
tuples become lists, `[ … ]` lists, `{ … }` maps, strings, numbers, bools, bare identifiers
(enum variants) as str. `//` and `/* */` comments are skipped. Enough for the track and prop
files; not a full RON implementation.

    python3 tools/blender/lib/toy_ron.py   # runs the self-check
"""
import re

_TOKEN = re.compile(r'\s+|//[^\n]*|/\*.*?\*/|"(?:[^"\\]|\\.)*"|[-+]?(?:\d+\.?\d*|\.\d+)'
                    r'(?:[eE][-+]?\d+)?|[A-Za-z_]\w*|[()\[\]{}:,]', re.S)


def _tokens(text):
    pos = 0
    out = []
    while pos < len(text):
        m = _TOKEN.match(text, pos)
        if not m:
            raise ValueError(f'RON: unexpected {text[pos:pos + 20]!r} at {pos}')
        t = m.group(0)
        pos = m.end()
        if not (t.isspace() or t.startswith('//') or t.startswith('/*')):
            out.append(t)
    return out


def loads(text):
    toks = _tokens(text)
    i = 0

    def peek(k=0):
        return toks[i + k] if i + k < len(toks) else None

    def take(expect=None):
        nonlocal i
        t = toks[i]
        if expect is not None and t != expect:
            raise ValueError(f'RON: expected {expect!r}, got {t!r} (token {i})')
        i += 1
        return t

    def value():
        t = peek()
        if t == '(':
            return group(None)
        if t == '[':
            take('[')
            out = []
            while peek() != ']':
                out.append(value())
                if peek() == ',':
                    take(',')
            take(']')
            return out
        if t == '{':
            take('{')
            out = {}
            while peek() != '}':
                k = value()
                take(':')
                out[k] = value()
                if peek() == ',':
                    take(',')
            take('}')
            return out
        take()
        if t.startswith('"'):
            return bytes(t[1:-1], 'utf-8').decode('unicode_escape')
        if t in ('true', 'false'):
            return t == 'true'
        if re.match(r'[-+.\d]', t):
            return float(t) if any(c in t for c in '.eE') else int(t)
        if peek() == '(':  # named struct / enum variant with fields
            return group(t)
        return t

    def group(name):
        take('(')
        if peek() == ')':
            take(')')
            return {'_name': name} if name else []
        named = peek(1) == ':' and re.match(r'[A-Za-z_]', peek() or '')
        if named:
            out = {'_name': name} if name else {}
            while peek() != ')':
                k = take()
                take(':')
                out[k] = value()
                if peek() == ',':
                    take(',')
        else:
            out = []
            while peek() != ')':
                out.append(value())
                if peek() == ',':
                    take(',')
            if name:
                out = {'_name': name, '_items': out}
        take(')')
        return out

    v = value()
    if i != len(toks):
        raise ValueError(f'RON: trailing tokens from {toks[i]!r}')
    return v


def load(path):
    with open(path, encoding='utf-8') as f:
        return loads(f.read())


if __name__ == '__main__':
    d = loads('''// c
    (style: ToyTrack, points: [(pos: (1, 0.3, -2.5), width: 10, wall_left: true), (pos: (0,0,0))],
     props: {"a": (colliders: [Box(half: (0.7, 0.7, 0.7), offset: (0, 0.7, 0))], surface: Wood)},
     gaps: [(from: 2, to: 3, dir: Fwd)], e: (), s: "x\\"y")''')
    assert d['style'] == 'ToyTrack' and d['points'][0]['pos'] == [1, 0.3, -2.5]
    assert d['points'][0]['wall_left'] is True and d['gaps'][0]['dir'] == 'Fwd'
    box = d['props']['a']['colliders'][0]
    assert box['_name'] == 'Box' and box['half'] == [0.7, 0.7, 0.7]
    assert d['props']['a']['surface'] == 'Wood' and d['e'] == [] and d['s'] == 'x"y'
    print("toy_ron ok")
