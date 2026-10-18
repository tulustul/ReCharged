"""Tyre changer (island equipment): grey cabinet with pedals, a red turntable clamping a wheel,
a tilting column at the back (+Y... Bevy +z) with the horizontal arm and mount head, an air
gauge. Footprint 4 x 3.2 u.

    blender -b -P tools/blender/props/workshop/tyre_changer.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('paint_grey', 'chest_red', 'steel_dark', 'steel', 'tyre', 'rubber', 'steel_black',
      'plastic_black', 'label_white')
bm = core.new_bm()
geo.box(bm, V((0, 0, 1.55)), (3.8, 3.2, 3.1), mi('paint_grey'))
geo.box(bm, V((0, -1.15, 8.0)), (0.9, 0.9, 1.0), mi('paint_grey'))       # column head
geo.box(bm, V((0, -0.2, 7.7)), (0.5, 2.2, 0.45), mi('paint_grey'))       # horizontal arm
geo.box(bm, V((-2.1, 0.6, 0.15)), (0.6, 1.4, 0.3), mi('steel_dark'))     # pedals
box_ = core.obj_from_bm('box', bm, bevel=0.03, segments=1, angle=40)
bm = core.new_bm()
geo.cylinder(bm, V((0, -1.15, 3.0)), V((0, -1.15, 7.6)), 0.38, mi('paint_grey'), n=12)  # column
geo.cylinder(bm, V((0, 0.85, 7.5)), V((0, 0.85, 5.0)), 0.13, mi('steel'), n=8)           # head bar
geo.cylinder(bm, V((0, 0.85, 5.0)), V((0, 0.85, 4.75)), 0.3, mi('plastic_black'), n=10)  # mount head
geo.cylinder(bm, V((0, 0.2, 3.1)), V((0, 0.2, 3.3)), 1.35, mi('chest_red'), n=20)        # turntable
for k in range(4):  # clamp jaws
    a = k * 1.5708
    c = V((1.2 * (1 if k == 0 else -1 if k == 2 else 0), 0.2 + 1.2 * (1 if k == 1 else -1 if k == 3 else 0), 3.3))
    geo.box(bm, c + V((0, 0, 0.12)), (0.25, 0.25, 0.24), mi('steel_dark'))
tyre(bm, V((0, 0.2, 3.3 + C.TYRE['w'] / 2 + 0.05)), n=24, seg=2)
steel_rim(bm, V((0, 0.2, 3.3 + C.TYRE['w'] / 2 + 0.05)), rim_r=RIM_R, w=C.TYRE['w'] * 0.82, n=20)
geo.cylinder(bm, V((1.92, -0.6, 2.4)), V((2.02, -0.6, 2.4)), 0.28, mi('label_white'), n=12)  # gauge
rnd = core.obj_from_bm('round', bm, weighted=False, angle=50)
finish('tyre_changer', [box_, rnd], budget=KIT, car=(0.0, 5.0, 180))
