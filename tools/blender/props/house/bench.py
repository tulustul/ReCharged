"""Dining bench: white-painted storage bench with an oak lid, a long striped seat cushion and two
scatter cushions; 2.25 m x 36 cm, seat 45 cm (10.0 x 1.6 x 2.0 u). Solid to the floor (it seals
the space under the dining table). Front (+Y).

    blender -b -P tools/blender/props/house/bench.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

L, D, H = 10.0, 1.6, 2.0
core.reset()
materials('paint_white', 'oak', 'fabric_blue', 'fabric_cream', 'fabric_mustard', 'oak_end')

bm = core.new_bm()
geo.box(bm, V((0, 0, (H - 0.32) / 2 + 0.02)), (L - 0.1, D - 0.1, H - 0.34), mi('paint_white'))
body = core.obj_from_bm('body', bm, bevel=0.03, segments=2)
bm = core.new_bm()
geo.box(bm, V((0, 0, H - 0.25)), (L, D, 0.1), mi('oak'))
lid = core.obj_from_bm('lid', box_uv(bm, 'x'), bevel=0.02, segments=2)
bm = core.new_bm()
for i in range(3):  # three framed front panels + plinth shadow line
    x = -L / 3 + i * L / 3
    geo.box(bm, V((x, D / 2 - 0.04, 0.95)), (L / 3 - 0.5, 0.04, 1.1), mi('paint_white'))
geo.box(bm, V((0, D / 2 - 0.05, 0.06)), (L - 0.2, 0.02, 0.1), mi('oak_end'))
panels = core.obj_from_bm('panels', bm, bevel=0.015, segments=1)
bm = core.new_bm()
rbox(bm, V((0, 0.0, H - 0.11)), (L - 0.2, D - 0.12, 0.2), 0.08, mi('fabric_blue'), seg=3,
     puff=0.05)
for x, col, yaw in ((-3.6, 'fabric_mustard', 12), (3.3, 'fabric_cream', -15)):
    rbox(bm, V((x, -0.25, H + 0.55)), (1.5, 0.4, 1.4), 0.24, mi(col), seg=3, puff=0.18,
         rot=(yaw, -14, 0), axis='y')
cushions = core.obj_from_bm('cushions', box_uv(bm, 'x'), weighted=False, angle=60)

done('bench', [body, lid, panels, cushions], car=(0.0, 2.8, 0))
