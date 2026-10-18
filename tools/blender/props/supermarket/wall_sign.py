"""Department wall signs (decor, art bible §3.5): big lit-edge boards high on the store walls
(16 x 3 u, centre at 15 u): FROZEN FOODS, DAIRY, FRESH PRODUCE, DELI & MEAT, CUSTOMER SERVICE.
Front +Y (away from the wall); origin on the floor at the wall. Physics-free.

    blender -b -P tools/blender/props/supermarket/wall_sign.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

SIGNS = {'frozen': 'dept_frozen', 'dairy': 'dept_dairy', 'produce': 'dept_produce',
         'deli': 'dept_deli', 'service': 'dept_service'}
W, H, Z = 16.0, 3.0, 15.0


def build(name):
    start('signs_glow', 'steel_dark')
    bm = core.new_bm()
    geo.box(bm, V((0, 0.15, Z)), (W + 0.3, 0.3, H + 0.3), mi('steel_dark'))
    fs = geo.box(bm, V((0, 0.34, Z)), (W, 0.08, H), mi('signs_glow'))
    r = region(SIGNS[name])
    for f in fs:
        f.normal_update()
        if f.normal.y > 0.9:
            map_face(bm, f, r, 'signs')
        else:
            fill_uv(bm, [f], [r[0] + 2, r[1] + 2, 2, 2], 'signs')
    return propkit.finish(ENV, f'wall_sign_{name}', [part('sign', bm, bevel=0.03, segments=1)],
                          shots=[('front', V((0.2, 1.0, 0.0)), 50)])


for n in SIGNS:
    build(n)
