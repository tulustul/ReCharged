"""Jumbo red pillar block: a 1.4 u tall, 1.4 u wide cylinder of red-stained wood with rounded
rims. Dynamic (knockable).

    blender -b -P tools/blender/props/toy/block_pillar.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

R, HT, RIM = 0.7, 1.4, 0.04
core.reset()
wood('red_wood', RED, 0.4, contrast=0.08, seed=11)
prof = [(0.0, 0.0), (R - RIM, 0.0)]
prof += [(R - RIM + RIM * math.sin(a), RIM - RIM * math.cos(a))
         for a in (math.pi / 6 * k for k in range(1, 4))]
prof += [(R, HT - RIM)] + [(R - RIM + RIM * math.cos(a), HT - RIM + RIM * math.sin(a))
                            for a in (math.pi / 6 * k for k in range(1, 4))]
prof += [(0.0, HT)]
bm = core.new_bm()
# revolve_x builds around X: swap to a vertical axis by revolving (z, r) and rotating after
geo.revolve_x(bm, [(z, r) for r, z in prof], core.mi('red_wood'), n=32)
geo.bmesh.ops.rotate(bm, verts=bm.verts, cent=V((0, 0, 0)), matrix=Matrix.Rotation(-math.pi / 2,
                                                                                   3, 'Y'))
cyl_uv(bm, bm.faces, V((0, 0, 0)), 0.5)
body = smooth('body', bm, angle=50)
propkit.finish(ENV, 'block_pillar', [body], car=(1.8, 0.8, -20))
