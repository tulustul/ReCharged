"""Shelving gondola (hero, art bible §3.5): double-sided, 2 bays of 1.25 m (11.1 x 4.8 x 8.9 u),
painted steel uprights and shelves, a perforated spine, yellow price strips, and stocked with
facings of invented-brand products on both sides. Variants a-d differ in the product mix.
The base deck's kick plate sits behind the track barrier (both 1.2 u tall).

    blender -b -P tools/blender/props/supermarket/gondola.py -- [--preview DIR] [--only a]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, D, DU, H = C.GONDOLA_LEN, C.GONDOLA_DEPTH, C.GONDOLA_UPPER, C.GONDOLA_H
PLINTH = C.PLINTH_H
SHELF_Z = [1.2, 2.85, 4.3, 5.75, 7.2]     # base shelf deck, then four upper shelves
TOP = 8.7                                  # spine top (uprights reach H)
# variant -> (side +Y categories, side -Y categories), bottom shelf first
VARIANTS = {
    'a': (['cereal', 'cereal', 'cereal', 'tea', 'tea'],
          ['crackers', 'crackers', 'crackers', 'crackers', 'paper']),
    'b': (['cans', 'cans', 'cans', 'jars', 'jars'],
          ['pasta', 'pasta', 'pasta', 'pasta', 'tea']),
    'c': (['drinks', 'drinks', 'drinks', 'drinks', 'drinks'],
          ['chips', 'chips', 'chips', 'chips', 'chips']),
    'd': (['soap', 'soap', 'soap', 'paper', 'paper'],
          ['tea', 'pasta', 'crackers', 'cereal', 'tea']),
}


def build(variant):
    start('shelf_steel', 'shelf_light', 'kick_plate', 'pegboard', 'signs', 'packaging',
          'packaging_gloss', 'can_label', 'brushed', 'white_plastic')
    rnd = rng(ord(variant) * 31)
    frame = core.new_bm()
    # base deck: kick plates both sides, steel top
    geo.box(frame, V((0, 0, PLINTH / 2)), (L, D, PLINTH), mi('kick_plate'))
    # spine: perforated back panel between the shelves, uprights at the ends and the middle
    geo.box(frame, V((0, 0, (PLINTH + TOP) / 2)), (L - 0.2, 0.16, TOP - PLINTH), mi('pegboard'))
    for x in (-L / 2 + 0.13, 0.0, L / 2 - 0.13):
        geo.box(frame, V((x, 0, H / 2)), (0.26, 0.5, H), mi('shelf_steel'))
    geo.box(frame, V((0, 0, (TOP + H) / 2)), (L, 0.3, H - TOP), mi('shelf_steel'))  # top rail
    shelves = core.new_bm()
    strips = core.new_bm()
    for i, z in enumerate(SHELF_Z):
        depth = D / 2 if i == 0 else DU / 2
        thick = 0.15 if i == 0 else 0.07
        for s in (-1, 1):
            y0 = 0.08 if i else 0.0
            yc = s * (y0 + depth) / 2
            geo.box(shelves, V((0, yc, z + thick / 2)), (L - 0.3, depth - y0, thick),
                    mi('shelf_light'))
            # price strip on the shelf edge
            fs = geo.box(strips, V((0, s * (depth + 0.02), z + thick - 0.11)), (L - 0.3, 0.04, 0.24),
                         mi('signs'))
            for f in fs:
                f.normal_update()
                if abs(f.normal.y) > 0.9:
                    strip_uv(strips, f, region('price_strip'), 'signs', V((-s, 0, 0)), L - 0.3,
                             C.BAY * 3.2, offset=rnd.uniform(0.0, 0.3))
                else:
                    fill_uv(strips, [f], region('blank_white'), 'signs')
    prods = core.new_bm()
    sides = VARIANTS[variant]
    for si, s in enumerate((1, -1)):
        cats = sides[si]
        for i, z in enumerate(SHELF_Z):
            depth = D / 2 if i == 0 else DU / 2
            thick = 0.15 if i == 0 else 0.07
            ztop = SHELF_Z[i + 1] if i + 1 < len(SHELF_Z) else TOP - 0.1
            max_h = ztop - (z + thick) - 0.12
            for bay in range(C.GONDOLA_BAYS):
                x0 = -L / 2 + 0.26 + bay * C.BAY
                x1 = x0 + C.BAY - 0.3
                fill_shelf(prods, x0, x1, s * (depth - 0.04), depth - 0.25, z + thick, max_h,
                           cats[i], rnd, front=s)
    parts = [part('frame', frame, bevel=0.02), part('shelves', shelves, bevel=0.012),
             part('strips', strips), part('products', prods, angle=50)]
    return propkit.finish(ENV, f'gondola_{variant}', parts, budget=HERO,
                          car=(L / 2 + 2.0, D / 2 + 3.5, 200))


argv = propkit._argv()
only = argv[argv.index('--only') + 1] if '--only' in argv else None
for v in C.GONDOLA_VARIANTS:
    if only and v != only:
        continue
    build(v)
