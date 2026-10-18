"""Gondola endcap (hero family of the shelving gondola, art bible §3.5): a one-bay promotional
display closing a gondola run (4.8 wide, 2.4 deep, 8.9 u tall), front +Y toward the cross aisle,
with a SPECIAL OFFER header. Variants a-d carry different promotions.

    blender -b -P tools/blender/props/supermarket/endcap.py -- [--preview DIR] [--only a]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

W, D, H = C.GONDOLA_DEPTH, C.ENDCAP_DEPTH, C.GONDOLA_H
PLINTH = C.PLINTH_H
Y_BACK = -D / 2
Y_UPPER = Y_BACK + D * 0.7            # front edge of the upper shelves (matches the collider)
SHELF_Z = [1.2, 3.0, 4.6, 6.2]
PROMO = {'a': ['drinks', 'drinks', 'drinks', 'drinks'], 'b': ['cans', 'cans', 'jars', 'jars'],
         'c': ['chips', 'chips', 'chips', 'chips'], 'd': ['soap', 'soap', 'paper', 'paper']}


def build(variant):
    start('shelf_steel', 'shelf_light', 'kick_plate', 'pegboard', 'signs', 'packaging',
          'packaging_gloss', 'can_label', 'brushed', 'white_plastic')
    rnd = rng(ord(variant) * 57 + 3)
    frame = core.new_bm()
    geo.box(frame, V((0, 0, PLINTH / 2)), (W, D, PLINTH), mi('kick_plate'))
    geo.box(frame, V((0, Y_BACK + 0.08, (PLINTH + 7.6) / 2)), (W - 0.3, 0.16, 7.6 - PLINTH),
            mi('pegboard'))
    for x in (-W / 2 + 0.14, W / 2 - 0.14):   # uprights stand on the deck (no shared corners)
        geo.box(frame, V((x, Y_BACK + 0.26, (PLINTH + H) / 2)), (0.26, 0.5, H - PLINTH),
                mi('shelf_steel'))
    sign = core.new_bm()
    fs = geo.box(sign, V((0, Y_BACK + 0.3, 8.4)), (W - 0.1, 0.12, 0.9), mi('signs'))
    for f in fs:
        f.normal_update()
        if f.normal.y > 0.9:
            map_face(sign, f, region('sale_wide'), 'signs')
        else:
            fill_uv(sign, [f], [region('sale_wide')[0] + 4, region('sale_wide')[1] + 4, 2, 2],
                    'signs')
    shelves = core.new_bm()
    prods = core.new_bm()
    for i, z in enumerate(SHELF_Z):
        y1 = D / 2 if i == 0 else Y_UPPER
        thick = 0.15 if i == 0 else 0.07
        geo.box(shelves, V((0, (Y_BACK + 0.16 + y1) / 2, z + thick / 2)),
                (W - 0.52, y1 - Y_BACK - 0.16, thick), mi('shelf_light'))
        fs = geo.box(shelves, V((0, y1 + 0.02, z + thick - 0.11)), (W - 0.52, 0.04, 0.24),
                     mi('signs'))
        for f in fs:
            f.normal_update()
            if f.normal.y > 0.9:
                strip_uv(shelves, f, region('price_strip'), 'signs', V((-1, 0, 0)), W - 0.52,
                         C.BAY * 3.2, offset=rnd.uniform(0.0, 0.5))
            else:
                fill_uv(shelves, [f], region('blank_white'), 'signs')
        ztop = SHELF_Z[i + 1] if i + 1 < len(SHELF_Z) else 7.55
        fill_shelf(prods, -W / 2 + 0.3, W / 2 - 0.3, y1 - 0.04, y1 - Y_BACK - 0.45, z + thick,
                   ztop - (z + thick) - 0.12, PROMO[variant][i], rnd, front=1, gaps=0.03)
    parts = [part('frame', frame, bevel=0.02), part('sign', sign, bevel=0.015),
             part('shelves', shelves, bevel=0.01), part('products', prods, angle=50)]
    return propkit.finish(ENV, f'endcap_{variant}', parts, budget=HERO, car=(3.5, 3.0, 210))


argv = propkit._argv()
only = argv[argv.index('--only') + 1] if '--only' in argv else None
for v in C.GONDOLA_VARIANTS:
    if only and v != only:
        continue
    build(v)
