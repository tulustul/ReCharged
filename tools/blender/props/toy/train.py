"""Wooden pull-along train (1 m): red engine with a blue boiler, yellow chimney and coloured
wheels, a wagon of blocks and a yellow box wagon, beech chassis, along Y (engine at the front,
+Y). Static.

    blender -b -P tools/blender/props/toy/train.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
beech()
for name, c in (('red', RED), ('blue', BLUE), ('yellow', YELLOW), ('green', GREEN)):
    wood(f'{name}_wood', c, 0.4, contrast=0.07, seed=hash(name) % 97)
plastic('dark', '#1D1D20', 0.4)
mi = core.mi
body, rnd = core.new_bm(), core.new_bm()
CARS = [(1.55, 'engine'), (0.0, 'blocks'), (-1.5, 'box')]
for y, kind in CARS:
    geo.box(body, V((0, y, 0.38)), (0.85, 1.3, 0.22), mi('beech'))                  # chassis
    for dy in (-0.42, 0.42):
        for s in (-1, 1):
            geo.cylinder(rnd, V((s * 0.43, y + dy, 0.24)), V((s * 0.55, y + dy, 0.24)), 0.24,
                         mi('yellow_wood' if kind == 'engine' else 'red_wood'), n=16)
            geo.cylinder(rnd, V((s * 0.55, y + dy, 0.24)), V((s * 0.58, y + dy, 0.24)), 0.07,
                         mi('dark'), n=8)
    if kind == 'engine':
        geo.box(body, V((0, y - 0.35, 0.92)), (0.8, 0.6, 0.86), mi('red_wood'))       # cab
        geo.box(body, V((0, y - 0.35, 1.4)), (0.95, 0.75, 0.1), mi('beech'))         # roof
        geo.cylinder(rnd, V((0, y - 0.05, 0.8)), V((0, y + 0.62, 0.8)), 0.33, mi('blue_wood'),
                     n=18)
        geo.cylinder(rnd, V((0, y + 0.4, 1.1)), V((0, y + 0.4, 1.45)), 0.12, mi('yellow_wood'),
                     n=12, r1=0.17)
        geo.sphere(rnd, V((0, y + 0.66, 0.8)), 0.12, mi('dark'), n=10, rings=5)
    elif kind == 'blocks':
        for k, (dx, dy, c) in enumerate(((-0.18, -0.3, 'red'), (0.18, -0.3, 'yellow'),
                                         (0.0, 0.25, 'green'), (0.0, -0.3, 'blue'))):
            z = 0.69 if k < 3 else 1.05
            geo.box(body, V((dx, y + dy, z)), (0.36, 0.36, 0.36), mi(f'{c}_wood'))
    else:
        geo.box(body, V((0, y, 0.85)), (0.8, 1.1, 0.75), mi('yellow_wood'))
        geo.box(body, V((0, y, 1.27)), (0.9, 1.2, 0.08), mi('red_wood'))
    if y > -1.4:  # coupling peg to the next car
        geo.cylinder(rnd, V((0, y - 0.65, 0.38)), V((0, y - 0.85, 0.38)), 0.05, mi('dark'), n=6)
geo.tube(rnd, [V((0, 2.25, 0.4)), V((0.3, 2.9, 0.06)), V((0.9, 3.4, 0.03))], 0.025, mi('red_wood'),
         n=5)  # pull string
propkit.finish(ENV, 'train', [part('body', box_uv(body), bevel=0.03, segments=2),
                              smooth('round', box_uv(rnd))], car=(1.9, 0.8, 180))
