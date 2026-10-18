"""Red 20 l steel jerry can with the embossed X, three-bar handle and spout cap (knockable).

    blender -b -P tools/blender/props/workshop/jerry_can.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('drum_red', 'steel_dark', 'steel')
L, T, H = 1.5, 0.74, 2.09
bm = core.new_bm()
geo.slab(bm, [(-T / 2, L / 2, 0.0, H - 0.25, 0.08), (T / 2, L / 2, 0.0, H - 0.25, 0.08)],
         mi('drum_red'))
geo.slab(bm, [(-T / 2, L / 2 - 0.25, H - 0.3, H - 0.1, 0.06), (T / 2, L / 2 - 0.25, H - 0.3,
                                                                 H - 0.1, 0.06)], mi('drum_red'))
for sy in (-1, 1):  # embossed X on both faces
    for a, b in (((-0.55, 0.15), (0.55, 1.6)), ((-0.55, 1.6), (0.55, 0.15))):
        p0 = V((a[0], sy * (T / 2 + 0.02), a[1]))
        p1 = V((b[0], sy * (T / 2 + 0.02), b[1]))
        geo.prism(bm, geo.rrect(0.14, 0.05, 0.0, 1)[::2], p0, p1, mi('drum_red'),
                  up=V((0, sy, 0)))
for x in (-0.35, 0.0, 0.35):  # handle bars
    geo.box(bm, V((x, 0, H - 0.05)), (0.09, 0.42, 0.12), mi('drum_red'))
geo.box(bm, V((0, 0, H - 0.0)), (0.9, 0.14, 0.08), mi('drum_red'))
can = core.obj_from_bm('can', bm, bevel=0.015, segments=1, angle=40)
bm = core.new_bm()
geo.cylinder(bm, V((0.55, 0, H - 0.12)), V((0.62, 0, H + 0.12)), 0.13, mi('steel_dark'), n=10)
cap = core.obj_from_bm('cap', bm, weighted=False, angle=40)
finish('jerry_can', [can, cap], budget=KIT, car=(2.0, -0.8, 30))
