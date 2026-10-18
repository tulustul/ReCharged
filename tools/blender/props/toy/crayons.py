"""Jumbo crayon pack: an open green-and-yellow cardboard box with crayon tips poking out and a
fan of loose crayons on the floor in front (real size, 11 cm crayons). Static.

    blender -b -P tools/blender/props/toy/crayons.py -- [--preview DIR]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
cardboard('pack', '#3E9B4F', 0.8)
cardboard('pack_y', '#F2C230', 0.8)
COLS = [RED, BLUE, YELLOW, GREEN, ORANGE, PURPLE, PINK, '#7A4A2A']
for k, c in enumerate(COLS):
    core.material(f'wax{k}', c, 0.0, 0.5)
plastic('band', '#1D1D20', 0.6)
mi = core.mi
pack, wax = core.new_bm(), core.new_bm()
L, R = 0.5, 0.05  # crayon length and radius (jumbo)
PW, PD, PH = 0.66, 0.26, 0.56  # upright open pack
geo.box(pack, V((0, 0, PH / 2)), (PW, PD, PH), mi('pack'))
geo.box(pack, V((0, PD / 2 + 0.005, PH * 0.42)), (PW * 0.86, 0.012, PH * 0.42), mi('pack_y'))
geo.box(pack, V((0, PD / 2 + 0.11, PH + 0.08)), (PW, 0.2, 0.015), mi('pack_y'))   # open lid


def crayon(p, d, k):
    d = V(d).normalized()
    a, b = V(p), V(p) + d * L
    geo.cylinder(wax, a, b, R, mi(f'wax{k % len(COLS)}'), n=10)
    geo.cylinder(wax, b, b + d * 0.1, R, mi(f'wax{k % len(COLS)}'), n=10, r1=0.012)
    geo.cylinder(wax, a + d * 0.1, a + d * 0.16, R * 1.06, mi('band'), n=10)


for k in range(6):  # standing in the pack, tips poking out of the open top
    x = -0.25 + k * 0.1
    crayon((x, 0.0, PH - 0.42 + 0.06 * (k % 2)), (0.03 * (k - 2.5), 0.0, 1), k)
rnd = random.Random(5)
for k in range(7):  # loose crayons fanned out on the floor in front
    a = math.radians(-60 + k * 20 + rnd.uniform(-6, 6))
    p = V((0.12 * math.sin(a), 0.4 + 0.1 * math.cos(a), R))
    crayon(p, (math.sin(a), math.cos(a), 0), k + 3)
propkit.finish(ENV, 'crayons', [part('pack', pack, bevel=0.008, segments=1), smooth('wax', wax)],
               car=(1.3, 0.8, -20))
