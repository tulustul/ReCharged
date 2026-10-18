"""Department wall bands (decor, art bible §3.5): 22.2 u lengths of painted soffit band with a
white pinstripe, mounted at 10-11.6 u above the cases (blue frozen, teal dairy, red deli, green
produce). Front +Y, origin on the floor at the wall. Physics-free.

    blender -b -P tools/blender/props/supermarket/wall_band.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

COLOURS = {'blue': '#2F6FD6', 'teal': '#1F8A8A', 'red': '#B8322A', 'green': '#3E9B4F'}
L, Z0, H = 2 * C.GONDOLA_LEN, 10.0, 1.6


def build(name, colour):
    MATS[f'band_{name}'] = dict(base=colour, rough=0.6)
    start(f'band_{name}', 'white_plastic')
    bm = core.new_bm()
    geo.box(bm, V((0, 0.06, Z0 + H / 2)), (L, 0.12, H), mi(f'band_{name}'))
    geo.box(bm, V((0, 0.13, Z0 + 0.25)), (L, 0.02, 0.1), mi('white_plastic'))
    return propkit.finish(ENV, f'wall_band_{name}', [part('band', bm)],
                          shots=[('front', V((0.3, 1.0, 0.1)), 50)])


for n, c in COLOURS.items():
    build(n, c)
