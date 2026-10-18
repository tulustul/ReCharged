"""Pallet bin of melons (kit 'fruit', art bible §3.5): an octagonal printed cardboard bin on a
pallet (5.3 x 4.4 x 4 u) heaped with watermelons and cantaloupes.

    blender -b -P tools/blender/props/supermarket/produce_bin.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, W, H = C.BIN_L, C.BIN_W, C.BIN_H


def build():
    start('pallet_wood', 'pallet_wood_dark', 'cardboard', 'packaging', 'melon', 'fruit_yellow',
          'leafy_dark')
    rnd = rng(41)
    wood = core.new_bm()
    pallet(wood, L, W)
    box = core.new_bm()
    # octagonal bin wall: a prism of a chamfered rectangle
    c = 0.7
    oct2 = [(L / 2 - 0.1, -W / 2 + 0.1 + c), (L / 2 - 0.1, W / 2 - 0.1 - c),
            (L / 2 - 0.1 - c, W / 2 - 0.1), (-L / 2 + 0.1 + c, W / 2 - 0.1),
            (-L / 2 + 0.1, W / 2 - 0.1 - c), (-L / 2 + 0.1, -W / 2 + 0.1 + c),
            (-L / 2 + 0.1 + c, -W / 2 + 0.1), (L / 2 - 0.1 - c, -W / 2 + 0.1)]
    fs = geo.prism(box, oct2, V((0, 0, C.PALLET_H)), V((0, 0, H - 0.7)), mi('packaging'),
                   up=V((0, 1, 0)))
    for f in fs:
        f.normal_update()
        if abs(f.normal.z) < 0.5:
            map_face(box, f, cell('carton_3'))
        else:
            fill_uv(box, [f], side('carton_3'))
    melons = core.new_bm()
    geo.box(melons, V((0, 0, H - 0.75)), (L - 0.5, W - 0.5, 0.05), mi('leafy_dark'))
    for i in range(4):
        for j in range(3):
            x = -L / 2 + 0.9 + i * (L - 1.8) / 3
            y = -W / 2 + 0.9 + j * (W - 1.8) / 2
            r = rnd.uniform(0.5, 0.6)
            kind = 'melon' if (i + j) % 3 else 'fruit_yellow'
            geo.sphere(melons, V((x, y, H - 0.7 + r * 0.55)), r, mi(kind), n=10, rings=6,
                       scale=(1.0, 0.85 if kind == 'melon' else 1.0, 0.85))
    parts = [part('wood', wood, bevel=0.02, segments=1), part('box', box, bevel=0.03, segments=1),
             smooth('melons', melons)]
    return propkit.finish(ENV, 'produce_bin', parts, car=(L / 2 + 2, W / 2 + 2, 210))


build()
