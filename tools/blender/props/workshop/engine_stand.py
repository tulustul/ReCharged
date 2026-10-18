"""Engine on a stand (island equipment): an inline-four block with cylinder head, red rocker
cover, timing cover and pulleys at the front (+Y... Bevy -z), a rusty exhaust manifold, on a
rotating stand (post on a castored T-base). Footprint ~5 x 3 u.

    blender -b -P tools/blender/props/workshop/engine_stand.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('steel_dark', 'aluminium', 'plastic_red', 'plastic_black', 'car_exhaust_w', 'lift_blue',
      'rubber', 'steel', extra={'car_exhaust_w': dict(base='#8A6A52', metallic=0.5, rough=0.7)})
Z = 2.6
bm = core.new_bm()
geo.box(bm, V((0, 0, Z)), (2.2, 2.9, 1.9), mi('steel_dark'))                        # block
geo.box(bm, V((0, 0, Z + 1.2)), (1.7, 2.8, 0.55), mi('aluminium'))                  # head
geo.box(bm, V((0, 0, Z + 1.62)), (1.3, 2.6, 0.3), mi('plastic_red'))                # rocker cover
geo.box(bm, V((0, 0, Z - 1.15)), (1.8, 2.4, 0.45), mi('steel_dark'))                # oil pan
geo.box(bm, V((0, 1.55, Z + 0.1)), (1.8, 0.25, 2.0), mi('aluminium'))               # timing cover
geo.box(bm, V((0.0, -1.7, Z)), (0.8, 0.5, 0.8), mi('lift_blue'))                    # stand head
geo.box(bm, V((0, -1.6, 1.1)), (0.45, 0.45, 2.2), mi('lift_blue'))                  # post
geo.box(bm, V((0, -1.6, 0.25)), (0.4, 0.4, 0.3), mi('lift_blue'))
geo.box(bm, V((0, -0.2, 0.22)), (0.35, 3.2, 0.25), mi('lift_blue'))                 # base bar
geo.box(bm, V((0, -1.6, 0.22)), (3.0, 0.35, 0.25), mi('lift_blue'))                 # T bar
box_ = core.obj_from_bm('box', bm, bevel=0.03, segments=1, angle=40)
bm = core.new_bm()
for z, r in ((Z - 0.5, 0.55), (Z + 0.6, 0.35)):                                     # pulleys
    geo.cylinder(bm, V((0, 1.68, z)), V((0, 1.9, z)), r, mi('steel'), n=14)
geo.cylinder(bm, V((0.7, 1.72, Z + 0.3)), V((0.7, 1.95, Z + 0.3)), 0.3, mi('aluminium'), n=12)
for k in range(4):  # exhaust manifold runners on the +x side
    y = -1.05 + k * 0.7
    geo.tube(bm, [V((1.1, y, Z + 0.95)), V((1.45, y, Z + 0.8)), V((1.5, 0.0, Z + 0.1))], 0.12,
             mi('car_exhaust_w'), n=6, corner=0.2, cseg=2)
for p in (V((0, 1.4, 0.12)), V((1.3, -1.6, 0.12)), V((-1.3, -1.6, 0.12))):           # casters
    geo.cylinder(bm, p - V((0.08, 0, 0)), p + V((0.08, 0, 0)), 0.12, mi('rubber'), n=8)
rnd = core.obj_from_bm('round', bm, weighted=False, angle=40)
finish('engine_stand', [box_, rnd], budget=KIT, car=(3.0, 3.0, 200))
