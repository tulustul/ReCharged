"""Toy packaging: a red cardboard box (67 x 27 x 50 cm) with a big open window showing a toy car,
a yellow starburst badge, blue base band and side panels. Floor stacks near the shelves. Static.

    blender -b -P tools/blender/props/toy/toy_box.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

W, D, H = 3.0, 1.2, 2.2
core.reset()
cardboard('print_red', '#C8382C', 0.8)
cardboard('print_blue', '#2D62C4', 0.8)
cardboard('print_yellow', '#F0C23A', 0.8)
cardboard('inner', '#F1ECE0', 0.85)
plastic('car', '#21A35A', 0.25)
plastic('tyre', '#1F1F21', 0.6)
mi = core.mi
box, front = core.new_bm(), core.new_bm()
WX0, WX1, WZ0, WZ1 = -0.72, 1.22, 0.55, 1.72   # window opening (front face)
# pieces overlap by 1 cm instead of touching: coincident corners would weld into non-manifold
# geometry that breaks the bevel
geo.box(box, V((0, -0.295, H / 2)), (W, D - 0.59, H), mi('print_red'))            # back half
for x0, x1, z0, z1 in ((-W / 2, WX0, 0, H), (WX1, W / 2, 0, H), (WX0 - 0.01, WX1 + 0.01, 0, WZ0),
                       (WX0 - 0.01, WX1 + 0.01, WZ1, H)):                         # front frame
    geo.box(box, V(((x0 + x1) / 2, 0.3, (z0 + z1) / 2)), (x1 - x0, 0.6, z1 - z0), mi('print_red'))
for s in (-1, 1):  # side panels in blue
    geo.box(front, V((s * (W / 2 + 0.006), 0, H / 2)), (0.02, D - 0.1, H - 0.1), mi('print_blue'))
y = D / 2 + 0.006
geo.box(front, V((0, y, 0.22)), (W - 0.1, 0.02, 0.3), mi('print_blue'))           # base band
cx, cz = (WX0 + WX1) / 2, (WZ0 + WZ1) / 2
geo.box(front, V((cx, 0.01, cz)), (WX1 - WX0, 0.02, WZ1 - WZ0), mi('inner'))       # tray back
geo.box(front, V((cx, 0.3, WZ0 + 0.01)), (WX1 - WX0, 0.6, 0.02), mi('inner'))      # tray floor
for dz in (WZ1 - cz + 0.04, WZ0 - cz - 0.04):  # window frame
    geo.box(front, V((cx, y, cz + dz)), (WX1 - WX0 + 0.16, 0.03, 0.08), mi('inner'))
for dx in (WX0 - cx - 0.04, WX1 - cx + 0.04):
    geo.box(front, V((cx + dx, y, cz)), (0.08, 0.03, WZ1 - WZ0 + 0.16), mi('inner'))
car = [(cx, 0.32, 0.8, 1.1, 0.42, 0.22), (cx + 0.05, 0.32, 1.02, 0.55, 0.36, 0.22)]
for x, yy, z, sx, sy, sz in car:  # the toy car in the window tray
    geo.box(front, V((x, yy, z)), (sx, sy, sz), mi('car'))
for dx in (-0.35, 0.38):
    geo.cylinder(front, V((cx + dx, 0.06, 0.72)), V((cx + dx, 0.58, 0.72)), 0.14, mi('tyre'), n=12)
relief(front, star2(n=8, r_out=0.5, r_in=0.36), V((-1.05, y, 1.75)), (0, 1, 0), (0, 0, 1), 0.85,
       0.015, mi('print_yellow'))
geo.box(front, V((0.25, y, 1.92)), (2.0, 0.02, 0.22), mi('print_yellow'))           # title band
propkit.finish(ENV, 'toy_box', [part('box', box, bevel=0.025, segments=2),
                                part('print', front, bevel=0.0)], car=(2.6, 1.6, 200))
