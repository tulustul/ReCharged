"""Stacked produce crates (kit 'crate', art bible §3.5): two black plastic crates with hand
holes (2.66 x 2.14 x 1.78 u) full of oranges and apples, the top one slightly turned.

    blender -b -P tools/blender/props/supermarket/crate_stack.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, W, H = C.CRATE_L, C.CRATE_W, C.CRATE_H


def crate(bm, fruit, z, rot, kinds, rnd):
    m = Matrix.Rotation(rot, 4, 'Z')
    a = core.new_bm()
    geo.box(a, V((0, 0, z + 0.04)), (L - 0.1, W - 0.1, 0.08), mi('black_plastic'))
    for s in (-1, 1):
        geo.box(a, V((s * (L / 2 - 0.06), 0, z + H / 2)), (0.12, W, H - 0.02), mi('black_plastic'))
        geo.box(a, V((0, s * (W / 2 - 0.06), z + H / 2)), (L - 0.2, 0.12, H - 0.02),
                mi('black_plastic'))
        geo.box(a, V((s * (L / 2 + 0.005), 0, z + H - 0.3)), (0.02, 0.6, 0.2), mi('steel_dark'))
    fruit_bed(a, -L / 2 + 0.2, L / 2 - 0.2, -W / 2 + 0.2, W / 2 - 0.2, z + H - 0.2, kinds, rnd,
              r=0.22)
    a.transform(m)
    me = bpy.data.meshes.new('tmp')
    a.to_mesh(me)
    a.free()
    bm.from_mesh(me)
    bpy.data.meshes.remove(me)
    del fruit


def build():
    start('black_plastic', 'steel_dark', 'fruit_orange', 'fruit_red', 'fruit_green')
    rnd = rng(61)
    bm = core.new_bm()
    crate(bm, None, 0.0, 0.0, ['fruit_orange'], rnd)
    crate(bm, None, H, 0.08, ['fruit_red', 'fruit_green'], rnd)
    return propkit.finish(ENV, 'crate_stack', [smooth('crates', bm, angle=40)],
                          car=(L / 2 + 2, W / 2 + 2, 210))


build()
