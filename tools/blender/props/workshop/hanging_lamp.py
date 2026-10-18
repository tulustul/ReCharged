"""Hanging twin-tube work light (decor): a 6.7 u (1.5 m) white batten with a reflector and two
warm tubes (always on: work lights), hung on two cables 8.6 u long. Origin at the bottom of the
tubes; place it so the cables meet the ceiling.

    blender -b -P tools/blender/props/workshop/hanging_lamp.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

L = 6.7
start('paint_white', 'light_tube', 'steel_dark')
bm = core.new_bm()
geo.slab(bm, [(-0.4, L / 2, 0.25, 0.55, 0.08), (0.4, L / 2, 0.25, 0.55, 0.08)], mi('paint_white'),
         seg=2)
geo.box(bm, V((0, 0, 0.32)), (L - 0.2, 0.62, 0.06), mi('paint_white'))
body = core.obj_from_bm('body', bm, bevel=0.02, segments=1, angle=40)
bm = core.new_bm()
for y in (-0.16, 0.16):
    geo.cylinder(bm, V((-L / 2 + 0.25, y, 0.13)), V((L / 2 - 0.25, y, 0.13)), 0.11, mi('light_tube'),
                 n=10)
for x in (-L / 2 + 0.6, L / 2 - 0.6):
    geo.cylinder(bm, V((x, 0, 0.55)), V((x, 0, 9.15)), 0.025, mi('steel_dark'), n=4)
rnd = core.obj_from_bm('round', bm, weighted=False, angle=40)
finish('hanging_lamp', [body, rnd], budget=KIT, car=(2.0, -1.5, 30))
