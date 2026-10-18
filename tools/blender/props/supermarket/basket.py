"""Shopping basket (kit, art bible §3.5): red plastic, 2.0 x 1.1 x 1.33 u, ribbed walls with
slots and two folding black handles. Knockable (light).

    blender -b -P tools/blender/props/supermarket/basket.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

start('red_plastic', 'black_plastic', 'steel_dark')
bm = core.new_bm()
L, D, H = 2.0, 1.33, 1.05
T = 0.06
geo.box(bm, V((0, 0, T / 2)), (L - 0.16, D - 0.16, T), mi('red_plastic'))
for s in (-1, 1):   # walls flare a little toward the rim
    geo.prism(bm, [(-T / 2, 0), (T / 2, 0), (T / 2, H), (-T / 2, H)],
              V((-L / 2 + 0.05, s * (D / 2 - 0.05), 0)), V((L / 2 - 0.05, s * (D / 2 - 0.05), 0)),
              mi('red_plastic'))
    geo.prism(bm, [(-T / 2, 0), (T / 2, 0), (T / 2, H), (-T / 2, H)],
              V((s * (L / 2 - 0.05), -D / 2 + 0.05, 0)), V((s * (L / 2 - 0.05), D / 2 - 0.05, 0)),
              mi('red_plastic'))
    for k in range(4):   # dark slots in the long walls
        x = -0.6 + k * 0.4
        geo.box(bm, V((x, s * (D / 2 - 0.02), 0.55)), (0.22, 0.02, 0.45), mi('steel_dark'))
for s in (-1, 1):       # handles folded down along the long sides
    geo.tube(bm, [V((-0.6, s * 0.75, H - 0.05)), V((-0.6, s * 0.8, H + 0.15)),
                  V((0.6, s * 0.8, H + 0.15)), V((0.6, s * 0.75, H - 0.05))], 0.05,
             mi('black_plastic'), n=6, corner=0.12)
propkit.finish(ENV, 'basket', [part('basket', bm, bevel=0.015, segments=1)], car=(2.2, 0.8, 200))
