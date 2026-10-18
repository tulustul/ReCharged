"""Jumbo wooden ABC block: a 1.4 u (31 cm) beech cube; every face has a recessed painted field
with a raised number or shape flush with the border, so towers get variety from yaw alone.
Dynamic (knockable).

    blender -b -P tools/blender/props/toy/block_cube.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

S = 1.4  # edge (u)
H = S / 2
DEPTH = 0.045  # field recess
core.reset()
beech()
for c, hexc in (('red', RED), ('blue', BLUE), ('yellow', YELLOW), ('green', GREEN)):
    painted(f'field_{c}', hexc, 0.35)

# (outward normal, in-plane up, field colour, symbol)
FACES = [((0, 1, 0), (0, 0, 1), 'red', '1'), ((1, 0, 0), (0, 0, 1), 'blue', 'star'),
         ((0, -1, 0), (0, 0, 1), 'yellow', '2'), ((-1, 0, 0), (0, 0, 1), 'green', 'heart'),
         ((0, 0, 1), (0, 1, 0), 'red', '3'), ((0, 0, -1), (0, 1, 0), 'blue', 'circle')]

bm = core.new_bm()
geo.box(bm, V((0, 0, H)), (S, S, S), core.mi('beech'))
bevel_bm(bm, 0.03)
centre = {}
for f in bm.faces:
    for n, _, col, _ in FACES:
        if f.normal.dot(V(n)) > 0.999 and f.calc_area() > 1.0:  # the six flat faces
            centre[f] = col
geo.bmesh.ops.inset_individual(bm, faces=list(centre), thickness=0.12, depth=-DEPTH,
                               use_even_offset=True)
for f, col in centre.items():
    f.material_index = core.mi(f'field_{col}')
body = part('body', box_uv(bm), bevel=0.0)

syms = core.new_bm()
for n, u, col, sym in FACES:
    top = V((0, 0, H)) + V(n) * (H - DEPTH)  # field surface
    h = DEPTH - 0.006  # symbols stop just under the border
    if sym == 'star':
        relief(syms, star2(r_out=0.5, r_in=0.21), top, n, u, 0.74, h, core.mi('beech'))
    elif sym == 'heart':
        relief(syms, heart2(), top, n, u, 0.74, h, core.mi('beech'))
    elif sym == 'circle':
        relief(syms, geo.circle2(0.3, 32), top, n, u, 1.0, h, core.mi('beech'))
    else:
        for b in digit_bars(sym, 0.62):
            relief(syms, b, top, n, u, 1.0, h, core.mi('beech'))
s_ob = part('symbols', box_uv(syms), bevel=0.008, segments=1)
propkit.finish(ENV, 'block_cube', [body, s_ob], car=(2.2, 0.6, -20))
