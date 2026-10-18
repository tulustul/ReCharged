"""Armchair: a cosy mustard lounge chair on short tapered oak legs, 85 x 80 cm, seat 43 cm,
80 cm tall (3.8 x 3.6 x 3.6 u), puffy seat and back cushions, rounded arms. Front (+Y).
Collider: a block to the floor + back + arms.

    blender -b -P tools/blender/props/house/armchair.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

W, D, LEGS = 3.8, 3.6, 0.5
core.reset()
materials('fabric_mustard', 'oak', 'fabric_dark', 'fabric_cream')
fab = mi('fabric_mustard')

bm = core.new_bm()
rbox(bm, V((0, 0, LEGS + 0.42)), (W - 0.04, D - 0.04, 0.84), 0.14, fab, seg=3)  # seat base
for f in bm.faces:
    if f.normal.z < -0.9:
        f.material_index = mi('fabric_dark')
for s in (-1, 1):  # arms
    rbox(bm, V((s * (W / 2 - 0.36), 0.05, LEGS + 1.05)), (0.72, D - 0.1, 2.1 - 0.0), 0.3, fab,
         seg=4, axis='y')
rbox(bm, V((0, -D / 2 + 0.38, LEGS + 1.55)), (W - 1.3, 0.7, 3.1), 0.28, fab, seg=4)  # back
body = core.obj_from_bm('body', box_uv(bm, 'x'), weighted=False, angle=50)

bm = core.new_bm()
rbox(bm, V((0, 0.35, LEGS + 1.12)), (W - 1.5, D - 1.0, 0.56), 0.2, fab, seg=4, puff=0.08)
rbox(bm, V((0, -D / 2 + 0.95, LEGS + 2.25)), (W - 1.55, 0.6, 1.55), 0.25, fab, seg=4, puff=0.12,
     rot=(0, -12, 0))
rbox(bm, V((0.55, -0.45, LEGS + 1.95)), (1.25, 0.36, 1.15), 0.22, mi('fabric_cream'), seg=3,
     puff=0.2, rot=(-18, -18, 6), axis='y')  # small cushion
cushions = core.obj_from_bm('cushions', box_uv(bm, 'x'), weighted=False, angle=60)

bm = core.new_bm()
for sx in (-1, 1):
    for sy in (-1, 1):
        leg(bm, (sx * 1.6, sy * 1.5, 0), LEGS + 0.02, 0.07, 0.11, mi('oak'), n=12,
            splay=(sx * 0.08, sy * 0.08))
legs = core.obj_from_bm('legs', box_uv(bm, 'z'), weighted=False, angle=60)

done('armchair', [body, cushions, legs], car=(3.4, 0.5, 200))
