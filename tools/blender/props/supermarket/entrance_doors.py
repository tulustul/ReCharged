"""Automatic entrance doors (decor, art bible §3.5): a dark aluminium portal (14 x 9.5 u) with
two sliding glass leaves (blend), a sensor box and a WELCOME header, standing just inside the
shopfront glazing. Front +Y faces into the store; origin on the floor. Physics-free (the shell
wall behind it is the collider).

    blender -b -P tools/blender/props/supermarket/entrance_doors.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

W, H = 14.0, 9.5
start('steel_dark', 'glass', 'black_plastic', 'signs', 'rubber')
frame = core.new_bm()
for x in (-W / 2 + 0.2, W / 2 - 0.2):
    geo.box(frame, V((x, 0, H / 2)), (0.4, 0.5, H), mi('steel_dark'))
geo.box(frame, V((0, 0, H - 0.6)), (W, 0.6, 1.2), mi('steel_dark'))
geo.box(frame, V((0, 0.33, H - 1.35)), (1.0, 0.25, 0.3), mi('black_plastic'))     # sensor
fs = geo.box(frame, V((0, 0.32, H - 0.6)), (5.0, 0.04, 0.9), mi('signs'))
r = region('welcome')
for f in fs:
    f.normal_update()
    if f.normal.y > 0.9:
        map_face(frame, f, r, 'signs')
    else:
        fill_uv(frame, [f], [r[0] + 2, r[1] + 2, 2, 2], 'signs')
leaves = core.new_bm()
for s in (-1, 1):   # the two leaves, slightly open
    xc = s * (W / 4 + 0.6)
    geo.box(leaves, V((xc, 0.1, (H - 1.2) / 2)), (W / 2 - 1.2, 0.12, H - 1.3), mi('steel_dark'))
glass = core.new_bm()
for s in (-1, 1):
    xc = s * (W / 4 + 0.6)
    geo.box(glass, V((xc, 0.18, (H - 1.2) / 2 + 0.1)), (W / 2 - 1.6, 0.04, H - 1.9), mi('glass'))
geo.box(frame, V((0, 0.0, 0.03)), (W - 0.8, 0.8, 0.06), mi('rubber'))              # floor track
propkit.finish(ENV, 'entrance_doors', [part('frame', frame, bevel=0.02, segments=1),
                                       part('leaves', leaves), part('glass', glass)],
               car=(W / 2 + 2, 2.0, 200))
