"""Garden bench (hero): a weathered hardwood three-seater, 1.9 m long (8.4 u), seat at 0.45 m
(2.0 u), slatted seat and back, flat armrests. Nothing spans the 7.4 u between the end frames
below 1.62 u, so the cars drive under the seat front to back (catalog: garden_bench).

    blender -b -P tools/blender/props/garden/bench.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

L, SEAT, ARM, TOP = 8.4, 2.0, 2.9, 3.9   # length, seat top, armrest top, backrest top
FX = 3.95                                # end-frame centre (|x|)
LEG = 0.3                                # leg section
TILT = math.radians(12)                  # backrest lean

start('bench_wood', 'steel_dark')
W = mi('bench_wood')
bm = new = core.new_bm()
seed = 0


def nb(centre, size, axis):
    global seed
    seed += 1
    board(bm, centre, size, W, axis=axis, seed=seed)


def back_point(z):
    """Centre line of the leaning back legs (y) at height z."""
    return -1.2 - max(0.0, z - 1.9) * math.tan(TILT)


for sx in (-1, 1):
    x = sx * FX
    nb((x, 1.0, 1.375), (LEG, LEG, 2.75), 2)                      # front leg
    nb((x, -1.2, 0.95), (LEG, LEG, 1.9), 2)                       # back leg, lower part
    # leaning upper back leg as a tilted board
    p0, p1 = V((x, back_point(1.9), 1.9)), V((x, back_point(TOP), TOP))
    fs = geo.prism(bm, geo.rrect(LEG, LEG, 0.0, 1), p0 - V((0, 0, 0.15)), p1, W)
    board_uv(bm, fs, p0, (p1 - p0).normalized(), seed=sx + 10)
    nb((x, -0.05, 1.74), (0.26, 2.3, 0.24), 1)                     # seat side rail
    nb((x, -0.1, 0.55), (0.22, 2.2, 0.2), 1)                       # low stretcher
    nb((x, 0.05, ARM - 0.07), (0.42, 2.75, 0.14), 1)               # armrest
# seat slats along the bench, resting on the side rails
for k in range(6):
    y = -1.0 + k * 0.405
    nb((0, y, SEAT - 0.07), (L - 0.3, 0.33, 0.14), 0)
nb((0, 1.06, 1.74), (2 * FX - 0.28, 0.12, 0.24), 0)              # front apron
nb((0, -1.12, 1.74), (2 * FX - 0.28, 0.12, 0.24), 0)             # back apron
# backrest: bottom and top rails + vertical slats, all leaning back with the back legs
for z0, z1 in ((2.15, 2.4), (3.55, TOP)):
    zc = (z0 + z1) / 2
    c = V((0, back_point(zc), zc))
    fs = geo.box(bm, V((0, 0, 0)), (2 * FX - 0.28, 0.13, z1 - z0), W)
    bmesh.ops.transform(bm, matrix=Matrix.Translation(c) @ Matrix.Rotation(-TILT, 4, 'X'),
                        verts=list({v for f in fs for v in f.verts}))
    board_uv(bm, fs, c - V((FX, 0, 0)), V((1, 0, 0)), seed=int(z0 * 10))
n_sl = 15
for k in range(n_sl):
    x = -FX + 0.55 + k * (2 * FX - 1.1) / (n_sl - 1)
    zc = (2.4 + 3.55) / 2
    c = V((x, back_point(zc) + 0.02, zc))
    fs = geo.box(bm, V((0, 0, 0)), (0.24, 0.08, 3.55 - 2.4 + 0.1), W)
    bmesh.ops.transform(bm, matrix=Matrix.Translation(c) @ Matrix.Rotation(-TILT, 4, 'X'),
                        verts=list({v for f in fs for v in f.verts}))
    board_uv(bm, fs, c, V((0, -math.sin(TILT), math.cos(TILT))), seed=40 + k)
wood = part('wood', bm, bevel=0.03, segments=2)

# carriage-bolt heads where rails meet the legs
bolts = core.new_bm()
S = mi('steel_dark')
for sx in (-1, 1):
    for y, z in ((1.0, 1.74), (-1.2, 1.74), (1.0, 0.55), (-1.2, 0.55)):
        x = sx * (FX + LEG / 2 + 0.005)
        geo.cylinder(bolts, V((x, y, z)), V((x + sx * 0.04, y, z)), 0.07, S, n=10)
bolt_ob = smooth('bolts', bolts)

finish('bench', [wood, bolt_ob], hero=True, car=(0.0, -0.6, 0.0))
