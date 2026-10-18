"""Kitchen island: navy shaker cabinets under a pale quartz top, 2.6 x 1.0 m, 90 cm high
(11.5 x 4.4 x 4.0 u); the top overhangs 1.2 u at the front (+Y) where two oak-and-black bar
stools are tucked in. A fruit bowl and a cookbook on the top. Collider: base + top + stools.

    blender -b -P tools/blender/props/house/kitchen_island.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

L, H, TOP = 11.5, 4.0, 0.2
BACK, FRONT = -2.2, 1.0            # cabinet depth in y; the top runs to +2.2
core.reset()
materials('paint_navy', 'worktop', 'brass', 'oak', 'metal_black', 'plastic_black', 'ceramic',
          'fabric_mustard', 'book_red', 'pages')
navy, brass = mi('paint_navy'), mi('brass')

bm = core.new_bm()
geo.box(bm, V((0, (BACK + FRONT) / 2 + 0.1, 0.24)), (L - 0.3, FRONT - BACK - 0.5, 0.48),
        mi('plastic_black'))
geo.box(bm, V((0, (BACK + FRONT) / 2, (0.48 + H - TOP) / 2)), (L - 0.1, FRONT - BACK,
                                                                H - TOP - 0.48), navy)
body = core.obj_from_bm('body', bm, bevel=0.02, segments=1)

bm = core.new_bm()
unit = L / 4
for i in range(4):  # shaker doors on the back (C side)
    x = -L / 2 + unit * (i + 0.5)
    y = BACK - 0.04
    geo.box(bm, V((x, y, 1.95)), (unit - 0.06, 0.08, 2.9), navy)
    geo.box(bm, V((x, y - 0.03, 1.95)), (unit - 0.5, 0.04, 2.4), navy)
    geo.box(bm, V((x + (unit / 2 - 0.35) * (1 if i % 2 else -1), y - 0.12, 2.6)), (0.07, 0.06, 0.9),
            brass)
for s in (-1, 1):  # end panels
    geo.box(bm, V((s * (L / 2 - 0.02), (BACK + FRONT) / 2, (H - TOP) / 2)), (0.08,
                                                                          FRONT - BACK + 0.04, H - TOP),
            navy)
fronts = core.obj_from_bm('fronts', bm, bevel=0.012, segments=1)

bm = core.new_bm()
geo.box(bm, V((0, 0, H - TOP / 2)), (L, 4.4, TOP), mi('worktop'))
top = core.obj_from_bm('top', bm, bevel=0.03, segments=2)

bm = core.new_bm()
for sx in (-3.5, 3.5):  # bar stools under the overhang
    sy = 1.6
    rbox(bm, V((sx, sy, 3.05)), (1.25, 1.25, 0.16), 0.15, mi('oak'), seg=2)
    for k in range(4):
        a = math.pi / 4 + math.pi / 2 * k
        top_p = V((sx + 0.42 * math.cos(a), sy + 0.42 * math.sin(a), 2.98))
        foot = V((sx + 0.62 * math.cos(a), sy + 0.62 * math.sin(a), 0.0))
        geo.cylinder(bm, foot, top_p, 0.045, mi('metal_black'), n=6)
    geo.cylinder(bm, V((sx - 0.55, sy, 1.0)), V((sx + 0.55, sy, 1.0)), 0.03, mi('metal_black'), n=6)
geo.cylinder(bm, V((-1.5, -0.6, H)), V((-1.5, -0.6, H + 0.35)), 0.5, mi('ceramic'), n=20, r1=0.8)
for k in range(5):
    a = 2 * math.pi * k / 5
    geo.sphere(bm, V((-1.5 + 0.32 * math.cos(a), -0.6 + 0.32 * math.sin(a), H + 0.48)), 0.24,
               mi('fabric_mustard' if k % 2 else 'book_red'), n=10, rings=6)
geo.box(bm, V((2.4, -0.5, H + 0.09)), (1.6, 1.2, 0.18), mi('book_red'))
geo.box(bm, V((2.42, -0.5, H + 0.09)), (1.52, 1.21, 0.12), mi('pages'))
extras = core.obj_from_bm('extras', bm, bevel=0.008, segments=1, angle=40)

done('kitchen_island', [body, fronts, top, extras], car=(0.0, -5.2, 0))
