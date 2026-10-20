"""Places `toy_track_support` pillars under every elevated stretch of the toy-track ribbon of a
track file and rewrites the block between the `// supports:begin` / `// supports:end` lines
of its `props` list (prints the block when the markers are missing).

The centreline mirrors crates/rc_track/src/spline.rs (closed centripetal Catmull-Rom, one-sided
tangents at active gaps, smoothstep width/bank, banked frames), computed for each direction
with its own active gaps; pillars that exist in both directions get `dir: Both`. A pillar stands
outside each edge (clear of the barrier), its bracket under the ribbon's flat bottom; the
prop's origin is the bracket top and its column runs down into the floor. Pillars that would
pierce a lower part of the track, or a keep-out circle (the castle inside the helix), are
skipped.

    python3 tools/blender/props/toy/supports.py [assets/maps/toy_town/track.ron]
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, '..', '..')))
from lib import toy_ron  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
KIND = 'toy_track_support'
THICK = 0.25          # ToyTrack ribbon thickness below the surface (mesh.rs)
CHAMFER = 1.05        # flat bottom reaches half width + this
PILLAR = 1.65         # pillar centre: half width + this (column half size 0.4: clear of walls)
TIP = 0.2             # bracket tip: half width + this
MIN_CLEAR = 0.5       # support only where the bottom is this far above the floor
FLOOR = 0.05          # play-mat height
SPACING = 9.0         # target pillar spacing along the centreline (u)
KEEP_OUT = [((-40.0, -25.0), 10.3)]  # (centre x, z), radius: the castle inside the helix


def sub(a, b): return [a[i] - b[i] for i in range(3)]
def add(a, b): return [a[i] + b[i] for i in range(3)]
def mul(a, k): return [a[i] * k for i in range(3)]
def dot(a, b): return sum(a[i] * b[i] for i in range(3))
def norm(a): return mul(a, 1.0 / math.sqrt(dot(a, a)))
def dist(a, b): return math.sqrt(dot(sub(a, b), sub(a, b)))


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def catmull_rom(p, u):
    knot = lambda a, b: math.sqrt(max(dist(a, b), 1e-4))  # noqa: E731
    t1 = knot(p[0], p[1])
    t2 = t1 + knot(p[1], p[2])
    t3 = t2 + knot(p[2], p[3])
    t = t1 + (t2 - t1) * u

    def mix(a, b, ta, tb):
        return add(mul(a, (tb - t) / (tb - ta)), mul(b, (t - ta) / (tb - ta)))

    a1, a2, a3 = mix(p[0], p[1], 0, t1), mix(p[1], p[2], t1, t2), mix(p[2], p[3], t2, t3)
    return mix(mix(a1, a2, 0, t2), mix(a2, a3, t1, t3), t1, t2)


def smoothstep(t):
    t = min(max(t, 0.0), 1.0)
    return t * t * (3 - 2 * t)


def frame(fwd, bank):
    up0 = norm(sub([0, 1, 0], mul(fwd, fwd[1])))
    right0 = cross(fwd, up0)
    up = add(mul(up0, math.cos(bank)), mul(right0, math.sin(bank)))
    return cross(fwd, up), up


def resolve(track):
    """Per point: pos, width (inherited), bank (radians, not inherited)."""
    pts, width = [], 10.0
    for p in track['points']:
        width = p.get('width', width)
        pts.append((p['pos'], width, math.radians(p.get('bank_deg', 0.0))))
    return pts


def centreline(pts, gaps):
    """Evaluator (seg, u) -> (pos, forward, half width, bank) and the dense polyline."""
    n = len(pts)
    gap = [False] * n
    for a, b in gaps:
        s = a
        while s != b:
            gap[s] = True
            s = (s + 1) % n
    pos = lambda i: pts[i % n][0]  # noqa: E731

    def ctrl(i):
        p1, p2 = pos(i), pos(i + 1)
        p0 = sub(mul(p1, 2), p2) if gap[(i - 1) % n] else pos(i - 1)
        p3 = sub(mul(p2, 2), p1) if gap[(i + 1) % n] else pos(i + 2)
        return [p0, p1, p2, p3]

    def at(i, u):
        c = ctrl(i)
        p = catmull_rom(c, u)
        q = catmull_rom(c, min(u + 1e-3, 1.0)) if u < 1.0 else p
        r = catmull_rom(c, u - 1e-3) if u >= 1.0 else p
        fwd = norm(sub(q, r) if u >= 1.0 else sub(q, p))
        w = smoothstep(u)
        hw = 0.5 * (pts[i][1] + (pts[(i + 1) % n][1] - pts[i][1]) * w)
        bank = pts[i][2] + (pts[(i + 1) % n][2] - pts[i][2]) * w
        return p, fwd, hw, bank

    dense = []  # (pos, half width, arc length)
    s = 0.0
    for i in range(n):
        if gap[i]:
            continue
        steps = max(8, int(math.ceil(dist(pos(i), pos(i + 1)) / 0.5)))
        prev = None
        for k in range(steps + 1):
            p, _, hw, _ = at(i, k / steps)
            if prev is not None:
                s += dist(p, prev)
            dense.append((p, hw, s, i))
            prev = p
    return at, gap, dense


def supports(track, rev):
    pts = resolve(track)
    n = len(pts)
    gaps = [(g['from'], g['to']) for g in track.get('gaps', [])
            if g.get('dir', 'Both') in ('Both', 'Rev' if rev else 'Fwd')]
    at, gap, dense = centreline(pts, gaps)
    out = []
    for i in range(n):
        if gap[i]:
            continue
        length = dist(pts[i][0], pts[(i + 1) % n][0])
        k = max(1, round(length / SPACING))
        for j in range(k):
            p, fwd, hw, bank = at(i, j / k)
            right, up = frame(fwd, bank)
            flat = norm([right[0], 0.0, right[2]])
            s_here = min((dist(p, d[0]), d[2]) for d in dense if d[3] == i)[1]
            for side in (-1, 1):
                edge = add(add(p, mul(right, side * (hw + CHAMFER))), mul(up, -THICK))
                tip = add(add(p, mul(right, side * (hw + TIP))), mul(up, -THICK))
                top = min(edge[1], tip[1]) - 0.01
                if top - FLOOR < MIN_CLEAR:
                    continue
                c = add(p, mul(flat, side * (hw + PILLAR) * math.cos(bank)))
                c[1] = top
                if any(math.hypot(c[0] - x, c[2] - z) < r for (x, z), r in KEEP_OUT):
                    continue
                if pierces(c, dense, s_here):
                    continue
                d = mul(flat, -side)  # front toward the centreline
                yaw = math.degrees(math.atan2(-d[0], -d[2]))
                out.append((c, yaw))
    return out


def pierces(c, dense, s_here, column=0.45):
    """The column (from the floor to the bracket) would cut through another part of the track."""
    total = dense[-1][2]
    for p, hw, s, _ in dense:
        ds = abs(s - s_here)
        if min(ds, total - ds) < 20.0:
            continue
        if p[1] < c[1] - 0.3 and math.hypot(c[0] - p[0], c[2] - p[2]) < hw + CHAMFER + column:
            return True
    return False


def merge(fwd, rev):
    lines, used = [], set()
    for c, yaw in fwd:
        m = next((k for k, (d, _) in enumerate(rev) if k not in used and dist(c, d) < 0.06), None)
        if m is not None:
            used.add(m)
        lines.append((c, yaw, 'Both' if m is not None else 'Fwd'))
    lines += [(c, yaw, 'Rev') for k, (c, yaw) in enumerate(rev) if k not in used]
    return lines


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'assets', 'maps',
                                                              'toy_town', 'track.ron')
    track = toy_ron.load(path)
    lines = merge(supports(track, False), supports(track, True))
    body = [f'        (kind: "{KIND}", pos: ({c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f}), '
            f'yaw: {yaw:.1f}' + ('' if d == 'Both' else f', dir: {d}') + '),'
            for c, yaw, d in lines]
    text = open(path, encoding='utf-8').read()
    a, b = text.find('// supports:begin'), text.find('// supports:end')
    if a < 0 or b < 0:
        print('\n'.join(body))
        return
    a = text.index('\n', a) + 1
    b = text.rindex('\n', 0, b) + 1
    with open(path, 'w', encoding='utf-8') as f:
        f.write(text[:a] + '\n'.join(body) + '\n' + text[b:])
    counts = {d: sum(1 for *_, e in lines if e == d) for d in ('Both', 'Fwd', 'Rev')}
    print(f'{path}: {len(lines)} supports {counts}')


if __name__ == '__main__':
    main()
