"""A bunch of helium balloons pressed against the ceiling (decor, out of reach): six glossy
balloons with knots and curly ribbons hanging about 4 u down. Origin at the ceiling contact;
everything is below it. Placed by a map's decor.ron at the ceiling height.

    blender -b -P tools/blender/props/toy/balloons.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
COLS = [RED, YELLOW, BLUE, GREEN, PINK, PURPLE]
for k, c in enumerate(COLS):
    plastic(f'balloon{k}', c, 0.15)
plastic('ribbon', '#F2F0EA', 0.5)
mi = core.mi
# (x, y, radius) in plan; the tallest-squashed ones touch the ceiling
SPOTS = [(0, 0, 0.8), (1.45, 0.3, 0.72), (-1.3, 0.55, 0.75), (0.4, -1.4, 0.7), (-0.9, -1.1, 0.68),
         (0.6, 1.5, 0.66)]
balloons, ribbons = core.new_bm(), core.new_bm()
for k, (x, y, r) in enumerate(SPOTS):
    c = V((x, y, -r * 1.18))
    geo.sphere(balloons, c, r, mi(f'balloon{k}'), n=14, rings=9, scale=(1, 1, 1.18))
    knot = c - V((0, 0, r * 1.18))
    geo.cylinder(balloons, knot + V((0, 0, 0.05)), knot - V((0, 0, 0.12)), 0.1,
                 mi(f'balloon{k}'), n=6, r1=0.04)
    geo.helix(ribbons, knot - V((0, 0, 0.1)), knot - V((0.25 * (k % 3 - 1), 0.2, 4.0)), 0.09,
              0.02, 9 + k, mi('ribbon'), n=3, steps_per_turn=6)
propkit.finish(ENV, 'balloons', [smooth('balloons', balloons), smooth('ribbons', ribbons)],
               below=6.0, car=(3.0, 3.0, 200), before_preview=lambda ob: setattr(ob.location, 'z', 7.0))
