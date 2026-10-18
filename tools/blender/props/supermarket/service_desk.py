"""Customer-service desk (kit, art bible §3.5): a long counter (20.6 x 6 x 4.5 u, front +Y)
with a CUSTOMER SERVICE fascia, a raised transaction ledge, a monitor and a parcel shelf.

    blender -b -P tools/blender/props/supermarket/service_desk.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, W, H = C.DESK_L, C.DESK_W, C.DESK_H

start('white_plastic', 'kick_plate', 'brushed', 'signs', 'black_plastic', 'screen',
      'blue_plastic', 'cardboard', 'packaging')
body = core.new_bm()
geo.box(body, V((0, 0, 0.2)), (L, W, 0.4), mi('kick_plate'))
geo.box(body, V((0, 0, (0.4 + H - 0.6) / 2)), (L, W, H - 1.0), mi('white_plastic'))
geo.box(body, V((0, W / 2 - 0.5, H - 0.3)), (L, 1.0, 0.6), mi('blue_plastic'))
top = core.new_bm()
geo.box(top, V((0, 0, H - 0.55)), (L + 0.1, W + 0.1, 0.1), mi('brushed'))
geo.box(top, V((0, W / 2 - 0.5, H + 0.05)), (L + 0.1, 1.2, 0.1), mi('brushed'))
fascia = core.new_bm()
r = region('dept_service')
fs = geo.box(fascia, V((0, W / 2 + 0.05, 2.6)), (9.0, 0.1, 1.7), mi('signs'))
for f in fs:
    f.normal_update()
    if f.normal.y > 0.9:
        map_face(fascia, f, r, 'signs')
    else:
        fill_uv(fascia, [f], [r[0] + 2, r[1] + 2, 2, 2], 'signs')
gear = core.new_bm()
for x in (-5.0, 4.0):
    geo.box(gear, V((x, 0.2, H - 0.25)), (1.4, 1.0, 0.5), mi('black_plastic'))
    geo.box(gear, V((x, -0.1, H + 0.6)), (1.6, 0.1, 1.1), mi('black_plastic'))
    geo.box(gear, V((x, -0.04, H + 0.6)), (1.45, 0.02, 0.95), mi('screen'))
rnd = rng(9)
for k in range(5):   # parcels waiting on the back of the counter
    w = rnd.uniform(1.0, 1.6)
    product_box(gear, V((-9.0 + k * 1.9, -W / 2 + 1.2, H - 0.5)), w, 1.2, rnd.uniform(0.6, 1.2),
                'carton_%d' % (1 + k % 3), front=-1)
propkit.finish(ENV, 'service_desk', [part('body', body, bevel=0.03), part('top', top, bevel=0.02),
                                     part('fascia', fascia), part('gear', gear, bevel=0.01)],
               car=(L / 2 + 2, W / 2 + 3, 210))
