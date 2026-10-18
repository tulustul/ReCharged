"""Wash bay (hero): white-tiled splash walls on both sides of the lane (11.6 u apart, 2.6 u tall,
stainless cap rails, hazard bumpers on the ends), a blue gantry over the middle with spray bars
and nozzles, a coiled pressure-washer hose and lance on the gantry post, a red pressure washer on
a cart behind the right wall, a grated trench drain across the floor. Long axis = Y (Bevy z).
The floor itself is the track's Water ground polygon.

    blender -b -P tools/blender/props/workshop/wash_bay.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

WA = C.WASH
HL, HI, WH, WT = WA['length'] / 2, WA['inner'] / 2, WA['wall_h'], WA['wall_t']
PX, PS, BY = WA['post_x'], WA['post'], WA['beam_y']

start('tile_white', 'steel', 'lift_blue', 'hazard', 'steel_dark', 'plastic_black', 'plastic_red',
      'safety_yellow', 'chrome', 'light_panel', 'rubber', 'plastic_blue', 'galvanised')

# --- splash walls: tiled faces, stainless caps, hazard bumpers on the ends --------------------
bm = core.new_bm()
for sx in (-1, 1):
    x = sx * (HI + WT / 2)
    f = geo.box(bm, V((x, 0, WH / 2)), (WT, 2 * HL, WH), mi('tile_white'))
    box_uv(bm, f, 0.375)
wall = core.obj_from_bm('walls', bm, bevel=0.03, segments=1, angle=40)

bm = core.new_bm()
for sx in (-1, 1):
    x = sx * (HI + WT / 2)
    geo.box(bm, V((x, 0, WH + 0.06)), (WT + 0.12, 2 * HL + 0.1, 0.12), mi('steel'))
    for sy in (-1, 1):
        f = geo.box(bm, V((x, sy * (HL + 0.12), WH / 2)), (WT + 0.16, 0.24, WH), mi('hazard'))
        box_uv(bm, f, 0.6)
# gantry: posts, beam, a light bar under the beam
for sx in (-1, 1):
    geo.box(bm, V((sx * PX, 0, BY / 2)), (PS, PS, BY), mi('lift_blue'))
    geo.box(bm, V((sx * PX, 0, 0.05)), (PS + 0.5, PS + 0.5, 0.1), mi('steel_dark'))
geo.box(bm, V((0, 0, BY + 0.4)), (2 * PX + PS, PS + 0.1, 0.8), mi('lift_blue'))
geo.box(bm, V((0, 0, BY - 0.05)), (2 * PX - PS - 0.6, 0.35, 0.1), mi('light_panel'))
# pressure washer on a cart behind the right wall
cx = HI + WT + 1.6
geo.box(bm, V((cx, 4.5, 1.3)), (1.7, 2.4, 1.6), mi('plastic_red'))
geo.box(bm, V((cx, 4.5, 2.2)), (1.5, 2.0, 0.25), mi('plastic_black'))
geo.box(bm, V((cx - 0.6, 4.5, 0.25)), (0.35, 2.6, 0.3), mi('steel_dark'))
hard = core.obj_from_bm('hard', bm, bevel=0.03, segments=1, angle=40)

bm = core.new_bm()
# spray bars along the beam and down both posts, with nozzle stubs
geo.cylinder(bm, V((-PX + 0.6, -0.55, BY - 0.25)), V((PX - 0.6, -0.55, BY - 0.25)), 0.11,
             mi('galvanised'), n=8)
for k in range(9):
    x = -PX + 1.4 + k * (2 * PX - 2.8) / 8
    geo.cylinder(bm, V((x, -0.55, BY - 0.36)), V((x, -0.55, BY - 0.55)), 0.06, mi('plastic_black'),
                 n=6)
for sx in (-1, 1):
    x = sx * (PX - 0.5)
    geo.cylinder(bm, V((x, -0.55, 0.9)), V((x, -0.55, BY - 0.25)), 0.11, mi('galvanised'), n=8)
    for k in range(5):
        z = 1.6 + k * 1.4
        geo.cylinder(bm, V((x, -0.55, z)), V((x - sx * 0.22, -0.55, z)), 0.06, mi('plastic_black'),
                     n=6)
# coiled hose on the left post with the lance hooked beside it
px = -PX - PS / 2 - 0.25
for k in range(5):
    r = 0.75 + 0.04 * k
    pts = [V((px - 0.08 * k, 0.45 + r * math.cos(a), 5.0 + r * math.sin(a)))
           for a in [2 * math.pi * i / 16 for i in range(17)]]
    geo.tube(bm, pts, 0.07, mi('plastic_black'), n=6)
geo.cylinder(bm, V((px - 0.2, -0.7, 3.2)), V((px - 0.2, -0.7, 6.4)), 0.06, mi('chrome'), n=6)
geo.cylinder(bm, V((px - 0.2, -0.7, 2.9)), V((px - 0.2, -0.7, 3.4)), 0.13, mi('plastic_blue'), n=8)
# hose from the washer over the wall to the gantry
geo.tube(bm, [V((cx, 4.5, 2.3)), V((cx - 0.8, 3.0, WH + 0.6)), V((HI, 1.5, WH + 0.3)),
              V((PX - 0.2, 0.6, 1.2)), V((PX - 0.2, 0.3, 0.9))], 0.08, mi('plastic_black'), n=6,
         corner=0.8, cseg=3)
for wy in (3.6, 5.4):  # washer cart wheels
    geo.cylinder(bm, V((cx + 0.6, wy, 0.4)), V((cx + 1.0, wy, 0.4)), 0.4, mi('rubber'), n=12)
# grated trench drain across the floor (flush, visual)
geo.box(bm, V((0, -3.0, 0.012)), (2 * HI - 0.2, 1.1, 0.024), mi('steel_dark'))
for k in range(18):
    geo.box(bm, V((-HI + 0.6 + k * (2 * HI - 1.2) / 17, -3.0, 0.03)), (0.08, 1.0, 0.03),
            mi('galvanised'))
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

finish('wash_bay', [wall, hard, round_], budget=HERO, car=(0.0, HL + 3.0, 180.0))
extra_shots('wash_bay', [('rc_entry', (1.5, HL + 14.0, 1.0), (0.0, 0.0, 3.0), 26)])
