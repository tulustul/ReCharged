"""Wall produce rack (kit, art bible §3.5): an 11.1 x 4 x 8 u refrigerated rack against the east
wall, front +Y: tiered slanted shelves of lettuces, broccoli and peppers, a chrome misting bar,
a lit canopy with the FRESH PRODUCE fascia, and a stainless kick plate.

    blender -b -P tools/blender/props/supermarket/produce_rack.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, D, H = C.RACK_L, C.RACK_D, C.RACK_H
TIERS = [(1.6, 1.8), (3.0, 1.2), (4.3, 0.6)]   # (z, front y)


def build():
    start('steel_dark', 'brushed', 'chrome', 'leafy', 'leafy_dark', 'fruit_red', 'fruit_yellow',
          'fruit_orange', 'cooler_glow', 'signs_glow', 'pallet_wood')
    rnd = rng(53)
    body = core.new_bm()
    geo.box(body, V((0, -D / 2 + 0.2, H / 2)), (L, 0.4, H), mi('steel_dark'))
    geo.box(body, V((0, 0, 0.6)), (L, D, 1.2), mi('brushed'))
    for s in (-1, 1):
        geo.extrude_x(body, [(-D / 2, 0), (D / 2, 0), (D / 2, 1.5), (0.4, 5.2), (0.4, H),
                             (-D / 2, H)], s * (L / 2 - 0.1) - 0.1, s * (L / 2 - 0.1) + 0.1,
                      mi('steel_dark'))
    geo.box(body, V((0, -0.2, H - 0.4)), (L, D - 1.2, 0.8), mi('steel_dark'))
    veg = core.new_bm()
    for z, yf in TIERS:
        y0 = -D / 2 + 0.4
        geo.box(body, V((0, (y0 + yf) / 2, z - 0.05)), (L - 0.4, yf - y0, 0.1), mi('pallet_wood'))
        for k in range(10):
            x0 = -L / 2 + 0.35 + k * (L - 0.7) / 10
            x1 = x0 + (L - 0.7) / 10 - 0.08
            kinds = rnd.choice([['leafy', 'leafy_dark'], ['leafy'], ['fruit_red'],
                                ['fruit_yellow', 'fruit_orange'], ['leafy_dark']])
            fruit_bed(veg, x0 - 0.05, x1 + 0.05, y0 + 0.1, yf - 0.1, z + 0.4, kinds, rnd, r=0.46,
                      slope=0.25)
    glow = core.new_bm()
    geo.box(glow, V((0, 0.9, H - 0.85)), (L - 0.5, 0.3, 0.08), mi('cooler_glow'))
    geo.cylinder(glow, V((-L / 2 + 0.3, 0.7, H - 1.2)), V((L / 2 - 0.3, 0.7, H - 1.2)), 0.06,
                 mi('chrome'), n=8)
    sign = core.new_bm()
    r = region('dept_produce')
    fs = geo.box(sign, V((0, 1.23, H - 0.4)), (3.9, 0.06, 0.72), mi('signs_glow'))  # canopy face
    for f in fs:
        f.normal_update()
        if f.normal.y > 0.9:
            map_face(sign, f, r, 'signs')
        else:
            fill_uv(sign, [f], [r[0] + 2, r[1] + 2, 2, 2], 'signs')
    parts = [part('body', body), smooth('veg', veg),
             part('glow', glow), part('sign', sign)]
    return propkit.finish(ENV, 'produce_rack', parts, car=(L / 2 + 2, D / 2 + 2, 210))


build()
