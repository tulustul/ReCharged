"""Blue 205 l steel oil drum (0.57 x 0.88 m) with rolling hoops, chimes, two bungs and a hand
pump on the lid.

    blender -b -P tools/blender/props/workshop/oil_drum.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('drum_blue', 'steel', 'steel_dark', 'plastic_black', 'oil_dark', 'plastic_red')
R, H = 1.265, 3.9
prof = [(R - 0.05, 0.0), (R, 0.06), (R, 0.12), (R - 0.03, 0.16), (R - 0.03, 1.22), (R + 0.03, 1.28),
        (R + 0.03, 1.36), (R - 0.03, 1.42), (R - 0.03, 2.48), (R + 0.03, 2.54), (R + 0.03, 2.62),
        (R - 0.03, 2.68), (R - 0.03, H - 0.16), (R, H - 0.12), (R, H - 0.04), (R - 0.06, H),
        (R - 0.09, H - 0.06), (R - 0.1, H - 0.1), (0.0, H - 0.1)]
bm = core.new_bm()
revolve(bm, prof, mi('drum_blue'), n=32)
for x, y, r in ((0.6, 0.35, 0.16), (-0.75, -0.2, 0.1)):  # bungs
    geo.cylinder(bm, V((x, y, H - 0.1)), V((x, y, H - 0.02)), r, mi('steel'), n=10)
# hand pump: tube, body, handle, spout
geo.cylinder(bm, V((0.6, 0.35, H - 0.04)), V((0.6, 0.35, H + 1.3)), 0.08, mi('steel_dark'), n=8)
geo.cylinder(bm, V((0.6, 0.35, H + 0.9)), V((0.6, 0.35, H + 1.5)), 0.2, mi('plastic_red'), n=10)
geo.tube(bm, [V((0.6, 0.35, H + 1.5)), V((0.6, 0.35, H + 1.85)), V((0.1, 0.1, H + 2.0))], 0.05,
         mi('steel_dark'), n=6, corner=0.15, cseg=2)
geo.tube(bm, [V((0.6, 0.55, H + 1.2)), V((0.6, 1.0, H + 1.15)), V((0.6, 1.1, H + 0.9))], 0.06,
         mi('plastic_black'), n=6, corner=0.1, cseg=2)
drum = core.obj_from_bm('drum', bm, weighted=False, angle=40)
finish('oil_drum', [drum], budget=KIT, car=(2.9, -1.0, 20))
