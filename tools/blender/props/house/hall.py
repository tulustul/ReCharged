"""Hall pieces, built together:
console    narrow oak hall console, 1.2 m x 30 cm x 80 cm (5.3 x 1.3 x 3.55 u) with a ceramic
           lamp (warm bulb), a key bowl and a stack of post; static, light in the catalog.
shoe_rack  low two-tier oak shoe bench, 90 x 32 x 48 cm (4.0 x 1.4 x 2.1 u) with shoes; static.
front_door decor: the front door in its frame (4.4 x 0.3 x 9.8 u), panelled, glazed top lights,
           brass knocker and letter plate; stands against the hall's west wall.
doormat    decor: coir doormat 3.6 x 2.4 u.
cupboard   built-in white coat closet, 3.0 x 4.8 x 9.0 u (68 cm wide, 1.08 m deep, 2 m tall), a
           panelled door on the front (+Y); fills the gap between the top of the stairs and the
           hall wall (static).

    blender -b -P tools/blender/props/house/hall.py -- [--preview DIR] [name ...]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

ARGS = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
if '--preview' in ARGS:
    i = ARGS.index('--preview')
    ARGS = ARGS[:i] + ARGS[i + 2:]


def console():
    core.reset()
    materials('oak', 'ceramic', 'lamp_shade', 'bulb', 'brass', 'book_cream', 'book_blue',
              'terracotta')
    L, D, H = 5.3, 1.3, 3.55
    bm = core.new_bm()
    geo.box(bm, V((0, 0, H - 0.08)), (L, D, 0.16), mi('oak'))
    geo.box(bm, V((0, 0, 0.9)), (L - 0.4, D - 0.25, 0.1), mi('oak'))  # low shelf
    for sx in (-1, 1):
        for sy in (-1, 1):
            geo.prism(bm, geo.rrect(0.18, 0.18, 0.03, 2), V((sx * (L / 2 - 0.2), sy * (D / 2 - 0.15), H - 0.16)),
                      V((sx * (L / 2 - 0.2), sy * (D / 2 - 0.15), 0.0)), mi('oak'), up=V((1, 0, 0)),
                      scale1=0.75)
    table = core.obj_from_bm('table', box_uv(bm, 'x'), bevel=0.02, segments=2)
    bm = core.new_bm()
    lx = -1.6  # table lamp: ceramic base, linen shade, warm bulb
    geo.cylinder(bm, V((lx, 0, H)), V((lx, 0, H + 1.1)), 0.42, mi('ceramic'), n=20, r1=0.28)
    geo.cylinder(bm, V((lx, 0, H + 1.1)), V((lx, 0, H + 1.6)), 0.04, mi('brass'), n=8)
    geo.sphere(bm, V((lx, 0, H + 1.75)), 0.16, mi('bulb'), n=10, rings=6)
    n = 24
    rings = [[V((lx + r * math.cos(2 * math.pi * k / n), r * math.sin(2 * math.pi * k / n), z))
              for k in range(n)] for r, z in ((0.85, H + 1.4), (0.6, H + 2.3))]
    geo._bridge(bm, rings, mi('lamp_shade'), cap0=False, cap1=False)
    geo.cylinder(bm, V((1.0, 0.05, H)), V((1.0, 0.05, H + 0.25)), 0.35, mi('terracotta'), n=16,
                 r1=0.5)  # key bowl
    for k, col in enumerate(('book_cream', 'book_blue', 'book_cream')):  # post
        geo.box(bm, V((2.0 + 0.04 * k, -0.1, H + 0.02 + 0.025 * k)), (0.9, 0.5, 0.02), mi(col))
    deco = core.obj_from_bm('deco', box_uv(bm, 'z'), weighted=False, angle=50)
    return 'console', [table, deco], (-1.0, 2.6, 200)


def shoe_rack():
    core.reset()
    materials('oak', 'book_red', 'plastic_white', 'rubber', 'leather', 'fabric_blue', 'fabric_grey')
    L, D, H = 4.0, 1.4, 2.1
    bm = core.new_bm()
    geo.box(bm, V((0, 0, H - 0.07)), (L, D, 0.14), mi('oak'))
    geo.box(bm, V((0, 0, 0.75)), (L - 0.1, D - 0.1, 0.08), mi('oak'))
    for sx in (-1, 1):
        geo.box(bm, V((sx * (L / 2 - 0.07), 0, H / 2)), (0.14, D, H), mi('oak'))
    rack = core.obj_from_bm('rack', box_uv(bm, 'x'), bevel=0.02, segments=2)
    bm = core.new_bm()

    def shoe(x, z, col, yaw=0.0, scale=1.0):
        M = frame_m((x, 0, z), yaw=yaw)
        obox(bm, M, (0.46 * scale, 1.2 * scale, 0.1 * scale), mi('rubber'), (0, 0, 0.05 * scale))
        obox(bm, M, (0.42 * scale, 0.95 * scale, 0.3 * scale), mi(col), (0, -0.1 * scale, 0.25 * scale))
        obox(bm, M, (0.43 * scale, 0.35 * scale, 0.2 * scale), mi(col), (0, 0.4 * scale, 0.18 * scale))

    for x, col, yaw in ((-1.3, 'leather', 4), (-0.75, 'leather', -3), (0.4, 'fabric_blue', 8),
                        (0.95, 'fabric_blue', -6)):
        shoe(x, H, col, yaw)
    for x, col, yaw in ((-1.2, 'book_red', 10), (-0.6, 'book_red', -5), (0.6, 'fabric_grey', 0),
                        (1.2, 'fabric_grey', 6)):
        shoe(x, 0.79, col, yaw, 0.85)
    shoes = core.obj_from_bm('shoes', bm, bevel=0.03, segments=1)
    return 'shoe_rack', [rack, shoes], (2.8, 0.6, 200)


def front_door():
    core.reset()
    materials('paint_navy', 'paint_white', 'glass', 'brass')
    W, H = 4.0, 9.3
    bm = core.new_bm()
    geo.box(bm, V((0, 0.05, H / 2)), (W, 0.12, H), mi('paint_navy'))  # leaf
    for zc, h in ((2.4, 3.6), (5.9, 2.6)):  # panels
        for dx in (-0.95, 0.95):
            geo.box(bm, V((dx, 0.13, zc)), (1.4, 0.05, h), mi('paint_navy'))
    leaf = core.obj_from_bm('leaf', bm, bevel=0.015, segments=1)
    bm = core.new_bm()
    for dx in (-0.95, 0.95):  # glazed top lights
        geo.box(bm, V((dx, 0.13, 8.1)), (1.3, 0.04, 1.4), mi('glass'))
    for x in (-W / 2 - 0.2, W / 2 + 0.2):  # frame / architrave
        geo.box(bm, V((x, 0.1, (H + 0.4) / 2)), (0.4, 0.2, H + 0.4), mi('paint_white'))
    geo.box(bm, V((0, 0.1, H + 0.2)), (W + 0.8, 0.2, 0.4), mi('paint_white'))
    geo.box(bm, V((0, 0.14, 4.4)), (0.9, 0.04, 0.22), mi('brass'))  # letter plate
    geo.cylinder(bm, V((1.5, 0.1, 4.6)), V((1.5, 0.32, 4.6)), 0.14, mi('brass'), n=12)  # knob
    geo.box(bm, V((0, 0.14, 6.9)), (0.25, 0.06, 0.6), mi('brass'))  # knocker
    frame = core.obj_from_bm('frame', bm, bevel=0.012, segments=1)
    return 'front_door', [leaf, frame], (0.0, 3.4, 180)


def doormat():
    core.reset()
    materials('fabric_mustard', 'plastic_black')
    bm = core.new_bm()
    rbox(bm, V((0, 0, 0.025)), (3.6, 2.4, 0.05), 0.02, mi('fabric_mustard'), seg=1)
    geo.box(bm, V((0, 0, 0.052)), (3.2, 2.0, 0.005), mi('plastic_black'))
    geo.box(bm, V((0, 0, 0.056)), (3.0, 1.8, 0.005), mi('fabric_mustard'))
    return 'doormat', [core.obj_from_bm('mat', box_uv(bm, 'x'), weighted=False)], (2.6, 0.4, 200)


def cupboard():
    core.reset()
    materials('paint_white', 'brass', 'oak_end', 'terracotta', 'leaf')
    W, D, H = 3.0, 4.8, 9.0
    bm = core.new_bm()
    geo.box(bm, V((0, 0, H / 2)), (W, D, H), mi('paint_white'))
    body = core.obj_from_bm('body', bm, bevel=0.03, segments=2)
    bm = core.new_bm()
    fy = D / 2 + 0.03
    geo.box(bm, V((0, fy - 0.02, 0.2)), (W - 0.1, 0.05, 0.4), mi('oak_end'))  # plinth shadow
    geo.box(bm, V((0, fy, 4.7)), (2.5, 0.06, 8.4), mi('paint_white'))  # door with raised panels
    for zc, h in ((2.6, 3.6), (6.9, 3.2)):
        geo.box(bm, V((0, fy + 0.04, zc)), (1.9, 0.04, h), mi('paint_white'))
    geo.cylinder(bm, V((0.95, fy + 0.03, 4.6)), V((0.95, fy + 0.2, 4.6)), 0.08, mi('brass'), n=10)
    doors = core.obj_from_bm('doors', bm, bevel=0.012, segments=1)
    bm = core.new_bm()
    geo.cylinder(bm, V((0, 1.2, H)), V((0, 1.2, H + 0.5)), 0.32, mi('terracotta'), n=14, r1=0.38)
    for k in range(5):
        a = 2 * math.pi * k / 5
        geo.sphere(bm, V((0.25 * math.cos(a), 1.2 + 0.25 * math.sin(a), H + 0.75)), 0.26,
                   mi('leaf'), n=8, rings=5, scale=(1, 1, 0.7))
    plant = core.obj_from_bm('plant', bm, weighted=False, angle=50)
    return 'cupboard', [body, doors, plant], (3.6, 2.6, 200)


for fn in (console, shoe_rack, front_door, doormat, cupboard):
    if ARGS and fn.__name__ not in ARGS:
        continue
    name, parts, car = fn()
    done(name, parts, car=car)
