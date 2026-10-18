"""Narrow box ramp (kit, art bible §3.5): a flattened carton laid over stepped cartons, 3.55 u
wide, rising 3.6 u over 12 u toward +Y (Bevy -Z): the risky way up onto the water pallets and
the Golden Battery (forward layouts only).

    blender -b -P tools/blender/props/supermarket/water_ramp.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

R = C.WATER_RAMP
W, RISE, RUN = R['width'], R['rise'], R['run']


def build():
    start('cardboard', 'cardboard_dark', 'tape', 'packaging')
    bm = core.new_bm()
    y0, y1 = -RUN / 2, RUN / 2
    for s in (-1, 1):   # triangular side boards
        x = s * (W / 2 - 0.05)
        geo.extrude_x(bm, [(y0, 0.0), (y1, 0.0), (y1, RISE)], x - 0.05, x + 0.05,
                      mi('cardboard_dark'))
    # the flattened carton: a 0.07 u sheet along the slope
    prof = lambda x: [V((x, y0, 0.0)), V((x, y1, RISE - 0.07)), V((x, y1, RISE)),  # noqa: E731
                      V((x, y0 + 0.25, 0.07))]
    top = geo._finish(bm, geo._bridge(bm, [prof(-W / 2), prof(W / 2)], mi('packaging')))
    for f in top:
        f.normal_update()
        if f.normal.z > 0.5:
            map_face(bm, f, cell('carton_2'), up=V((0, 1, 0)))
        else:
            fill_uv(bm, [f], side('carton_2'))
    for t in (0.33, 0.66):  # tape across the sheet
        geo.box(bm, V((0, y0 + RUN * t, RISE * t + 0.03)), (W - 0.2, 0.3, 0.02), mi('tape'))
    return propkit.finish(ENV, 'water_ramp', [part('ramp', bm, bevel=0.015)],
                          car=(W / 2 + 2, y0, 0))


build()
