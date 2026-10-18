"""Roller door (decor) for an 18 u wide, 18 u tall opening in the room wall: guide rails with
hazard-striped feet, the drum housing across the top, the slatted curtain raised to 14.5 u with
its bottom bar and handle, a motor box with a push-button station, two yellow bollards.
Origin at the opening's centre on the wall's inner face at floor level; front (+Y) into the room.

    blender -b -P tools/blender/props/workshop/roller_door.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

W, H, OPEN = 18.0, 18.0, 14.5
start('door_slats', 'steel', 'steel_dark', 'hazard', 'safety_yellow', 'paint_grey', 'rubber',
      'aluminium', 'plastic_black', 'plastic_red', 'plastic_green')
bm = core.new_bm()
for sx in (-1, 1):  # guide rails (C-channels) on the wall face
    x = sx * (W / 2 + 0.25)
    geo.box(bm, V((x, 0.3, H / 2 + 0.6)), (0.5, 0.6, H + 1.2), mi('steel'))
    f = geo.box(bm, V((x, 0.32, 1.2)), (0.56, 0.66, 2.4), mi('hazard'))
    box_uv(bm, f, 0.6)
geo.box(bm, V((0, 0.8, H + 1.15)), (W + 1.6, 1.6, 2.3), mi('paint_grey'))         # drum housing
geo.box(bm, V((W / 2 + 1.5, 0.6, H + 0.6)), (1.0, 1.2, 1.4), mi('steel_dark'))     # motor
geo.box(bm, V((W / 2 + 1.6, 0.2, 5.0)), (0.7, 0.4, 1.2), mi('paint_grey'))         # button box
frame = core.obj_from_bm('frame', bm, bevel=0.04, segments=1, angle=40)

bm = core.new_bm()
f = geo.box(bm, V((0, 0.12, OPEN + (H - OPEN) / 2)), (W, 0.12, H - OPEN), mi('door_slats'))
planar_uv(bm, f, (1, 0, 0), (0, 0, 1), 0.33)
geo.box(bm, V((0, 0.14, OPEN - 0.15)), (W, 0.24, 0.3), mi('aluminium'))            # bottom bar
geo.box(bm, V((0, 0.14, OPEN - 0.36)), (W, 0.16, 0.12), mi('rubber'))              # seal
geo.box(bm, V((0, 0.34, OPEN - 0.05)), (1.2, 0.18, 0.12), mi('plastic_black'))     # handle
curtain = core.obj_from_bm('curtain', bm, bevel=0.02, segments=1, angle=40)

bm = core.new_bm()
for k, mat in enumerate(('plastic_green', 'plastic_red', 'plastic_black')):
    geo.cylinder(bm, V((W / 2 + 1.6, 0.4, 5.35 - k * 0.35)), V((W / 2 + 1.6, 0.52, 5.35 - k * 0.35)),
                 0.12, mi(mat), n=10)
for sx in (-1, 1):  # bollards protecting the rails
    c = V((sx * (W / 2 + 1.4), 2.2, 0))
    geo.cylinder(bm, c, c + V((0, 0, 4.0)), 0.5, mi('safety_yellow'), n=14)
    geo.cylinder(bm, c + V((0, 0, 4.0)), c + V((0, 0, 4.25)), 0.45, mi('safety_yellow'), n=14)
    for z in (2.6, 3.2):
        geo.cylinder(bm, c + V((0, 0, z)), c + V((0, 0, z + 0.3)), 0.51, mi('plastic_black'), n=14)
rnd = core.obj_from_bm('round', bm, weighted=False, angle=40)
finish('roller_door', [frame, curtain, rnd], budget=KIT, car=(0.0, 6.0, 180.0))
