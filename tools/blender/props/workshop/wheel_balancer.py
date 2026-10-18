"""Wheel balancer (island equipment): grey body with a display panel and weight tray, a wheel on
the shaft at +X under a raised half hood. Footprint ~6 x 2.6 u.

    blender -b -P tools/blender/props/workshop/wheel_balancer.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('paint_grey', 'steel_dark', 'steel', 'tyre', 'rubber', 'steel_black', 'screen',
      'plastic_black', 'chest_red')
bm = core.new_bm()
geo.box(bm, V((-1.1, 0, 2.0)), (2.9, 2.4, 4.0), mi('paint_grey'))
geo.box(bm, V((-1.1, 0.4, 4.25)), (2.2, 0.6, 0.5), mi('paint_grey'))       # display housing
geo.box(bm, V((-1.1, 0.0, 4.02)), (2.6, 2.0, 0.06), mi('steel_dark'))      # weight tray
box_ = core.obj_from_bm('box', bm, bevel=0.03, segments=1, angle=40)
bm = core.new_bm()
geo.box(bm, V((-1.1, 0.71, 4.28)), (1.8, 0.02, 0.34), mi('screen'))
geo.cylinder(bm, V((0.35, 0, 2.1)), V((1.9, 0, 2.1)), 0.12, mi('steel'), n=8)       # shaft
tyre(bm, V((1.3, 0, 2.1)), axis=(1, 0, 0), n=24, seg=2)
steel_rim(bm, V((1.3, 0, 2.1)), axis=(1, 0, 0), rim_r=RIM_R, w=C.TYRE['w'] * 0.82, n=20)
pts = [V((1.3, 1.9 * math.cos(a), 2.1 + 1.9 * math.sin(a))) for a in
       [math.radians(t) for t in range(20, 181, 20)]]  # raised hood: half ring
geo.tube(bm, pts, 0.35, mi('chest_red'), n=6)
rnd = core.obj_from_bm('round', bm, weighted=False, angle=50)
finish('wheel_balancer', [box_, rnd], budget=KIT, car=(0.0, 4.5, 180))
