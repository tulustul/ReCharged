"""Spare wheel lying flat: used tyre on a black pressed-steel rim with vent holes and studs
(knockable).

    blender -b -P tools/blender/props/workshop/wheel.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('tyre', 'rubber', 'steel_black', 'steel')
bm = core.new_bm()
w = C.TYRE['w']
tyre(bm, V((0, 0, w / 2)), spin=15)
steel_rim(bm, V((0, 0, w / 2)), rim_r=RIM_R, w=w * 0.82, n=28)
whl = core.obj_from_bm('wheel', bm, weighted=False, angle=50)
finish('wheel', [whl], budget=KIT, car=(2.6, -0.8, 30))
