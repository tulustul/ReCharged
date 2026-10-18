"""Floor markings (decor, flat at 0.012 u): `bay_marking` — a yellow lift-bay rectangle
(22 x 30 u, 0.45 u lines) with hazard-striped corner blocks; `walkway_line` — a 20 u yellow
line; `stains` — oil smudges and tyre scuffs over a 12 u square; `start_line` — chequered
tape across the start straight. Four glbs from one script.

    blender -b -P tools/blender/props/workshop/floor_marks.py -- [--preview DIR]
"""
import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

Z = 0.012


def flat(bm, pts, mat, z=Z):
    vs = [bm.verts.new((x, y, z)) for x, y in pts]
    f = bm.faces.new(vs)
    f.material_index = mi(mat)
    f.normal_update()
    if f.normal.z < 0:
        f.normal_flip()
    return f


def rect(bm, x0, y0, x1, y1, mat, z=Z):
    return flat(bm, [(x0, y0), (x1, y0), (x1, y1), (x0, y1)], mat, z)


def build(name):
    start('floor_paint', 'hazard', 'grime', 'label_white', 'hazard_black')
    bm = core.new_bm()
    if name == 'bay_marking':
        W, D, T = 22.0, 30.0, 0.45
        rect(bm, -W / 2, -D / 2, W / 2, -D / 2 + T, 'floor_paint')
        rect(bm, -W / 2, D / 2 - T, W / 2, D / 2, 'floor_paint')
        rect(bm, -W / 2, -D / 2 + T, -W / 2 + T, D / 2 - T, 'floor_paint')
        rect(bm, W / 2 - T, -D / 2 + T, W / 2, D / 2 - T, 'floor_paint')
        for sx in (-1, 1):
            for sy in (-1, 1):
                f = rect(bm, sx * W / 2 - (2.0 if sx > 0 else 0), sy * D / 2 - (2.0 if sy > 0 else 0),
                         sx * W / 2 + (0 if sx > 0 else 2.0), sy * D / 2 + (0 if sy > 0 else 2.0),
                         'hazard', Z + 0.002)
                planar_uv(bm, [f], (1, 0, 0), (0, 1, 0), 0.5)
    elif name == 'walkway_line':
        rect(bm, -10.0, -0.22, 10.0, 0.22, 'floor_paint')
    elif name == 'start_line':  # chequered tape across a 10 u lane, two rows of 0.6 u squares
        n, q = 17, 0.6
        for i in range(n):
            for j in range(2):
                x0 = -n * q / 2 + i * q
                rect(bm, x0, -q + j * q, x0 + q, j * q, 'label_white' if (i + j) % 2 else
                     'hazard_black')
        rect(bm, -n * q / 2, 0.6, n * q / 2, 0.75, 'label_white')
    else:
        rng = random.Random(5)
        for _ in range(6):  # oil smudges: irregular blobs
            cx, cy = rng.uniform(-5, 5), rng.uniform(-5, 5)
            rx, ry = rng.uniform(0.3, 1.1), rng.uniform(0.25, 0.8)
            rot, seed = rng.uniform(0, math.pi), rng.uniform(0, 6)
            pts = []
            for i in range(16):
                a = 2 * math.pi * i / 16
                k = 1 + 0.25 * math.sin(3 * a + seed) + 0.12 * math.sin(5 * a + 2 * seed)
                x, y = rx * k * math.cos(a), ry * k * math.sin(a)
                pts.append((cx + x * math.cos(rot) - y * math.sin(rot),
                            cy + x * math.sin(rot) + y * math.cos(rot)))
            flat(bm, pts, 'grime', Z + rng.uniform(0, 0.002))
        for _ in range(3):  # tyre scuffs: thin curved strips
            cx, cy, r = rng.uniform(-3, 3), rng.uniform(-3, 3), rng.uniform(3, 6)
            a0 = rng.uniform(0, 2 * math.pi)
            for side in (-0.35, 0.35):
                pts_o, pts_i = [], []
                for i in range(9):
                    a = a0 + 0.9 * i / 8
                    for rr, lst in ((r + side + 0.09, pts_o), (r + side - 0.09, pts_i)):
                        lst.append((cx + rr * math.cos(a), cy + rr * math.sin(a)))
                flat(bm, pts_o + pts_i[::-1], 'grime', Z + 0.003)
    ob = core.obj_from_bm(name, bm, weighted=False, angle=40)
    finish(name, [ob], budget=KIT, car=(0.0, -6.0, 0.0))


for n in ('bay_marking', 'walkway_line', 'stains', 'start_line'):
    build(n)
