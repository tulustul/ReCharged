"""Mechanic's creeper: padded black vinyl board with a headrest on a steel frame and six swivel
casters, 1 m long along Y (knockable: it rolls).

    blender -b -P tools/blender/props/workshop/creeper.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('vinyl_black', 'steel_dark', 'plastic_red', 'plastic_black')
bm = core.new_bm()
geo.slab(bm, [(-2.2, 0.95, 0.24, 0.4, 0.07), (2.2, 0.95, 0.24, 0.4, 0.07)], mi('vinyl_black'))
geo.slab(bm, [(1.4, 0.55, 0.4, 0.52, 0.05), (2.05, 0.5, 0.4, 0.5, 0.05)], mi('vinyl_black'))
for sx in (-1, 1):
    geo.box(bm, V((sx * 0.9, 0, 0.21)), (0.12, 4.3, 0.1), mi('plastic_red'))
pad = core.obj_from_bm('board', bm, bevel=0.02, segments=1, angle=40)
bm = core.new_bm()
for sx in (-1, 1):
    for y in (-1.8, 0.0, 1.8):
        geo.box(bm, V((sx * 0.85, y, 0.17)), (0.14, 0.14, 0.06), mi('steel_dark'))
        geo.cylinder(bm, V((sx * 0.85 - 0.05, y + 0.05, 0.08)), V((sx * 0.85 + 0.05, y + 0.05, 0.08)),
                     0.08, mi('plastic_black'), n=10)
casters = core.obj_from_bm('casters', bm, weighted=False, angle=40)
finish('creeper', [pad, casters], budget=KIT, car=(2.3, -1.4, 30))
