"""Air compressor (decor): a red horizontal 200 l tank on feet with the motor, finned pump head
and belt guard on top, pressure switch, gauge, drain cock and a coiled air hose on a hook.

    blender -b -P tools/blender/props/workshop/compressor.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('chest_red', 'steel_dark', 'steel', 'plastic_black', 'paint_grey', 'label_white',
      'plastic_yellow')
R, L = 1.2, 7.0
bm = core.new_bm()
fs, vs = revolve(bm, [(0.0, -L / 2), (R * 0.75, -L / 2), (R, -L / 2 + 0.3), (R, L / 2 - 0.3),
                      (R * 0.75, L / 2), (0.0, L / 2)], mi('chest_red'), n=24)
place(bm, vs, center=(0, 0, 1.6), axis=(1, 0, 0))
for x in (-L / 2 + 1.0, L / 2 - 1.0):
    for y in (-0.7, 0.7):
        geo.cylinder(bm, V((x, y, 0.0)), V((x, y, 0.75)), 0.12, mi('steel_dark'), n=6)
geo.cylinder(bm, V((-1.2, 0, 2.8)), V((-1.2, 0, 4.4)), 0.65, mi('paint_grey'), n=12)   # motor
for k in range(5):  # finned pump head
    z = 2.85 + k * 0.22
    geo.cylinder(bm, V((1.4, 0, z)), V((1.4, 0, z + 0.12)), 0.55 - k * 0.04, mi('steel_dark'), n=12)
geo.cylinder(bm, V((0.2, 0.0, 4.0)), V((0.2, 0.0, 4.4)), 0.25, mi('steel'), n=10)          # gauge
geo.cylinder(bm, V((0.2, 0.26, 4.2)), V((0.2, 0.3, 4.2)), 0.18, mi('label_white'), n=12)
for k in range(4):  # coiled hose hanging at the tank end
    r = 0.5 + 0.03 * k
    pts = [V((L / 2 + 0.3, r * math.cos(a), 1.7 + r * math.sin(a)))
           for a in [2 * math.pi * i / 14 for i in range(15)]]
    geo.tube(bm, pts, 0.05, mi('plastic_yellow'), n=6)
rnd = core.obj_from_bm('round', bm, weighted=False, angle=40)
bm = core.new_bm()
geo.box(bm, V((0.1, 0.62, 3.45)), (2.9, 0.2, 1.3), mi('chest_red'))                      # belt guard
geo.box(bm, V((-0.2, -0.5, 3.6)), (0.5, 0.4, 0.6), mi('plastic_black'))                  # switch
geo.box(bm, V((0, 0, 2.75)), (3.8, 1.6, 0.12), mi('steel_dark'))                         # deck plate
hard = core.obj_from_bm('hard', bm, bevel=0.02, segments=1, angle=40)
finish('compressor', [rnd, hard], budget=KIT, car=(0.0, -3.5, 0))
