"""Red trolley jack, 0.7 m: low body on two steel front wheels and rear swivel casters, lifting
arm with a saddle at the front (+Y), long handle raised toward the back.

    blender -b -P tools/blender/props/workshop/jack.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('chest_red', 'steel', 'steel_dark', 'plastic_black', 'rubber')
bm = core.new_bm()
for sx in (-1, 1):  # side plates
    geo.slab(bm, [(-1.5, 0.08, 0.18, 0.7, 0.04), (0.9, 0.08, 0.18, 0.62, 0.04),
                  (1.5, 0.08, 0.2, 0.42, 0.04)], mi('chest_red'), cx=sx * 0.62)
geo.box(bm, V((0, -0.9, 0.4)), (1.24, 0.5, 0.35), mi('chest_red'))           # pump block
geo.box(bm, V((0, 0.6, 0.62)), (0.7, 1.4, 0.14), mi('chest_red'))            # lift arm
body = core.obj_from_bm('body', bm, bevel=0.02, segments=1, angle=40)
bm = core.new_bm()
geo.cylinder(bm, V((0, 1.25, 0.66)), V((0, 1.25, 0.84)), 0.32, mi('steel_dark'), n=12)  # saddle
geo.cylinder(bm, V((0, -0.6, 0.45)), V((0, -0.2, 0.9)), 0.12, mi('steel'), n=8)  # ram
for sx in (-1, 1):
    geo.cylinder(bm, V((sx * 0.62, 1.15, 0.2)), V((sx * 0.78, 1.15, 0.2)), 0.2, mi('steel'), n=12)
    geo.cylinder(bm, V((sx * 0.5, -1.35, 0.13)), V((sx * 0.6, -1.35, 0.13)), 0.13, mi('rubber'),
                 n=10)
geo.tube(bm, [V((0, -1.0, 0.45)), V((0, -2.2, 1.6)), V((0, -3.3, 2.6))], 0.07, mi('steel_dark'),
         n=6)  # handle
geo.cylinder(bm, V((0, -3.2, 2.5)), V((0, -3.6, 2.85)), 0.11, mi('plastic_black'), n=8)
rnd = core.obj_from_bm('round', bm, weighted=False, angle=40)
finish('jack', [body, rnd], budget=KIT, car=(2.5, 1.0, 30))
