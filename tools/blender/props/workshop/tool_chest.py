"""Red tool chest (hero): a roll cab on four swivel casters with a top chest, 1.4 x 0.65 x 1.5 m
(6.2 x 2.9 x 6.8 u): drawer fronts with full-width aluminium pulls toward +Y (Bevy -Z), a lid
with a chrome handle, a push handle on the side, a rubber mat on the cab top.

    blender -b -P tools/blender/props/workshop/tool_chest.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

CH = C.CHEST
W, D = CH['w'], CH['d']
CAST = 0.55            # caster height (the cab floats above it)
CAB_H = 3.75           # roll cab body height
TOP_D, TOP_H = 2.45, 2.25
LIP = 0.06             # drawer front reveal

start('chest_red', 'plastic_black', 'aluminium', 'chrome', 'rubber', 'steel_dark')
red = mi('chest_red')

bm = core.new_bm()
z0 = CAST
geo.box(bm, V((0, 0, z0 + CAB_H / 2)), (W, D, CAB_H), red)                     # roll cab
geo.box(bm, V((0, -0.12, z0 + CAB_H + TOP_H / 2 + 0.06)), (W - 0.1, TOP_D, TOP_H), red)
geo.box(bm, V((0, -0.12, z0 + CAB_H + TOP_H + 0.12)), (W - 0.06, TOP_D + 0.06, 0.12), red)  # lid
body = core.obj_from_bm('body', bm, bevel=0.04, segments=2, angle=40)

bm = core.new_bm()
fy = D / 2 + LIP / 2
# roll cab: left column 5 drawers, right column 3 drawers + a deep bottom drawer
cols = [(-W / 4 - 0.02, [0.5, 0.5, 0.62, 0.75, 1.1]), (W / 4 + 0.02, [0.5, 0.75, 0.92, 1.25])]
for cx, heights in cols:
    z = z0 + CAB_H - 0.12
    for h in heights:
        zc = z - h / 2
        geo.box(bm, V((cx, fy, zc)), (W / 2 - 0.22, LIP, h - 0.08), red)
        geo.box(bm, V((cx, fy + 0.07, zc + h / 2 - 0.16)), (W / 2 - 0.5, 0.08, 0.1),
                mi('aluminium'))
        z -= h
# top chest: 6 shallow drawers across two columns + a till under the lid
tfy = -0.12 + TOP_D / 2 + LIP / 2
for cx in (-W / 4 - 0.02, W / 4 + 0.02):
    z = z0 + CAB_H + TOP_H - 0.1
    for h in (0.36, 0.36, 0.45, 0.55):
        zc = z - h / 2
        geo.box(bm, V((cx, tfy, zc)), (W / 2 - 0.22, LIP, h - 0.07), red)
        geo.box(bm, V((cx, tfy + 0.06, zc + h / 2 - 0.12)), (W / 2 - 0.5, 0.07, 0.08),
                mi('aluminium'))
        z -= h
fronts = core.obj_from_bm('fronts', bm, bevel=0.015, segments=1, angle=40)

bm = core.new_bm()
geo.box(bm, V((0, 0, z0 + CAB_H + 0.03)), (W - 0.2, D - 0.2, 0.05), mi('rubber'))   # mat
geo.box(bm, V((0, 0, z0 - 0.06)), (W - 0.2, D - 0.2, 0.12), mi('steel_dark'))       # base
for sx in (-1, 1):  # caster forks
    for sy in (-1, 1):
        geo.box(bm, V((sx * (W / 2 - 0.45), sy * (D / 2 - 0.4), CAST - 0.14)), (0.4, 0.4, 0.16),
                mi('steel_dark'))
hard = core.obj_from_bm('hard', bm, bevel=0.02, segments=1, angle=40)

bm = core.new_bm()
for sx in (-1, 1):
    for sy in (-1, 1):
        c = V((sx * (W / 2 - 0.45), sy * (D / 2 - 0.4) + 0.08, 0.26))
        geo.cylinder(bm, c - V((0.1, 0, 0)), c + V((0.1, 0, 0)), 0.25, mi('plastic_black'), n=12)
# push handle on the right side and the lid handle
hx = W / 2 + 0.35
geo.tube(bm, [V((W / 2, -0.9, z0 + CAB_H - 0.5)), V((hx, -0.9, z0 + CAB_H - 0.5)),
              V((hx, 0.9, z0 + CAB_H - 0.5)), V((W / 2, 0.9, z0 + CAB_H - 0.5))], 0.07,
         mi('chrome'), n=8, corner=0.18, cseg=2)
ly = -0.12 + TOP_D / 2 + 0.03
geo.tube(bm, [V((-1.3, ly, z0 + CAB_H + TOP_H + 0.1)), V((-1.3, ly + 0.2, z0 + CAB_H + TOP_H + 0.1)),
              V((1.3, ly + 0.2, z0 + CAB_H + TOP_H + 0.1)), V((1.3, ly, z0 + CAB_H + TOP_H + 0.1))],
         0.05, mi('chrome'), n=6, corner=0.08, cseg=2)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

finish('tool_chest', [body, fronts, hard, round_], budget=HERO, car=(-1.5, 4.5, 200.0))
extra_shots('tool_chest', [('rc', (2.2, 13.0, 1.0), (0.0, 0.0, 3.0), 28)])
