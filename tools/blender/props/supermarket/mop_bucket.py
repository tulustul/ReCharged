"""Mop bucket (kit, art bible §3.5): a yellow bucket on casters with a grey wringer and a mop
leaning in it (2.2 x 2.0 u, handle 5.5 u). Knockable (heavy-ish); parked where the freezer
leaks.

    blender -b -P tools/blender/props/supermarket/mop_bucket.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

start('yellow_plastic', 'steel_dark', 'brushed', 'rubber', 'white_plastic', 'blue_plastic')
body = core.new_bm()
geo.prism(body, geo.rrect(2.0, 1.6, 0.35, 4), V((0, 0, 0.3)), V((0, 0, 1.9)),
          mi('yellow_plastic'), up=V((0, 1, 0)), scale1=1.08)
geo.box(body, V((0.45, 0, 2.2)), (0.9, 1.2, 0.6), mi('steel_dark'))          # wringer press
geo.box(body, V((0.45, 0, 2.55)), (0.2, 1.3, 0.1), mi('brushed'))
geo.tube(body, [V((0.9, 0, 2.3)), V((1.2, 0, 3.6))], 0.06, mi('brushed'), n=6)  # press lever
for sx in (-1, 1):
    for sy in (-1, 1):
        geo.cylinder(body, V((sx * 0.75 - 0.06, sy * 0.6, 0.15)), V((sx * 0.75 + 0.06, sy * 0.6, 0.15)),
                     0.15, mi('rubber'), n=8)
mop = core.new_bm()
geo.cylinder(mop, V((-0.4, 0.1, 0.6)), V((-1.1, 0.4, 5.5)), 0.07, mi('blue_plastic'), n=6)
for k in range(10):   # mop strands
    a = k * 0.6
    geo.cylinder(mop, V((-0.4, 0.1, 0.6)), V((-0.4 + 0.35 * math.cos(a), 0.1 + 0.35 * math.sin(a),
                                              1.6)), 0.08, mi('white_plastic'), n=4)
propkit.finish(ENV, 'mop_bucket', [part('bucket', body, bevel=0.02, segments=1),
                                   smooth('mop', mop)], car=(2.5, 1.0, 200))
