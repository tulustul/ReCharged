"""Tool wall (decor): a wall bench (12 x 3 u, 4 u high) with a vise, a bench grinder and a few
tools, under a pegboard hung with spanners, screwdrivers, hammers and pliers, and a work lamp.
Origin at the wall face on the floor, front +Y.

    blender -b -P tools/blender/props/workshop/tool_wall.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

L, D, TOP = 12.0, 3.0, 4.0
start('butcher_block', 'paint_grey', 'steel_dark', 'pegboard', 'chrome', 'steel', 'plastic_red',
      'plastic_yellow', 'plastic_black', 'lift_blue', 'chest_red', 'light_tube')
bm = core.new_bm()
f = geo.box(bm, V((0, D / 2, TOP - 0.12)), (L, D, 0.24), mi('butcher_block'))
planar_uv(bm, f, (1, 0, 0), (0, 1, 0), 0.25)
for x in (-L / 2 + 0.3, L / 2 - 0.3):
    for y in (0.3, D - 0.3):
        geo.box(bm, V((x, y, (TOP - 0.24) / 2)), (0.3, 0.3, TOP - 0.24), mi('paint_grey'))
geo.box(bm, V((0, D / 2, 0.7)), (L - 0.4, D - 0.4, 0.06), mi('paint_grey'))
geo.box(bm, V((L / 4, D / 2 + 0.1, 2.4)), (L / 2 - 0.8, D - 0.6, 2.0), mi('chest_red'))  # drawers
f = geo.box(bm, V((0, 0.06, TOP + 3.4)), (L - 0.6, 0.12, 6.0), mi('pegboard'))
planar_uv(bm, f, (1, 0, 0), (0, 0, 1), 0.25)
geo.box(bm, V((0, 0.13, TOP + 6.45)), (L - 0.4, 0.14, 0.12), mi('paint_grey'))
bench = core.obj_from_bm('bench', bm, bevel=0.02, segments=1, angle=40)

bm = core.new_bm()
# vise (left end) and grinder (right end)
vx = -L / 2 + 1.4
geo.box(bm, V((vx, D - 0.6, TOP + 0.35)), (1.0, 1.2, 0.7), mi('lift_blue'))
geo.box(bm, V((vx, D - 0.05, TOP + 0.75)), (1.1, 0.25, 0.5), mi('steel_dark'))
geo.cylinder(bm, V((vx, D - 1.1, TOP + 0.75)), V((vx, D + 0.4, TOP + 0.75)), 0.08, mi('chrome'), n=6)
gx = L / 2 - 1.5
geo.box(bm, V((gx, D / 2, TOP + 0.5)), (0.9, 0.8, 1.0), mi('paint_grey'))
for sx in (-1, 1):
    geo.cylinder(bm, V((gx + sx * 0.5, D / 2, TOP + 0.75)), V((gx + sx * 0.95, D / 2, TOP + 0.75)),
                 0.42, mi('steel_dark'), n=12)
# pegboard tools: spanners (graded), screwdrivers, hammers, pliers
for k in range(8):
    x = -L / 2 + 1.0 + k * 0.5
    ln = 1.0 + k * 0.12
    geo.box(bm, V((x, 0.18, TOP + 5.6 - ln / 2)), (0.12, 0.05, ln), mi('chrome'))
    geo.box(bm, V((x, 0.18, TOP + 5.62)), (0.26, 0.05, 0.22), mi('chrome'))
for k in range(6):
    x = -L / 2 + 6.0 + k * 0.42
    geo.cylinder(bm, V((x, 0.22, TOP + 5.7)), V((x, 0.22, TOP + 5.1)), 0.11,
                 mi(('plastic_red', 'plastic_yellow')[k % 2]), n=8)
    geo.cylinder(bm, V((x, 0.22, TOP + 5.1)), V((x, 0.22, TOP + 4.3)), 0.035, mi('steel'), n=6)
for k in range(2):  # hammers
    x = 1.6 + k * 1.1
    geo.box(bm, V((x, 0.2, TOP + 4.6)), (0.12, 0.08, 1.3), mi('steel_dark'))
    geo.box(bm, V((x, 0.22, TOP + 5.3)), (0.6, 0.18, 0.22), mi('steel'))
for k in range(3):  # pliers
    x = 3.6 + k * 0.6
    for a in (-0.12, 0.12):
        geo.prism(bm, geo.rrect(0.1, 0.06, 0.0, 1)[::2], V((x, 0.2, TOP + 5.4)),
                  V((x + a, 0.2, TOP + 4.5)), mi('plastic_red'), up=V((0, 1, 0)))
# a work lamp over the bench
geo.box(bm, V((0, 1.2, TOP + 6.9)), (5.0, 0.6, 0.25), mi('paint_grey'))
geo.box(bm, V((0, 1.2, TOP + 6.75)), (4.6, 0.4, 0.06), mi('light_tube'))
tools = core.obj_from_bm('tools', bm, weighted=False, angle=40)
finish('tool_wall', [bench, tools], budget=KIT, car=(0.0, D + 3.0, 180.0))
