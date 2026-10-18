"""Overstock on top of a gondola (decor, art bible §3.5): a row of cartons and shrink-wrapped
cases sitting on the 8.9 u gondola top (11.1 u long), out of reach of the cars. Physics-free;
place at the gondola's position and yaw.

    blender -b -P tools/blender/props/supermarket/topstock.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

PRINTED = ['soda_cola', 'chips_1', 'soap_1', 'cereal_2', 'water', 'pasta_1', 'cereal_5',
           'soap_3', 'carton_1', 'carton_2', 'carton_3', 'carton_1']


def build(variant):
    start('packaging', 'tape')
    rnd = rng(300 + ord(variant))
    bm = core.new_bm()
    x = -C.GONDOLA_LEN / 2 + 0.2
    while x < C.GONDOLA_LEN / 2 - 1.2:
        w = rnd.uniform(1.3, 2.4)
        if rnd.random() < 0.18:     # gap
            x += w * 0.6
            continue
        d, h = rnd.uniform(1.6, 2.8), rnd.uniform(1.0, 2.2)
        name = rnd.choice(PRINTED)
        fs = geo.box(bm, V((x + w / 2, rnd.uniform(-0.3, 0.3), C.GONDOLA_H + h / 2)),
                     (w - 0.06, d, h), mi('packaging'))
        for f in fs:
            f.normal_update()
            if abs(f.normal.z) < 0.5:
                map_face(bm, f, cell(name))
            else:
                fill_uv(bm, [f], side(name))
        x += w
    return propkit.finish(ENV, f'topstock_{variant}', [part('stock', bm, bevel=0.02, segments=1)],
                          shots=[('front', V((0.3, 1.0, 0.4)), 50)])


for v in ('a', 'b'):
    build(v)
