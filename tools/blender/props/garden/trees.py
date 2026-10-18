"""Background trees beyond the fences (decor), one glb each: a broad oak (32 u), a silver birch
(26 u) and a column of leylandii (22 u). Trunks are tapered tubes with a few limbs, canopies
clusters of lumpy leaf masses in two greens.

    blender -b -P tools/blender/props/garden/trees.py -- [--preview DIR] [--only NAME]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
only = argv[argv.index('--only') + 1] if '--only' in argv else None


def limb(bm, a, b, r0, r1, mat):
    geo.tube(bm, [a, a.lerp(b, 0.5) + V((0, 0, 0.3)), b], r0, mat, n=7, r_end=r1)


def oak():
    start('bark', 'leaf', 'leaf_dark', 'leaf_light')
    rng = random.Random(31)
    tr = core.new_bm()
    geo.tube(tr, [V((0, 0, -0.3)), V((0.3, 0, 7)), V((0.1, 0.2, 13))], 1.0, mi('bark'), n=10,
             r_end=0.55)
    tips = []
    for k in range(6):
        a = 2 * math.pi * k / 6 + rng.uniform(-0.3, 0.3)
        s = V((0.1, 0.2, rng.uniform(9, 13)))
        e = s + V((math.cos(a) * rng.uniform(5, 8), math.sin(a) * rng.uniform(5, 8),
                   rng.uniform(4, 9)))
        limb(tr, s, e, 0.5, 0.18, mi('bark'))
        tips.append(e)
    trunk = smooth('trunk', tr, angle=50)
    cn = core.new_bm()
    for e in tips + [V((0, 0, 24)), V((3, 2, 26)), V((-3, -2, 25))]:
        foliage_ball(cn, e + V((0, 0, 1.5)), rng.uniform(6.0, 7.5),
                     mi(rng.choice(('leaf', 'leaf_dark', 'leaf'))), rng, n=6, detail=2, squash=0.8)
    return [trunk, smooth('canopy', cn, angle=60)]


def birch():
    start('paint_white', 'bark', 'leaf_light', 'leaf')
    rng = random.Random(32)
    tr = core.new_bm()
    geo.tube(tr, [V((0, 0, -0.3)), V((0.4, 0.2, 9)), V((0.0, 0.5, 20))], 0.55, mi('paint_white'),
             n=9, r_end=0.2)
    for z in (2.0, 4.5, 7.0, 10.0):  # dark bark bands
        geo.cylinder(tr, V((0.15, 0.05, z)), V((0.17, 0.08, z + 0.35)), 0.56, mi('bark'), n=9)
    tips = []
    for k in range(5):
        a = 2 * math.pi * k / 5 + rng.uniform(-0.4, 0.4)
        s = V((0.2, 0.3, rng.uniform(10, 17)))
        e = s + V((math.cos(a) * rng.uniform(3, 5), math.sin(a) * rng.uniform(3, 5), rng.uniform(3, 6)))
        limb(tr, s, e, 0.22, 0.08, mi('paint_white'))
        tips.append(e)
    trunk = smooth('trunk', tr, angle=50)
    cn = core.new_bm()
    for e in tips + [V((0, 0.5, 23))]:
        foliage_ball(cn, e, rng.uniform(3.5, 4.5), mi(rng.choice(('leaf_light', 'leaf'))), rng, n=7,
                     detail=2, squash=1.1)
    return [trunk, smooth('canopy', cn, angle=60)]


def conifer():
    start('bark', 'leaf_dark', 'leaf')
    rng = random.Random(33)
    cn = core.new_bm()
    geo.cylinder(cn, V((0, 0, -0.2)), V((0, 0, 3)), 0.6, mi('bark'), n=8)
    for k in range(12):
        z = 2.5 + k * 1.6
        r = 4.2 * (1 - k / 14) + 0.6
        foliage_ball(cn, V((rng.uniform(-0.3, 0.3), rng.uniform(-0.3, 0.3), z)), r,
                     mi('leaf_dark' if k % 3 else 'leaf'), rng, n=5, detail=2, squash=0.6)
    return [smooth('conifer', cn, angle=60)]


for name, fn in (('tree_oak', oak), ('tree_birch', birch), ('conifer', conifer)):
    if only and name != only:
        continue
    finish(name, fn(), hero=True, below=0.8, car=(6.0, -6.0, 30.0))
