"""Empty EUR pallet (kit 'pallet', art bible §3.5): 5.33 x 3.55 x 0.64 u, deck boards, nine
blocks and three runners; drivable (Wood).

    blender -b -P tools/blender/props/supermarket/pallet.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

start('pallet_wood', 'pallet_wood_dark')
bm = core.new_bm()
pallet(bm, C.PALLET_L, C.PALLET_W)
propkit.finish(ENV, 'pallet', [part('pallet', bm, bevel=0.02, segments=1)],
               car=(C.PALLET_L / 2 + 2, 2.5, 210))
