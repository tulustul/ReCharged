"""Flat street kit: road paint (yellow centre dash, white crosswalk bar, stop line), cast-iron
manhole cover, a scatter of fallen maple leaves and a raked leaf pile (soft bump).

    blender -b -P tools/blender/props/street/kit_flat.py -- [--preview DIR] [--only NAME]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

import bmesh  # noqa: E402


def flat_cams(span, h=0.7):
    return [('hero', (span * 0.9, -span * 1.1, span * 0.7), (0, 0, 0), 35),
            ('rc', (span * 0.25, -span * 1.6, h), (0, 0, 0), 24)]


def paint(name, size, mat):
    start(mat)
    bm = core.new_bm()
    sx, sy = size
    # a slightly worn edge: rounded rectangle slab 0.02 thick
    geo.prism(bm, geo.rrect(sy, 0.02, 0.0, 1, 0, 0.01), V((-sx / 2, 0, 0)), V((sx / 2, 0, 0)),
              mi(mat))
    ob = core.obj_from_bm(name, bm, weighted=False)
    street_props.finish(ENV, name, [ob], car=(0, -sy / 2 - 2.5, 90),
                        cams=flat_cams(max(sx, sy)))


def manhole():
    start('iron', 'void')
    bm = core.new_bm()
    geo.cylinder(bm, V((0, 0, -0.02)), V((0, 0, 0.02)), 1.4, mi('iron'), n=28)
    for r in (1.25, 0.95, 0.62):  # raised concentric rings
        pts = [(r * math.cos(a), r * math.sin(a), 0.03) for a in
               [2 * math.pi * i / 28 for i in range(29)]]
        geo.tube(bm, pts, 0.05, mi('iron'), n=4)
    for i in range(6):  # spokes
        a = math.pi * i / 6
        rot = Matrix.Rotation(a, 4, 'Z')
        vs = [rot @ V(p) for p in ((-1.2, -0.04, 0.02), (1.2, -0.04, 0.02), (1.2, 0.04, 0.02),
                                   (-1.2, 0.04, 0.02))]
        geo.prism(bm, [(-0.04, 0.0), (0.04, 0.0), (0.04, 0.05), (-0.04, 0.05)],
                  (vs[0] + vs[3]) / 2, (vs[1] + vs[2]) / 2, mi('iron'))
    for dx in (-0.35, 0.35):  # pick holes
        geo.cylinder(bm, V((dx, 0.85, 0.021)), V((dx, 0.85, 0.024)), 0.08, mi('void'), n=8)
    ob = core.obj_from_bm('manhole', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'manhole', [ob], below=0.05, car=(0, -3.0, 90), cams=flat_cams(3.0))


# right half of a maple leaf (tip at +y, stem at -y), unit size; mirrored for the left half
_LEAF = [(0.0, 1.0), (0.13, 0.64), (0.33, 0.74), (0.25, 0.44), (0.6, 0.56), (0.95, 0.36),
         (0.62, 0.2), (0.47, 0.06), (0.6, -0.14), (0.3, -0.12), (0.07, -0.24), (0.03, -0.24),
         (0.03, -0.58)]


def leaf_shape(size):
    """Maple-leaf outline (2D, centred, CCW): five lobes and a stem."""
    right = [V((x * size, y * size)) for x, y in _LEAF]
    left = [V((-p.x, p.y)) for p in reversed(right[1:])]
    return geo.ccw(right + left[:-1] if left[-1].x == 0 else right + left)


def leaf(bm, c, size, yaw, tilt, mat):
    """Flat leaf polygon at c, rotated by yaw, tilted a little."""
    rot = Matrix.Rotation(yaw, 3, 'Z') @ Matrix.Rotation(tilt, 3, 'X')
    vs = [bm.verts.new(V(c) + rot @ V((p.x, p.y, 0.0))) for p in leaf_shape(size)]
    f = bm.faces.new(vs)
    f.material_index = mat
    if f.normal.z < 0:
        f.normal_flip()
    return f


def leaves():
    start('leaf_red', 'leaf_orange', 'leaf_yellow', 'leaf_brown')
    rnd = random.Random(5)
    bm = core.new_bm()
    cols = [mi('leaf_red'), mi('leaf_orange'), mi('leaf_yellow'), mi('leaf_brown')]
    for i in range(46):
        r = 2.5 * math.sqrt(rnd.random())
        a = rnd.uniform(0, 2 * math.pi)
        leaf(bm, (r * math.cos(a), r * math.sin(a), 0.012 + 0.004 * (i % 3)),
             rnd.uniform(0.18, 0.3), rnd.uniform(0, 6.3), rnd.uniform(-0.15, 0.15),
             rnd.choices(cols, [0.35, 0.3, 0.2, 0.15])[0])
    ob = core.obj_from_bm('leaves', bm, weighted=False)
    street_props.finish(ENV, 'leaves', [ob], below=0.05, car=(0, -3.6, 90), cams=flat_cams(5.0))


def leaf_pile():
    start('leaf_red', 'leaf_orange', 'leaf_yellow', 'leaf_brown')
    rnd = random.Random(9)
    bm = core.new_bm()
    cols = [mi('leaf_red'), mi('leaf_orange'), mi('leaf_yellow'), mi('leaf_brown')]
    # lumpy dome matching the collider's spherical cap (r 6 at -5.5: 0.5 u high, 2.4 u radius):
    # 0.42 u + up to 0.08 u of jitter, so cars ride on the leaves instead of sinking into them
    out = bmesh.ops.create_uvsphere(bm, u_segments=20, v_segments=10, radius=1.0)
    for v in out['verts']:
        x, y, z = v.co
        z = max(z, 0.0)
        jitter = rnd.uniform(-0.15, 0.15)
        v.co = V((x * 2.6 + jitter, y * 2.6 + jitter, z * 0.42 + rnd.uniform(-0.03, 0.08) * z))
    for f in {f for v in out['verts'] for f in v.link_faces}:
        f.material_index = cols[3] if rnd.random() < 0.7 else cols[1]
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.z < -0.01], context='VERTS')
    for i in range(190):  # loose leaves on and around the mound
        r = 3.2 * math.sqrt(rnd.random()) if i < 150 else rnd.uniform(2.6, 3.6)
        a = rnd.uniform(0, 2 * math.pi)
        x, y = r * math.cos(a), r * math.sin(a)
        d = math.hypot(x / 2.6, y / 2.6)
        z = (0.42 * math.sqrt(max(0.0, 1 - d * d)) + 0.06 if d < 1 else 0.0) + 0.03
        leaf(bm, (x, y, z), rnd.uniform(0.22, 0.34), rnd.uniform(0, 6.3), rnd.uniform(-0.5, 0.5),
             rnd.choices(cols, [0.35, 0.3, 0.25, 0.1])[0])
    ob = core.obj_from_bm('leaf_pile', bm, weighted=False, angle=60)
    street_props.finish(ENV, 'leaf_pile', [ob], below=0.15, car=(0, -4.5, 90),
                        cams=flat_cams(6.0))


JOBS = {
    'paint_dash': lambda: paint('paint_dash', (10.0, 0.45), 'paint_yellow'),
    'paint_bar': lambda: paint('paint_bar', (14.0, 2.2), 'paint_white'),
    'paint_stop': lambda: paint('paint_stop', (22.0, 0.9), 'paint_white'),
    'manhole': manhole, 'leaves': leaves, 'leaf_pile': leaf_pile,
}
only = propkit._argv()[propkit._argv().index('--only') + 1] if '--only' in propkit._argv() \
    else None
for name, job in JOBS.items():
    if only in (None, name):
        job()
