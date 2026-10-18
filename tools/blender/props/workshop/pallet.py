"""Wooden pallet (1.2 x 0.8 x 0.144 m): five top boards on three stringers with nine blocks
and three bottom boards; long axis X.

    blender -b -P tools/blender/props/workshop/pallet.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('timber')
X, Y = 5.33, 3.55
bm = core.new_bm()
tm = mi('timber')
for k in range(5):  # top deck boards
    w = 0.65 if k in (0, 4) else 0.44
    y = -Y / 2 + w / 2 if k == 0 else (Y / 2 - w / 2 if k == 4 else -Y / 2 + 0.65 + 0.42 + (k - 1) * 0.6)
    f = geo.box(bm, V((0, y, 0.6)), (X, w, 0.09), tm)
    box_uv(bm, f, 0.5)
for y in (-Y / 2 + 0.44, 0.0, Y / 2 - 0.44):  # stringer boards
    f = geo.box(bm, V((0, y, 0.51)), (X, 0.44, 0.09), tm)
    box_uv(bm, f, 0.5)
    for x in (-X / 2 + 0.4, 0.0, X / 2 - 0.4):  # blocks
        f = geo.box(bm, V((x, y, 0.27)), (0.6, 0.44, 0.38), tm)
        box_uv(bm, f, 0.5)
for x in (-X / 2 + 0.4, 0.0, X / 2 - 0.4):  # bottom boards
    f = geo.box(bm, V((x, 0, 0.04)), (0.62, Y, 0.08), tm)
    box_uv(bm, f, 0.5)
pal = core.obj_from_bm('pallet', bm, bevel=0.015, segments=1, angle=40)
finish('pallet', [pal], budget=KIT, car=(3.8, -1.4, 30))
