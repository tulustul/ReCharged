"""Autumn maple trees (kit, background-scale): tapered trunk with root flare, main branches
reaching into a lumpy canopy of displaced icosphere clusters in mixed fall colours.
Only the trunk collides (catalog: street_maple_*).

  maple_a  red maple, ~50 u (11 m) tall, red/orange
  maple_b  sugar maple, ~44 u, orange/yellow

    blender -b -P tools/blender/props/street/maple.py -- [--preview DIR] [--only NAME]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

import bmesh  # noqa: E402


def cluster(bm, c, r, mat, rnd, squash=0.8, level=1):
    """Lumpy leaf cluster: a subdivided icosphere with jittered vertices."""
    out = bmesh.ops.create_icosphere(bm, subdivisions=level, radius=r,
                                     matrix=Matrix.Translation(V(c)))
    for v in out['verts']:
        d = v.co - V(c)
        k = 1.0 + rnd.uniform(-0.18, 0.22)
        v.co = V(c) + V((d.x * k, d.y * k, d.z * k * squash))
    for f in {f for v in out['verts'] for f in v.link_faces}:
        f.material_index = mat


def tree(name, height, crown, colors, seed, lean=(0.6, -0.3)):
    core.reset()
    street_props.use_outdoor(ground='#5C8F3C')
    mats('bark', *colors)
    rnd = random.Random(seed)
    bm = core.new_bm()
    bark = mi('bark')
    # trunk: tapered tube with a slight lean, root flare at the base
    split = height * 0.34
    top = V((lean[0], lean[1], split))
    geo.tube(bm, [(0, 0, -0.2), (0, 0, split * 0.4), (lean[0] * 0.5, lean[1] * 0.5, split * 0.75),
                  tuple(top)], 1.35, bark, n=12, r_end=1.0)
    for i in range(5):  # root flare
        a = 2 * math.pi * i / 5 + 0.3
        geo.tube(bm, [(0.5 * math.cos(a), 0.5 * math.sin(a), 1.6),
                      (1.5 * math.cos(a), 1.5 * math.sin(a), 0.15)], 0.55, bark, n=6, r_end=0.25)
    # main branches toward canopy anchor points
    zc = height - crown * 0.55
    anchors = []
    for i in range(5):
        a = 2 * math.pi * i / 5 + rnd.uniform(-0.3, 0.3)
        rr = crown * rnd.uniform(0.22, 0.34)
        anchors.append(V((rr * math.cos(a), rr * math.sin(a), zc + rnd.uniform(-2, 4))))
    for p in anchors:
        mid = top.lerp(p, 0.5) + V((0, 0, 3.0))
        geo.tube(bm, [tuple(top), tuple(mid), tuple(p)], 0.8, bark, n=7, corner=2.0, r_end=0.3)
        for k in range(2):  # secondary branches
            q = p + V((rnd.uniform(-4, 4), rnd.uniform(-4, 4), rnd.uniform(1, 4)))
            geo.tube(bm, [tuple(mid), tuple(q)], 0.4, bark, n=5, r_end=0.15)
    wood = core.obj_from_bm('wood', bm, weighted=False, angle=50)
    # canopy: clusters in a squashed ellipsoid, bigger toward the middle
    leaves = core.new_bm()
    mats_ = [mi(c) for c in colors]
    weights = [0.5, 0.32, 0.18][:len(mats_)]
    for i in range(23):  # 7 big smooth core masses, 16 smaller ones on the crown surface
        big = i < 7
        while True:
            p = V((rnd.uniform(-1, 1), rnd.uniform(-1, 1), rnd.uniform(-0.7, 1)))
            if (0.15 if big else 0.6) <= p.length <= (0.6 if big else 1.0):
                break
        c = V((p.x * crown / 2, p.y * crown / 2, zc + p.z * crown * 0.34))
        r = crown * (rnd.uniform(0.2, 0.25) if big else rnd.uniform(0.11, 0.16))
        mat = rnd.choices(mats_, weights)[0]
        cluster(leaves, c, r, mat, rnd, squash=0.78, level=3 if big else 2)
    canopy = core.obj_from_bm('canopy', leaves, weighted=False, angle=70)
    return street_props.finish(ENV, name, [wood, canopy], budget=KIT, below=0.25,
                               car=(5.0, -6.0, 30),
                               cams=[('hero', (70, -75, 30), (0, 0, height * 0.5), 35),
                                     ('rc', (6, -14, 0.7), (0, 0, 14), 20),
                                     ('under', (2.5, -3.5, 0.7), (0, 0, height), 16)])


TREES = {
    'maple_a': (50.0, 40.0, ('leaf_red', 'leaf_orange', 'leaf_yellow'), 11),
    'maple_b': (44.0, 36.0, ('leaf_orange', 'leaf_yellow', 'leaf_red'), 23),
}
only = propkit._argv()[propkit._argv().index('--only') + 1] if '--only' in propkit._argv() \
    else None
for name, (h, cr, cols, seed) in TREES.items():
    if only in (None, name):
        tree(name, h, cr, cols, seed)
