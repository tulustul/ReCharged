"""Stacking-rings toy (30 cm): beech base and peg with six rainbow rings and a red ball cap.
Static.

    blender -b -P tools/blender/props/toy/stacking_rings.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
beech()
COLS = [RED, ORANGE, YELLOW, GREEN, BLUE, PURPLE]
for k, c in enumerate(COLS):
    plastic(f'ring{k}', c)
mi = core.mi
base, rings = core.new_bm(), core.new_bm()
geo.cylinder(base, V((0, 0, 0)), V((0, 0, 0.14)), 0.62, mi('beech'), n=28)
geo.cylinder(base, V((0, 0, 0.14)), V((0, 0, 1.25)), 0.09, mi('beech'), n=12)
z = 0.14
for k in range(6):  # tori around the X axis (x = height), turned upright below
    r, t = 0.56 - 0.045 * k, 0.09 - 0.004 * k
    prof = [(z + t + t * math.sin(a), r - t + t * math.cos(a))  # counter-clockwise: faces out
            for a in (2 * math.pi * j / 10 for j in range(11))]
    geo.revolve_x(rings, prof, mi(f'ring{k}'), n=28)
    z += 2 * t
geo.bmesh.ops.rotate(rings, verts=rings.verts, cent=V((0, 0, 0)),
                     matrix=Matrix.Rotation(-math.pi / 2, 3, 'Y'))
geo.sphere(rings, V((0, 0, z + 0.14)), 0.17, mi('ring0'), n=14, rings=7)
propkit.finish(ENV, 'stacking_rings', [smooth('base', box_uv(base)), smooth('rings', rings)],
               car=(1.8, 0.8, -20))
