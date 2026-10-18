"""Grey 12 l plastic bucket with a wire handle and a sponge (knockable; wash bay).

    blender -b -P tools/blender/props/workshop/bucket.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('plastic_grey', 'steel', 'safety_yellow', 'plastic_black')
bm = core.new_bm()
outer = [(0.0, 0.0), (0.58, 0.0), (0.6, 0.04), (0.67, 1.26), (0.71, 1.3), (0.71, 1.34),
         (0.66, 1.34)]
inner = [(0.62, 1.32), (0.56, 0.08), (0.0, 0.08)]
revolve(bm, outer + inner, mi('plastic_grey'), n=28)
pts = [V((0.72 * math.cos(a), 0.0, 1.05 + 0.72 * math.sin(a))) for a in
       [math.pi * i / 12 for i in range(13)]]
geo.tube(bm, pts, 0.025, mi('steel'), n=6)
for sx in (-1, 1):
    geo.cylinder(bm, V((sx * 0.66, 0, 1.05)), V((sx * 0.74, 0, 1.05)), 0.07, mi('plastic_black'),
                 n=8)
geo.slab(bm, [(-0.25, 0.35, 0.08, 0.45, 0.06), (0.25, 0.35, 0.08, 0.45, 0.06)],
         mi('safety_yellow'))  # sponge
bkt = core.obj_from_bm('bucket', bm, weighted=False, angle=50)
finish('bucket', [bkt], budget=KIT, car=(2.0, -0.6, 30))
