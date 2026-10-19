"""Plywood kicker (kit): an 18 mm plywood sheet (10.8 u wide) leaning on the sandbox rim at
12.6° up to 1.18 u, rising toward the front (+Y here, Bevy -Z), with a 2.5 u lead-in board at
half the slope in front of it (6.54 u run in all), the sheet propped on two brick stacks so it
does not sag (catalog: garden_kicker, two Wood wedges). Straight onto the 12.6° sheet at 30 u/s
the cars' suspension bottomed out (T19a).

    blender -b -P tools/blender/props/garden/kicker.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

W, TOP, T = 10.8, 1.18, 0.08
SLOPE = TOP / 5.29                       # the sheet's slope (12.6°)
TOE = 2.5                                # lead-in board run (half the slope)
TOE_TOP = TOE * SLOPE / 2
RUN = TOE + (TOP - TOE_TOP) / SLOPE      # 6.54 u: foot at y = -RUN/2, rim at +RUN/2
ANG = math.atan(SLOPE)
Y_JOINT = -RUN / 2 + TOE


def board(name, ya, za, yb, zb):
    """A plywood board whose top surface runs from (y = ya, z = za) to (y = yb, z = zb)."""
    bm = core.new_bm()
    fs = geo.box(bm, V((0, 0, 0)), (W, math.hypot(yb - ya, zb - za), T), mi('plywood'))
    vs = list({v for f in fs for v in f.verts})
    bmesh.ops.transform(bm, matrix=Matrix.Translation(V((0, (ya + yb) / 2, (za + zb) / 2)))
                        @ Matrix.Rotation(math.atan2(zb - za, yb - ya), 4, 'X')
                        @ Matrix.Translation(V((0, 0, -T / 2))), verts=vs)
    box_uv(bm, fs, scale=0.25, along=0)
    return part(name, bm, bevel=0.015, segments=1)


start('plywood', 'brick')
toe = board('toe', -RUN / 2, 0.0, Y_JOINT, TOE_TOP)
sheet = board('sheet', Y_JOINT, TOE_TOP, RUN / 2, TOP)

# two brick stacks under the upper third (bricks 0.95 × 0.46 × 0.29 u, slightly irregular)
bricks = core.new_bm()
B = mi('brick')
y = RUN / 2 - 1.6
z_under = TOE_TOP + (y - Y_JOINT) * SLOPE - T / math.cos(ANG)
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

finish('kicker', [toe, sheet, brick_ob], below=0.1, car=(0.0, -RUN / 2 - 1.8, 0.0))
