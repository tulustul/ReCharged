"""Big rubber duck (22 cm bath toy): glossy yellow body with an upturned tail, orange beak, black
eyes with a white glint. Faces +Y. Static.

    blender -b -P tools/blender/props/toy/duck.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
plastic('duck', '#F4C21C', 0.18)
plastic('beak', '#F07A1F', 0.2)
plastic('eye', '#151417', 0.1)
plastic('glint', '#F7F5EE', 0.1)
mi = core.mi
bm = core.new_bm()
geo.sphere(bm, V((0, -0.05, 0.36)), 0.5, mi('duck'), n=24, rings=12, scale=(0.95, 1.3, 0.72))
geo.sphere(bm, V((0, -0.62, 0.55)), 0.2, mi('duck'), n=14, rings=7, scale=(0.9, 1.0, 1.3))
geo.sphere(bm, V((0, 0.33, 0.93)), 0.34, mi('duck'), n=22, rings=11)
geo.sphere(bm, V((0, 0.68, 0.86)), 0.17, mi('beak'), n=16, rings=8, scale=(1.25, 1.1, 0.42))
for s in (-1, 1):
    geo.sphere(bm, V((s * 0.17, 0.6, 1.04)), 0.06, mi('eye'), n=10, rings=5)
    geo.sphere(bm, V((s * 0.19, 0.645, 1.065)), 0.018, mi('glint'), n=6, rings=4)
    geo.sphere(bm, V((s * 0.36, -0.05, 0.48)), 0.25, mi('duck'), n=14, rings=7,
               scale=(0.45, 1.2, 0.6))  # wings
propkit.finish(ENV, 'duck', [smooth('duck', bm)], car=(1.6, 0.5, -20))
