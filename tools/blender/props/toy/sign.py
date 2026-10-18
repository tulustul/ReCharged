"""Hanging aisle sign (decor, out of reach): a blue rounded board with a white rim, an RC-car
pictogram on the front and a teddy-bear pictogram on the back, hung on two rods. Origin at the
ceiling attachment (the rod tops); the board hangs 2.5-5 u below it.

    blender -b -P tools/blender/props/toy/sign.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

W, H, T = 7.0, 2.6, 0.2
Z0, Z1 = -5.1, -2.5   # board bottom / top below the ceiling
core.reset()
painted('board', '#2F64C9', 0.4)
painted('rim', '#F2F0EA', 0.4)
core.material('rod', '#C9CCD1', 1.0, 0.35)
mi = core.mi
board, pic = core.new_bm(), core.new_bm()
extrude(board, 'y', geo.rrect(W + 0.3, H + 0.3, 0.7, 6, 0, (Z0 + Z1) / 2), -T / 2 - 0.02,
        T / 2 + 0.02, mi('rim'))
extrude(board, 'y', geo.rrect(W, H, 0.6, 6, 0, (Z0 + Z1) / 2), -T / 2 - 0.05, T / 2 + 0.05,
        mi('board'))
for x in (-W * 0.35, W * 0.35):
    geo.cylinder(board, V((x, 0, Z1 + 0.1)), V((x, 0, 0.0)), 0.05, mi('rod'), n=8)
zc = (Z0 + Z1) / 2
y = T / 2 + 0.06  # front: RC car (body, cabin, wheels, wing)
extrude(pic, 'y', geo.rrect(2.6, 0.55, 0.2, 3, 0, zc - 0.1), y, y + 0.03, mi('rim'))
extrude(pic, 'y', geo.rrect(1.1, 0.5, 0.18, 3, -0.1, zc + 0.35), y, y + 0.03, mi('rim'))
extrude(pic, 'y', [(-1.25, zc + 0.15), (-0.95, zc + 0.15), (-1.05, zc + 0.65), (-1.45, zc + 0.65)],
        y, y + 0.03, mi('rim'))
for x in (-0.85, 0.85):
    extrude(pic, 'y', geo.circle2(0.36, 20, x, zc - 0.45), y, y + 0.04, mi('rim'))
    extrude(pic, 'y', geo.circle2(0.15, 16, x, zc - 0.45), y + 0.04, y + 0.06, mi('board'))
y = -T / 2 - 0.06  # back: teddy-bear head
extrude(pic, 'y', geo.circle2(0.75, 28, 0, zc - 0.1), y - 0.03, y, mi('rim'))
for x in (-0.62, 0.62):
    extrude(pic, 'y', geo.circle2(0.3, 16, x, zc + 0.55), y - 0.03, y, mi('rim'))
for x in (-0.28, 0.28):
    extrude(pic, 'y', geo.circle2(0.1, 12, x, zc + 0.05), y - 0.05, y - 0.03, mi('board'))
extrude(pic, 'y', geo.circle2(0.14, 12, 0, zc - 0.3), y - 0.05, y - 0.03, mi('board'))
propkit.finish(ENV, 'sign', [part('board', board, bevel=0.02, segments=2),
                             part('pictograms', pic, bevel=0.0)], below=5.3,
               car=(0.0, 4.0, 180), before_preview=lambda ob: setattr(ob.location, 'z', 7.0))
