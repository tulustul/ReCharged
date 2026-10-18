"""Jumbo roof block: a blue-stained 45 degree gable prism, 1.4 u wide and deep, 0.7 u high, the
cap of a block tower. Dynamic (knockable).

    blender -b -P tools/blender/props/toy/block_roof.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
wood('blue_wood', BLUE, 0.4, contrast=0.08, seed=12)
bm = core.new_bm()
extrude(bm, 'x', [(-0.7, 0.0), (0.7, 0.0), (0.0, 0.7)], -0.7, 0.7, core.mi('blue_wood'))
body = part('body', box_uv(bevel_bm(bm, 0.03)), bevel=0.0)
propkit.finish(ENV, 'block_roof', [body], car=(1.6, 0.8, -20))
