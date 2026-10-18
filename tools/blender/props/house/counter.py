"""Kitchen counter runs (two variants, built together): 2.4 m base units (10.7 x 2.7 x 4.0 u)
with a recessed plinth, shaker doors and drawers in warm white, brass bar pulls, an oak worktop
and a subway-tile backsplash. `counter`: sink with tap, wall cabinets, kettle, utensil jar,
herbs. `counter_hob`: induction hob over an oven, an extractor hood instead of wall cabinets,
a pan and a chopping board. Front (+Y), back against the wall at y = -1.35.

    blender -b -P tools/blender/props/house/counter.py -- [--preview DIR]   (builds both)
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

L, D, H, TOP = 10.7, 2.7, 4.0, 0.18
WALL_Y = -D / 2


def shaker(bm, x, z, w, h, y, mat):
    """Shaker door/drawer front: frame + recessed centre panel."""
    geo.box(bm, V((x, y, z)), (w - 0.06, 0.08, h - 0.06), mat)
    geo.box(bm, V((x, y + 0.03, z)), (w - 0.5, 0.04, h - 0.5), mat)


def build(variant):
    core.reset()
    materials('paint_white', 'oak', 'brass', 'steel', 'tile_wall', 'plastic_black', 'ceramic',
              'leaf', 'terracotta', 'metal_black', 'glass', 'oak_end')
    white, brass = mi('paint_white'), mi('brass')
    bm = core.new_bm()
    geo.box(bm, V((0, 0.0, 0.24)), (L, D - 0.55, 0.48), mi('plastic_black'))  # plinth (dark)
    geo.box(bm, V((0, -0.1, (0.48 + H - TOP) / 2)), (L, D - 0.2, H - TOP - 0.48), white)
    carcass = core.obj_from_bm('carcass', bm, bevel=0.02, segments=1)

    bm = core.new_bm()
    fy = D / 2 - 0.16
    unit = L / 4
    for i in range(4):
        x = -L / 2 + unit * (i + 0.5)
        if variant == 'hob' and i == 1:  # oven
            geo.box(bm, V((x, fy, 1.95)), (unit - 0.1, 0.08, 2.6), mi('plastic_black'))
            geo.box(bm, V((x, fy + 0.05, 1.85)), (unit - 0.6, 0.04, 1.6), mi('glass'))
            geo.box(bm, V((x, fy + 0.16, 3.05)), (unit - 0.8, 0.12, 0.08), mi('steel'))
            geo.box(bm, V((x, fy + 0.02, 3.55)), (unit - 0.1, 0.06, 0.4), mi('plastic_black'))
            continue
        if i == 0:  # drawer stack
            for k, (z, h) in enumerate(((1.0, 1.0), (2.15, 1.0), (3.2, 0.9))):
                shaker(bm, x, z, unit, h, fy, white)
                geo.box(bm, V((x, fy + 0.12, z + h / 2 - 0.25)), (0.9, 0.06, 0.07), brass)
        else:
            shaker(bm, x, 1.5, unit, 2.0, fy, white)
            shaker(bm, x, 3.25, unit, 0.95, fy, white)
            geo.box(bm, V((x, fy + 0.12, 3.45)), (0.9, 0.06, 0.07), brass)
            geo.box(bm, V((x + unit / 2 - 0.35, fy + 0.12, 2.1)), (0.07, 0.06, 0.8), brass)
    fronts = core.obj_from_bm('fronts', bm, bevel=0.012, segments=1)

    bm = core.new_bm()
    geo.box(bm, V((0, 0.05, H - TOP / 2)), (L + 0.02, D + 0.1, TOP), mi('oak'))
    top = core.obj_from_bm('top', box_uv(bm, 'x'), bevel=0.025, segments=2)

    bm = core.new_bm()
    geo.box(bm, V((0, WALL_Y + 0.04, H + 1.3)), (L, 0.08, 2.6), mi('tile_wall'))
    splash = core.obj_from_bm('splash', box_uv(bm, 'z'), weighted=False)

    bm = core.new_bm()
    if variant == 'sink':
        sx = 1.4
        geo.box(bm, V((sx, 0.1, H + 0.01)), (3.0, 1.7, 0.03), mi('steel'))  # sink rim
        geo.box(bm, V((sx, 0.1, H - 0.2)), (2.6, 1.3, 0.4), mi('plastic_black'))  # basin shadow
        geo.tube(bm, [V((sx, WALL_Y + 0.5, H)), V((sx, WALL_Y + 0.5, H + 1.4)),
                      V((sx, WALL_Y + 1.2, H + 1.4)), V((sx, WALL_Y + 1.25, H + 1.1))],
                 0.07, mi('brass'), n=8, corner=0.25, cseg=3)
        # kettle, utensil jar, herbs in pots
        geo.cylinder(bm, V((-2.8, -0.2, H)), V((-2.8, -0.2, H + 1.05)), 0.42, mi('steel'), n=16,
                     r1=0.32)
        geo.cylinder(bm, V((-2.8, -0.2, H + 1.05)), V((-2.8, -0.2, H + 1.15)), 0.12,
                     mi('plastic_black'), n=10)
        geo.cylinder(bm, V((-4.2, -0.6, H)), V((-4.2, -0.6, H + 0.6)), 0.26, mi('ceramic'), n=14)
        for k in range(4):
            a = 2 * math.pi * k / 4
            geo.cylinder(bm, V((-4.2, -0.6, H + 0.4)), V((-4.2 + 0.18 * math.cos(a),
                         -0.6 + 0.18 * math.sin(a), H + 1.2)), 0.04, mi('oak'), n=5)
        for k, x in enumerate((3.8, 4.5)):
            geo.cylinder(bm, V((x, -0.75, H)), V((x, -0.75, H + 0.45)), 0.25, mi('terracotta'),
                         n=12, r1=0.3)
            geo.sphere(bm, V((x, -0.75, H + 0.7)), 0.34, mi('leaf'), n=8, rings=5)
        for k in range(3):  # wall cabinets
            x = -L / 2 + L / 3 * (k + 0.5)
            geo.box(bm, V((x, WALL_Y + 0.8, 8.4)), (L / 3 - 0.06, 1.55, 3.6), mi('paint_white'))
            shaker(bm, x, 8.4, L / 3, 3.6, WALL_Y + 1.6, mi('paint_white'))
            geo.box(bm, V((x + L / 6 - 0.45, WALL_Y + 1.72, 7.2)), (0.07, 0.06, 0.8), brass)
    else:
        hx = -L / 2 + L / 4 * 1.5
        geo.box(bm, V((hx, 0.0, H + 0.015)), (2.4, 1.8, 0.03), mi('glass'))  # induction hob
        for dx, dy, r in ((-0.55, 0.4, 0.38), (0.55, 0.4, 0.3), (-0.55, -0.4, 0.3), (0.55, -0.4, 0.38)):
            geo.cylinder(bm, V((hx + dx, dy, H + 0.03)), V((hx + dx, dy, H + 0.035)), r,
                         mi('metal_black'), n=16)
        geo.cylinder(bm, V((hx - 0.55, 0.4, H + 0.04)), V((hx - 0.55, 0.4, H + 0.42)), 0.55,
                     mi('steel'), n=18)  # a pan
        geo.box(bm, V((hx + 1.9, 0.6, H + 0.02)), (0.1, 0.7, 0.03), mi('steel'))
        geo.box(bm, V((hx, WALL_Y + 0.9, 7.5)), (3.4, 1.7, 0.5), mi('steel'))  # extractor hood
        geo.cylinder(bm, V((hx, WALL_Y + 0.6, 7.75)), V((hx, WALL_Y + 0.6, 11.5)), 0.55,
                     mi('steel'), n=4, r1=0.55)
        geo.box(bm, V((2.6, 0.0, H + 0.04)), (2.0, 1.2, 0.08), mi('oak_end'))  # chopping board
        for k in range(3):  # open oak shelf with jars
            geo.cylinder(bm, V((2.2 + k * 0.7, WALL_Y + 0.35, 7.85)), V((2.2 + k * 0.7,
                         WALL_Y + 0.35, 8.45)), 0.24, mi('glass' if k % 2 else 'ceramic'), n=12)
        geo.box(bm, V((2.9, WALL_Y + 0.4, 7.8)), (3.6, 0.7, 0.1), mi('oak'))
    extras = core.obj_from_bm('extras', bm, bevel=0.008, segments=1, angle=40)
    done('counter' if variant == 'sink' else 'counter_hob', [carcass, fronts, top, splash, extras],
         car=(-3.0, 3.6, 200))


build('sink')
build('hob')
