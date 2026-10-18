"""Black 15 l oil drain pan with a pour spout, old oil inside (knockable; under the lifts).

    blender -b -P tools/blender/props/workshop/drain_pan.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('plastic_black', 'oil_dark')
bm = core.new_bm()
revolve(bm, [(0.0, 0.0), (1.12, 0.0), (1.33, 0.4), (1.36, 0.45), (1.28, 0.45), (1.08, 0.08),
             (0.0, 0.08)], mi('plastic_black'), n=32)
revolve(bm, [(1.2, 0.27), (0.0, 0.27)], mi('oil_dark'), n=32)
geo.slab(bm, [(1.2, 0.18, 0.3, 0.44, 0.04), (1.55, 0.1, 0.36, 0.42, 0.03)], mi('plastic_black'))
pan = core.obj_from_bm('pan', bm, weighted=False, angle=50)
finish('drain_pan', [pan], budget=KIT, car=(2.6, -0.8, 30))
