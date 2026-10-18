"""Ceiling light rows (decor, art bible §3.5): an 11.1 u twin-tube fluorescent batten hung on
two wires below the 22 u ceiling. 'on' glows cold white (the rows a store leaves on after
closing), 'off' shows grey tubes. Physics-free, origin on the floor.

    blender -b -P tools/blender/props/supermarket/ceiling_light.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

CEIL, Z = 22.0, 19.6
MATS['tube_on'] = dict(base='#F4F8FA', rough=0.3, emission=(0.88, 0.95, 1.0), strength=6.0)
MATS['tube_off'] = dict(base='#C9CDD2', rough=0.35)


def build(state):
    start('white_plastic', 'steel_dark', f'tube_{state}')
    bm = core.new_bm()
    L = C.GONDOLA_LEN
    geo.box(bm, V((0, 0, Z + 0.25)), (0.55, L, 0.2), mi('white_plastic'))
    for x in (-0.12, 0.12):
        geo.cylinder(bm, V((x, -L / 2 + 0.15, Z + 0.08)), V((x, L / 2 - 0.15, Z + 0.08)), 0.07,
                     mi(f'tube_{state}'), n=8)
    for y in (-L / 2 + 0.6, L / 2 - 0.6):
        geo.cylinder(bm, V((0, y, Z + 0.35)), V((0, y, CEIL)), 0.02, mi('steel_dark'), n=4)
    return propkit.finish(ENV, f'ceiling_light_{state}', [smooth('batten', bm, angle=40)],
                          shots=[('front', V((0.6, 0.3, -0.25)), 50)])


for st in ('on', 'off'):
    build(st)
