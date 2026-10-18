"""Two-post lift with a generic saloon at wheel-work height (hero): the N lane runs under
its tail. See liftkit.py; dimensions in catalog.py (LIFT, CARS['b']).

    blender -b -P tools/blender/props/workshop/lift_b.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import liftkit  # noqa: E402
from wkit import HERO, finish  # noqa: E402

parts = liftkit.build('b')
finish('lift_b', parts, budget=HERO, car=(-4.0, -13.0, 0.0))
liftkit.previews('lift_b', 'b')
