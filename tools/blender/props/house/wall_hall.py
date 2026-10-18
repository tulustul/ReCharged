"""Hall wall: the wall between the hall and the living room, 32 u long (x -16..16; placed with
yaw -90 so +x runs south), 0.6 u thick, 11.5 u high, with two wide cased openings (9 u wide,
9.3 u high: the B lane at x -12.5..-3.5 and the A lane at x 1.5..10.5) and headers above.
White skirting and casings, greige plaster, a mirror and a coat hook rail on the hall side,
a picture on the living-room side. Front (+Y) = the living room.

    blender -b -P tools/blender/props/house/wall_hall.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

L, T, H, OPEN_H = 32.0, 0.6, 11.5, 9.3
SOLID = [(-16.0, -12.5), (-3.5, 1.5), (10.5, 16.0)]
OPENINGS = [(-12.5, -3.5), (1.5, 10.5)]
core.reset()
materials('plaster', 'paint_white', 'oak', 'mirror', 'brass', 'book_blue', 'book_mustard',
          'book_cream', 'fabric_grey')
white = mi('paint_white')

bm = core.new_bm()
for x0, x1 in SOLID:
    geo.box(bm, V(((x0 + x1) / 2, 0, H / 2)), (x1 - x0, T, H), mi('plaster'))
for x0, x1 in OPENINGS:
    geo.box(bm, V(((x0 + x1) / 2, 0, (OPEN_H + H) / 2)), (x1 - x0 + 0.02, T, H - OPEN_H),
            mi('plaster'))
wall = core.obj_from_bm('wall', bm, bevel=0.04, segments=2)

bm = core.new_bm()
for s in (-1, 1):
    y = s * (T / 2 + 0.03)
    for x0, x1 in SOLID:
        geo.box(bm, V(((x0 + x1) / 2, y, 0.22)), (x1 - x0, 0.06, 0.44), white)
    for x0, x1 in OPENINGS:  # casings around the openings
        for x in (x0 - 0.17, x1 + 0.17):
            geo.box(bm, V((x, y + s * 0.02, OPEN_H / 2)), (0.38, 0.1, OPEN_H), white)
        geo.box(bm, V(((x0 + x1) / 2, y + s * 0.02, OPEN_H + 0.17)), (x1 - x0 + 0.72, 0.1, 0.38),
                white)
for x0, x1 in OPENINGS:  # jamb linings inside the openings
    for x in (x0 + 0.03, x1 - 0.03):
        geo.box(bm, V((x, 0, OPEN_H / 2)), (0.06, T + 0.02, OPEN_H), white)
    geo.box(bm, V(((x0 + x1) / 2, 0, OPEN_H - 0.03)), (x1 - x0, T + 0.02, 0.06), white)
trim = core.obj_from_bm('trim', bm, bevel=0.012, segments=1)

bm = core.new_bm()
# hall side (-y): mirror over the middle pier, coat rail with hooks at the north pier
yh = -T / 2 - 0.05
geo.box(bm, V((-1.0, yh, 6.6)), (2.6, 0.1, 4.2), mi('oak'))
geo.box(bm, V((-1.0, yh - 0.04, 6.6)), (2.2, 0.04, 3.8), mi('mirror'))
geo.box(bm, V((-14.25, yh - 0.03, 7.6)), (3.0, 0.12, 0.3), mi('oak'))
for k in range(4):
    geo.cylinder(bm, V((-15.3 + k * 0.7, yh - 0.08, 7.55)), V((-15.3 + k * 0.7, yh - 0.35, 7.7)),
                 0.05, mi('brass'), n=6)
geo.box(bm, V((-14.6, yh - 0.35, 5.6)), (0.9, 0.5, 3.4), mi('fabric_grey'))  # a hanging coat
geo.box(bm, V((-13.6, yh - 0.3, 6.1)), (0.7, 0.4, 2.6), mi('book_mustard'))  # a raincoat
# living-room side (+y): picture over the middle pier
yl = T / 2 + 0.05
geo.box(bm, V((-1.0, yl, 6.8)), (3.2, 0.1, 2.4), mi('oak'))
geo.box(bm, V((-1.0, yl + 0.03, 6.8)), (2.9, 0.06, 2.1), mi('book_cream'))
geo.box(bm, V((-1.6, yl + 0.065, 6.5)), (1.2, 0.012, 1.0), mi('book_blue'))
geo.box(bm, V((-0.4, yl + 0.065, 7.2)), (1.1, 0.012, 0.7), mi('book_mustard'))
deco = core.obj_from_bm('deco', bm, bevel=0.01, segments=1)

done('wall_hall', [wall, trim, deco], budget=KIT, car=(6.0, 3.0, 180))
