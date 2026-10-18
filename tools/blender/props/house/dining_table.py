"""Dining table (hero): a 2.6 x 0.9 m oak table, 75 cm high (11.5 x 4.0 x 3.33 u) on tapered
square legs; 2.8 u clear under the apron. A linen runner, a fruit bowl, a jug of flowers and two
place settings on top (out of the cars' reach).

    blender -b -P tools/blender/props/house/dining_table.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

L, D, H, TOP = 11.5, 4.0, 3.33, 0.18
core.reset()
materials('oak', 'fabric_cream', 'ceramic', 'terracotta', 'leaf', 'fabric_mustard', 'book_red')

bm = core.new_bm()
geo.box(bm, V((0, 0, H - TOP / 2)), (L, D, TOP), mi('oak'))
top = core.obj_from_bm('top', box_uv(bm, 'x'), bevel=0.05, segments=3)

bm = core.new_bm()
ax, ay, ah = L / 2 - 0.45, D / 2 - 0.4, 0.36
for s in (-1, 1):
    geo.box(bm, V((0, s * ay, H - TOP - ah / 2)), (2 * ax, 0.12, ah), mi('oak'))
    geo.box(bm, V((s * ax, 0, H - TOP - ah / 2)), (0.12, 2 * ay, ah), mi('oak'))
for sx in (-1, 1):
    for sy in (-1, 1):  # square legs tapering towards the floor
        geo.prism(bm, geo.rrect(0.31, 0.31, 0.04, 2), V((sx * 5.35, sy * 1.65, H - TOP)),
                  V((sx * 5.35, sy * 1.65, 0.0)), mi('oak'), up=V((1, 0, 0)), scale1=0.7)
frame = core.obj_from_bm('frame', box_uv(bm, 'z'), bevel=0.012, segments=1)

# table dressing
bm = core.new_bm()
geo.box(bm, V((0, 0, H + 0.006)), (L - 2.4, 1.5, 0.012), mi('fabric_cream'))  # runner
geo.cylinder(bm, V((1.8, 0.1, H)), V((1.8, 0.1, H + 0.32)), 0.45, mi('ceramic'), n=20, r1=0.75)
for i, (dx, dy) in enumerate([(-0.25, 0.1), (0.2, -0.15), (0.05, 0.3)]):
    geo.sphere(bm, V((1.8 + dx, 0.1 + dy, H + 0.42)), 0.22, mi('fabric_mustard' if i else 'book_red'),
               n=10, rings=6)
geo.cylinder(bm, V((-1.6, -0.1, H)), V((-1.6, -0.1, H + 0.75)), 0.3, mi('terracotta'), n=16,
             r1=0.22)
for k in range(7):  # a loose bunch of stems and leaves
    a = 2 * math.pi * k / 7
    tip = V((-1.6 + 0.45 * math.cos(a), -0.1 + 0.45 * math.sin(a), H + 1.55 + 0.12 * (k % 3)))
    geo.cylinder(bm, V((-1.6, -0.1, H + 0.7)), tip, 0.025, mi('leaf'), n=5)
    geo.sphere(bm, tip, 0.14, mi('leaf'), n=8, rings=5, scale=(1.0, 1.0, 0.6))
for sx in (-1, 1):  # two place settings
    geo.cylinder(bm, V((sx * 3.6, sx * 0.9, H)), V((sx * 3.6, sx * 0.9, H + 0.05)), 0.62,
                 mi('ceramic'), n=24)
dress = core.obj_from_bm('dress', bm, weighted=False, angle=50)

done('dining_table', [top, frame, dress], budget=HERO, car=(0.0, -3.2, 10))
