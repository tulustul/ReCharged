"""Wet-mud rims around the three puddles on the dirt path (decor), one glb each
(puddle_rim_1..3, origin at the puddle centre): a darker, glossier band of soil from the water's
edge (plan.puddle) out to 1.35× its size, lying between the dirt (0.03) and the water (0.06).

    blender -b -P tools/blender/props/garden/puddle_rims.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403
import plan  # noqa: E402

MATS['mud'] = dict(base='#4A3524', rough=0.35, tex='grit')
for k, (cx, cz, _, _) in enumerate(plan.PUDDLES):
    start('mud')
    inner, outer = plan.puddle(k, 0.97), plan.puddle(k, 1.35)
    bm = core.new_bm()
    # Bevy (x, z) -> Blender (x, -z), relative to the puddle centre; y = 0.045 above the ground
    vi = [bm.verts.new(V((x - cx, -(z - cz), 0.015))) for x, z in inner]
    vo = [bm.verts.new(V((x - cx, -(z - cz), 0.002))) for x, z in outer]
    n = len(vi)
    for j in range(n):
        f = bm.faces.new((vi[j], vo[j], vo[(j + 1) % n], vi[(j + 1) % n]))
        f.material_index = 0
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    for f in bm.faces:
        if f.normal.z < 0:
            f.normal_flip()
    box_uv(bm, scale=0.5)
    finish(f'puddle_rim_{k + 1}', [part('rim', bm)], car=(4.0, -2.0, 0.0))
