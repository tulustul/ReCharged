"""Storm-drain dip (hero): a concrete dish 24 x 18 u, 2 u deep, along the kerb with a cast-iron
grate at the bottom, the kerb inlet (dark mouth under a concrete lintel), a painted fish marker
and the catch-basin lid on the planting strip. The Golden Battery floats in the mouth.

    blender -b -P tools/blender/props/street/storm_drain.py -- [--preview DIR]

Prop space (Blender): kerb face line along X at y = 0, road toward +Y, origin at road level.
Matches catalog.py DRAIN / street_storm_drain colliders.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

import catalog as C  # noqa: E402

d = C.DRAIN
HL, W, DEPTH, BOT, SIDE = d['length'] / 2, d['width'], d['depth'], d['bottom'] / 2, d['side']
GRATE = (-2.0, 2.0, 0.4, 4.4)  # x0, x1, y0, y1 of the grate opening


def surface_z(x, y):
    sx = -DEPTH + max(0.0, min(1.0, (abs(x) - BOT) / (HL - BOT))) * DEPTH
    sy = -DEPTH + max(0.0, min(1.0, (y - (W - SIDE)) / SIDE)) * DEPTH
    return max(sx, sy)


def dish(bm):
    xs = sorted(set([round(-HL + i * 1.25, 3) for i in range(int(2 * HL / 1.25) + 1)]
                    + [-BOT, BOT, GRATE[0], GRATE[1], 0.0, HL]))
    ys = sorted(set([0.0, GRATE[2], 2.0, GRATE[3], 7.0, 10.5, W - SIDE, W - SIDE / 2, W]))
    vs = [[bm.verts.new(V((x, y, surface_z(x, y)))) for y in ys] for x in xs]
    for i in range(len(xs) - 1):
        for j in range(len(ys) - 1):
            cx, cy = (xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2
            if GRATE[0] < cx < GRATE[1] and GRATE[2] < cy < GRATE[3]:
                continue  # grate opening
            f = bm.faces.new((vs[i][j], vs[i + 1][j], vs[i + 1][j + 1], vs[i][j + 1]))
            f.material_index = mi('gutter') if cy < 2.0 else mi('concrete')
            if f.normal.z < 0:
                f.normal_flip()


def build():
    start('concrete', 'gutter', 'kerb', 'iron', 'void', 'paint_white')
    street_props.use_outdoor(hole=(-HL, HL, 0.0, W))
    bm = core.new_bm()
    dish(bm)
    surf = core.obj_from_bm('dish', bm, weighted=False, angle=20)

    # deep kerb over the dish length (face reaches below the dish bottom)
    k = core.new_bm()
    stone = [(0.0, -DEPTH - 0.1), (0.0, 0.52), (-0.025, 0.6), (-0.07, 0.638), (-0.13, KERB_H),
             (-0.55, KERB_H), (-KERB_D, 0.62), (-KERB_D, 0.45), (-0.4, -DEPTH - 0.1)]
    for x0, x1 in ((-HL, -3.4), (-3.4, 3.4), (3.4, HL)):  # stones; the middle one is the lintel
        sweep(k, [(x0 + 0.015, 0, 0, 1), (x1 - 0.015, 0, 0, 1)], stone, mi('kerb'))
    kerb = core.obj_from_bm('kerb', k, bevel=0.03, segments=1, angle=30)

    # inlet mouth: dark slot framed by thin concrete lips, plus the grate and its pit
    m = core.new_bm()
    zb, zt = -DEPTH + 0.05, -0.75
    geo.box(m, V((0, 0.012, (zb + zt) / 2)), (6.0, 0.02, zt - zb), mi('void'))
    for x in (-3.08, 3.08):
        geo.box(m, V((x, 0.06, (zb + zt) / 2)), (0.16, 0.12, zt - zb + 0.1), mi('concrete'))
    geo.box(m, V((0, 0.08, zt + 0.08)), (6.3, 0.16, 0.16), mi('concrete'))  # lintel lip
    x0, x1, y0, y1 = GRATE
    geo.box(m, V(((x0 + x1) / 2, (y0 + y1) / 2, -DEPTH - 0.32)), (x1 - x0, y1 - y0, 0.04),
            mi('void'))  # pit floor
    for x in (x0, x1):
        geo.box(m, V((x, (y0 + y1) / 2, -DEPTH - 0.16)), (0.06, y1 - y0, 0.32), mi('iron'))
    for y in (y0, y1):
        geo.box(m, V(((x0 + x1) / 2, y, -DEPTH - 0.16)), (x1 - x0, 0.06, 0.32), mi('iron'))
    n = 9
    for i in range(n):  # bars across the travel direction
        x = x0 + 0.2 + (x1 - x0 - 0.4) * i / (n - 1)
        geo.box(m, V((x, (y0 + y1) / 2, -DEPTH - 0.07)), (0.18, y1 - y0 - 0.1, 0.14), mi('iron'))
    for y in ((y0 + y1) / 2,):  # one cross bar
        geo.box(m, V(((x0 + x1) / 2, y, -DEPTH - 0.09)), (x1 - x0 - 0.1, 0.16, 0.1), mi('iron'))
    # catch-basin lid on the planting strip behind the kerb
    geo.box(m, V((0, -2.6, LAWN_H + 0.005)), (6.4, 4.0, 0.06), mi('concrete'))
    geo.cylinder(m, V((0, -2.6, LAWN_H)), V((0, -2.6, LAWN_H + 0.05)), 1.1, mi('iron'), n=20)
    parts = core.obj_from_bm('mouth', m, bevel=0.015, segments=1, angle=30)

    # painted fish marker on the kerb top above the mouth (no text)
    f = core.new_bm()
    body = [V((0.8 * math.cos(a), 0.26 * math.sin(a))) for a in
            [2 * math.pi * i / 16 for i in range(16)]]
    tail = [V((-0.7, 0.0)), V((-1.15, 0.3)), V((-1.15, -0.3))]
    for poly in (body, tail):
        pts = [V((p.x, -0.32 + p.y)) for p in poly]
        verts = [f.verts.new(V((p.x, p.y, KERB_H + 0.004))) for p in geo.ccw(pts)]
        face = f.faces.new(verts)
        face.material_index = mi('paint_white')
    fish = core.obj_from_bm('fish', f, weighted=False)
    propkit.finish(ENV, 'storm_drain', [surf, kerb, parts, fish], budget=HERO, below=DEPTH + 0.5,
                   car=(-7.5, 7.0, 90),
                   shots=[('hero', V((0.6, 1.0, 0.75)), 40), ('front', V((0.05, 1.0, 0.25)), 40),
                          ('top', V((0.0001, 0.3, 1.0)), 45), ('rc', None, 24)])


build()
