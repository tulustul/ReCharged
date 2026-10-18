"""Wet-floor A-frame (kit, art bible §3.5): yellow plastic, 1.4 x 2.7 x 1.0 u, the CAUTION WET
FLOOR pictogram on both faces, a carry handle on top. Knockable (light); stands where the ice
starts and ends.

    blender -b -P tools/blender/props/supermarket/wet_floor_sign.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

W, H, D = C.SIGN_W, C.SIGN_H, C.SIGN_D
start('yellow_plastic', 'signs', 'black_plastic')
bm = core.new_bm()
tilt = math.atan2(D / 2 - 0.05, H - 0.2)
r = region('wet_floor')
for s in (-1, 1):
    a = core.new_bm()
    fs = geo.box(a, V((0, 0, (H - 0.2) / 2)), (W, 0.06, H - 0.2), mi('yellow_plastic'))
    for f in fs:
        f.normal_update()
        if abs(f.normal.y) > 0.9:
            f.material_index = mi('signs')
            map_face(a, f, [r[0] + 4, r[1] + 4, r[2] - 8, r[3] - 8], 'signs')
        else:
            fill_uv(a, [f], [r[0] + 2, r[1] + 2, 2, 2], 'signs')
    a.transform(Matrix.Translation(V((0, s * (D / 2 - 0.05), 0))) @
                Matrix.Rotation(s * tilt, 4, 'X'))
    me = bpy.data.meshes.new('tmp')
    a.to_mesh(me)
    a.free()
    bm.from_mesh(me)
    bpy.data.meshes.remove(me)
geo.box(bm, V((0, 0, H - 0.12)), (W * 0.9, 0.3, 0.2), mi('yellow_plastic'))
geo.box(bm, V((0, 0, H - 0.02)), (0.6, 0.12, 0.08), mi('black_plastic'))
propkit.finish(ENV, 'wet_floor_sign', [part('sign', bm, bevel=0.015, segments=1)],
               car=(2.0, 1.0, 200))
