"""Checkout stand (hero 'checkout lane', art bible §3.5): 6 x 4 x 15 u, front +Y = the lane
entry. Conveyor belt (rubber) with rollers and a divider bar on the -X (lane) side, scanner,
bagging shelf with a bag rack, the cashier's POS and screen, card terminal, chair, a candy rack
facing the next lane, and the lane-number light on a pole (lit on open lanes 1-3).

    blender -b -P tools/blender/props/supermarket/checkout.py -- [--preview DIR] [--only N]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

W, H, L = C.STAND_W, C.STAND_H, C.STAND_L
XB0, XB1 = -W / 2, -W / 2 + 2.3          # belt counter across X
XP = W / 2 - 0.6                         # back panel (next lane's wall)


def disc_uv(bm, faces, rect, centre, radius):
    """Map the (triangulated) caps of a disc with its axis along Y onto a square sign region,
    readable from both sides; the rim gets the region's edge colour."""
    uv = bm.loops.layers.uv.verify()
    x, y, w, h = rect
    for f in faces:
        f.normal_update()
        if abs(f.normal.y) > 0.9:
            for lp in f.loops:
                d = lp.vert.co - centre
                s = (d.x * (-1 if f.normal.y > 0 else 1)) / (2 * radius) + 0.5
                t = d.z / (2 * radius) + 0.5
                lp[uv].uv = atlas_uv('signs', x + s * w, y + (1 - t) * h)
        else:
            fill_uv(bm, [f], [x + 2, y + 2, 2, 2], 'signs')


