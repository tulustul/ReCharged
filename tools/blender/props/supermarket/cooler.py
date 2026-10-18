"""Open multi-deck dairy cooler (hero 'freezer row', art bible §3.5): 11.1 x 4.4 x 9.3 u, front
+Y; tiered shelves of milk cartons, yogurt and cheese under a canopy light strip, a glowing
white liner, black air grille and a stainless kick plate (behind the track barrier). Lines the
back wall of the store.

    blender -b -P tools/blender/props/supermarket/cooler.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

W, D, H = C.COOLER_W, C.COOLER_D, C.COOLER_H
KICK = C.PLINTH_H
YB = -D / 2 + 0.3                       # liner (back wall) plane
# (shelf z, front edge y, category) from the well upward; the tiers recede toward the top
TIERS = [(1.75, 1.75, 'milk'), (3.1, 1.35, 'milk'), (4.35, 1.0, 'yogurt'),
         (5.55, 0.7, 'cheese'), (6.7, 0.4, 'yogurt')]
CANOPY = 7.9


def build():
    start('steel_dark', 'white_plastic', 'black_plastic', 'brushed', 'freezer_inner',
          'cooler_glow', 'shelf_light', 'signs', 'packaging', 'packaging_gloss', 'can_label')
    rnd = rng(23)
    body = core.new_bm()
    geo.box(body, V((0, -D / 2 + 0.15, H / 2)), (W, 0.3, H), mi('white_plastic'))
    for s in (-1, 1):   # end panels: deep at the well, shallow under the canopy
        x = s * (W / 2 - 0.1)
        prof = [(-D / 2, 0), (D / 2, 0), (D / 2, 1.9), (0.9, CANOPY), (1.5, CANOPY),
                (1.5, H), (-D / 2, H)]
        geo.extrude_x(body, [(y, z) for y, z in prof], x - 0.1, x + 0.1, mi('white_plastic'))
    geo.box(body, V((0, 0.0, KICK / 2)), (W - 0.4, D - 0.02, KICK), mi('brushed'))
    geo.box(body, V((0, D / 2 - 0.2, KICK + 0.25)), (W - 0.4, 0.4, 0.5), mi('black_plastic'))
    geo.box(body, V((0, 0.0, (KICK + 1.7) / 2)), (W - 0.4, D - 0.5, 1.7 - KICK),
            mi('white_plastic'))
    geo.box(body, V((0, -0.35, (CANOPY + H) / 2)), (W - 0.2, D - 0.7 + 1.85 - 1.0,
                                                    H - CANOPY), mi('steel_dark'))
    glow = core.new_bm()
    geo.box(glow, V((0, YB + 0.02, 4.8)), (W - 0.45, 0.04, 6.2), mi('freezer_inner'))
    geo.box(glow, V((0, 1.25, CANOPY - 0.06)), (W - 0.5, 0.35, 0.1), mi('cooler_glow'))
    shelves = core.new_bm()
    prods = core.new_bm()
    for i, (z, yf, cat) in enumerate(TIERS):
        geo.box(shelves, V((0, (YB + yf) / 2, z - 0.04)), (W - 0.45, yf - YB, 0.08),
                mi('shelf_light'))
        fs = geo.box(shelves, V((0, yf + 0.02, z - 0.1)), (W - 0.45, 0.04, 0.22), mi('signs'))
        for f in fs:
            f.normal_update()
            if f.normal.y > 0.9:
                strip_uv(shelves, f, region('price_strip'), 'signs', V((-1, 0, 0)), W - 0.45,
                         C.BAY * 3.2, offset=rnd.uniform(0.0, 0.3))
            else:
                fill_uv(shelves, [f], region('blank_white'), 'signs')
        top = TIERS[i + 1][0] if i + 1 < len(TIERS) else CANOPY
        for half in (-1, 1):
            x0 = -W / 2 + 0.3 if half < 0 else 0.05
            fill_shelf(prods, x0, x0 + W / 2 - 0.35, yf - 0.05, min(1.2, yf - YB - 0.1), z,
                       top - z - 0.18, cat, rnd, front=1, gaps=0.06)
    parts = [part('body', body, bevel=0.04), part('glow', glow), part('shelves', shelves),
             part('products', prods, angle=50)]
    return propkit.finish(ENV, 'cooler', parts, budget=HERO, car=(W / 2 + 2.5, D / 2 + 3, 200))


build()
