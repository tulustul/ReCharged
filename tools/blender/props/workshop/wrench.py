"""Chrome combination spanner, 45 cm (2 u), lying flat: open jaw one end, ring the other
(knockable).

    blender -b -P tools/blender/props/workshop/wrench.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

start('chrome')
bm = core.new_bm()
geo.box(bm, V((0, 0, 0.05)), (1.4, 0.18, 0.08), mi('chrome'))               # shank
fs, vs = revolve(bm, [(0.13, 0.01), (0.2, 0.01), (0.22, 0.05), (0.2, 0.09), (0.13, 0.09),
                      (0.13, 0.01)], mi('chrome'), n=16)
place(bm, vs, center=(0.82, 0, 0))                                            # ring end
for sy in (-1, 1):                                                           # open jaw
    geo.box(bm, V((-0.86, sy * 0.15, 0.05)), (0.3, 0.1, 0.08), mi('chrome'))
geo.box(bm, V((-0.72, 0, 0.05)), (0.08, 0.36, 0.08), mi('chrome'))
wr = core.obj_from_bm('wrench', bm, bevel=0.01, segments=1, angle=40)
finish('wrench', [wr], budget=KIT, car=(1.8, -0.6, 30))
