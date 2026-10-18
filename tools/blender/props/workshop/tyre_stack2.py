"""Stack of 2 used car tyres lying flat (205/55 R16, 0.63 m): chicane walls and tyre walls.

    blender -b -P tools/blender/props/workshop/tyre_stack2.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('tyre', 'rubber')
finish('tyre_stack2', [tyre_stack(2, seed=2)], budget=KIT, car=(3.0, -1.2, 25))
