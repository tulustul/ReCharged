"""Book-stack ramp (hero): the kids' jump ramp up onto the coffee table. Sixteen big atlases and
art books lie on the slope (their covers are the driving surface, exactly on the catalog wedge),
propped on stepped stacks of books; two magazines bridge the toe. 5.5 u wide, 8.0 u long, rising
1.55 u (11 deg, gentle: boosted cars must not fly over the next checkpoint gate) towards the
front (+Y in Blender = -Z in game, the high end).

    blender -b -P tools/blender/props/house/book_ramp.py -- [--preview DIR]
"""
import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

W, L, H = 5.5, 8.0, 1.55
THETA = math.atan2(H, L)
COLS = 4
TOP_T = 0.16                     # slanted book thickness
LAYER = 0.161                    # support layer thickness
rnd = random.Random(16)
core.reset()
materials('pages', *BOOK_COLOURS)


def z_plane(y):
    return (y + L / 2) * H / L


# --- slanted top books: the driving surface ---------------------------------------------------
bm = core.new_bm()
cw = W / COLS
toe_y = -L / 2 + TOP_T * L / H  # the top face starts at z = TOP_T (book bottom on the floor)
slope_len = (L / 2 - toe_y) / math.cos(THETA)
up = V((0, math.cos(THETA), math.sin(THETA)))
nrm = V((0, -math.sin(THETA), math.cos(THETA)))
for c in range(COLS):
    x = -W / 2 + cw * (c + 0.5)
    lens = [slope_len / 4] * 4
    d = 0.0
    for k, ln in enumerate(lens):
        mid = d + ln / 2
        top = V((x + rnd.uniform(-0.03, 0.03), toe_y, z_plane(toe_y))) + up * mid
        centre = top - nrm * (TOP_T / 2)
        M = frame_m(centre, yaw=rnd.uniform(-1.5, 1.5) + (180 if (c + k) % 2 else 0),
                    pitch=math.degrees(THETA) * (1 if (c + k) % 2 == 0 else -1))  # yaw 180 flips it
        book(bm, M, (cw - 0.035, ln - 0.03, TOP_T), BOOK_COLOURS[(3 * c + 2 * k) % 8],
             board=0.025)
        d += ln
top_books = core.obj_from_bm('top_books', bm, bevel=0.012, segments=2, angle=40)

# --- stepped support stacks under the slanted books --------------------------------------------
bm = core.new_bm()
for c in range(COLS):
    x0 = -W / 2 + cw * c
    k = 0
    while True:
        z_top = (k + 1) * LAYER
        y_k = (z_top + TOP_T / math.cos(THETA)) * L / H - L / 2
        if L / 2 - 0.05 - y_k < 0.45:
            break
        y = y_k + 0.02
        while L / 2 - 0.05 - y > 0.45:
            ln = min(rnd.uniform(1.1, 1.9), L / 2 - 0.05 - y)
            if L / 2 - 0.05 - (y + ln) < 0.5:
                ln = L / 2 - 0.05 - y
            w = cw - rnd.uniform(0.04, 0.16)
            xc = x0 + cw / 2 + rnd.uniform(-0.04, 0.04)
            M = frame_m((xc, y + ln / 2, z_top - LAYER / 2), yaw=(0 if rnd.random() < 0.5 else 180)
                        + rnd.uniform(-2, 2))
            flat_book(bm, M, (w, ln - 0.03, LAYER - 0.006), rnd.choice(BOOK_COLOURS))
            y += ln
        k += 1
stack = core.obj_from_bm('stack', bm, bevel=0.01, segments=1, angle=40)

# --- two glossy magazines fanned over the toe --------------------------------------------------
bm = core.new_bm()
for i, (col, dx, dyaw) in enumerate([('book_teal', -1.2, -6), ('book_red', 1.0, 4)]):
    ln = 1.25
    a = math.atan2(TOP_T + 0.02, ln * 0.85)
    M = frame_m((dx, -L / 2 + ln / 2 * math.cos(a) + 0.05, ln / 2 * math.sin(a) + 0.012 * i),
                yaw=dyaw, pitch=math.degrees(a))
    obox(bm, M, (1.9, ln, 0.035), mi(col))
mags = core.obj_from_bm('magazines', bm, bevel=0.008, segments=1)

done('book_ramp', [top_books, stack, mags], budget=HERO, car=(-1.2, -6.2, 0))
