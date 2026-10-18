"""Produce display table (kit 'fruit', art bible §3.5): a black steel table (9 x 5.3 x 4 u, length
along X) carrying six tilted wooden crates of oranges, apples, lemons and limes, with price
cards. Two rows of them make the produce island the S-bend curves around.

    blender -b -P tools/blender/props/supermarket/produce_table.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, W, H = C.TABLE_L, C.TABLE_W, C.TABLE_H
DECK = 2.6          # table top (crates on top, fruit up to H)
FRUIT = [['fruit_orange'], ['fruit_red', 'fruit_red', 'fruit_green'], ['fruit_yellow'],
         ['fruit_green'], ['fruit_red'], ['fruit_orange', 'fruit_yellow']]


def build():
    start('steel_dark', 'pallet_wood', 'pallet_wood_dark', 'fruit_orange', 'fruit_red',
          'fruit_green', 'fruit_yellow', 'signs', 'white_plastic')
    rnd = rng(31)
    frame = core.new_bm()
    geo.box(frame, V((0, 0, DECK - 0.1)), (L, W, 0.2), mi('steel_dark'))
    # slatted wooden skirt all round (the island is solid: no driving under the tables)
    skirt = core.new_bm()
    for k in range(4):
        z = k * (DECK - 0.2) / 4
        geo.box(skirt, V((0, 0, z + 0.27)), (L - 0.3, W - 0.3, 0.54), mi('pallet_wood'
                                                                        if k % 2 else
                                                                        'pallet_wood_dark'))
    crates = core.new_bm()
    fruit = core.new_bm()
    cw, cd = (L - 0.3) / 3, (W - 0.3) / 2
    for i in range(3):
        for j in range(2):
            x0 = -L / 2 + 0.15 + cw * i
            y0 = -W / 2 + 0.15 + cd * j
            h0 = 1.0 if j == 0 else 0.6          # back row stands taller (tiered)
            zb = DECK
            ztop = zb + h0
            for xa, xb in ((x0 + 0.05, x0 + 0.15), (x0 + cw - 0.15, x0 + cw - 0.05)):
                geo.box(crates, V(((xa + xb) / 2, y0 + cd / 2, zb + h0 / 2)), (xb - xa, cd - 0.1, h0),
                        mi('pallet_wood'))
            for ya, yb in ((y0 + 0.05, y0 + 0.15), (y0 + cd - 0.15, y0 + cd - 0.05)):
                geo.box(crates, V((x0 + cw / 2, (ya + yb) / 2, zb + h0 / 2)), (cw - 0.1, yb - ya, h0),
                        mi('pallet_wood_dark'))
            fruit_bed(fruit, x0 + 0.2, x0 + cw - 0.2, y0 + 0.2, y0 + cd - 0.2, ztop - 0.05,
                      FRUIT[i * 2 + j], rnd, r=0.26)
    cards = core.new_bm()
    for i in range(3):   # price cards on sticks at the front crates
        x = -L / 2 + 0.15 + cw * (i + 0.5)
        fs = geo.box(cards, V((x, W / 2 + 0.05, DECK + 0.9)), (0.9, 0.05, 0.6), mi('signs'))
        r = region('price_strip')
        for f in fs:
            f.normal_update()
            if f.normal.y > 0.9:
                map_face(cards, f, [r[0] + 120 * i + 4, r[1] + 4, 112, 24], 'signs')
            else:
                fill_uv(cards, [f], region('blank_white'), 'signs')
    parts = [part('frame', frame, bevel=0.03, segments=1), part('skirt', skirt, bevel=0.02,
                                                                 segments=1),
             part('crates', crates, bevel=0.02,
                                                                 segments=1),
             smooth('fruit', fruit), part('cards', cards)]
    return propkit.finish(ENV, 'produce_table', parts, car=(L / 2 + 2, W / 2 + 2, 210))


build()
