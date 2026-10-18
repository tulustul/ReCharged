"""Hose-reel cart (hero): green plastic A-frame cart on two wheels with a crank, a carry handle
and a drum wound with bright green hose (coils of torus rings); the hose end drops off the
drum to the ground at the front, where the spiral of hose on the island starts
(catalog: garden_hose_reel; 2.6 × 2.4 × 4.4 u).

    blender -b -P tools/blender/props/garden/hose_reel.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

AX = V((0, 0.0, 1.55))       # drum axle
DRUM_W, FLANGE_R, CORE_R = 1.9, 1.08, 0.55
HOSE_R = 0.085

start('plastic_green', 'plastic_black', 'hose_green', 'rubber', 'steel', 'brass')
G, K, H, R, S = mi('plastic_green'), mi('plastic_black'), mi('hose_green'), mi('rubber'), mi('steel')
frame = core.new_bm()
for sx in (-1, 1):
    x = sx * (DRUM_W / 2 + 0.16)
    # A-frame side: front foot, rear leg to the wheel, upright to the handle
    geo.tube(frame, [V((x, 0.95, 0.05)), AX + V((x, 0, 0)), V((x, -0.85, 0.45))], 0.11, G, n=8,
             corner=0.3)
    geo.tube(frame, [AX + V((x, 0, 0)), V((x, -0.35, 3.4)), V((x, -0.55, 4.1))], 0.1, G, n=8,
             corner=0.3)
    geo.box(frame, V((x, 1.0, 0.06)), (0.36, 0.5, 0.12), K)            # foot pad
    geo.cylinder(frame, AX + V((x - 0.12 * sx, 0, 0)), AX + V((x + 0.14 * sx, 0, 0)), 0.22, K, n=16)
geo.tube(frame, [V((-DRUM_W / 2 - 0.16, -0.55, 4.1)), V((-0.6, -0.62, 4.32)),
                 V((0.6, -0.62, 4.32)), V((DRUM_W / 2 + 0.16, -0.55, 4.1))], 0.1, G, n=8,
         corner=0.25)                                                   # carry handle
geo.cylinder(frame, V((-0.55, -0.62, 4.32)), V((0.55, -0.62, 4.32)), 0.14, K, n=10)  # grip
geo.cylinder(frame, V((-DRUM_W / 2 - 0.2, -0.85, 0.45)), V((DRUM_W / 2 + 0.2, -0.85, 0.45)),
             0.06, S, n=8)                                              # wheel axle
geo.cylinder(frame, V((-DRUM_W / 2 - 0.16, 0.95, 0.12)), V((DRUM_W / 2 + 0.16, 0.95, 0.12)),
             0.08, G, n=8)                                              # front stretcher
# crank on the right: arm and knob
cx = DRUM_W / 2 + 0.32
geo.box(frame, AX + V((cx, 0, -0.25)), (0.1, 0.16, 0.7), K)
geo.cylinder(frame, AX + V((cx, 0, -0.55)), AX + V((cx + 0.45, 0, -0.55)), 0.07, K, n=8)
frame_ob = smooth('frame', frame, angle=45)

drum = core.new_bm()
for sx in (-1, 1):  # flanges with a thick rim
    x = sx * DRUM_W / 2
    geo.cylinder(drum, AX + V((x - 0.05, 0, 0)), AX + V((x + 0.05, 0, 0)), FLANGE_R, K, n=28)
    geo.tube(drum, [AX + V((x, FLANGE_R * math.cos(a), FLANGE_R * math.sin(a)))
                    for a in (2 * math.pi * k / 28 for k in range(29))], 0.06, K, n=6)
geo.cylinder(drum, AX + V((-DRUM_W / 2, 0, 0)), AX + V((DRUM_W / 2, 0, 0)), CORE_R, K, n=16)
drum_ob = smooth('drum', drum, angle=40)

hose = core.new_bm()
coil_r = FLANGE_R - 0.12
n_turns = int((DRUM_W - 0.1) / (2 * HOSE_R))
for k in range(n_turns):
    x = -DRUM_W / 2 + 0.05 + HOSE_R + k * 2 * HOSE_R
    ring = [AX + V((x, coil_r * math.cos(a), coil_r * math.sin(a)))
            for a in (2 * math.pi * j / 24 for j in range(25))]
    geo.tube(hose, ring, HOSE_R, H, n=6)
# the free end: off the bottom front of the drum, down to the ground and away (+Y)
geo.tube(hose, [AX + V((0.4, coil_r * 0.7, -coil_r * 0.7)), V((0.45, 1.25, 0.6)),
                V((0.5, 1.5, HOSE_R)), V((0.7, 2.3, HOSE_R))], HOSE_R, H, n=6, corner=0.4)
# (hose_spiral.glb continues from (0.7, 2.3) when placed at the reel's pose)
hose_ob = smooth('hose', hose, angle=50)

wheels = core.new_bm()
for sx in (-1, 1):
    x = sx * (DRUM_W / 2 + 0.42)
    c = V((x, -0.85, 0.45))
    geo.cylinder(wheels, c - V((0.12, 0, 0)), c + V((0.12, 0, 0)), 0.45, R, n=20)
    geo.cylinder(wheels, c - V((0.13, 0, 0)), c + V((0.13, 0, 0)), 0.24, G, n=12)
wheel_ob = smooth('wheels', wheels, angle=35)
finish('hose_reel', [frame_ob, drum_ob, hose_ob, wheel_ob], hero=True, car=(2.6, 1.5, -60.0))
