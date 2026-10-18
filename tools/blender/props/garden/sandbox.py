"""Sandbox (hero): a kids' pine sandbox, 2.4 × 1.4 m (10.7 × 6.2 u), two stacked boards per side
with a flat cap rail at 1.18 u (the kickers' lip height), corner posts, triangular corner seats at
the back, raked sand with a little castle and spade marks just above the Dirt ground polygon at
1.0 that carries the physics (catalog: garden_sandbox; long side along x).

    blender -b -P tools/blender/props/garden/sandbox.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

LX, LY, T = 10.7, 6.2, 0.18      # outer size, board thickness
CAP_W, CAP_T, TOP = 0.34, 0.08, 1.18
SAND = 1.0

start('pine', 'sand', 'plastic_red', 'plastic_yellow')
P, SD = mi('pine'), mi('sand')
bm = core.new_bm()
k = 0
for sy in (-1, 1):            # long sides (along x)
    y = sy * (LY / 2 - T / 2)
    for z0 in (0.0, 0.55):
        k += 1
        board(bm, (0, y, z0 + 0.27), (LX, T, 0.53), P, axis=0, seed=k)
for sx in (-1, 1):            # short sides (along y), between the long ones
    x = sx * (LX / 2 - T / 2)
    for z0 in (0.0, 0.55):
        k += 1
        board(bm, (x, 0, z0 + 0.27), (T, LY - 2 * T, 0.53), P, axis=1, seed=k)
for sx in (-1, 1):            # corner posts inside the corners
    for sy in (-1, 1):
        k += 1
        board(bm, (sx * (LX / 2 - T - 0.2), sy * (LY / 2 - T - 0.2), 0.52),
              (0.4, 0.4, 1.04), P, axis=2, seed=k)
# cap rail, mitred look via overlapping caps
for sy in (-1, 1):
    k += 1
    board(bm, (0, sy * (LY / 2 - CAP_W / 2 + 0.04), TOP - CAP_T / 2), (LX + 0.08, CAP_W, CAP_T),
          P, axis=0, seed=k)
for sx in (-1, 1):
    k += 1
    board(bm, (sx * (LX / 2 - CAP_W / 2 + 0.04), 0, TOP - CAP_T / 2),
          (CAP_W, LY - 2 * CAP_W + 0.16, CAP_T), P, axis=1, seed=k)
# triangular corner seats on the back corners (-y = back)
for sx in (-1, 1):
    c = V((sx * (LX / 2 - 0.02), -(LY / 2 - 0.02), TOP))
    tri = [V((0, 0)), V((-sx * 1.9, 0)), V((0, 1.9))]
    pts = [c + V((p.x, p.y, 0)) for p in tri]
    vs0 = [bm.verts.new(p) for p in pts]
    vs1 = [bm.verts.new(p + V((0, 0, 0.12))) for p in pts]
    fs = [bm.faces.new(vs0), bm.faces.new(vs1)]
    for i in range(3):
        j = (i + 1) % 3
        fs.append(bm.faces.new((vs0[i], vs0[j], vs1[j], vs1[i])))
    for f in fs:
        f.material_index = P
    bmesh.ops.recalc_face_normals(bm, faces=fs)
    board_uv(bm, fs, c, V((-sx, 1, 0)).normalized(), seed=90 + sx)
wood = part('wood', bm, bevel=0.025, segments=2)

# raked sand: a gently lumpy surface 0.01-0.06 above the physics polygon, a small castle
sand = core.new_bm()
nx, ny = 26, 15
ix, iy = LX / 2 - T, LY / 2 - T
rng = random.Random(4)
bumps = [(rng.uniform(-ix, ix), rng.uniform(-iy, iy), rng.uniform(0.6, 1.6),
          rng.uniform(0.01, 0.045)) for _ in range(9)]


def h(x, y):
    z = SAND + 0.012
    for bx, by, r, a in bumps:
        z += a * math.exp(-((x - bx) ** 2 + (y - by) ** 2) / (r * r))
    return z


grid = [[sand.verts.new(V((-ix + 2 * ix * i / nx, -iy + 2 * iy * j / ny,
                           h(-ix + 2 * ix * i / nx, -iy + 2 * iy * j / ny))))
         for j in range(ny + 1)] for i in range(nx + 1)]
for i in range(nx):
    for j in range(ny):
        f = sand.faces.new((grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1]))
        f.material_index = SD
box_uv(sand, scale=0.5)
# castle in the front-left corner (off the jump line): a lumpy cone and a bucket-shaped tower
cx, cy = 3.9, 1.7
revolve(sand, [(0, SAND), (1.1, SAND), (0.8, SAND + 0.35), (0.45, SAND + 0.55), (0, SAND + 0.62)],
        SD, n=14, centre=(cx, cy, 0))
revolve(sand, [(0, SAND + 0.55), (0.42, SAND + 0.55), (0.34, SAND + 1.0), (0, SAND + 1.0)], SD,
        n=12, centre=(cx + 0.2, cy - 0.1, 0))
sand_ob = smooth('sand', sand, angle=70)

# a red toy spade stuck in the sand by the castle
toy = core.new_bm()
R = mi('plastic_red')
geo.cylinder(toy, V((cx - 1.3, cy + 0.5, SAND - 0.1)), V((cx - 1.45, cy + 0.65, SAND + 1.3)),
             0.07, R, n=8)
blade_c = V((cx - 1.26, cy + 0.46, SAND + 0.05))
geo.box(toy, blade_c, (0.5, 0.06, 0.55), R)
geo.box(toy, V((cx - 1.47, cy + 0.67, SAND + 1.38)), (0.36, 0.1, 0.1), mi('plastic_yellow'))
toy_ob = part('toy', toy, bevel=0.015, segments=2)

finish('sandbox', [wood, sand_ob, toy_ob], hero=True, car=(0.0, -LY / 2 - 3.5, 0.0))
