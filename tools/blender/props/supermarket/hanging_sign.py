"""Hanging aisle signs (decor, art bible §3.5): double-sided AISLE n boards (6 x 3.75 u) hung on
two wires from the 22 u ceiling over each aisle, readable from both ends; plus a CHECKOUT board.
Physics-free (assets/data/decor/aisle_rush.ron), origin on the floor.

    blender -b -P tools/blender/props/supermarket/hanging_sign.py -- [--preview DIR] [--only 3]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

CEIL, Z0 = 22.0, 15.0


def board(bm, rect, w, h, z0):
    fs = geo.box(bm, V((0, 0, z0 + h / 2)), (w, 0.12, h), mi('signs'))
    for f in fs:
        f.normal_update()
        if abs(f.normal.y) > 0.9:
            map_face(bm, f, rect, 'signs')
        else:
            fill_uv(bm, [f], [rect[0] + 2, rect[1] + 2, 2, 2], 'signs')


def build(name, rect, w, h):
    start('signs', 'brushed', 'steel_dark')
    bm = core.new_bm()
    board(bm, rect, w, h, Z0)
    geo.box(bm, V((0, 0, Z0 + h + 0.06)), (w + 0.1, 0.18, 0.12), mi('brushed'))
    wires = core.new_bm()
    for x in (-w / 2 + 0.4, w / 2 - 0.4):
        geo.cylinder(wires, V((x, 0, Z0 + h + 0.1)), V((x, 0, CEIL)), 0.025, mi('steel_dark'), n=4)
    return propkit.finish(ENV, name, [part('board', bm, bevel=0.02, segments=1),
                                      smooth('wires', wires)], shots=[
        ('front', V((0.25, 1.0, 0.05)), 50)])


argv = propkit._argv()
only = argv[argv.index('--only') + 1] if '--only' in argv else None
for n in range(1, 8):
    if only and str(n) != only:
        continue
    build(f'hanging_sign_{n}', region(f'aisle_{n}'), 6.0, 3.75)
if not only or only == 'checkout':
    build('hanging_sign_checkout', region('dept_checkout'), 12.0, 2.25)
