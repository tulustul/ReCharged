"""Car battery: black case with a grey lid, red and black terminal caps and a carry strap
(knockable).

    blender -b -P tools/blender/props/workshop/battery.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('plastic_black', 'plastic_grey', 'plastic_red', 'steel_dark')
bm = core.new_bm()
geo.box(bm, V((0, 0, 0.37)), (1.24, 0.78, 0.74), mi('plastic_black'))
geo.box(bm, V((0, 0, 0.77)), (1.22, 0.76, 0.06), mi('plastic_grey'))
body = core.obj_from_bm('body', bm, bevel=0.02, segments=1, angle=40)
bm = core.new_bm()
for x, mat in ((0.42, 'plastic_red'), (-0.42, 'plastic_black')):
    geo.cylinder(bm, V((x, 0.18, 0.8)), V((x, 0.18, 0.92)), 0.1, mi(mat), n=10, r1=0.08)
geo.tube(bm, [V((-0.6, 0.0, 0.6)), V((-0.6, 0.0, 1.05)), V((0.6, 0.0, 1.05)), V((0.6, 0.0, 0.6))],
         0.03, mi('steel_dark'), n=6, corner=0.12, cseg=2)
caps = core.obj_from_bm('caps', bm, weighted=False, angle=40)
finish('battery', [body, caps], budget=KIT, car=(1.8, -0.6, 30))
