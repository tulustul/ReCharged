"""Pallet of stock (kit 'pallet', art bible §3.5): a EUR pallet stacked with three layers of
cartons under stretch wrap, corner boards and a shipping label (5.33 x 3.55 x 5.6 u, length
along X). Variant a: plain kraft shipping cartons; b: printed product cases. They close the
aisles that are not on the route and wall off the back aisle.

    blender -b -P tools/blender/props/supermarket/pallet_stack.py -- [--preview DIR] [--only a]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, W, H = C.PALLET_L, C.PALLET_W, C.STACK_H
PH = C.PALLET_H
PRINTED = ['soda_cola', 'chips_1', 'soap_1', 'cereal_2', 'water', 'pasta_1', 'can_beans']


def build(variant):
    start('pallet_wood', 'pallet_wood_dark', 'packaging', 'wrap', 'cardboard', 'tape', 'signs')
    rnd = rng(77 + ord(variant))
    wood = core.new_bm()
    pallet(wood, L, W)
    boxes = core.new_bm()
    layers = 3
    lh = (H - PH) / layers
    for layer in range(layers):
        nx, ny = (3, 2) if layer % 2 == 0 else (2, 3)
        bw, bd = (L - 0.06) / nx, (W - 0.06) / ny
        for i in range(nx):
            for j in range(ny):
                c = V((-L / 2 + 0.03 + bw * (i + 0.5), -W / 2 + 0.03 + bd * (j + 0.5),
                       PH + layer * lh))
                name = ('carton_%d' % (1 + (i + 2 * j + layer) % 3) if variant == 'a'
                        else rnd.choice(PRINTED))
                fs = geo.box(boxes, V((c.x, c.y, c.z + (lh - 0.02) / 2)),
                             (bw - 0.03, bd - 0.03, lh - 0.02), mi('packaging'))
                for f in fs:
                    f.normal_update()
                    if abs(f.normal.z) < 0.5:
                        map_face(boxes, f, cell(name))
                    else:
                        fill_uv(boxes, [f], side(name) if variant == 'b' else side('carton_1'))
    trim = core.new_bm()
    for sx in (-1, 1):       # cardboard corner boards
        for sy in (-1, 1):
            geo.box(trim, V((sx * (L / 2 + 0.01), sy * (W / 2 + 0.01), (PH + H) / 2)),
                    (0.12, 0.12, H - PH), mi('cardboard'))
    r = region('carton_print')
    fs = geo.box(trim, V((L / 4, W / 2 + 0.05, H - 1.4)), (1.4, 0.02, 1.4), mi('signs'))
    for f in fs:
        f.normal_update()
        if f.normal.y > 0.9:
            map_face(trim, f, r, 'signs')
        else:
            fill_uv(trim, [f], [r[0] + 3, r[1] + 3, 2, 2], 'signs')
    wrap = core.new_bm()
    geo.box(wrap, V((0, 0, (PH + 0.3 + H - 0.4) / 2)), (L + 0.06, W + 0.06, H - PH - 0.7),
            mi('wrap'))
    parts = [part('wood', wood, bevel=0.02, segments=1), part('boxes', boxes, bevel=0.03,
                                                              segments=1),
             part('trim', trim), part('wrap', wrap)]
    return propkit.finish(ENV, f'pallet_stack_{variant}', parts, car=(L / 2 + 2, W / 2 + 2, 210))


argv = propkit._argv()
only = argv[argv.index('--only') + 1] if '--only' in argv else None
for v in ('a', 'b'):
    if only and v != only:
        continue
    build(v)
