"""Low playroom cubby shelf (1.05 m wide, 72 cm tall, fits under the shop windows): white
frame with a beech top, 3 x 2 cubbies holding coloured fabric bins, a ball, a small plush and a
block stack. Front +Y. Static.

    blender -b -P tools/blender/props/toy/cubby.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

W, D, H, C, T = 4.7, 1.6, 3.2, 1.45, 0.1   # size, cubby size, board thickness
core.reset()
painted('white', '#F2F0EA', 0.45)
beech()
for name, c in (('bin_r', '#D2483B'), ('bin_b', '#3C6FC9'), ('bin_y', '#E9BE3B'),
                ('bin_g', '#4B9B57')):
    fabric(name, c, 0.92)
plastic('ball', RED, 0.3)
fabric('plush', '#B07A4A')
plastic('dark', '#1C1C1E', 0.3)
for name, c in (('blk_b', BLUE), ('blk_y', YELLOW)):
    painted(name, c, 0.35)
mi = core.mi
frame, top, fill, rnd = core.new_bm(), core.new_bm(), core.new_bm(), core.new_bm()
x0, z0 = -W / 2, 0.25
for i in range(4):  # verticals
    geo.box(frame, V((x0 + T / 2 + i * (C + T * 0.99), 0, (z0 + H) / 2)), (T, D, H - z0), mi('white'))
for j in range(3):  # horizontals
    geo.box(frame, V((0, 0, z0 + j * (C + T * 0.99) + T / 2)), (W - 0.01, D - 0.02, T), mi('white'))
geo.box(frame, V((0, -D / 2 + 0.03, (z0 + H) / 2)), (W - 0.02, 0.05, H - z0 - 0.02), mi('white'))
geo.box(frame, V((0, 0.05, z0 / 2)), (W - 0.2, D - 0.2, z0), mi('white'))             # plinth
geo.box(top, V((0, 0, H + 0.06)), (W + 0.12, D + 0.1, 0.12), mi('beech'))
contents = ['bin_r', 'ball', 'bin_b', 'plush', 'bin_y', 'blocks']
for k, what in enumerate(contents):
    i, j = k % 3, k // 3
    cx = x0 + T + C / 2 + i * (C + T)
    cz = z0 + T + j * (C + T)
    if what.startswith('bin'):
        geo.box(fill, V((cx, 0.05, cz + 0.6)), (C - 0.12, D - 0.25, 1.2), mi(what))
        geo.box(fill, V((cx, D / 2 - 0.19, cz + 0.95)), (0.42, 0.02, 0.14), mi('dark'))  # handle
    elif what == 'ball':
        geo.sphere(rnd, V((cx, 0.1, cz + 0.6)), 0.6, mi('ball'), n=18, rings=9)
    elif what == 'plush':
        geo.sphere(rnd, V((cx, 0.15, cz + 0.4)), 0.42, mi('plush'), n=14, rings=7)
        geo.sphere(rnd, V((cx, 0.2, cz + 0.98)), 0.3, mi('plush'), n=14, rings=7)
        for s in (-1, 1):
            geo.sphere(rnd, V((cx + s * 0.22, 0.18, cz + 1.22)), 0.1, mi('plush'), n=8, rings=5)
            geo.sphere(rnd, V((cx + s * 0.1, 0.47, cz + 1.04)), 0.04, mi('dark'), n=6, rings=4)
    else:
        for dx, dz, m in ((-0.3, 0.0, 'blk_b'), (0.3, 0.0, 'blk_y'), (0.0, 0.55, 'blk_b')):
            geo.box(fill, V((cx + dx, 0.1, cz + dz + 0.27)), (0.55, 0.55, 0.55), mi(m))
propkit.finish(ENV, 'cubby', [part('frame', frame, bevel=0.02, segments=2),
                              part('top', box_uv(top), bevel=0.03, segments=2),
                              part('fill', fill, bevel=0.03, segments=1), smooth('round', rnd)],
               car=(3.4, 2.0, 200))
