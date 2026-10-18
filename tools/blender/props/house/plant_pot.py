"""Potted fiddle-leaf fig: glazed terracotta pot (36 cm, 1.6 u) with soil and a 1.2 m tree of big
glossy leaves (to 5.6 u). Static; collider = the pot.

    blender -b -P tools/blender/props/house/plant_pot.py -- [--preview DIR]
"""
import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

rnd = random.Random(4)
core.reset()
materials('terracotta', 'soil', 'leaf', 'leaf_dark', 'oak_end')

bm = core.new_bm()
prof = [(0.0, 0.0), (0.62, 0.0), (0.66, 0.05), (0.8, 1.45), (0.84, 1.55), (0.78, 1.6),
        (0.72, 1.42)]
# revolve the pot profile around z (rings of a polyline)
n = 28
rings = [[V((r * math.cos(2 * math.pi * k / n), r * math.sin(2 * math.pi * k / n), z))
          for k in range(n)] for r, z in prof[1:]]
geo._bridge(bm, rings, mi('terracotta'), cap0=True, cap1=False)
geo.cylinder(bm, V((0, 0, 1.36)), V((0, 0, 1.42)), 0.72, mi('soil'), n=n)
pot = core.obj_from_bm('pot', bm, weighted=False, angle=40)

bm = core.new_bm()
trunk = [V((0, 0, 1.38)), V((0.08, 0.05, 2.6)), V((-0.05, 0.1, 3.8)), V((0.05, 0.0, 4.9))]
geo.tube(bm, trunk, 0.075, mi('oak_end'), n=7, corner=0.4, cseg=2, r_end=0.035)
leaves = 0
for i in range(46):
    t = i / 45
    z = 2.0 + 3.5 * t
    a = i * 2.4 + rnd.uniform(-0.3, 0.3)
    reach = 0.35 + 0.9 * math.sin(math.pi * min(1.0, t * 1.15)) + rnd.uniform(-0.1, 0.1)
    base = V((0.05 * math.cos(a), 0.05 * math.sin(a), z))
    tip = base + V((reach * math.cos(a), reach * math.sin(a), 0.25 + 0.3 * rnd.random()))
    # leaf: a cupped oval quad strip from base to tip
    ax = (tip - base).normalized()
    side = V((-math.sin(a), math.cos(a), 0)) * rnd.uniform(0.28, 0.4)
    pts = []
    for k in range(5):
        u = k / 4
        w = math.sin(math.pi * min(0.98, 0.15 + 0.85 * u)) * (1.0 if k else 0.2)
        c = base.lerp(tip, u) + V((0, 0, 0.12 * math.sin(math.pi * u)))
        pts.append((c - side * w, c + V((0, 0, 0.05 * w)), c + side * w))
    vs = [[bm.verts.new(p) for p in row] for row in pts]
    mat = mi('leaf' if i % 3 else 'leaf_dark')
    for r0, r1 in zip(vs, vs[1:]):
        for j in range(2):
            f = bm.faces.new((r0[j], r0[j + 1], r1[j + 1], r1[j]))
            f.material_index = mat
    leaves += 1
plant = core.obj_from_bm('plant', bm, weighted=False, angle=70)

done('plant_pot', [pot, plant], car=(2.2, 1.0, 200))
