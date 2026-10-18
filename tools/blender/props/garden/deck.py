"""Raised timber deck (structure): 42 × 36 u (9.5 × 8 m), 0.45 m (2.0 u) high, against the house
(+Y here = Bevy -Z, north) and the east fence (+X). Decking boards run north-south over a dark
underlay; the open south (-Y) and west (-X) sides get a fascia board,
recessed skirting boards and a three-riser step on the south side (catalog: garden_deck, whose
Wood box is the drivable top).

    blender -b -P tools/blender/props/garden/deck.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

HX, HY, TOP = 21.0, 18.0, 2.0
BOARD, GAP, T = 0.58, 0.045, 0.11
STEP_X, STEP_W = -9.0, 4.6           # step centre on the south side, width

start('deck_wood', 'pine_dark', 'felt')
D, P, F = mi('deck_wood'), mi('pine_dark'), mi('felt')

bm = core.new_bm()
n = int(2 * HX / (BOARD + GAP))
pitch = 2 * HX / n
rng = random.Random(11)
for k in range(n):  # full-length boards (butt joints cost a third of the triangle budget)
    x = -HX + (k + 0.5) * pitch
    board(bm, (x, 0, TOP - T / 2), (pitch - GAP, 2 * HY, T), D, axis=1, seed=k)
boards = part('boards', bm, bevel=0.022, segments=1)

frame = core.new_bm()
# dark underlay so the board gaps read as shadow (also hides the Wood ground polygon at 1.8)
geo.box(frame, V((0, 0, TOP - T - 0.03)), (2 * HX - 0.1, 2 * HY - 0.1, 0.04), F)
k = 0
for side in ('south', 'west'):
    for z0, z1, inset in ((TOP - T - 0.5, TOP - 0.01, -0.06),      # fascia, proud of the boards
                          (0.95, 1.37, 0.05), (0.5, 0.92, 0.05), (0.03, 0.47, 0.05)):
        k += 1
        if side == 'south':
            board(frame, (0, -HY + inset + 0.04, (z0 + z1) / 2), (2 * HX, 0.08, z1 - z0), P,
                  axis=0, seed=k)
        else:
            board(frame, (-HX + inset + 0.04, 0, (z0 + z1) / 2), (0.08, 2 * HY, z1 - z0), P,
                  axis=1, seed=k)
# corner post at the open corner, and posts every ~7 u behind the skirting rhythm
for x, y in [(-HX + 0.2, -HY + 0.2)] + [(-HX + 7 * i, -HY + 0.12) for i in range(1, 6)] \
        + [(-HX + 0.12, -HY + 7 * i) for i in range(1, 5)]:
    k += 1
    board(frame, (x, y, (TOP - T - 0.5) / 2), (0.36, 0.36, TOP - T - 0.5), P, axis=2, seed=k)
# three-riser step on the south side (risers 0.67 u = 15 cm, treads 1.3 u)
for i, z in enumerate((0.67, 1.33)):
    depth = 1.3 * (2 - i)
    k += 1
    board(frame, (STEP_X, -HY - depth / 2, z - 0.06), (STEP_W, depth, 0.12), D, axis=0, seed=k)
    board(frame, (STEP_X, -HY - depth + 0.04, z / 2 - 0.03), (STEP_W, 0.08, z - 0.1), P, axis=0,
          seed=k + 50)
    for sx in (-1, 1):
        board(frame, (STEP_X + sx * (STEP_W / 2 - 0.05), -HY - depth / 2, z / 2 - 0.06),
              (0.1, depth, z - 0.12), P, axis=1, seed=k + 70)
skirt = part('frame', frame, bevel=0.02, segments=1)

finish('deck', [boards, skirt], hero=True, car=(-HX + 8.0, -HY - 3.0, 0.0),
       shots=[('hero', V((-1.0, -1.2, 0.55)), 50), ('corner', V((-0.6, -0.9, 0.18)), 50),
              ('top', V((0.0001, -0.25, 1.0)), 50), ('rc', None, 24)])
