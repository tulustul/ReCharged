"""Folding workshop crane / engine hoist (island equipment): red A-frame legs on casters, a mast
with a hydraulic ram, the boom reaching forward (+Y, Bevy -z) with a chain and hook.

    blender -b -P tools/blender/props/workshop/engine_crane.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('chest_red', 'steel_dark', 'steel', 'rubber', 'chrome', 'safety_yellow')
red = mi('chest_red')
bm = core.new_bm()
for sx in (-1, 1):  # legs splaying forward
    geo.prism(bm, [(-0.18, 0.0), (0.18, 0.0), (0.18, 0.32), (-0.18, 0.32)], V((sx * 0.6, -1.5, 0.3)),
              V((sx * 1.5, 2.5, 0.3)), red, up=V((0, 0, 1)))
geo.box(bm, V((0, -1.5, 0.45)), (1.6, 0.4, 0.4), red)                                    # rear bar
geo.box(bm, V((0, -1.5, 3.9)), (0.55, 0.55, 7.2), red)                                   # mast
a, b = V((0, -1.5, 7.3)), V((0, 3.4, 8.1))                                               # boom
geo.prism(bm, [(-0.25, -0.25), (0.25, -0.25), (0.25, 0.25), (-0.25, 0.25)], a, b, red,
          up=V((0, 0, 1)))
frame = core.obj_from_bm('frame', bm, bevel=0.03, segments=1, angle=40)
bm = core.new_bm()
geo.cylinder(bm, V((0, -1.2, 2.2)), V((0, 1.0, 7.0)), 0.16, mi('steel_dark'), n=8)       # ram
geo.cylinder(bm, V((0, -1.1, 2.3)), V((0, -0.5, 3.6)), 0.26, mi('steel'), n=8)
for k in range(9):  # chain links down from the boom tip
    z = 7.8 - k * 0.32
    geo.cylinder(bm, V((0, 3.35, z)), V((0, 3.35, z - 0.26)), 0.07, mi('steel'), n=6)
geo.tube(bm, [V((0, 3.35, 4.9)), V((0, 3.35, 4.5)), V((0, 3.6, 4.4)), V((0, 3.75, 4.6))], 0.08,
         mi('safety_yellow'), n=6, corner=0.15, cseg=2)                                  # hook
for p in (V((-1.5, 2.5, 0.15)), V((1.5, 2.5, 0.15)), V((-0.6, -1.5, 0.15)), V((0.6, -1.5, 0.15))):
    geo.cylinder(bm, p - V((0.1, 0, 0)), p + V((0.1, 0, 0)), 0.15, mi('rubber'), n=8)
rnd = core.obj_from_bm('round', bm, weighted=False, angle=40)
finish('engine_crane', [frame, rnd], budget=KIT, car=(3.0, 3.0, 200))
