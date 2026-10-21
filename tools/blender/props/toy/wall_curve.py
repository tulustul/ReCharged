"""Curved interior wall: a 90° arc of the toy-store wall (section as toy_wall: 20 u tall,
plaster, beech skirting and crown, rainbow stripes on both faces). The origin is the arc's
circle centre on the floor; the wall spans -45°..45° around the front (+Y here, prop -Z), inner
(concave) face at radius 20, outer face at 20.8. Static.

    blender -b -P tools/blender/props/toy/wall_curve.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

R, SEG = 20.4, 32  # centreline radius, arc segments
core.reset()
wall_materials()
arc = [(-R * math.sin(a), R * math.cos(a))
       for a in (math.radians(-45 + 90 * i / SEG) for i in range(SEG + 1))]
propkit.finish(ENV, 'wall_curve', [smooth('wall', wall_sweep(core.new_bm(), arc), angle=30)],
               car=(0.0, 16.0, 0))
