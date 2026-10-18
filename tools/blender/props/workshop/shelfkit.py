"""Galvanised steel shelving (decor) stocked with workshop supplies; shelf_a / shelf_b differ in
stock. 9 u long, 2.4 u deep, 9 u tall, origin at the wall face on the floor, front +Y."""
import random

from wkit import *  # noqa: F401,F403

L, D, H = 9.0, 2.4, 9.0
SHELVES = (0.35, 2.6, 4.9, 7.2)
STOCK = ('cardboard', 'cardboard', 'amber_plastic', 'plastic_blue', 'plastic_red',
         'plastic_green', 'plastic_yellow', 'plastic_black', 'label_white')


def build(name, seed):
    start('galvanised', 'steel_dark', *dict.fromkeys(STOCK))
    rng = random.Random(seed)
    bm = core.new_bm()
    for x in (-L / 2 + 0.12, L / 2 - 0.12):
        for y in (0.12, D - 0.12):
            geo.box(bm, V((x, y, H / 2)), (0.16, 0.16, H), mi('galvanised'))
    for z in SHELVES:
        geo.box(bm, V((0, D / 2, z)), (L, D, 0.08), mi('galvanised'))
    for y in (0.12, D - 0.12):  # back / front cross braces
        geo.box(bm, V((0, y, H - 0.1)), (L, 0.1, 0.14), mi('steel_dark'))
    frame = core.obj_from_bm('frame', bm, bevel=0.015, segments=1, angle=40)
    bm = core.new_bm()
    for z in SHELVES:
        x = -L / 2 + 0.35
        while True:
            kind = rng.choice(('box', 'box', 'jugs', 'bottles', 'tins'))
            if kind == 'box':
                w, d, h = rng.uniform(1.0, 2.0), rng.uniform(1.4, 2.1), rng.uniform(0.8, 1.9)
                n = 1
            else:
                n = rng.randint(2, 4)
                w = n * {'jugs': 0.95, 'bottles': 0.5, 'tins': 0.85}[kind]
            if x + w > L / 2 - 0.3:
                break
            if kind == 'box':
                geo.box(bm, V((x + w / 2, D / 2, z + 0.04 + h / 2)), (w, d, h), mi('cardboard'))
                geo.box(bm, V((x + w / 2, D / 2 + d / 2 + 0.004, z + 0.04 + h * 0.6)),
                        (w * 0.5, 0.01, h * 0.25), mi('label_white'))
            elif kind == 'jugs':
                mat = rng.choice(('amber_plastic', 'plastic_blue', 'plastic_green'))
                for k in range(n):
                    cx = x + k * 0.95 + 0.45
                    geo.box(bm, V((cx, D / 2, z + 0.04 + 0.62)), (0.85, 0.5, 1.24), mi(mat))
                    geo.cylinder(bm, V((cx + 0.25, D / 2, z + 1.28)),
                                 V((cx + 0.25, D / 2, z + 1.44)), 0.11, mi('plastic_black'), n=6)
            elif kind == 'bottles':
                mat = rng.choice(('plastic_red', 'plastic_yellow', 'plastic_blue'))
                for k in range(n):
                    cx = x + k * 0.5 + 0.25
                    geo.cylinder(bm, V((cx, D / 2 + rng.uniform(-0.4, 0.4), z + 0.04)),
                                 V((cx, D / 2, z + 1.1)), 0.22, mi(mat), n=8, r1=0.16)
            else:
                for k in range(n):
                    cx = x + k * 0.85 + 0.4
                    geo.cylinder(bm, V((cx, D / 2, z + 0.04)), V((cx, D / 2, z + 0.9)), 0.38,
                                 mi(rng.choice(('plastic_red', 'label_white', 'plastic_green'))),
                                 n=10)
            x += w + rng.uniform(0.15, 0.5)
    stock = core.obj_from_bm('stock', bm, bevel=0.01, segments=1, angle=40)
    finish(name, [frame, stock], budget=KIT, car=(0.0, D + 3.0, 180.0))
