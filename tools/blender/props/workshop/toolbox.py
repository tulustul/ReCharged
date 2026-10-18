"""Red cantilever toolbox, 0.5 m, with a black handle and steel catches (knockable).

    blender -b -P tools/blender/props/workshop/toolbox.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('chest_red', 'plastic_black', 'steel')
bm = core.new_bm()
geo.box(bm, V((0, 0, 0.36)), (2.1, 0.96, 0.72), mi('chest_red'))
for sy in (-1, 1):  # the two cantilever lids
    geo.box(bm, V((0, sy * 0.24, 0.84)), (2.06, 0.46, 0.22), mi('chest_red'))
body = core.obj_from_bm('body', bm, bevel=0.025, segments=2, angle=40)
bm = core.new_bm()
for sx in (-1, 1):
    geo.box(bm, V((sx * 0.75, 0, 1.08)), (0.1, 0.12, 0.26), mi('plastic_black'))
    geo.box(bm, V((sx * 0.7, -0.49, 0.62)), (0.16, 0.04, 0.18), mi('steel'))
geo.box(bm, V((0, 0, 1.2)), (1.6, 0.14, 0.1), mi('plastic_black'))
hw = core.obj_from_bm('hardware', bm, bevel=0.01, segments=1, angle=40)
finish('toolbox', [body, hw], budget=KIT, car=(2.4, -0.6, 30))
