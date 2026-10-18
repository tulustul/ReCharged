"""Lamp props (catalog: garden_porch_light / _fairy_lights; real lights at night come from the
catalog's `lights`, the `lamp_warm` bulbs are dimmed by day). Porch light: a black lantern on a
wall bracket 9 u up the house wall, facing the garden (+Y here). Fairy lights: a string of warm
bulbs sagging between two posts 16 u apart.

    blender -b -P tools/blender/props/garden/lights.py -- [--preview DIR] [--only NAME]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
only = argv[argv.index('--only') + 1] if '--only' in argv else None


def porch_light():
    start('enamel_black', 'lamp_warm', 'lamp_glass')
    K = mi('enamel_black')
    bm = core.new_bm()
    geo.box(bm, V((0, 0.06, 9.3)), (0.45, 0.12, 0.9), K)                       # wall plate
    geo.tube(bm, [V((0, 0.1, 9.55)), V((0, 0.55, 9.6)), V((0, 0.85, 9.45))], 0.05, K, n=6,
             corner=0.15)                                                       # bracket
    c = V((0, 0.85, 8.95))
    geo.box(bm, c + V((0, 0, 0.42)), (0.5, 0.5, 0.08), K)                        # lantern top
    geo.cylinder(bm, c + V((0, 0, 0.46)), c + V((0, 0, 0.62)), 0.2, K, n=4, r1=0.04)
    geo.box(bm, c - V((0, 0, 0.42)), (0.46, 0.46, 0.08), K)                      # bottom
    for sx in (-1, 1):
        for sy in (-1, 1):
            geo.box(bm, c + V((sx * 0.21, sy * 0.21, 0)), (0.05, 0.05, 0.8), K)  # corner bars
    frame = part('lantern', bm, bevel=0.01, segments=1)
    glass = core.new_bm()
    geo.box(glass, c, (0.4, 0.4, 0.76), mi('lamp_glass'))  # glowing panes (dimmed by day)
    geo.cylinder(glass, c - V((0, 0, 0.25)), c + V((0, 0, 0.15)), 0.09, mi('lamp_warm'), n=10)
    geo.sphere(glass, c + V((0, 0, 0.18)), 0.1, mi('lamp_warm'), n=10, rings=5)
    return [frame, smooth('glass', glass)], dict(car=(1.5, 1.0, 20.0))


def fairy_lights():
    start('pine_dark', 'plastic_black', 'lamp_warm')
    bm = core.new_bm()
    for sx in (-1, 1):
        board(bm, (sx * 8, 0, 2.9), (0.24, 0.24, 5.8), mi('pine_dark'), axis=2, seed=sx + 3)
    posts = part('posts', bm, bevel=0.02, segments=1)
    wire = core.new_bm()
    pts = [V((x, 0, 5.6 - 1.6 * (1 - (x / 8) ** 2))) for x in (-8 + 16 * i / 32 for i in range(33))]
    geo.tube(wire, pts, 0.025, mi('plastic_black'), n=4)
    for i in range(2, 31, 2):
        p = pts[i] - V((0, 0, 0.12))
        geo.cylinder(wire, pts[i], p, 0.04, mi('plastic_black'), n=5)
        geo.sphere(wire, p - V((0, 0, 0.1)), 0.11, mi('lamp_warm'), n=8, rings=5,
                   scale=(1, 1, 1.3))
    return [posts, smooth('string', wire)], dict(car=(0.0, -3.0, 0.0))


for name, fn in (('porch_light', porch_light), ('fairy_lights', fairy_lights)):
    if only and name != only:
        continue
    parts, kw = fn()
    finish(name, parts, **kw)
