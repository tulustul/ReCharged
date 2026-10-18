"""Pennant bunting (decor, out of reach): 15 triangular flags in toy colours on a string spanning
30 u with a 1.6 u sag. Origin at the middle of the span at the attachment height (the ends);
everything hangs below it. Placed by assets/data/decor/toy_town.ron near the ceiling.

    blender -b -P tools/blender/props/toy/bunting.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

SPAN, SAG, N = 30.0, 1.6, 15
core.reset()
COLS = [RED, YELLOW, BLUE, GREEN, '#F2F0EA']
for k, c in enumerate(COLS):
    fabric(f'flag{k}', c, 0.8)
plastic('string', '#F2F0EA', 0.6)
mi = core.mi


def sag(x):
    return -SAG * (1 - (2 * x / SPAN) ** 2)


bm = core.new_bm()
geo.tube(bm, [V((x, 0, sag(x))) for x in (SPAN * (k / 24 - 0.5) for k in range(25))], 0.035,
         mi('string'), n=5)
for k in range(N):
    x = SPAN * ((k + 0.5) / N - 0.5)
    a, b = V((x - 0.55, 0, sag(x - 0.55))), V((x + 0.55, 0, sag(x + 0.55)))
    tip = (a + b) / 2 + V((0, 0, -1.25))
    geo.loft(bm, [[a + V((0, -0.02, 0)), b + V((0, -0.02, 0)), tip + V((0, -0.02, 0))],
                  [a + V((0, 0.02, 0)), b + V((0, 0.02, 0)), tip + V((0, 0.02, 0))]],
             mi(f'flag{k % len(COLS)}'))
propkit.finish(ENV, 'bunting', [smooth('bunting', bm)], below=3.0, car=(0.0, 4.0, 180),
               before_preview=lambda ob: setattr(ob.location, 'z', 5.0))
