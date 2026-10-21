"""Tyre rack (decor): a blue steel wall rack, 12 u long, two levels of five used tyres standing
upright side by side on round bars. Origin at the wall face, floor level; front +Y.

    blender -b -P tools/blender/props/workshop/tyre_rack.py -- [--preview DIR]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

L, D = 12.0, 3.2
LEVELS = (0.9, 4.4)
start('lift_blue', 'galvanised', 'tyre', 'rubber', 'steel_dark')
rng = random.Random(3)
bm = core.new_bm()
for x in (-L / 2, 0.0, L / 2):  # uprights + feet
    for y in (0.2, D - 0.2):
        geo.box(bm, V((x, y, 4.3)), (0.3, 0.3, 8.6), mi('lift_blue'))
    geo.box(bm, V((x, D / 2, 0.06)), (0.5, D, 0.12), mi('steel_dark'))
for z in LEVELS:  # level frames
    for y in (0.2, D - 0.2):
        geo.box(bm, V((0, y, z)), (L, 0.24, 0.3), mi('lift_blue'))
frame = core.obj_from_bm('frame', bm, bevel=0.03, segments=1, angle=40)
bm = core.new_bm()
for z in LEVELS:
    for y in (0.95, D - 0.95):  # round bars the tyres sit on
        geo.cylinder(bm, V((-L / 2, y, z + 0.25)), V((L / 2, y, z + 0.25)), 0.12, mi('galvanised'),
                     n=7)
    for k in range(5):
        x = -L / 2 + 1.2 + k * (L - 2.4) / 4
        r = C.TYRE['r'] * rng.uniform(0.96, 1.04)
        tyre(bm, V((x + rng.uniform(-0.1, 0.1), D / 2, z + 0.25 + r - 0.15)), axis=(1, 0, 0), r=r,
             n=12, spin=rng.uniform(0, 360), seg=1)
tyres = core.obj_from_bm('tyres', bm, weighted=False, angle=60)
finish('tyre_rack', [frame, tyres], budget=KIT, car=(0.0, D + 3.0, 180.0))
