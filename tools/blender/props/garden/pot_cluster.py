"""Flower-pot cluster (hero): three big terracotta pots — a clipped box ball, lavender, pink
geraniums with trailing petunias — plus a small pot lying on its side, 5.6 × 4.6 u, up to 5 u
tall: the flower-pot chicane's corner markers (catalog: garden_pot_cluster; the collider
cylinders match the pots below their rims).

    blender -b -P tools/blender/props/garden/pot_cluster.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

start('terracotta', 'terracotta_dark', 'soil', 'leaf', 'leaf_light', 'leaf_blue', 'leaf_dark',
      'stem', 'flower_pink', 'flower_purple', 'flower_white', 'flower_yellow')
rng = random.Random(21)
# (Blender x, y = -Bevy z) of the catalog cylinders: big, medium, mid-large
BIG, MED, MID = V((-1.3, -0.6, 0)), V((1.45, -1.05, 0)), V((0.75, 1.25, 0))
bm = core.new_bm()
s_big = pot(bm, BIG, 2.3, 1.3)
s_mid = pot(bm, MID, 1.8, 1.02, body='terracotta_dark')
s_med = pot(bm, MED, 1.44, 0.82)
# a small pot knocked on its side by the big one
lying = core.new_bm()
pot(lying, V((0, 0, -0.35)), 0.7, 0.4)
bmesh.ops.transform(lying, matrix=Matrix.Translation(V((-0.2, 1.9, 0.43)))
                    @ Matrix.Rotation(math.radians(90), 4, 'Y') @ Matrix.Rotation(0.6, 4, 'X'),
                    verts=lying.verts)
pots_ob = smooth('pots', bm, angle=50)
lying_ob = smooth('lying', lying, angle=50)
pl = core.new_bm()
box_ball(pl, BIG + V((0, 0, s_big + 1.05)), 1.25, rng)
lavender(pl, MID + V((0, 0, s_mid)), 0.95, 1.7, rng)
geranium(pl, MED + V((0, 0, s_med + 0.3)), 0.85, rng, flower='flower_pink')
trailing(pl, MED + V((0, 0, s_med)), 0.82, rng)
plants = smooth('plants', pl, angle=70)
finish('pot_cluster', [pots_ob, lying_ob, plants], hero=True, car=(3.6, -2.4, 30.0))
