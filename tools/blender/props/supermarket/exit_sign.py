"""Illuminated EXIT sign with a twin-head emergency light (kit, art bible §3.5): wall-mounted at
12 u, origin on the floor at the wall, front +Y. The sign glows green (always); the two lamp
heads add the night's emergency spots (lights in catalog.py).

    blender -b -P tools/blender/props/supermarket/exit_sign.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

start('white_plastic', 'signs_glow', 'lamp_tube', 'steel_dark')
bm = core.new_bm()
geo.box(bm, V((0, 0.3, 12.6)), (2.8, 0.6, 1.5), mi('white_plastic'))
fs = geo.box(bm, V((0, 0.62, 12.6)), (2.4, 0.04, 1.2), mi('signs_glow'))
r = region('exit')
for f in fs:
    f.normal_update()
    if f.normal.y > 0.9:
        map_face(bm, f, r, 'signs')
    else:
        fill_uv(bm, [f], [r[0] + 2, r[1] + 2, 2, 2], 'signs')
geo.box(bm, V((0, 0.35, 13.9)), (1.6, 0.7, 0.5), mi('white_plastic'))
heads = core.new_bm()
for s in (-1, 1):
    geo.cylinder(heads, V((s * 1.0, 0.5, 13.8)), V((s * 1.0, 0.9, 13.5)), 0.32, mi('white_plastic'),
                 n=12)
    geo.cylinder(heads, V((s * 1.0, 0.9, 13.5)), V((s * 1.0, 0.94, 13.48)), 0.26, mi('lamp_tube'),
                 n=12)
propkit.finish(ENV, 'exit_sign', [part('sign', bm, bevel=0.02, segments=1),
                                  smooth('heads', heads, angle=40)], car=(3.0, 2.0, 200))
