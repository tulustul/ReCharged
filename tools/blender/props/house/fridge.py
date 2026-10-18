"""Fridge-freezer: brushed-steel two-door fridge, 90 x 65 x 190 cm (4.0 x 2.9 x 8.4 u), long bar
handles; magnets hold the kids' drawings and a shopping list. Front (+Y).

    blender -b -P tools/blender/props/house/fridge.py -- [--preview DIR]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

W, D, H = 4.0, 2.9, 8.4
rnd = random.Random(2)
core.reset()
materials('steel', 'plastic_black', 'book_cream', 'book_red', 'book_blue', 'book_mustard',
          'book_teal', 'leaf')

bm = core.new_bm()
geo.box(bm, V((0, -0.05, H / 2)), (W, D - 0.1, H), mi('steel'))
body = core.obj_from_bm('body', bm, bevel=0.08, segments=3)

bm = core.new_bm()
fy = D / 2 - 0.05
geo.box(bm, V((0, fy, 2.0)), (W - 0.06, 0.1, 3.9), mi('steel'))  # freezer door
geo.box(bm, V((0, fy, 6.2)), (W - 0.06, 0.1, 4.3), mi('steel'))  # fridge door
geo.box(bm, V((0, fy - 0.01, 4.0)), (W, 0.06, 0.08), mi('plastic_black'))  # split line
for z0, z1 in ((0.8, 3.4), (4.6, 7.8)):
    geo.box(bm, V((W / 2 - 0.35, fy + 0.25, (z0 + z1) / 2)), (0.12, 0.12, z1 - z0), mi('steel'))
    for z in (z0, z1):
        geo.box(bm, V((W / 2 - 0.35, fy + 0.12, z)), (0.1, 0.22, 0.12), mi('steel'))
geo.box(bm, V((0, fy - 0.12, 0.2)), (W - 0.4, 0.06, 0.25), mi('plastic_black'))  # vent
doors = core.obj_from_bm('doors', bm, bevel=0.03, segments=2)

bm = core.new_bm()
for k, (x, z, w, h) in enumerate([(-0.9, 6.6, 1.5, 1.1), (0.5, 7.2, 1.0, 1.3), (-0.6, 5.0, 1.2, 0.9),
                                  (0.7, 2.4, 1.1, 1.5), (-0.8, 1.9, 1.3, 1.0)]):
    yy = fy + 0.06
    geo.box(bm, V((x, yy, z)), (w, 0.01, h), mi('book_cream'))  # paper
    for j in range(2):  # crayon blobs
        geo.box(bm, V((x + rnd.uniform(-0.3, 0.3) * w, yy + 0.008, z + rnd.uniform(-0.25, 0.25) * h)),
                (rnd.uniform(0.2, 0.45), 0.006, rnd.uniform(0.12, 0.3)),
                mi(rnd.choice(['book_red', 'book_blue', 'book_mustard', 'book_teal', 'leaf'])))
    geo.cylinder(bm, V((x, yy + 0.005, z + h / 2 - 0.08)), V((x, yy + 0.06, z + h / 2 - 0.08)), 0.09,
                 mi(rnd.choice(['book_red', 'book_blue', 'book_mustard'])), n=8)  # magnet
art = core.obj_from_bm('art', bm, weighted=False)

done('fridge', [body, doors, art], car=(3.4, 2.2, 200))
