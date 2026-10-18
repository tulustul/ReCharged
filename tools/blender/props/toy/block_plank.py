"""Jumbo beech plank block, 2.8 x 1.4 x 0.7 u (two cubes long, half a cube high): natural wood
with rounded edges, for tower floors and bridges. Dynamic (knockable).

    blender -b -P tools/blender/props/toy/block_plank.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
beech()
bm = core.new_bm()
geo.box(bm, V((0, 0, 0.35)), (2.8, 1.4, 0.7), core.mi('beech'))
body = part('body', box_uv(bevel_bm(bm, 0.03)), bevel=0.0)
propkit.finish(ENV, 'block_plank', [body], car=(0.0, 2.6, 160))
