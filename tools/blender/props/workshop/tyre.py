"""A loose used car tyre lying flat (205/55 R16; knockable).

    blender -b -P tools/blender/props/workshop/tyre.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('tyre', 'rubber')
finish('tyre', [tyre_stack(1, seed=5)], budget=KIT, car=(2.6, -0.8, 30))
