"""Low bookcases (two variants, a and b): white-painted, 1.6 m x 35 cm x 90 cm (7.1 x 1.55 x
4.0 u), three bays with two shelves, packed with books (standing, leaning, stacked), a basket
or a plant in one bay and a few things on top. Room dividers along the track; the back panel is
finished too. Front (+Y). Collider: one block.

    blender -b -P tools/blender/props/house/bookcase.py -- [--preview DIR]   (builds both)
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

L, D, H, BOARD = 7.1, 1.55, 4.0, 0.1
# flat_book axes -> a standing book: width (x) runs back (-y) so the spine faces the front,
# height (y) up, thickness (z) along -x
from mathutils import Matrix  # noqa: E402
STAND = Matrix(((0, 0, -1, 0), (-1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 0, 1)))


def build(name, seed):
    rnd = random.Random(seed)
    core.reset()
    materials('paint_white', 'oak', 'pages', *BOOK_COLOURS, 'terracotta', 'leaf', 'fabric_grey',
              'ceramic', 'brass')
    white = mi('paint_white')
    bm = core.new_bm()
    geo.box(bm, V((0, 0, H - BOARD / 2)), (L, D, BOARD), mi('oak'))  # oak top
    for x in (-L / 2 + BOARD / 2, -L / 6, L / 6, L / 2 - BOARD / 2):
        geo.box(bm, V((x, 0, (H - BOARD) / 2)), (BOARD, D, H - BOARD), white)
    for z in (0.2, 1.45, 2.7):
        geo.box(bm, V((0, 0, z)), (L - 0.05, D - 0.05, BOARD), white)
    geo.box(bm, V((0, -D / 2 + 0.03, (H - BOARD) / 2)), (L - 0.05, 0.06, H - BOARD), white)
    geo.box(bm, V((0, D / 2 - 0.06, 0.08)), (L - 0.1, 0.06, 0.16), white)  # kick plate
    case = core.obj_from_bm('case', box_uv(bm, 'x'), bevel=0.015, segments=1)
    bm = core.new_bm()
    bay_w = L / 3 - BOARD
    special = rnd.randrange(9)
    for bay in range(3):
        x0 = -L / 2 + BOARD + bay * (L / 3)
        for shelf, z0 in enumerate((0.25, 1.5, 2.75)):
            z_floor = z0 + BOARD / 2
            room = (1.2 if shelf < 2 else H - BOARD - z_floor) - 0.06
            if bay * 3 + shelf == special:  # a woven basket / a little plant instead
                if shelf < 2:
                    geo.cylinder(bm, V((x0 + bay_w / 2, 0.05, z_floor)), V((x0 + bay_w / 2, 0.05,
                                 z_floor + 0.8)), 0.62, mi('fabric_grey'), n=16, r1=0.66)
                continue
            x = x0 + 0.04
            while x < x0 + bay_w - 0.15:
                r = rnd.random()
                if r < 0.12 and x < x0 + bay_w - 1.0:  # a horizontal stack
                    z = z_floor
                    for _ in range(rnd.randint(2, 4)):
                        t = rnd.uniform(0.1, 0.16)
                        w = rnd.uniform(0.75, 0.95)
                        M = frame_m((x + w / 2, 0.08, z + t / 2), yaw=90 + rnd.uniform(-4, 4))
                        flat_book(bm, M, (rnd.uniform(1.0, 1.25), w, t), rnd.choice(BOOK_COLOURS))
                        z += t
                    x += 1.0
                    continue
                t = rnd.uniform(0.09, 0.2)
                h = min(rnd.uniform(0.75, 1.12), room)
                d = rnd.uniform(0.95, 1.3)
                lean = 0.0
                if r > 0.93 and x > x0 + 0.3:
                    lean = rnd.uniform(8, 16)
                M = frame_m((x + t / 2, D / 2 - 0.1 - d / 2, z_floor + h / 2), roll=-lean)
                # standing book: spine (local -x of flat_book) faces the front (+y)
                flat_book(bm, M @ STAND, (d, h, t), rnd.choice(BOOK_COLOURS))
                x += t + (0.05 if lean else 0.004)
    # things on top
    geo.cylinder(bm, V((-2.4, 0, H)), V((-2.4, 0, H + 0.62)), 0.36, mi('terracotta'), n=16,
                 r1=0.44)
    for k in range(6):
        geo.sphere(bm, V((-2.4 + 0.28 * ((k % 3) - 1), 0.2 * (k // 3 - 0.5), H + 0.85 + 0.1 * (k % 2))),
                   0.3, mi('leaf'), n=8, rings=5, scale=(1.0, 1.0, 0.75))
    for i in range(3):
        c = V((1.6, 0.1, H + 0.07 + 0.12 * i))
        flat_book(bm, frame_m(c, yaw=rnd.uniform(-8, 8)), (1.2, 0.9, 0.11), rnd.choice(BOOK_COLOURS))
    geo.cylinder(bm, V((2.85, 0.1, H)), V((2.85, 0.1, H + 0.5)), 0.18, mi('ceramic'), n=12)
    books = core.obj_from_bm('books', bm, angle=50)
    done(name, [case, books], car=(0.0, 3.0, 180))


build('bookcase', 3)
build('bookcase_b', 11)
