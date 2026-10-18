"""Section-loft body shells: smooth, closed, with real wheel wells.

A section (right half, bottom centre -> top centre) is the filleted polyline
    P0 (0, z0) - Pa (xi, z0) - Pb (xi, za) - P1 (hb, za) - Pm (hw + bulge, zm) - P2 (hw, zb)
    - P3 (hg, zg) - P4 (hr, zt) - P5 (0, zt + crown)
i.e. underside, wheel-well inner wall (Pa-Pb, only where an arch lifts the outer underside to
`za`), arch ceiling, side panel (optional bulge), shoulder, deck, greenhouse side, roof.
Every parameter is a monotone-cubic curve over y keyed by the car script; unspecified
parameters follow defaults derived from the others. Underside, well walls and arch ceilings
get `plastic`, the rest `paint`.
"""
import math

from mathutils import Vector

from . import core, geo

V = Vector
PARAMS = ('hb', 'hw', 'bulge', 'zm', 'z0', 'zb', 'hg', 'zg', 'hr', 'zt', 'crown',
          'rb', 'rm', 'rs', 'rg', 'rr')


class Profile:
    def __init__(self, keys):
        """keys: [(y, {param: value, ...}), ...]; a parameter may be keyed at any subset of y."""
        self.c = {}
        for name in PARAMS:
            pts = [(y, d[name]) for y, d in keys if name in d]
            if pts:
                self.c[name] = geo.Curve(pts) if len(pts) > 1 else (lambda v: (lambda y: v))(pts[0][1])

    def at(self, y):
        c = self.c
        p = {k: f(y) for k, f in c.items()}
        p.setdefault('hb', p['hw'] * 0.94)
        p.setdefault('bulge', 0.0)
        p.setdefault('zm', 0.45)
        p.setdefault('hg', p['hw'] - 0.03)
        p.setdefault('zg', p['zb'] + 0.01)
        p.setdefault('zt', p['zg'] + 0.01)
        p.setdefault('hr', p['hg'] - 0.02)
        p.setdefault('crown', 0.01)
        for r, v in (('rb', 0.03), ('rm', 0.2), ('rs', 0.04), ('rg', 0.03), ('rr', 0.04)):
            p.setdefault(r, v)
        return p


def arches(D, clear=0.025, gap=0.03):
    """Front and rear arch tuples (y, z_centre, radius, x_inner) from car-dimensions."""
    out = []
    for i, y in ((0, D['wheelbase'] / 2), (1, -D['wheelbase'] / 2)):
        r, w, t = D['radius'][i], D['wheel_width'][i], D['track'][i]
        out.append((y, r, r + clear, t / 2 - w / 2 - gap))
    return out


SUB = 3  # interior points on the flank (P3-P4) so fender lifts bend it smoothly


def section(p, z0, za, xi, seg):
    xi = min(xi, p['hb'] - 0.03)
    zm = za + (p['zb'] - za) * p['zm']
    pts = [(0.0, z0), (xi, z0), (xi, za), (p['hb'], za), (p['hw'] + p['bulge'], zm),
           (p['hw'], p['zb']), (p['hg'], p['zg']), (p['hr'], p['zt']), (0.0, p['zt'] + p['crown'])]
    radii = [0, 0, 0, p['rb'], p['rm'], p['rs'], p['rg'], p['rr'], 0]
    out = geo.rounded_poly(pts, radii, seg, closed=False)
    a = 1 + 6 * (seg + 1) - 1  # last point of the P3 fillet: subdivide the flank P3-P4
    out[a + 1:a + 1] = [out[a].lerp(out[a + 1], i / (SUB + 1)) for i in range(1, SUB + 1)]
    return out


def ring(half, y):
    """Full ring (CCW seen from +Y) from a right-half section."""
    right = [V((q.x, y, q.y)) for q in half]
    left = [V((-q.x, y, q.y)) for q in half[-2:0:-1]]
    return right + left


def _arch(y, z0, arch_list):
    """(lifted outer underside height, well inner x) at station y."""
    za, xi, best = z0, None, None
    for ya, zc, ra, x_in in arch_list:
        dy = y - ya
        if best is None or abs(dy) < best:
            best, xi = abs(dy), x_in
        if abs(dy) < ra:
            za = max(za, zc + math.sqrt(ra * ra - dy * dy))
    return za, (xi if xi is not None else 1e3)


def stations(y0, y1, arch_list, step=0.05, ends=(0.0, 0.0), extra=()):
    ys = set()
    n = max(2, int(round((y1 - y0) / step)))
    ys.update(y0 + (y1 - y0) * i / n for i in range(n + 1))
    for ya, _, ra, _ in arch_list:
        for s in (-1, 1):
            ys.update((ya + s * (ra + 2e-4), ya + s * (ra - 2e-4)))
        ys.update(ya + ra * math.cos(math.pi * k / 10) for k in range(1, 10))
    for e, sgn, y_end in ((ends[0], 1, y0), (ends[1], -1, y1)):
        if e > 0:
            ys.update(y_end + sgn * e * (1 - math.cos(math.pi / 2 * k / 5)) for k in range(1, 5))
    ys.update(extra)
    out = sorted(y for y in ys if y0 - 1e-9 <= y <= y1 + 1e-9)
    clean = [out[0]]
    for y in out[1:]:
        if y - clean[-1] > 1e-4:
            clean.append(y)
    return clean


