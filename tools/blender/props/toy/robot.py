"""Retro tin toy robot (67 cm), red with chrome details, waving a checkered flag in its raised
right hand: the race starter. Faces +Y. Static.

    blender -b -P tools/blender/props/toy/robot.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
painted('tin', '#C93A2E', 0.3)
painted('panel', '#2F5FB8', 0.3)
painted('lamp', YELLOW, 0.2)
core.material('chrome', '#E4E6EA', 1.0, 0.18)
plastic('black', '#1D1D20', 0.35)
plastic('white', '#F2F0EA', 0.35)
mi = core.mi
tin, chrome, round_ = core.new_bm(), core.new_bm(), core.new_bm()
for s in (-1, 1):
    geo.box(tin, V((s * 0.38, 0.05, 0.15)), (0.55, 0.85, 0.3), mi('tin'))            # feet
    geo.cylinder(chrome, V((s * 0.38, 0, 0.3)), V((s * 0.38, 0, 1.0)), 0.16, mi('chrome'), n=12)
geo.box(tin, V((0, 0, 1.65)), (1.4, 0.9, 1.3), mi('tin'))                            # body
geo.box(tin, V((0, 0.46, 1.7)), (0.9, 0.04, 0.8), mi('panel'))                       # chest
for k, x in enumerate((-0.25, 0.0, 0.25)):
    geo.cylinder(round_, V((x, 0.47, 1.85)), V((x, 0.55, 1.85)), 0.08,
                 mi(('lamp', 'white', 'lamp')[k]), n=12)
geo.box(chrome, V((0, 0.48, 1.5)), (0.6, 0.04, 0.22), mi('chrome'))                  # grille
geo.cylinder(chrome, V((0, 0, 2.3)), V((0, 0, 2.45)), 0.25, mi('chrome'), n=16)      # neck
geo.box(tin, V((0, 0, 2.85)), (0.95, 0.8, 0.75), mi('tin'))                          # head
for s in (-1, 1):
    geo.cylinder(round_, V((s * 0.22, 0.4, 2.95)), V((s * 0.22, 0.47, 2.95)), 0.14, mi('white'),
                 n=16)
    geo.cylinder(round_, V((s * 0.22, 0.46, 2.95)), V((s * 0.22, 0.5, 2.95)), 0.07, mi('black'),
                 n=12)
    geo.cylinder(chrome, V((s * 0.48, 0, 2.85)), V((s * 0.56, 0, 2.85)), 0.15, mi('chrome'), n=12)
geo.box(chrome, V((0, 0.41, 2.65)), (0.45, 0.03, 0.12), mi('chrome'))                # mouth
geo.cylinder(chrome, V((0, 0, 3.22)), V((0, 0, 3.6)), 0.03, mi('chrome'), n=6)       # antenna
geo.sphere(round_, V((0, 0, 3.65)), 0.08, mi('lamp'), n=10, rings=6)
# arms: left down, right raised holding the flag
geo.tube(chrome, [V((-0.75, 0, 2.1)), V((-0.95, 0.1, 1.6)), V((-0.95, 0.25, 1.15))], 0.12,
         mi('chrome'), n=10, corner=0.2)
geo.tube(chrome, [V((0.75, 0, 2.1)), V((1.05, 0.05, 2.6)), V((1.1, 0.15, 3.1))], 0.12,
         mi('chrome'), n=10, corner=0.2)
for p in (V((-0.95, 0.25, 1.1)), V((1.1, 0.15, 3.15))):
    geo.sphere(round_, p, 0.17, mi('tin'), n=12, rings=6)
pole0, pole1 = V((1.1, 0.15, 2.7)), V((1.1, 0.15, 4.4))
geo.cylinder(chrome, pole0, pole1, 0.035, mi('black'), n=6)
flag = core.new_bm()
for i in range(4):
    for j in range(3):
        geo.box(flag, V((1.1 + 0.16 + 0.28 * i, 0.15, 4.25 - 0.26 * j)), (0.28, 0.03, 0.26),
                mi('black' if (i + j) % 2 else 'white'))
propkit.finish(ENV, 'robot', [part('tin', tin, bevel=0.04, segments=2), smooth('chrome', chrome),
                              smooth('round', round_), part('flag', flag, bevel=0.0)],
               car=(2.4, 1.8, 200))
