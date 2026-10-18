"""The unrolled hose (decor): continues from the hose reel's free end at (0.7, 2.3) and lies on
the lawn in a loose spiral, two turns out to a 5 u radius, around the reel — the "hose-reel
spiral" the track winds around. Place it at the reel's position and yaw.

    blender -b -P tools/blender/props/garden/hose_spiral.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

R = 0.085
start('hose_green', 'brass')
bm = core.new_bm()
rng = random.Random(41)
r0, r1, turns = math.hypot(0.7, 2.3), 5.0, 2.0
a0 = math.atan2(2.3, 0.7)
pts = []
steps = 220
for k in range(steps + 1):
    t = k / steps
    a = a0 - 2 * math.pi * turns * t           # clockwise seen from above
    r = r0 + (r1 - r0) * t + 0.12 * math.sin(7 * a + 1.3)
    pts.append(V((r * math.cos(a), r * math.sin(a), R + 0.01 * rng.random())))
geo.tube(bm, pts, R, mi('hose_green'), n=6)
end, d = pts[-1], (pts[-1] - pts[-2]).normalized()
geo.cylinder(bm, end, end + d * 0.35, R * 1.6, mi('brass'), n=8, r1=R * 1.2)   # spray nozzle
finish('hose_spiral', [smooth('hose', bm, angle=50)], below=0.06, car=(6.5, 0.0, 0.0))
