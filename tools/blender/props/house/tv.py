"""Flat TV (55", off) on a centre foot: screen 5.5 x 3.2 u, 0.12 u thick, glossy black glass,
graphite back; stands on the sideboard (origin at the foot's base). Front (+Y).

    blender -b -P tools/blender/props/house/tv.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

W, H, T, LIFT = 5.5, 3.2, 0.12, 0.3
core.reset()
materials('plastic_black', 'screen', 'metal_black')

bm = core.new_bm()
geo.box(bm, V((0, 0, LIFT + H / 2)), (W, T, H), mi('plastic_black'))
body = core.obj_from_bm('body', bm, bevel=0.02, segments=2)
bm = core.new_bm()
geo.box(bm, V((0, T / 2 + 0.003, LIFT + H / 2 + 0.02)), (W - 0.12, 0.006, H - 0.16), mi('screen'))
screen = core.obj_from_bm('screen', bm, weighted=False)
bm = core.new_bm()
geo.box(bm, V((0, -0.05, 0.04)), (2.0, 1.0, 0.08), mi('metal_black'))
geo.box(bm, V((0, -0.12, LIFT / 2 + 0.1)), (0.5, 0.1, LIFT + 0.2), mi('metal_black'))
foot = core.obj_from_bm('foot', bm, bevel=0.015, segments=1)

done('tv', [body, screen, foot], car=(3.8, 1.5, 200))
