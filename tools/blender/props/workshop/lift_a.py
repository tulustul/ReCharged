"""Two-post lift with a generic hatchback raised high for underbody work (hero): the N lane runs under
its tail. See liftkit.py; dimensions in catalog.py (LIFT, CARS['a']).

    blender -b -P tools/blender/props/workshop/lift_a.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import liftkit  # noqa: E402
from wkit import HERO, finish  # noqa: E402

parts = liftkit.build('a')
finish('lift_a', parts, budget=HERO, car=(-4.0, -13.0, 0.0))
liftkit.previews('lift_a', 'a')
