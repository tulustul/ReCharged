"""Plywood plank: a 2.4 m sheet cut to 80 cm (10.8 x 3.6 u, 2 cm thick) leaning from the hall
floor onto the stair landing (rises 2.3 u over 10.53 u, 12.3 deg, towards +Y = the front).
Its top face lies on the catalog wedge (a shim fills the toe); grip tape and a chalk arrow.

    blender -b -P tools/blender/props/house/plank.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

W, RUN, RISE, T = 3.6, 10.53, 2.3, 0.09
A = math.atan2(RISE, RUN)
SLOPE = math.hypot(RUN, RISE)
core.reset()
materials('plywood', 'plywood_edge', 'rubber', 'plastic_white')
up = V((0, math.cos(A), math.sin(A)))
nrm = V((0, -math.sin(A), math.cos(A)))
start = T / math.tan(A)               # the top face starts T above the floor (board on the floor)
length = (SLOPE - T / math.sin(A))

bm = core.new_bm()
centre = V((0, -RUN / 2 + start, T)) + up * (length / 2) - nrm * (T / 2)
fs = obox(bm, frame_m(centre, pitch=math.degrees(A)), (W, length, T), mi('plywood'))
for f in fs:
    if abs(f.normal.dot(nrm)) < 0.5:
        f.material_index = mi('plywood_edge')
y0 = -RUN / 2 + start
geo.extrude_x(bm, [(-RUN / 2, 0.0), (y0 + 0.05, 0.0), (y0 + 0.05, T * 1.05)], -W / 2 + 0.1,
              W / 2 - 0.1, mi('plywood_edge'))  # shim under the foot: no bump onto the board
board = core.obj_from_bm('board', box_uv(bm, 'y'), bevel=0.012, segments=1)

bm = core.new_bm()
for d in (1.6, length - 1.4):
    c = V((0, -RUN / 2 + start, T)) + up * d + nrm * 0.004
    obox(bm, frame_m(c, pitch=math.degrees(A)), (W - 0.5, 0.45, 0.01), mi('rubber'))
# chalk arrow pointing up the slope
mid = V((0, -RUN / 2 + start, T)) + up * (length * 0.55) + nrm * 0.004
M = frame_m(mid, pitch=math.degrees(A))
obox(bm, M, (0.35, 2.2, 0.008), mi('plastic_white'), center=(0, -0.6, 0))
for s in (-1, 1):
    obox(bm, M @ frame_m((s * 0.42, 0.75, 0), yaw=-s * 40), (0.3, 1.2, 0.008), mi('plastic_white'))
marks = core.obj_from_bm('marks', bm, weighted=False)

done('plank', [board, marks], car=(3.6, -3.0, 20), below=0.0)
