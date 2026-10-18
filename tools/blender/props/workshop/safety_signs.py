"""Safety signs (decor), on the wall: a yellow warning triangle with an exclamation mark, a red
fire-extinguisher sign above a red extinguisher on its bracket, a green first-aid sign with a
white cross. Pictograms only (no text). Origin at the wall face on the floor, front +Y.

    blender -b -P tools/blender/props/workshop/safety_signs.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('safety_yellow', 'hazard_black', 'plastic_red', 'label_white', 'plastic_green',
      'plastic_black', 'steel')
bm = core.new_bm()
# warning triangle (left)
tri = [(-1.4, 0.0), (1.4, 0.0), (0.0, 2.4)]
geo.extrude_y(bm, [(x - 3.5, z + 8.0) for x, z in tri], 0.0, 0.06, mi('hazard_black'))
inner = [(-1.12, 0.2), (1.12, 0.2), (0.0, 2.05)]
geo.extrude_y(bm, [(x - 3.5, z + 8.0) for x, z in inner], 0.06, 0.09, mi('safety_yellow'))
geo.box(bm, V((-3.5, 0.1, 8.0 + 1.25)), (0.22, 0.04, 0.8), mi('hazard_black'))
geo.box(bm, V((-3.5, 0.1, 8.0 + 0.55)), (0.22, 0.04, 0.22), mi('hazard_black'))
# extinguisher sign (centre) + white extinguisher silhouette
geo.box(bm, V((0, 0.04, 9.0)), (1.8, 0.08, 1.8), mi('plastic_red'))
geo.box(bm, V((0, 0.1, 8.85)), (0.5, 0.04, 1.0), mi('label_white'))
geo.box(bm, V((0, 0.1, 9.5)), (0.25, 0.04, 0.25), mi('label_white'))
# first aid (right)
geo.box(bm, V((3.5, 0.04, 9.0)), (1.8, 0.08, 1.8), mi('plastic_green'))
geo.box(bm, V((3.5, 0.1, 9.0)), (1.1, 0.04, 0.36), mi('label_white'))
geo.box(bm, V((3.5, 0.1, 9.0)), (0.36, 0.04, 1.1), mi('label_white'))
geo.box(bm, V((0, 0.2, 5.4)), (0.6, 0.4, 0.25), mi('steel'))  # bracket
signs = core.obj_from_bm('signs', bm, bevel=0.01, segments=1, angle=40)
bm = core.new_bm()
geo.cylinder(bm, V((0, 0.6, 2.4)), V((0, 0.6, 5.4)), 0.5, mi('plastic_red'), n=14)        # bottle
geo.cylinder(bm, V((0, 0.6, 5.4)), V((0, 0.6, 5.8)), 0.25, mi('plastic_black'), n=10)    # valve
geo.tube(bm, [V((0.2, 0.6, 5.7)), V((0.6, 0.9, 5.5)), V((0.6, 1.0, 3.6)), V((0.45, 0.95, 3.2))],
         0.07, mi('plastic_black'), n=6, corner=0.3, cseg=2)
ext = core.obj_from_bm('extinguisher', bm, weighted=False, angle=40)
finish('safety_signs', [signs, ext], budget=KIT, car=(0.0, 4.0, 180))
