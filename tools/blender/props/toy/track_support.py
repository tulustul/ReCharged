"""Toy-track support pillar: a segmented yellow column with a blue clip head and a bracket that
reaches forward under the ribbon's edge. The origin is the bracket top (placed at the ribbon's
underside by props/toy/supports.py); the column runs 8 u down so it meets the floor at any
height. Static.

    blender -b -P tools/blender/props/toy/track_support.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

DOWN = 8.0     # column length below the bracket top
HALF = 0.4     # column half size
core.reset()
plastic('yellow', YELLOW)
plastic('blue', BLUE)

col = core.new_bm()
geo.box(col, V((0, 0, (0.4 - DOWN) / 2)), (2 * HALF - 0.08, 2 * HALF - 0.08, DOWN + 0.4),
        core.mi('yellow'))  # core visible in the grooves
z = 0.35
while z > -DOWN:  # stacked segments with grooves between them
    z0 = max(z - 0.92, -DOWN)
    geo.box(col, V((0, 0, (z + z0) / 2)), (2 * HALF, 2 * HALF, z - z0), core.mi('yellow'))
    z -= 1.0
column = part('column', col, bevel=0.05, segments=2)

head = core.new_bm()
geo.slab(head, [(-0.47, 0.47, 0.33, 0.6, 0.1), (0.47, 0.47, 0.33, 0.6, 0.1)], core.mi('blue'))
bracket = core.new_bm()
extrude(bracket, 'x', rpoly([(HALF - 0.05, -0.85), (HALF + 0.55, -0.3),
                                         (1.45, -0.3), (1.45, 0.0), (HALF - 0.05, 0.0)],
                                        [0, 0.08, 0.1, 0.1, 0], 3), -0.33, 0.33, core.mi('blue'))
heads = part('head', head, bevel=0.03, segments=2)
brackets = part('bracket', bracket, bevel=0.03, segments=2)


def context(ob):
    """Preview only: lift the pillar to a 3.5 u bracket and lay a ribbon slab on it."""
    ob.location.z = 3.5
    core.material('ribbon', ORANGE, 0.0, 0.25)
    bm = core.new_bm()
    geo.box(bm, V((0, 6.65, 3.625)), (16.0, 12.1, 0.25), MATS.index('ribbon'))  # bottom 0.6..12.7
    geo.box(bm, V((0, 1.0, 4.225)), (16.0, 0.6, 1.45), MATS.index('ribbon'))  # the barrier
    core.obj_from_bm('ribbon', bm, weighted=False)


MATS = core.MATS
propkit.finish(ENV, 'track_support', [column, heads, brackets], below=DOWN,
               car=(3.0, 6.0, 90), before_preview=context)
