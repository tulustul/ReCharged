"""Staircase (hero): the hall stairs. Two steps up to a 5 x 4.8 u landing (2.3 u = 52 cm), then
twelve steps (17 cm risers, 27 cm treads) up to the first floor at the ceiling (11.5 u). Oak
treads with nosings, white risers and stringers, a striped runner on the upper flight, white
spindle balustrades with an oak handrail; the space under the upper flight is a panelled
cupboard with a door. The landing sides stay open: the plank shortcut drops off them.
Front (+Y) = the way up. Length 20.6 u (y -10.3 .. 10.3), width 4.8 u.

    blender -b -P tools/blender/props/house/staircase.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

HW = 2.4                          # half width
RISE, TREAD = 11.5 / 15, 1.2
Y0 = -10.3                        # bottom riser
LAND0, LAND1 = Y0 + 2 * TREAD, Y0 + 2 * TREAD + 5.0   # landing (y)
LAND_Z = 3 * RISE
NOSE, T_TREAD = 0.1, 0.12
core.reset()
materials('oak', 'paint_white', 'fabric_cream', 'fabric_blue', 'brass', 'oak_end')
oak, white = mi('oak'), mi('paint_white')

steps = []  # (y_front, y_back, z_top)
for k in range(2):
    steps.append((Y0 + k * TREAD, Y0 + (k + 1) * TREAD, (k + 1) * RISE))
steps.append((LAND0, LAND1, LAND_Z))
for k in range(1, 12):
    steps.append((LAND1 + (k - 1) * TREAD, LAND1 + k * TREAD, LAND_Z + k * RISE))
TOP_Y = LAND1 + 11 * TREAD

# treads (oak, with nosing) and risers (white)
bm = core.new_bm()
for y0, y1, z in steps:
    geo.box(bm, V((0, (y0 + y1) / 2 - NOSE / 2, z - T_TREAD / 2)), (2 * HW, y1 - y0 + NOSE,
                                                                     T_TREAD), oak)
treads = core.obj_from_bm('treads', box_uv(bm, 'x'), bevel=0.03, segments=2)
bm = core.new_bm()
prev_z = 0.0
for y0, y1, z in steps:
    geo.box(bm, V((0, y0 + 0.04, (prev_z + z - T_TREAD) / 2)), (2 * HW - 0.04, 0.08,
                                                               z - T_TREAD - prev_z), white)
    prev_z = z
# solid body under the steps: landing block and the cupboard under the upper flight (closed)
geo.box(bm, V((0, (Y0 + LAND1) / 2 + 0.04, LAND_Z / 2 - 0.06)), (2 * HW - 0.06, LAND1 - Y0 - 0.1,
                                                                 LAND_Z - 0.12), white)
risers = core.obj_from_bm('risers', bm, bevel=0.01, segments=1)

# stringers / side panels: a sloped white panel each side from the landing to the top, closed
# down to the floor under the flight (cupboard walls); the landing and bottom steps have plain
# white sides.
bm = core.new_bm()
for s in (-1, 1):
    x = s * (HW + 0.05)
    poly = [(Y0, 0.0), (Y0, RISE - T_TREAD), (Y0 + TREAD, RISE - T_TREAD),
            (Y0 + TREAD, 2 * RISE - T_TREAD), (LAND0, 2 * RISE - T_TREAD),
            (LAND0, LAND_Z - T_TREAD), (LAND1, LAND_Z - T_TREAD),
            (TOP_Y, 11.5 - T_TREAD + 0.35), (TOP_Y, 0.0)]
    geo.extrude_x(bm, [(y, z) for y, z in poly], x - 0.05, x + 0.05, white)
geo.box(bm, V((0, TOP_Y - 0.05, 11.5 / 2)), (2 * HW + 0.2, 0.1, 11.5), white)  # closed top end
sides = core.obj_from_bm('sides', bm, bevel=0.012, segments=1, angle=30)

# cupboard door (+x side, facing the A straight) with a brass knob
bm = core.new_bm()
dx, dy = HW + 0.1, LAND1 + 4.2
geo.box(bm, V((dx + 0.03, dy, 1.8)), (0.06, 2.9, 3.6), mi('oak_end'))  # frame (shadow gap)
geo.box(bm, V((dx + 0.07, dy, 1.75)), (0.06, 2.6, 3.4), white)  # door leaf
for zz in (0.95, 2.6):  # raised panels
    geo.box(bm, V((dx + 0.11, dy, zz)), (0.04, 2.0, 1.3), white)
geo.cylinder(bm, V((dx + 0.1, dy - 1.0, 1.75)), V((dx + 0.26, dy - 1.0, 1.75)), 0.08,
             mi('brass'), n=10)
door = core.obj_from_bm('door', bm, bevel=0.008, segments=1)

# striped runner on the upper flight
bm = core.new_bm()
for y0, y1, z in steps[3:]:
    geo.box(bm, V((0, (y0 + y1) / 2 - NOSE / 2, z + 0.012)), (2.6, y1 - y0 + NOSE - 0.02, 0.025),
            mi('fabric_cream'))
    for sx in (-1, 1):
        geo.box(bm, V((sx * 1.05, (y0 + y1) / 2 - NOSE / 2, z + 0.028)), (0.16, y1 - y0 + NOSE - 0.03,
                                                                         0.01), mi('fabric_blue'))
runner = core.obj_from_bm('runner', box_uv(bm, 'y'), bevel=0.006, segments=1)

# balustrades on the upper flight: newel posts, spindles, oak handrail
bm = core.new_bm()
pitch_z = RISE / TREAD
for s in (-1, 1):
    x = s * (HW - 0.12)
    py, pz = LAND1 + 0.15, LAND_Z
    geo.box(bm, V((x, py, pz + 1.9)), (0.26, 0.26, 3.8), white)  # newel post
    geo.box(bm, V((x, py, pz + 3.86)), (0.32, 0.32, 0.1), oak)
    end_y = LAND1 + (11.5 - 3.55 - LAND_Z) / pitch_z  # the handrail meets the ceiling here
    n = 15
    for i in range(1, n + 1):
        y = py + (end_y - py) * i / (n + 0.4)
        zb = LAND_Z + max(0.0, (y - LAND1)) * pitch_z
        geo.box(bm, V((x, y, zb + 1.7)), (0.08, 0.08, 3.4), white)
    a = V((x, py, LAND_Z + 3.55))
    b = V((x, end_y + 0.6, LAND_Z + 3.55 + (end_y + 0.6 - LAND1) * pitch_z))
    geo.prism(bm, geo.rrect(0.2, 0.14, 0.05, 2), a, b, oak, up=V((0, 0, 1)))
rail = core.obj_from_bm('rail', box_uv(bm, 'z'), bevel=0.01, segments=1)

done('staircase', [treads, risers, sides, door, runner, rail], budget=HERO,
     car=(4.2, -12.0, 20))
