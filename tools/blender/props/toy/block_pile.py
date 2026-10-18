"""Static heap of jumbo blocks tumbled on the floor (cubes, a pillar, a roof, a plank in stained
wood): clutter around the play areas without spending the dynamic-body budget. Static.

    blender -b -P tools/blender/props/toy/block_pile.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
for name, c, seed in (('red_wood', RED, 21), ('blue_wood', BLUE, 22), ('yellow_wood', YELLOW, 23),
                      ('green_wood', GREEN, 24)):
    wood(name, c, 0.4, contrast=0.08, seed=seed)
beech()
mi = core.mi
parts = []


def place(bm, loc, rot_deg):
    m = Matrix.Translation(V(loc)) @ Matrix.Rotation(math.radians(rot_deg[2]), 4, 'Z') \
        @ Matrix.Rotation(math.radians(rot_deg[1]), 4, 'Y') \
        @ Matrix.Rotation(math.radians(rot_deg[0]), 4, 'X')
    geo.bmesh.ops.transform(bm, matrix=m, verts=bm.verts)
    return bm


def cube(mat, loc, rot):
    bm = core.new_bm()
    geo.box(bm, V((0, 0, 0)), (1.4, 1.4, 1.4), mi(mat))
    parts.append(part(f'c{len(parts)}', box_uv(place(bevel_bm(bm, 0.03, mat=mi(mat)), loc, rot)),
                      bevel=0.0))


cube('red_wood', (0, 0, 0.7), (0, 0, 12))
cube('blue_wood', (1.45, 0.35, 0.7), (0, 0, -20))
cube('yellow_wood', (0.6, 0.15, 2.1), (0, 0, 35))
cube('green_wood', (-1.35, 1.1, 0.89), (0, 18, 50))  # tipped: centre at 0.7 (cos + sin 18°)
bm = core.new_bm()  # plank lying across
geo.box(bm, V((0, 0, 0)), (2.8, 1.4, 0.7), mi('beech'))
parts.append(part('plank', box_uv(place(bevel_bm(bm, 0.03, mat=mi('beech')), (0.4, -1.5, 0.35),
                                              (0, 0, 8))),
                  bevel=0.0))
bm = core.new_bm()  # roof block fallen on its side
extrude(bm, 'x', [(-0.7, 0.0), (0.7, 0.0), (0.0, 0.7)], -0.7, 0.7, mi('blue_wood'))
parts.append(part('roof', box_uv(place(bevel_bm(bm, 0.03, mat=mi('blue_wood')), (2.0, -1.1, 0.0),
                                             (0, 0, 70))),
                  bevel=0.0))
bm = core.new_bm()  # pillar lying on its side
geo.cylinder(bm, V((-0.7, 0, 0)), V((0.7, 0, 0)), 0.7, mi('red_wood'), n=24)
parts.append(smooth('pillar', box_uv(place(bm, (-1.6, -1.3, 0.7), (0, 0, -35)))))
propkit.finish(ENV, 'block_pile', parts, car=(3.4, 2.4, 200))