def shell(bm, prof, y0, y1, arches=(), seg=3, step=0.05, ends=(0.08, 0.08), end_scale=0.55,
          extra=(), min_thick=0.025, roof=None, cap=None):
    """Loft the shell from y0 (rear) to y1 (front); `ends` = rounding length at rear/front.
    Over each wheel well the skin is lifted (smoothly toward the hood) to clear the well
    ceiling by `min_thick`, so fenders bulge to follow the arch; `cap` limits that lift (and
    the ceiling) to the body height. `roof` = (y_low, y_high, material): paint the roof panel
    (P4 fillet to centre) between those stations with another material."""
    rings = []
    clip = 0.0
    ys = stations(y0, y1, arches, step, ends, tuple(extra) + (roof[:2] if roof else ()))
    for y in ys:
        p = prof.at(y)
        z0 = p['z0']
        za, xi = _arch(y, z0, arches)
        if cap is not None and za > cap - min_thick:
            clip = max(clip, za - (cap - min_thick))
            za = cap - min_thick
        if za > z0:  # lift the skin over the well: fenders follow the arch
            need = za + min_thick
            p = dict(p)
            p['zb'] = max(p['zb'], need)
            p['zg'] = max(p['zg'], need) if p['hg'] >= min(xi, p['hb'] - 0.03) else p['zg']
        half = section(p, z0, za, xi, seg)
        if za > z0:
            xw = min(xi, p['hb'] - 0.03)
            k0 = 1 + 5 * (seg + 1)  # from the shoulder fillet on
            for k in range(k0, len(half)):
                q = half[k]
                w = max(0.0, min(1.0, (q.x - (xw - 0.07)) / 0.07))
                w = w * w * (3 - 2 * w)
                if q.y < need and w > 0:
                    half[k] = V((q.x, q.y + (need - q.y) * w))
        for e, d in ((ends[0], y - y0), (ends[1], y1 - y)):  # round the nose/tail
            if e > 0 and d < e:
                t = d / e
                s = end_scale + (1 - end_scale) * math.sqrt(max(0.0, 1 - (1 - t) ** 2))
                cz = (z0 + p['zt'] + p['crown']) / 2
                half = [V((q.x * s, cz + (q.y - cz) * s)) for q in half]
        rings.append(ring(half, y))
    if clip > 0:
        print(f'shell: arch ceiling capped by up to {clip:.3f} (body height cap)')
    paint, plastic = core.mi('paint'), core.mi('plastic')
    fs = geo._bridge(bm, rings, paint)
    L = len(rings[0])
    k_mid = 1 + 2 * (seg + 1) + seg // 2  # underside columns end mid-way round the arch lip
    k_roof = 1 + 6 * (seg + 1) + SUB + seg // 2  # roof panel: from mid roof-edge fillet
    for i, f in enumerate(fs[:-2]):
        j = i % L
        col = min(j, L - 1 - j)
        if col < k_mid:
            f.material_index = plastic
        elif roof and col >= k_roof and roof[0] - 1e-6 <= ys[i // L] < roof[1] - 1e-6:
            f.material_index = core.mi(roof[2])
    return geo._finish(bm, fs)


def glass(bm, surf, prof, windshield=None, side=None, rear=None, m=0.018, r=0.02):
    """Window decals that follow the shell: windshield/rear (projected from above, between the
    roof-edge lines) and side windows (projected from the sides, between belt and roof edge).
    Each range is (y_low, y_high); margins `m` leave paint pillars and frames."""
    mat = core.mi('glass')

    def hr(y):
        return prof.at(y)['hr']

    for rng in (windshield, rear):
        if rng:
            ys = [rng[0] + (rng[1] - rng[0]) * i / 8 for i in range(9)]
            right = [V((hr(y) - m, y)) for y in ys]
            poly = right + [V((-p.x, p.y)) for p in reversed(right)]
            poly = geo.rounded_poly(geo.ccw(poly), r, 2)
            geo.decal(bm, surf, poly, V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)), mat, back=0.2)
    if side:
        ys = [side[0] + (side[1] - side[0]) * i / 16 for i in range(17)]
        bot = [V((y, prof.at(y)['zg'] + m * 0.7)) for y in ys]
        top = [V((y, prof.at(y)['zt'] - prof.at(y)['rr'] * 0.6 - m * 0.5)) for y in ys]
        ok = [t.y - b.y >= 0.012 for t, b in zip(top, bot)]  # keep where the greenhouse is tall
        i0 = ok.index(True)
        i1 = len(ok) - ok[::-1].index(True)
        bot, top = bot[i0:i1], top[i0:i1]
        for s in (-1, 1):  # u = (0, s, 0): plane x = s*y, normal points outward
            poly = [V((s * p.x, p.y)) for p in bot + top[::-1]]
            poly = geo.rounded_poly(geo.ccw(poly), r, 2)
            geo.decal(bm, surf, poly, V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)), mat, back=0.2)
