"""Jumbo arch block: 2.8 x 1.4 x 1.4 u of yellow-stained wood with a round arch (r 0.65) through
it, a bridge between towers. Dynamic (knockable).

    blender -b -P tools/blender/props/toy/block_arch.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
wood('yellow_wood', YELLOW, 0.4, contrast=0.08, seed=13)
R = 0.65
arc = [(R * math.cos(a), R * math.sin(a)) for a in (math.pi * (1 - k / 16) for k in range(17))]
outline = [(-1.4, 0.0), (-R, 0.0)] + arc[1:-1] + [(R, 0.0), (1.4, 0.0), (1.4, 1.4), (-1.4, 1.4)]
bm = core.new_bm()
extrude(bm, 'y', outline, -0.7, 0.7, core.mi('yellow_wood'))
body = part('body', box_uv(bevel_bm(bm, 0.03, angle=60)), bevel=0.0)
propkit.finish(ENV, 'block_arch', [body], car=(0.0, 2.6, 160))
