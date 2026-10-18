"""Paper grocery bags (kit 'bag', art bible §3.5): kraft bags (1.33 x 0.8 x 1.78 u) packed with a
baguette, a lettuce and a milk carton, left on the bagging shelves of the open lanes. Decor
(they stand on the 4 u counters, out of reach); origin at the bag's base.

    blender -b -P tools/blender/props/supermarket/paper_bag.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

start('signs', 'cardboard', 'leafy', 'pallet_wood', 'packaging')
bm = core.new_bm()
W, D, H = 1.33, 0.8, 1.78
fs = geo.box(bm, V((0, 0, H / 2)), (W, D, H), mi('signs'))
r = region('paper_bag')
for f in fs:
    f.normal_update()
    if abs(f.normal.y) > 0.9:
        map_face(bm, f, r, 'signs')
    else:
        fill_uv(bm, [f], [r[0] + 2, r[1] + 2, 2, 2], 'signs')
geo.box(bm, V((0, 0, H - 0.02)), (W - 0.06, D - 0.06, 0.04), mi('cardboard'))   # open top
fill = core.new_bm()
geo.cylinder(fill, V((-0.35, 0.05, H - 0.3)), V((-0.2, -0.05, H + 1.0)), 0.13, mi('pallet_wood'),
             n=8)
geo.sphere(fill, V((0.25, 0.05, H + 0.05)), 0.33, mi('leafy'), n=8, rings=5, scale=(1, 1, 0.8))
product_carton(fill, V((0.05, -0.15, H - 0.6)), 0.31, 0.31, 1.0, 'milk_1', front=1)
propkit.finish(ENV, 'paper_bag', [part('bag', bm, bevel=0.01, segments=1), smooth('fill', fill)],
               car=(1.5, 0.6, 200))
