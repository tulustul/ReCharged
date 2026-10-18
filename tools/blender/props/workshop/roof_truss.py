"""Roof truss (decor): an exposed steel Pratt truss spanning the workshop north-south (120 u)
under the 24 u ceiling, top and bottom chords, verticals and diagonals, with a purlin stub row
and a cable tray hung under it. Origin at the truss centre at floor level (the steel starts at
19.5 u); the span runs along Y.

    blender -b -P tools/blender/props/workshop/roof_truss.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

SPAN, Z0, Z1, PANEL = 120.0, 20.0, 23.6, 6.0
start('paint_grey', 'steel_dark', 'galvanised')
bm = core.new_bm()
g = mi('paint_grey')
geo.box(bm, V((0, 0, Z0)), (0.5, SPAN, 0.5), g)                       # bottom chord
geo.box(bm, V((0, 0, Z1)), (0.5, SPAN, 0.5), g)                       # top chord
n = int(SPAN / PANEL)
for k in range(n + 1):
    y = -SPAN / 2 + k * PANEL
    geo.box(bm, V((0, y, (Z0 + Z1) / 2)), (0.3, 0.3, Z1 - Z0), g)     # verticals
    if k < n:
        a, b = (V((0, y, Z0)), V((0, y + PANEL, Z1))) if k < n / 2 else \
            (V((0, y, Z1)), V((0, y + PANEL, Z0)))
        geo.prism(bm, [(-0.12, -0.12), (0.12, -0.12), (0.12, 0.12), (-0.12, 0.12)], a, b, g,
                  up=V((1, 0, 0)))
for k in range(0, n + 1, 2):  # purlin stubs on top (the ceiling hides the rest)
    y = -SPAN / 2 + k * PANEL
    geo.box(bm, V((0, y, Z1 + 0.25)), (3.0, 0.3, 0.3), mi('steel_dark'))
for k in range(n):  # cable tray hanging under the bottom chord
    y = -SPAN / 2 + (k + 0.5) * PANEL
    geo.box(bm, V((0.9, y, Z0 - 0.9)), (0.8, PANEL - 0.1, 0.12), mi('galvanised'))
    geo.box(bm, V((0.9, y - PANEL / 2 + 0.2, Z0 - 0.45)), (0.05, 0.05, 0.9), mi('steel_dark'))
tr = core.obj_from_bm('truss', bm, weighted=False, angle=40)
finish('roof_truss', [tr], budget=KIT, car=(4.0, 0.0, 0.0))
