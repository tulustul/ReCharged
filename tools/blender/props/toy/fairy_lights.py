"""Multicoloured fairy lights (decor, out of reach): 40 bulbs (`lamp_*`, dimmed by the time of
day) on a black wire spanning 30 u with a 2 u sag. Origin at the middle of the span at the
attachment height (the ends); everything hangs below it. Placed by a map's decor.ron near the
ceiling.

    blender -b -P tools/blender/props/toy/fairy_lights.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

SPAN, SAG, N = 30.0, 2.0, 40
core.reset()
plastic('wire', '#1D1D20', 0.4)
BULBS = {'lamp_red': ('#FF3B30', (1.0, 0.03, 0.0)), 'lamp_yellow': ('#FFD60A', (1.0, 0.65, 0.0)),
         'lamp_green': ('#34C759', (0.05, 1.0, 0.08)), 'lamp_blue': ('#2F7BFF', (0.05, 0.25, 1.0)),
         'lamp_pink': ('#FF4FB8', (1.0, 0.1, 0.55))}
for name, (base, glow) in BULBS.items():
    core.material(name, base, 0.0, 0.25, emission=glow, strength=30.0)
mi = core.mi
names = list(BULBS)


def sag(x):
    return -SAG * (1 - (2 * x / SPAN) ** 2)


wire = core.new_bm()
geo.tube(wire, [V((x, 0, sag(x))) for x in (SPAN * (k / 30 - 0.5) for k in range(31))], 0.03,
         mi('wire'), n=4)
bulbs = core.new_bm()
for k in range(N):
    x = SPAN * ((k + 0.5) / N - 0.5)
    top = V((x, 0, sag(x)))
    geo.cylinder(wire, top, top - V((0, 0, 0.16)), 0.06, mi('wire'), n=5)
    geo.sphere(bulbs, top - V((0, 0, 0.3)), 0.13, mi(names[k % len(names)]), n=8, rings=5,
               scale=(1, 1, 1.35))
propkit.finish(ENV, 'fairy_lights', [smooth('wire', wire), smooth('bulbs', bulbs)], below=3.0,
               car=(0.0, 4.0, 180), before_preview=lambda ob: setattr(ob.location, 'z', 5.0))
