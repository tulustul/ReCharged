"""Coffee table (hero): square light-oak table 1.25 x 1.25 m, 35 cm high (5.55 x 5.55 x 1.55 u)
on tapered legs, with a lower shelf holding books and magazines. The bare top is a drivable jump
deck between the two book-stack ramps (catalog: top box 1.37..1.55 + a solid block under it).

    blender -b -P tools/blender/props/house/coffee_table.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

S, H, TOP = 5.55, 1.55, 0.18      # size, height, top thickness
LEG_AT, SHELF_Z = 2.45, 0.42
core.reset()
materials('oak', 'oak_end', 'pages', 'book_red', 'book_blue', 'book_cream', 'book_green',
          'book_mustard', 'ceramic')

# top: thick slab with soft edges, grain along x
bm = core.new_bm()
geo.box(bm, V((0, 0, H - TOP / 2)), (S, S, TOP), mi('oak'))
top = core.obj_from_bm('top', box_uv(bm, 'x'), bevel=0.045, segments=3)

# apron under the top (inset) and the lower shelf
bm = core.new_bm()
a_in, a_h, a_t = S / 2 - 0.42, 0.22, 0.1
for s in (-1, 1):
    geo.box(bm, V((0, s * a_in, H - TOP - a_h / 2)), (2 * a_in, a_t, a_h), mi('oak'))
    geo.box(bm, V((s * a_in, 0, H - TOP - a_h / 2)), (a_t, 2 * a_in - a_t, a_h), mi('oak'))
geo.box(bm, V((0, 0, SHELF_Z)), (2 * LEG_AT, 2 * LEG_AT, 0.08), mi('oak'))
frame = core.obj_from_bm('frame', box_uv(bm, 'x'), bevel=0.015, segments=2)

# tapered round legs, slightly splayed outward
bm = core.new_bm()
for sx in (-1, 1):
    for sy in (-1, 1):
        leg(bm, (sx * LEG_AT, sy * LEG_AT, 0), H - TOP + 0.01, 0.075, 0.115, mi('oak'), n=14,
            splay=(sx * 0.06, sy * 0.06))
legs = core.obj_from_bm('legs', box_uv(bm, 'z'), weighted=False, angle=60)

# shelf dressing: a stack of art books, a slipping magazine pile, a ceramic bowl
bm = core.new_bm()
z = SHELF_Z + 0.04
for i, (w, d, t, col) in enumerate([(1.35, 1.1, 0.17, 'book_cream'), (1.2, 0.95, 0.15, 'book_blue'),
                                    (1.1, 0.9, 0.13, 'book_red')]):
    c = V((-1.1 + 0.05 * i, 0.6 - 0.04 * i, z + t / 2))
    geo.box(bm, c, (w, d, t), mi(col))
    geo.box(bm, c + V((0.03, 0, 0)), (w - 0.04, d + 0.006, t - 0.05), mi('pages'))
    z += t
for i, col in enumerate(['book_mustard', 'book_green', 'book_cream']):
    c = V((1.2 + 0.08 * i, -0.9 + 0.1 * i, SHELF_Z + 0.04 + 0.03 + 0.06 * i))
    geo.box(bm, c, (1.0, 1.3, 0.05), mi(col))
books = core.obj_from_bm('books', bm, bevel=0.012, segments=2)
bm = core.new_bm()
geo.revolve_x  # (bowl below is a revolved profile around z: built from cylinders)
geo.cylinder(bm, V((1.3, 1.2, SHELF_Z + 0.04)), V((1.3, 1.2, SHELF_Z + 0.3)), 0.3,
             mi('ceramic'), n=24, r1=0.45)
bowl = core.obj_from_bm('bowl', bm, weighted=False, angle=50)

done('coffee_table', [top, frame, legs, books, bowl], budget=HERO, car=(4.6, -1.8, 30))
