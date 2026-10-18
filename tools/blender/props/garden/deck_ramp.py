"""Decking ramp (kit): 10 u wide, 2.0 u rise over 7.5 u (15°) up to the deck, rising toward the
front (+Y here, Bevy -Z): decking boards across three stringers, closed sides
(catalog: garden_deck_ramp, a Wood wedge).

    blender -b -P tools/blender/props/garden/deck_ramp.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

W, RUN, RISE = 10.0, 7.5, 2.0
ANG = math.atan2(RISE, RUN)
SLOPE = math.hypot(RUN, RISE)
BOARD, GAP, T = 0.58, 0.05, 0.11

start('deck_wood', 'pine_dark')
D, P = mi('deck_wood'), mi('pine_dark')
bm = core.new_bm()
tilt = Matrix.Translation(V((0, -RUN / 2, 0))) @ Matrix.Rotation(ANG, 4, 'X')
n = int(SLOPE / (BOARD + GAP))
for k in range(n):
    s = BOARD / 2 + k * (SLOPE - BOARD) / (n - 1)   # distance up the slope (flush at both ends)
    fs = geo.box(bm, V((0, s, -T / 2)), (W, BOARD, T), D)
    bmesh.ops.transform(bm, matrix=tilt, verts=list({v for f in fs for v in f.verts}))
    board_uv(bm, fs, tilt @ V((-W / 2, s, 0)), V((1, 0, 0)), seed=k)
boards = part('boards', bm, bevel=0.025, segments=1)

frame = core.new_bm()
pts = [(-RUN / 2 + 0.3, 0.0), (RUN / 2, 0.0), (RUN / 2, RISE - T - 0.005)]
for x, t in ((-W / 2 + 0.06, 0.12), (0.0, 0.3), (W / 2 - 0.06, 0.12)):  # sides + middle stringer
    geo.extrude_x(frame, pts, x - t / 2, x + t / 2, P)
box_uv(frame, scale=0.5, along=1)
sides = part('frame', frame, bevel=0.02, segments=1)
finish('deck_ramp', [boards, sides], below=0.12, car=(0.0, -RUN / 2 - 2.0, 0.0))