def build(lane):
    lit = lane <= 3
    start('white_plastic', 'kick_plate', 'brushed', 'chrome', 'rubber_belt', 'black_plastic',
          'red_plastic', 'screen', 'glass', 'signs', 'signs_glow', 'packaging', 'steel_dark')
    rnd = rng(100 + lane)
    body = core.new_bm()
    # belt counter cabinet with a grey kick band, the POS block and the back panel
    geo.box(body, V(((XB0 + XB1) / 2, 0, 0.2)), (XB1 - XB0, L, 0.4), mi('kick_plate'))
    geo.box(body, V(((XB0 + XB1) / 2, 0, (0.4 + 3.85) / 2)), (XB1 - XB0, L, 3.45),
            mi('white_plastic'))
    geo.box(body, V((XP + 0.3, 0, 0.2)), (0.6, L, 0.4), mi('kick_plate'))
    geo.box(body, V((XP + 0.3, 0, (0.4 + H) / 2)), (0.6, L, H - 0.4), mi('white_plastic'))
    geo.box(body, V(((XB1 + 1.8) / 2, 0.1, 1.8)), (1.8 - XB1, 2.6, 3.6), mi('white_plastic'))
    geo.box(body, V((XP - 0.2, -L / 2 + 0.3, 1.9)), (1.2, 0.6, 3.8), mi('white_plastic'))
    top = core.new_bm()
    geo.box(top, V(((XB0 + XB1) / 2, 0, 3.92)), (XB1 - XB0 + 0.06, L + 0.06, 0.14), mi('brushed'))
    geo.box(top, V((XP + 0.3, 0, H - 0.04)), (0.66, L + 0.06, 0.1), mi('brushed'))
    # conveyor belt with end rollers and a divider bar
    belt = core.new_bm()
    y0, y1 = 0.9, L / 2 - 0.35
    geo.box(belt, V(((XB0 + XB1) / 2, (y0 + y1) / 2, 4.02)), (XB1 - XB0 - 0.3, y1 - y0, 0.08),
            mi('rubber_belt'))
    for y in (y0, y1):
        geo.cylinder(belt, V((XB0 + 0.12, y, 4.0)), V((XB1 - 0.12, y, 4.0)), 0.07, mi('chrome'),
                     n=8)
    geo.prism(belt, [(-0.12, 0), (0.12, 0), (0, 0.18)], V((XB0 + 0.3, 3.6, 4.06)),
              V((XB1 - 0.3, 3.6, 4.06)), mi('red_plastic'))
    # scanner: glass plate and the vertical tower, card terminal on a post
    gear = core.new_bm()
    geo.box(gear, V(((XB0 + XB1) / 2 + 0.2, 0.15, 4.0)), (1.4, 1.0, 0.04), mi('glass'))
    geo.box(gear, V((XB1 - 0.15, 0.15, 4.45)), (0.25, 0.9, 0.9), mi('black_plastic'))
    geo.box(gear, V((XB1 - 0.29, 0.15, 4.45)), (0.03, 0.6, 0.55), mi('screen'))
    geo.cylinder(gear, V((XB0 + 0.35, -1.6, 4.0)), V((XB0 + 0.35, -1.6, 4.9)), 0.05, mi('chrome'),
                 n=6)
    geo.box(gear, V((XB0 + 0.35, -1.6, 5.05)), (0.35, 0.25, 0.45), mi('black_plastic'))
    # register: till drawer block, the cashier's screen and a customer display
    geo.box(gear, V((0.6, 0.4, 3.85)), (1.3, 1.3, 0.5), mi('black_plastic'))
    geo.cylinder(gear, V((0.6, 0.4, 4.1)), V((0.6, 0.4, 5.0)), 0.06, mi('steel_dark'), n=6)
    geo.box(gear, V((0.75, 0.4, 5.35)), (0.1, 1.1, 0.75), mi('black_plastic'))
    geo.box(gear, V((0.81, 0.4, 5.35)), (0.02, 0.95, 0.6), mi('screen'))
    geo.cylinder(gear, V((XB1 + 0.1, 1.1, 3.6)), V((XB1 + 0.1, 1.1, 5.4)), 0.05, mi('steel_dark'),
                 n=6)
    geo.box(gear, V((XB1 + 0.04, 1.1, 5.55)), (0.08, 0.7, 0.35), mi('screen'))
    # bagging shelf rack (two posts and a rail)
    for y in (-L / 2 + 0.5, -2.2):
        geo.cylinder(gear, V((XB0 + 0.2, y, 3.98)), V((XB0 + 0.2, y, 5.3)), 0.05, mi('chrome'),
                     n=6)
    geo.cylinder(gear, V((XB0 + 0.2, -L / 2 + 0.5, 5.3)), V((XB0 + 0.2, -2.2, 5.3)), 0.05,
                 mi('chrome'), n=6)
    # cashier chair
    geo.cylinder(gear, V((1.2, -0.6, 0.02)), V((1.2, -0.6, 0.12)), 0.75, mi('black_plastic'), n=10)
    geo.cylinder(gear, V((1.2, -0.6, 0.1)), V((1.2, -0.6, 2.4)), 0.09, mi('chrome'), n=6)
    geo.cylinder(gear, V((1.2, -0.6, 2.4)), V((1.2, -0.6, 2.75)), 0.65, mi('black_plastic'), n=12)
    geo.box(gear, V((1.75, -0.6, 3.5)), (0.15, 1.0, 1.4), mi('black_plastic'))
    # candy rack facing the next lane (+X side of the back panel)
    candy = core.new_bm()
    for k, z in enumerate((1.0, 2.0, 3.0)):
        geo.box(candy, V((XP + 0.85, 3.3, z)), (0.5, 5.0, 0.05), mi('brushed'))
        x = 0.85
        cy = 0.9
        while cy < 5.6:
            name = rnd.choice(['chips_1', 'cookie_1', 'chips_4', 'cookie_2', 'chips_2'])
            w = rnd.uniform(0.4, 0.55)
            fs = geo.box(candy, V((XP + x, cy + w / 2, z + 0.32)), (0.4, w - 0.04, 0.6),
                         mi('packaging'))
            for f in fs:
                f.normal_update()
                if f.normal.x > 0.9:
                    map_face(candy, f, cell(name))
                else:
                    fill_uv(candy, [f], side(name))
            cy += w
    # lane number light on a pole at the entry end
    sign = core.new_bm()
    px, py = XP + 0.3, L / 2 - 0.4
    geo.cylinder(sign, V((px, py, H)), V((px, py, 9.2)), 0.09, mi('steel_dark'), n=8)
    disc = geo.cylinder(sign, V((px, py - 0.08, 9.9)), V((px, py + 0.08, 9.9)), 0.75,
                        mi('signs_glow' if lit else 'signs'), n=20)
    disc_uv(sign, disc, region(f'lane_{lane}'), V((px, py, 9.9)), 0.75)
    geo.cylinder(sign, V((px, py - 0.06, 9.9)), V((px, py + 0.06, 9.9)), 0.8, mi('steel_dark'), n=20)
    parts = [part('body', body, bevel=0.03), part('top', top, bevel=0.02),
             part('belt', belt, angle=50), part('gear', gear, bevel=0.01, angle=50),
             part('candy', candy), smooth('sign', sign, angle=40)]
    return propkit.finish(ENV, f'checkout_{lane}', parts, budget=HERO,
                          car=(-W / 2 - 2.0, L / 2 + 1.0, 180))


argv = propkit._argv()
only = int(argv[argv.index('--only') + 1]) if '--only' in argv else None
for n in C.LANES:
    if only and n != only:
        continue
    build(n)
