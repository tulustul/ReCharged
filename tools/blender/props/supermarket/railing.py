"""Entrance railing (kit, art bible §3.5): an 8 u stainless segment with posts at both ends,
a frosted kick panel (1.6 u, so an RC car cannot slip under) and a handrail at 4.4 u.

    blender -b -P tools/blender/props/supermarket/railing.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, P, H = C.RAIL_L, C.RAIL_PANEL_H, C.RAIL_H

start('brushed', 'chrome', 'glass_frost', 'black_plastic')
frame = core.new_bm()
for x in (-L / 2 + 0.1, L / 2 - 0.1):
    geo.cylinder(frame, V((x, 0, 0)), V((x, 0, H)), 0.1, mi('brushed'), n=10)
    geo.cylinder(frame, V((x, 0, 0)), V((x, 0, 0.05)), 0.26, mi('brushed'), n=12)
    geo.sphere(frame, V((x, 0, H)), 0.13, mi('chrome'), n=10, rings=6)
geo.cylinder(frame, V((-L / 2 + 0.1, 0, H - 0.05)), V((L / 2 - 0.1, 0, H - 0.05)), 0.08,
             mi('chrome'), n=10)
geo.cylinder(frame, V((-L / 2 + 0.1, 0, 3.0)), V((L / 2 - 0.1, 0, 3.0)), 0.05, mi('chrome'), n=8)
geo.box(frame, V((0, 0, P + 0.03)), (L - 0.3, 0.12, 0.06), mi('brushed'))
panel = core.new_bm()
geo.box(panel, V((0, 0, P / 2 + 0.02)), (L - 0.3, 0.06, P - 0.06), mi('glass_frost'))
propkit.finish(ENV, 'railing', [smooth('frame', frame, angle=40), part('panel', panel)],
               car=(L / 2 + 2, 2.5, 200))
