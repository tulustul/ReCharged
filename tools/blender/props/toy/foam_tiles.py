"""Interlocking foam play tiles (60 cm EVA squares, 1.2 cm thick): a loose stack of four and two
tiles laid on the floor, jigsaw tabs on every edge. Static.

    blender -b -P tools/blender/props/toy/foam_tiles.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

S, T = 2.66, 0.055      # tile size, thickness
core.reset()
COLS = [RED, BLUE, YELLOW, GREEN, ORANGE, PURPLE]
for k, c in enumerate(COLS):
    fabric(f'foam{k}', c, 0.85)
mi = core.mi


def outline():
    """Square with 4 alternating tabs/slots per edge (dovetail-ish trapezoids), CCW."""
    pts = []
    corners = [V((-S / 2, -S / 2)), V((S / 2, -S / 2)), V((S / 2, S / 2)), V((-S / 2, S / 2))]
    for e in range(4):
        a, b = corners[e], corners[(e + 1) % 4]
        d = (b - a) / S
        n = V((d.y, -d.x))  # outward normal for a CCW square
        pts.append(a)
        for k in range(4):
            t0 = (k + 0.5) / 4.5 * S
            out = 0.17 if k % 2 == 0 else -0.17
            for u, h in ((0.0, 0.0), (0.05, 1.0), (0.31, 1.0), (0.36, 0.0)):
                pts.append(a + d * (t0 + u) + n * out * h)
    return pts


bm = core.new_bm()
tiles = [((0, 0, 0), 0, 0), ((0.12, -0.08, T), 7, 1), ((-0.1, 0.06, 2 * T), -5, 2),
         ((0.05, 0.1, 3 * T), 12, 3), ((3.1, 0.6, 0), 20, 4), ((1.0, 3.0, 0), -14, 5)]
for (x, y, z), deg, k in tiles:
    rot = Matrix.Rotation(math.radians(deg), 2)
    pts = [rot @ p + V((x, y)) for p in outline()]
    extrude(bm, 'z', pts, z, z + T, mi(f'foam{k}'))
propkit.finish(ENV, 'foam_tiles', [part('tiles', bm, bevel=0.0)], car=(2.8, -2.0, 30))
