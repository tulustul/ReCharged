"""Stack of 4 used car tyres lying flat (205/55 R16, 0.63 m): chicane walls and tyre walls.

    blender -b -P tools/blender/props/workshop/tyre_stack4.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('tyre', 'rubber')
finish('tyre_stack4', [tyre_stack(4, seed=4, segs=28)], budget=KIT, car=(3.0, -1.2, 25))
