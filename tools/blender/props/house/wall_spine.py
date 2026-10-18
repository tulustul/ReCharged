"""Spine wall: the interior wall between the front half (hall, living room) and the back half
(kitchen-diner), 78 u long (x -39..39), 0.6 u thick, 11.5 u to the ceiling. Greige plaster,
white skirting both sides, a closed panelled kitchen door with casings (hall end), light
switches, framed pictures and kids' drawings, a radiator, a wall clock on the kitchen side; the
east end has a rounded corner. Front (+Y) = the kitchen side.

    blender -b -P tools/blender/props/house/wall_spine.py -- [--preview DIR]
"""
import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

L, T, H = 78.0, 0.6, 11.5
DOOR_X = -2.0                       # world x -20 (hall)
rnd = random.Random(8)
core.reset()
materials('plaster', 'paint_white', 'brass', 'oak', 'walnut', 'book_red', 'book_blue',
          'book_mustard', 'book_teal', 'book_cream', 'metal_black', 'ceramic')
white = mi('paint_white')

bm = core.new_bm()
geo.box(bm, V((0, 0, H / 2)), (L, T, H), mi('plaster'))
wall = core.obj_from_bm('wall', bm, bevel=0.06, segments=2)

# skirting both faces, door casings + leaf (both faces), switches
bm = core.new_bm()
for s in (-1, 1):
    y = s * (T / 2 + 0.03)
    for x0, x1 in ((-L / 2, DOOR_X - 2.35), (DOOR_X + 2.35, L / 2)):
        geo.box(bm, V(((x0 + x1) / 2, y, 0.22)), (x1 - x0, 0.06, 0.44), white)
    for dx in (-2.15, 2.15):  # casings
        geo.box(bm, V((DOOR_X + dx, y + s * 0.02, 4.8)), (0.38, 0.1, 9.6), white)
    geo.box(bm, V((DOOR_X, y + s * 0.02, 9.45)), (4.68, 0.1, 0.38), white)
    geo.box(bm, V((DOOR_X - 2.9, y, 4.8)), (0.32, 0.04, 0.42), white)  # light switch
geo.box(bm, V((DOOR_X, 0, 4.63)), (3.92, 0.18, 9.25), white)  # door leaf
for s in (-1, 1):
    for zc, h in ((2.4, 3.6), (7.0, 3.4)):  # raised panels
        for dx in (-0.95, 0.95):
            geo.box(bm, V((DOOR_X + dx, s * 0.11, zc)), (1.45, 0.05, h), white)
trim = core.obj_from_bm('trim', bm, bevel=0.012, segments=1)

bm = core.new_bm()
for s in (-1, 1):  # brass lever handles
    y = s * 0.12
    geo.cylinder(bm, V((DOOR_X + 1.55, y, 4.4)), V((DOOR_X + 1.55, y + s * 0.18, 4.4)), 0.12,
                 mi('brass'), n=12)
    geo.cylinder(bm, V((DOOR_X + 1.55, y + s * 0.18, 4.4)), V((DOOR_X + 0.85, y + s * 0.22, 4.4)),
                 0.05, mi('brass'), n=8)
handles = core.obj_from_bm('handles', bm, weighted=False)

# pictures: (x, z, w, h, side, frame, palette)
PICS = [(-30, 6.6, 3.0, 2.2, -1, 'oak'), (-25.5, 7.2, 2.0, 2.6, -1, 'metal_black'),
        (-12, 6.8, 2.6, 2.0, -1, 'walnut'), (6, 6.4, 4.4, 3.0, -1, 'oak'),
        (14.5, 6.9, 2.2, 2.8, -1, 'metal_black'), (-20, 6.5, 2.4, 1.8, 1, 'oak'),
        (24, 6.6, 3.2, 2.4, -1, 'walnut')]
bm = core.new_bm()
for x, z, w, h, s, fr in PICS:
    y = s * (T / 2 + 0.05)
    geo.box(bm, V((x, y, z)), (w, 0.1, h), mi(fr))
    geo.box(bm, V((x, y + s * 0.03, z)), (w - 0.3, 0.06, h - 0.3), mi('book_cream'))
    for k in range(3):  # abstract colour blocks
        cw, ch = rnd.uniform(0.3, 0.6) * (w - 0.5), rnd.uniform(0.25, 0.55) * (h - 0.5)
        cx = x + rnd.uniform(-1, 1) * (w - 0.5 - cw) / 2
        cz = z + rnd.uniform(-1, 1) * (h - 0.5 - ch) / 2
        geo.box(bm, V((cx, y + s * 0.065, cz)), (cw, 0.012, ch),
                mi(rnd.choice(['book_red', 'book_blue', 'book_mustard', 'book_teal'])))
# kitchen side: wall clock + kids' drawings
geo.cylinder(bm, V((-34, T / 2, 7.4)), V((-34, T / 2 + 0.12, 7.4)), 0.9, mi('ceramic'), n=24)
geo.cylinder(bm, V((-34, T / 2 + 0.1, 7.4)), V((-34, T / 2 + 0.15, 7.4)), 0.95, mi('metal_black'),
             n=24, r1=0.88)
for a, ln in ((math.radians(60), 0.55), (math.radians(-30), 0.75)):
    geo.box(bm, V((-34 + ln / 2 * math.cos(a), T / 2 + 0.17, 7.4 + ln / 2 * math.sin(a))),
            (ln, 0.02, 0.06), mi('metal_black'))
for i, x in enumerate((-8, -6.2, -4.5)):
    geo.box(bm, V((x, T / 2 + 0.01, 3.4 + 0.3 * (i % 2))), (1.4, 0.01, 1.0), mi('book_cream'))
    geo.box(bm, V((x - 0.2, T / 2 + 0.02, 3.5 + 0.3 * (i % 2))), (0.5, 0.01, 0.4),
            mi(['book_red', 'book_teal', 'book_mustard'][i]))
art = core.obj_from_bm('art', bm, bevel=0.01, segments=1)

# white panel radiator in the hall under the first pictures
bm = core.new_bm()
geo.box(bm, V((-30, -T / 2 - 0.25, 1.8)), (5.0, 0.3, 2.4), white)
for k in range(16):
    geo.box(bm, V((-32.35 + k * 0.313, -T / 2 - 0.42, 1.8)), (0.16, 0.06, 2.3), white)
radiator = core.obj_from_bm('radiator', bm, bevel=0.012, segments=1)

done('wall_spine', [wall, trim, handles, art, radiator], budget=KIT, car=(-2.0, -3.0, 160))
