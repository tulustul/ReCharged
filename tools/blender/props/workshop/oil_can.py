"""5 l engine-oil jug in amber plastic with a handle, black cap and a brand-free label band
(knockable).

    blender -b -P tools/blender/props/workshop/oil_can.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('amber_plastic', 'plastic_black', 'label_white')
bm = core.new_bm()
geo.slab(bm, [(-0.25, 0.44, 0.0, 1.0, 0.1), (0.25, 0.44, 0.0, 1.0, 0.1)], mi('amber_plastic'))
geo.slab(bm, [(-0.22, 0.38, 0.95, 1.12, 0.08), (0.22, 0.2, 0.95, 1.2, 0.08)],
         mi('amber_plastic'))
geo.box(bm, V((-0.22, 0, 1.12)), (0.14, 0.3, 0.16), mi('amber_plastic'))   # handle grip
geo.cylinder(bm, V((0.26, 0, 1.12)), V((0.26, 0, 1.3)), 0.12, mi('plastic_black'), n=10)
geo.box(bm, V((0.0, -0.255, 0.48)), (0.72, 0.012, 0.52), mi('label_white'))
geo.box(bm, V((0.0, -0.262, 0.62)), (0.6, 0.006, 0.12), mi('plastic_black'))
jug = core.obj_from_bm('jug', bm, bevel=0.01, segments=1, angle=40)
finish('oil_can', [jug], budget=KIT, car=(1.6, -0.6, 30))
