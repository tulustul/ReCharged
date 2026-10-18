"""Moulded toy-track jump ramp: an orange plastic wedge 4 u wide rising 2.5 u over 9 u
(15.5 deg) toward its front (+Y), with raised side lips, moulded ribs on the flanks and a yellow
top plate. Drivable (ToyTrack surface); leads onto the play table.

    blender -b -P tools/blender/props/toy/track_ramp.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

HW, HL, HT = 2.0, 4.5, 2.5   # half width, half length, rise
core.reset()
plastic('orange', ORANGE, 0.25)
plastic('lip', '#F59A3E', 0.25)
plastic('yellow', YELLOW)
mi = core.mi

body = core.new_bm()
extrude(body, 'x', [(-HL, 0.0), (HL, 0.0), (HL, HT), (HL - 0.6, HT)], -HW, HW, mi('orange'))
body_ob = part('body', body, bevel=0.04, segments=2)
lips = core.new_bm()
slope = HT / (2 * HL - 0.6)
for s in (-1, 1):  # side lips following the slope
    extrude(lips, 'x', [(-HL, 0.0), (HL - 0.6, HT), (HL - 0.6, HT + 0.25), (-HL + 0.5, 0.25)],
            s * HW - 0.3 * (s > 0), s * HW + 0.3 * (s < 0), mi('lip'))
lips_ob = part('lips', lips, bevel=0.03, segments=2)
ribs = core.new_bm()
for s in (-1, 1):  # moulded ribs on the flanks
    for k in range(1, 6):
        y = -HL + k * (2 * HL) / 6
        h = min(HT, (y + HL) * slope) - 0.15
        if h > 0.3:
            geo.box(ribs, V((s * (HW + 0.04), y, h / 2)), (0.1, 0.22, h), mi('orange'))
geo.box(ribs, V((0, HL - 0.3, HT + 0.01)), (2 * HW - 0.6, 0.6, 0.04), mi('yellow'))
ribs_ob = part('ribs', ribs, bevel=0.015, segments=1)
chev = core.new_bm()  # printed chevrons pointing up the ramp
n = V((0, -slope, 1)).normalized()
for k in range(3):
    y0 = -HL + 1.6 + k * 2.2
    for s in (-1, 1):
        pts = [V((0, y0 + 0.9, 0)), V((s * 1.3, y0, 0)), V((s * 1.3, y0 - 0.45, 0)),
               V((0, y0 + 0.45, 0))]
        vs = [chev.verts.new(V((p.x, p.y, (p.y + HL) * slope)) + n * 0.01) for p in pts]
        f = chev.faces.new(vs if s < 0 else vs[::-1])
        f.material_index = mi('yellow')
chev_ob = part('chevrons', chev, bevel=0.0)
propkit.finish(ENV, 'track_ramp', [body_ob, lips_ob, ribs_ob, chev_ob], car=(3.2, -3.0, 0))
