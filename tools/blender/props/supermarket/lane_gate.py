"""Closed-lane gate (kit, art bible §3.5): two stainless posts with a solid panel across a
checkout lane (6 x 2.7 u) and a red LANE CLOSED board on top. Front +Y.

    blender -b -P tools/blender/props/supermarket/lane_gate.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

W, H = C.GATE_W, C.GATE_H

start('brushed', 'chrome', 'glass_frost', 'signs', 'black_plastic')
frame = core.new_bm()
for x in (-W / 2 + 0.12, W / 2 - 0.12):
    geo.cylinder(frame, V((x, 0, 0)), V((x, 0, H + 0.9)), 0.11, mi('brushed'), n=10)
    geo.cylinder(frame, V((x, 0, 0)), V((x, 0, 0.06)), 0.3, mi('black_plastic'), n=12)
geo.cylinder(frame, V((-W / 2 + 0.12, 0, H)), V((W / 2 - 0.12, 0, H)), 0.07, mi('chrome'), n=8)
geo.cylinder(frame, V((-W / 2 + 0.12, 0, 0.25)), V((W / 2 - 0.12, 0, 0.25)), 0.06, mi('chrome'),
             n=8)
panel = core.new_bm()
geo.box(panel, V((0, 0, (0.3 + H - 0.05) / 2)), (W - 0.5, 0.06, H - 0.35), mi('glass_frost'))
board = core.new_bm()
fs = geo.box(board, V((0, 0, H + 0.55)), (3.2, 0.08, 0.8), mi('signs'))
for f in fs:
    f.normal_update()
    if abs(f.normal.y) > 0.9:
        map_face(board, f, region('closed'), 'signs')
    else:
        fill_uv(board, [f], [region('closed')[0] + 2, region('closed')[1] + 2, 2, 2], 'signs')
propkit.finish(ENV, 'lane_gate', [smooth('frame', frame, angle=40), part('panel', panel),
                                  part('board', board, bevel=0.01)], car=(W / 2 + 2, 2.5, 200))
