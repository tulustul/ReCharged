"""Closeboard fence panel (decor backdrop at the garden bounds): 1.8 m (8 u) wide and tall,
overlapping vertical feather-edge boards on three rails (rails toward the garden, +Y here),
a concrete gravel board, timber posts with caps at both ends (shared with the neighbours).

    blender -b -P tools/blender/props/garden/fence.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

W, H = 8.0, 8.0
start('fence_wood', 'concrete', 'pine_dark')
F, C = mi('fence_wood'), mi('concrete')
bm = core.new_bm()
n = 12
for k in range(n):  # feather-edge boards, alternating overlap
    x = -W / 2 + (k + 0.5) * W / n
    board(bm, (x, -0.06 if k % 2 else -0.02, 0.6 + (H - 0.6) / 2), (W / n + 0.12, 0.08, H - 0.6),
          F, axis=2, seed=k)
for z in (1.3, 4.3, 7.3):  # rails on the garden side
    board(bm, (0, 0.12, z), (W - 0.4, 0.2, 0.42), F, axis=0, seed=int(z * 3))
board(bm, (0, 0.0, 0.3), (W - 0.4, 0.25, 0.6), C, axis=0, seed=99)      # gravel board
for sx in (-1, 1):
    board(bm, (sx * W / 2, 0, (H + 0.4) / 2), (0.45, 0.45, H + 0.4), mi('pine_dark'), axis=2,
          seed=50 + sx)
    geo.box(bm, V((sx * W / 2, 0, H + 0.48)), (0.6, 0.6, 0.16), mi('pine_dark'))
finish('fence', [part('fence', bm, bevel=0.02, segments=1)], car=(0.0, 2.5, 0.0))
