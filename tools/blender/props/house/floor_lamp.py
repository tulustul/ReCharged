"""Floor lamp: weighted black base (31 cm), slim black stem with a brass collar, cream linen
drum shade (50 cm) with a warm bulb inside; 1.6 m tall (7.1 u). Light attachment in the catalog
at the bulb (0, 6.2, 0).

    blender -b -P tools/blender/props/house/floor_lamp.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

core.reset()
materials('metal_black', 'brass', 'lamp_shade', 'bulb')

bm = core.new_bm()
geo.cylinder(bm, V((0, 0, 0)), V((0, 0, 0.12)), 0.7, mi('metal_black'), n=28, r1=0.66)
geo.cylinder(bm, V((0, 0, 0.12)), V((0, 0, 5.9)), 0.07, mi('metal_black'), n=12, r1=0.06)
geo.cylinder(bm, V((0, 0, 2.9)), V((0, 0, 3.05)), 0.1, mi('brass'), n=12)
geo.cylinder(bm, V((0, 0, 5.9)), V((0, 0, 6.05)), 0.16, mi('brass'), n=12)
geo.sphere(bm, V((0, 0, 6.25)), 0.22, mi('bulb'), n=12, rings=8)
metal = core.obj_from_bm('metal', bm, weighted=False, angle=40)

bm = core.new_bm()  # open drum shade: outer + inner wall (no caps)
n = 32
for r, z0, z1 in ((1.1, 5.55, 7.05), (1.06, 5.57, 7.03)):
    rings = [[V((rr * math.cos(2 * math.pi * k / n), rr * math.sin(2 * math.pi * k / n), z))
              for k in range(n)] for rr, z in ((r, z0), (r * 0.96, z1))]
    geo._bridge(bm, rings, mi('lamp_shade'), cap0=False, cap1=False)
for k in range(4):  # spider arms to the collar
    a = math.pi / 2 * k + math.pi / 4
    geo.cylinder(bm, V((0, 0, 6.6)), V((1.0 * math.cos(a), 1.0 * math.sin(a), 6.62)), 0.02,
                 mi('brass'), n=5)
shade = core.obj_from_bm('shade', box_uv(bm, 'z'), weighted=False, angle=50)

done('floor_lamp', [metal, shade], car=(2.0, 0.8, 200))
