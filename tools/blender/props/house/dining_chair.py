"""Dining chair: Scandinavian oak chair, 45 x 50 cm, seat at 46 cm, 85 cm tall (2.0 x 2.2 x
3.8 u) with a cream seat pad, round tapered splayed legs and a curved two-rail back. Front
(+Y) = the seat's front edge. Static (slalom gates in the dining corner).

    blender -b -P tools/blender/props/house/dining_chair.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

SEAT_Z = 2.04
core.reset()
materials('oak', 'fabric_cream')
oak = mi('oak')

bm = core.new_bm()
geo.box(bm, V((0, 0.05, SEAT_Z - 0.06)), (2.0, 2.1, 0.12), oak)  # seat board
seat = core.obj_from_bm('seat', box_uv(bm, 'y'), bevel=0.035, segments=2)

bm = core.new_bm()
rbox(bm, V((0, 0.15, SEAT_Z + 0.07)), (1.8, 1.7, 0.16), 0.07, mi('fabric_cream'), seg=3,
     puff=0.08)
pad = core.obj_from_bm('pad', box_uv(bm, 'x'), weighted=False, angle=60)

bm = core.new_bm()
for sx in (-1, 1):
    for sy, back in ((1, False), (-1, True)):
        base = V((sx * 0.85, sy * 0.95, 0))
        if back:  # rear legs run up into the back posts
            pts = [base + V((sx * 0.06, -0.1, 0)), V((sx * 0.85, -0.95, SEAT_Z)),
                   V((sx * 0.82, -1.08, 3.8))]
            geo.tube(bm, pts, 0.085, oak, n=10, corner=0.6, cseg=3, r_end=0.07)
        else:
            geo.cylinder(bm, base + V((sx * 0.07, 0.08, 0)), V((sx * 0.85, 0.95, SEAT_Z - 0.1)),
                         0.07, oak, n=10, r1=0.09)
for z, h in ((2.75, 0.22), (3.45, 0.36)):  # curved back rails
    pts = [V((0.82 * math.sin(a), -1.06 - 0.12 * math.cos(a), z)) for a in
           (math.radians(t) for t in range(-62, 63, 31))]
    for a, b in zip(pts, pts[1:]):
        geo.prism(bm, geo.rrect(0.08, h, 0.03, 2), a, b, oak, up=V((0, 0, 1)))
for y in (0.9, -0.9):  # stretchers
    geo.cylinder(bm, V((-0.87, y, 0.75)), V((0.87, y, 0.75)), 0.04, oak, n=8)
frame = core.obj_from_bm('frame', box_uv(bm, 'z'), weighted=False, angle=50)

done('dining_chair', [seat, pad, frame], car=(2.6, 0.6, 30))
