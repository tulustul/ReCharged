"""Edging block (kit): 1.2 × 1.2 × 0.6 u, the size and brick-red tint of the track's barrier
blocks, with a riven top and chamfered edges; rows of them close the open ends of the raised beds
(catalog: garden_edging).

    blender -b -P tools/blender/props/garden/edging.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

start('edging_brick')
bm = core.new_bm()
geo.box(bm, V((0, 0, 0.6)), (1.2, 0.6, 1.2), mi('edging_brick'))
bmesh.ops.subdivide_edges(bm, edges=[e for e in bm.edges if abs(e.verts[0].co.z - e.verts[1].co.z)
                                     < 1e-6 and e.verts[0].co.z > 1.0], cuts=3, use_grid_fill=True)
rng = random.Random(5)
for v in bm.verts:
    if v.co.z > 1.19 and abs(v.co.x) < 0.59 and abs(v.co.y) < 0.29:
        v.co.z += rng.uniform(-0.03, 0.02)
box_uv(bm, scale=0.6)
finish('edging', [part('block', bm, bevel=0.05, segments=2)], car=(2.0, -1.0, 20.0))
