"""Sideboard / TV unit (hero): a mid-century walnut sideboard, 2.0 m x 45 cm x 55 cm (8.9 x 2.0
x 2.44 u) on short tapered legs: two slatted doors, two drawers with brass pulls, a stack of
books and a small vase at one end. The TV (house_tv) stands on it in the living room.
Front (+Y). Collider: one block down to the floor.

    blender -b -P tools/blender/props/house/sideboard.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

L, D, H, LEGS = 8.9, 2.0, 2.44, 0.42
core.reset()
materials('walnut', 'oak_end', 'brass', 'oak', 'ceramic', 'pages', 'book_blue', 'book_mustard',
          'book_cream')
wal = mi('walnut')

bm = core.new_bm()
geo.box(bm, V((0, 0, (LEGS + H) / 2)), (L, D, H - LEGS), wal)
case = core.obj_from_bm('case', box_uv(bm, 'x'), bevel=0.04, segments=2)

bm = core.new_bm()
fy = D / 2 + 0.02
for x in (-3.0, 3.0):  # slatted doors
    geo.box(bm, V((x, fy - 0.03, (LEGS + H) / 2)), (2.7, 0.06, H - LEGS - 0.3), mi('oak_end'))
    for k in range(9):
        geo.box(bm, V((x - 1.2 + k * 0.3, fy, (LEGS + H) / 2)), (0.2, 0.06, H - LEGS - 0.36), wal)
for z in (1.05, 1.85):  # drawers in the middle
    geo.box(bm, V((0, fy - 0.01, z)), (3.0, 0.05, 0.7), wal)
    geo.box(bm, V((0, fy + 0.04, z + 0.12)), (0.9, 0.08, 0.08), mi('brass'))
fronts = core.obj_from_bm('fronts', box_uv(bm, 'z'), bevel=0.012, segments=1)

bm = core.new_bm()
for sx in (-1, 1):
    for sy in (-1, 1):
        leg(bm, (sx * (L / 2 - 0.4), sy * (D / 2 - 0.3), 0), LEGS + 0.02, 0.07, 0.11, mi('oak'),
            n=10, splay=(sx * 0.05, sy * 0.05))
legs = core.obj_from_bm('legs', box_uv(bm, 'z'), weighted=False, angle=60)

bm = core.new_bm()
z = H
for i, (w, d, t, col) in enumerate([(1.3, 1.0, 0.16, 'book_blue'), (1.15, 0.9, 0.13, 'book_cream'),
                                    (1.05, 0.85, 0.12, 'book_mustard')]):
    c = V((3.3 + 0.03 * i, -0.1 + 0.03 * i, z + t / 2))
    geo.box(bm, c, (w, d, t), mi(col))
    geo.box(bm, c + V((0.02, 0, 0)), (w - 0.05, d + 0.006, t - 0.04), mi('pages'))
    z += t
geo.cylinder(bm, V((3.3, -0.1, z)), V((3.3, -0.1, z + 0.75)), 0.22, mi('ceramic'), n=16, r1=0.12)
deco = core.obj_from_bm('deco', bm, bevel=0.008, segments=1)

done('sideboard', [case, fronts, legs, deco], budget=HERO, car=(-1.5, 3.2, 200))
