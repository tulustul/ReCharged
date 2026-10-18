"""Shipping cartons (kit 'cardboard box' x2, art bible §3.5): kraft boxes with printed arrows
and tape, small (1.78 x 1.33 x 1.33 u) and large (2.66 x 1.78 x 1.78 u). Knockable.

    blender -b -P tools/blender/props/supermarket/boxes.py -- [--preview DIR] [--only small]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

SIZES = {'small': (1.78, 1.33, 1.33, 'carton_1'), 'large': (2.66, 1.78, 1.78, 'carton_3')}


def build(name):
    l, w, h, cellname = SIZES[name]
    start('packaging', 'tape')
    bm = core.new_bm()
    fs = geo.box(bm, V((0, 0, h / 2)), (l, w, h), mi('packaging'))
    for f in fs:
        f.normal_update()
        if abs(f.normal.z) < 0.5:
            map_face(bm, f, cell(cellname))
        else:
            fill_uv(bm, [f], side(cellname))
    geo.box(bm, V((0, 0, h + 0.005)), (l + 0.02, 0.32, 0.02), mi('tape'))
    for s in (-1, 1):
        geo.box(bm, V((s * (l / 2 + 0.005), 0, h - 0.35)), (0.02, 0.32, 0.7), mi('tape'))
    return propkit.finish(ENV, f'box_{name}', [part('box', bm, bevel=0.02, segments=1)],
                          car=(l / 2 + 2, 0.5, 200))


argv = propkit._argv()
only = argv[argv.index('--only') + 1] if '--only' in argv else None
for n in SIZES:
    if only and n != only:
        continue
    build(n)
