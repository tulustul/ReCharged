"""Catering tins (kit 'cans', hero 'can pyramid', art bible §3.5): 15 x 18 cm tins (0.67 x
0.8 u) with wrap-around labels of invented brands (tomato, beans, peach), rolled rims and a
brushed lid; stacked 6-3-1 into knockable pyramids by plan.py.

    blender -b -P tools/blender/props/supermarket/can.py -- [--preview DIR] [--only tomato]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

CELLS = {'tomato': 'can_tomato', 'beans': 'can_beans', 'peach': 'can_peach'}


def build(label):
    start('can_label', 'brushed', 'kick_plate')
    bm = core.new_bm()
    product_can(bm, V((0, 0, 0)), C.CAN_R, C.CAN_H, CELLS[label], n=16)
    for z in (0.02, C.CAN_H - 0.02):   # rolled rims
        geo.cylinder(bm, V((0, 0, z - 0.02)), V((0, 0, z + 0.02)), C.CAN_R * 1.01, mi('brushed'),
                     n=16)
    for z in (0.25, 0.55):             # ribs pressed into the label band
        geo.cylinder(bm, V((0, 0, z - 0.01)), V((0, 0, z + 0.01)), C.CAN_R * 1.004,
                     mi('can_label'), n=16)
    return propkit.finish(ENV, f'can_{label}', [smooth('can', bm, angle=40)],
                          car=(1.6, 0.4, 200))


argv = propkit._argv()
only = argv[argv.index('--only') + 1] if '--only' in argv else None
for lbl in C.CAN_LABELS:
    if only and lbl != only:
        continue
    build(lbl)
