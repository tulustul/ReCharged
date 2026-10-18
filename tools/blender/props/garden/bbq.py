"""Kettle barbecue (hero): black enamel bowl and lid with a vent and handle on a three-leg stand
with two wheels, an ash pan and a lower rack, 2.6 u wide, 4.4 u tall (catalog: garden_bbq).

    blender -b -P tools/blender/props/garden/bbq.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

R, RIM = 1.2, 2.75          # bowl radius, rim height
start('enamel_black', 'steel', 'steel_dark', 'plastic_black', 'rubber')
E, S, SD, K = mi('enamel_black'), mi('steel'), mi('steel_dark'), mi('plastic_black')
bm = core.new_bm()
bowl = [(0, RIM - R * 0.95)] + [(R * math.sin(a), RIM - R * 0.95 * math.cos(a))
                                for a in (math.radians(d) for d in range(10, 91, 10))]
revolve(bm, bowl + [(R + 0.04, RIM + 0.03), (0, RIM + 0.03)], E, n=32)
lid = [(0, RIM + 0.05), (R + 0.03, RIM + 0.05)] + \
    [(R * math.cos(a), RIM + 0.06 + R * 0.75 * math.sin(a))
     for a in (math.radians(d) for d in range(10, 91, 10))]
revolve(bm, lid, E, n=32)
geo.cylinder(bm, V((0, 0, RIM + 0.06 + R * 0.75)), V((0, 0, RIM + 0.06 + R * 0.75 + 0.06)), 0.32, S,
             n=16)                                                   # lid vent
body = smooth('body', bm, angle=40)
st = core.new_bm()
legs = [(a, (0.95, 0.95, 0.95)[i]) for i, a in enumerate((90, 210, 330))]
for i, (a, _) in enumerate(legs):
    top = V((math.cos(math.radians(a)) * R * 0.7, math.sin(math.radians(a)) * R * 0.7, RIM - 0.75))
    foot = V((math.cos(math.radians(a)) * 1.25, math.sin(math.radians(a)) * 1.25, 0.05))
    geo.cylinder(st, foot, top, 0.07, S, n=8)
    if i == 0:
        geo.cylinder(st, foot, foot - V((0, 0, 0.05)) + V((0, 0, 0.1)), 0.13, K, n=8)  # foot cap
# wheels on the two rear legs
for a in (210, 330):
    c = V((math.cos(math.radians(a)) * 1.3, math.sin(math.radians(a)) * 1.3, 0.38))
    d = V((-math.sin(math.radians(a)), math.cos(math.radians(a)), 0)) * 0.0 + V((1, 0, 0))
    geo.cylinder(st, c - V((0.1, 0, 0)), c + V((0.1, 0, 0)), 0.38, mi('rubber'), n=16)
geo.cylinder(st, V((-1.13, -0.65, 0.38)), V((1.13, -0.65, 0.38)), 0.05, S, n=6)   # axle
# lower rack triangle and ash pan
ring = [V((math.cos(math.radians(a)) * 0.98, math.sin(math.radians(a)) * 0.98, 0.95))
        for a in (90, 210, 330, 90)]
geo.tube(st, ring, 0.05, S, n=6)
revolve(st, [(0, 1.25), (0.5, 1.25), (0.55, 1.45), (0, 1.45)], SD, n=16)
# lid handle and side handles
geo.tube(st, [V((-0.4, 0, RIM + R * 0.78)), V((-0.4, 0, RIM + R * 0.95)), V((0.4, 0, RIM + R * 0.95)),
              V((0.4, 0, RIM + R * 0.78))], 0.05, S, n=6, corner=0.08)
geo.cylinder(st, V((-0.32, 0, RIM + R * 0.95)), V((0.32, 0, RIM + R * 0.95)), 0.1, K, n=8)
for sx in (-1, 1):
    geo.cylinder(st, V((sx * (R + 0.02), 0, RIM - 0.15)), V((sx * (R + 0.3), 0, RIM - 0.15)), 0.08,
                 K, n=8)
stand = smooth('stand', st, angle=40)
finish('bbq', [body, stand], hero=True, car=(2.4, -1.5, 30.0))
