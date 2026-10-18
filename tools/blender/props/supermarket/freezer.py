"""Upright glass-door freezer (hero 'freezer row', art bible §3.5): a 3-door unit (10 x 4.2 x
9.3 u, front +Y) with charcoal frames, glass doors (blend), chrome handles, cold LED strips on
the mullions, a softly glowing white interior stocked with frozen packs, a lit FROZEN header and
a stainless kick plate (behind the track barrier). Placed back to back for the freezer island.

    blender -b -P tools/blender/props/supermarket/freezer.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

W, D, H = C.FREEZER_W, C.FREEZER_D, C.FREEZER_H
KICK = C.PLINTH_H
Z0, Z1 = 1.35, 8.1          # door opening
YF = D / 2                  # front plane
DOORS = 3
CATS = ['frozen_box', 'frozen_box', 'icecream', 'frozen_bag', 'frozen_bag']
CATEGORIES.update({
    'frozen_box': ('box', ['frozen_pizza_1', 'frozen_pizza_2', 'frozen_bolt', 'frozen_fries'],
                   (1.25, 1.4), (0.35, 0.45)),
    'frozen_bag': ('bag', ['frozen_peas', 'frozen_veg', 'frozen_fries'], (0.9, 1.05),
                   (1.0, 1.1)),
    'icecream': ('can', ['frozen_ice_1', 'frozen_ice_2'], (0.42, 0.45), (0.5, 0.55)),
})


def build():
    start('steel_dark', 'black_plastic', 'brushed', 'chrome', 'glass', 'freezer_glow',
          'freezer_inner', 'shelf_light', 'packaging', 'packaging_gloss', 'can_label', 'signs_glow',
          'frost', 'white_plastic')
    rnd = rng(11)
    body = core.new_bm()
    # cabinet shell: back, sides, top, base (open front)
    geo.box(body, V((0, -D / 2 + 0.15, H / 2)), (W, 0.3, H), mi('steel_dark'))
    for s in (-1, 1):
        geo.box(body, V((s * (W / 2 - 0.12), 0, H / 2)), (0.24, D - 0.02, H), mi('steel_dark'))
    geo.box(body, V((0, 0, (Z1 + H) / 2)), (W - 0.02, D - 0.02, H - Z1), mi('steel_dark'))
    geo.box(body, V((0, -0.05, KICK / 2)), (W - 0.02, D - 0.1, KICK), mi('brushed'))
    geo.box(body, V((0, 0, (KICK + Z0) / 2)), (W - 0.02, D - 0.02, Z0 - KICK), mi('black_plastic'))
    # interior: glowing back wall and liner, shelves with frozen packs
    inner = core.new_bm()
    geo.box(inner, V((0, -D / 2 + 0.32, (Z0 + Z1) / 2)), (W - 0.5, 0.04, Z1 - Z0),
            mi('freezer_inner'))
    shelves = core.new_bm()
    prods = core.new_bm()
    zs = [Z0 + 0.05 + k * (Z1 - Z0 - 0.2) / 5 for k in range(5)]
    dw = (W - 0.48) / DOORS
    for i, z in enumerate(zs):
        geo.box(shelves, V((0, -0.25, z)), (W - 0.5, D - 1.4, 0.06), mi('shelf_light'))
        for door in range(DOORS):
            x0 = -W / 2 + 0.24 + door * dw + 0.1
            fill_shelf(prods, x0, x0 + dw - 0.2, D / 2 - 0.75, D - 1.6, z + 0.03,
                       (Z1 - Z0) / 5 - 0.25, CATS[i], rnd, front=1, gaps=0.05)
    # door frames, mullions with LED strips, glass, handles
    doors = core.new_bm()
    glass = core.new_bm()
    for door in range(DOORS + 1):
        x = -W / 2 + 0.24 + door * dw
        geo.box(doors, V((x, YF - 0.12, (Z0 + Z1) / 2)), (0.2, 0.24, Z1 - Z0), mi('black_plastic'))
        geo.box(doors, V((x, YF - 0.27, (Z0 + Z1) / 2)), (0.07, 0.06, Z1 - Z0 - 0.2),
                mi('freezer_glow'))
    for z in (Z0, Z1):
        geo.box(doors, V((0, YF - 0.12, z)), (W - 0.24, 0.24, 0.2), mi('black_plastic'))
    for door in range(DOORS):
        x0 = -W / 2 + 0.24 + door * dw
        xc = x0 + dw / 2
        for s in (-1, 1):   # door frame stiles
            geo.box(doors, V((xc + s * (dw / 2 - 0.14), YF - 0.06, (Z0 + Z1) / 2)),
                    (0.08, 0.1, Z1 - Z0 - 0.16), mi('black_plastic'))
        for z in (Z0 + 0.12, Z1 - 0.12):
            geo.box(doors, V((xc, YF - 0.06, z)), (dw - 0.2, 0.1, 0.08), mi('black_plastic'))
        geo.box(glass, V((xc, YF - 0.08, (Z0 + Z1) / 2)), (dw - 0.3, 0.03, Z1 - Z0 - 0.3),
                mi('glass'))
        hx = xc + (dw / 2 - 0.45) * (1 if door % 2 == 0 else -1)
        geo.cylinder(doors, V((hx, YF + 0.12, Z0 + 1.6)), V((hx, YF + 0.12, Z0 + 5.0)), 0.07,
                     mi('chrome'), n=8)
        for z in (Z0 + 1.7, Z0 + 4.9):
            geo.cylinder(doors, V((hx, YF - 0.02, z)), V((hx, YF + 0.14, z)), 0.05, mi('chrome'),
                         n=6)
        # frost creeping up the lower glass corners
        geo.box(glass, V((xc, YF - 0.06, Z0 + 0.32)), (dw - 0.34, 0.012, 0.22), mi('frost'))
    # lit header with the FROZEN sign
    head = core.new_bm()
    fs = geo.box(head, V((0, YF + 0.05, (Z1 + H) / 2 + 0.05)), (W - 0.6, 0.1, H - Z1 - 0.3),
                 mi('signs_glow'))
    for f in fs:
        f.normal_update()
        if f.normal.y > 0.9:
            map_face(head, f, region('frozen_header'), 'signs')
        else:
            fill_uv(head, [f], [region('frozen_header')[0] + 4, region('frozen_header')[1] + 4, 2,
                                2], 'signs')
    parts = [part('body', body, bevel=0.04, segments=2), part('inner', inner),
             part('shelves', shelves, bevel=0.01), part('products', prods, angle=50),
             part('doors', doors, bevel=0.015), smooth('glass', glass), part('head', head)]
    return propkit.finish(ENV, 'freezer', parts, budget=HERO, car=(W / 2 + 2.5, D / 2 + 3, 200))


build()
