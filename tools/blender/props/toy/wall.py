"""Straight interior wall panel of the toy store: 12 u long (x -6..6), 20 u tall (floor to
ceiling), 0.8 u thick; warm-white plaster, beech skirting and crown, a painted rainbow stripe
band at 4-5.2 u on both faces. Panels butt end to end. Static.

    blender -b -P tools/blender/props/toy/wall.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
wall_materials()
propkit.finish(ENV, 'wall', [smooth('wall', wall_sweep(core.new_bm(), [(-6, 0), (6, 0)]),
                                    angle=30)], car=(2.0, 4.0, 200))
