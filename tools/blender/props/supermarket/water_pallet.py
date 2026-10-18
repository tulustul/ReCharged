"""Pallet of bottled water (kit, art bible §3.5): a EUR pallet with two layers of shrink-wrapped
6-packs of 1.5 L bottles (5.33 x 3.55 x 3.6 u, length along X), caps up. Three in a row carry
the Golden Battery in the produce section; the top is drivable (Cardboard).

    blender -b -P tools/blender/props/supermarket/water_pallet.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, W, H = C.WATER_L, C.WATER_W, C.WATER_H
PH = C.PALLET_H


def build():
    start('pallet_wood', 'pallet_wood_dark', 'wrap', 'packaging_gloss', 'blue_plastic', 'tape')
    wood = core.new_bm()
    pallet(wood, L, W)
    packs = core.new_bm()
    caps = core.new_bm()
    nx, ny = 4, 3
    pw, pd = (L - 0.1) / nx, (W - 0.05) / ny
    layer_h = (H - PH) / 2
    for layer in range(2):
        z0 = PH + layer * layer_h
        for i in range(nx):
            for j in range(ny):
                cx, cy = -L / 2 + 0.05 + pw * (i + 0.5), -W / 2 + 0.025 + pd * (j + 0.5)
                fs = geo.box(packs, V((cx, cy, z0 + (layer_h - 0.12) / 2)),
                             (pw - 0.04, pd - 0.04, layer_h - 0.12), mi('packaging_gloss'))
                for f in fs:
                    f.normal_update()
                    if abs(f.normal.z) < 0.5:
                        map_face(packs, f, [cell('water')[0], cell('water')[1] + 32,
                                            cell('water')[2], 64])
                    else:
                        fill_uv(packs, [f], side('water'))
                for a in range(3 if layer else 0):   # caps of the six bottles (top layer)
                    for b in range(2):
                        x = cx - pw / 2 + pw * (a + 0.5) / 3
                        y = cy - pd / 2 + pd * (b + 0.5) / 2
                        geo.cylinder(caps, V((x, y, z0 + layer_h - 0.13)),
                                     V((x, y, z0 + layer_h - 0.01)), 0.09, mi('blue_plastic'), n=6)
    # stretch wrap band and a tape label
    wrap = core.new_bm()
    geo.box(wrap, V((0, 0, PH + 1.2)), (L + 0.02, W + 0.02, 1.4), mi('wrap'))
    parts = [part('wood', wood, bevel=0.02, segments=1), part('packs', packs, bevel=0.03, segments=1),
             smooth('caps', caps), part('wrap', wrap)]
    return propkit.finish(ENV, 'water_pallet', parts, car=(L / 2 + 2, W / 2 + 2, 210))


build()
