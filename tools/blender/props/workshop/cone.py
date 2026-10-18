"""Orange 50 cm traffic cone with a reflective band on a square base (knockable).

    blender -b -P tools/blender/props/workshop/cone.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('plastic_orange', 'reflective', 'plastic_black')
bm = core.new_bm()
geo.slab(bm, [(-0.83, 0.83, 0.0, 0.1, 0.12), (0.83, 0.83, 0.0, 0.1, 0.12)], mi('plastic_black'))
fs, vs = revolve(bm, [(0.64, 0.1), (0.62, 0.13), (0.13, 2.17), (0.1, 2.2), (0.0, 2.2)],
                 mi('plastic_orange'), n=24)
fs, vs = revolve(bm, [(0.42 + 0.012, 1.0), (0.335 + 0.012, 1.36)], mi('reflective'), n=24)
base = core.obj_from_bm('cone', bm, weighted=False, angle=50)
finish('cone', [base], budget=KIT, car=(2.2, -0.6, 30))
