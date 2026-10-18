"""Plywood kicker (kit): an 18 mm plywood sheet (2.44 × 1.22 m = 10.8 × 5.4 u) leaning on the
sandbox rim — 1.18 u rise over 5.29 u (12.6°), rising toward the front (+Y here, Bevy -Z) —
propped on two brick stacks so it does not sag (catalog: garden_kicker, a Wood wedge).

    blender -b -P tools/blender/props/garden/kicker.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

W, RUN, TOP, T = 10.8, 5.29, 1.18, 0.08
ANG = math.atan2(TOP, RUN)

start('plywood', 'brick')
bm = core.new_bm()
# the sheet: its top surface runs from (y = -RUN/2, z = 0) to (y = +RUN/2, z = TOP)
fs = geo.box(bm, V((0, 0, 0)), (W, math.hypot(RUN, TOP), T), mi('plywood'))
vs = list({v for f in fs for v in f.verts})
bmesh.ops.transform(bm, matrix=Matrix.Translation(V((0, 0, TOP / 2)))
                    @ Matrix.Rotation(ANG, 4, 'X') @ Matrix.Translation(V((0, 0, -T / 2))), verts=vs)
box_uv(bm, fs, scale=0.25, along=0)
sheet = part('sheet', bm, bevel=0.015, segments=1)

# two brick stacks under the upper third (bricks 0.95 × 0.46 × 0.29 u, slightly irregular)
bricks = core.new_bm()
B = mi('brick')
y = RUN / 2 - 1.6
z_under = (y + RUN / 2) * TOP / RUN - T / math.cos(ANG)
rng = random.Random(3)
for x in (-3.2, 3.0):
    z, n = 0.0, 0
    while z + 0.29 <= z_under + 0.01 and n < 4:
        geo.box(bricks, V((x + rng.uniform(-0.05, 0.05), y + rng.uniform(-0.05, 0.05), z + 0.145)),
                (0.95, 0.46, 0.29) if n % 2 == 0 else (0.46, 0.95, 0.29), B)
        z += 0.29
        n += 1
box_uv(bricks, scale=0.8)
brick_ob = part('bricks', bricks, bevel=0.02, segments=2)

finish('kicker', [sheet, brick_ob], below=0.1, car=(0.0, -RUN / 2 - 1.8, 0.0))
