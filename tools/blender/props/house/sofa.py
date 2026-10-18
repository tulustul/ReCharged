"""Sofa (hero): a big sage three-seater on tall tapered oak legs; the track runs underneath it
front-to-back. 2.4 x 0.95 m, 0.93 m tall (10.66 x 4.22 x 4.2 u), 1.75 u (39 cm) clear under the
frame so every car fits (the legs are stylised tall for the tunnel). The underside is a dark
dust cover (the tunnel ceiling); mustard and rust throw pillows, a knitted throw on one arm.
Catalog colliders: seat block 1.75..2.55, back 2.55..4.2 (rear 1.1 u), arms 2.55..3.2, legs.

    blender -b -P tools/blender/props/house/sofa.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

W, D = 10.66, 4.22               # width (x), depth (y; front = +y)
CLEAR, BASE_TOP = 1.75, 2.15     # underside of the frame, top of the upholstered base
ARM_W, ARM_TOP = 0.9, 3.2
core.reset()
materials('fabric_sage', 'fabric_dark', 'oak', 'fabric_mustard', 'fabric_rust', 'knit_cream')
sage, dark = mi('fabric_sage'), mi('fabric_dark')

# upholstered base: dark dust cover underneath, sage sides
bm = core.new_bm()
rbox(bm, V((0, 0, (CLEAR + BASE_TOP) / 2)), (W - 0.02, D - 0.02, BASE_TOP - CLEAR), 0.09, sage,
     seg=3)
for f in bm.faces:
    if f.normal.z < -0.9:
        f.material_index = dark
base = core.obj_from_bm('base', box_uv(bm, 'x'), weighted=False, angle=50)

# arms (rounded tops) and the back frame
bm = core.new_bm()
for s in (-1, 1):
    rbox(bm, V((s * (W - ARM_W) / 2, 0, (CLEAR + ARM_TOP) / 2 + 0.02)),
         (ARM_W, D, ARM_TOP - CLEAR - 0.04), 0.3, sage, seg=4, axis='y')
rbox(bm, V((0, -D / 2 + 0.38, (BASE_TOP + 3.75) / 2)), (W - 2 * ARM_W + 0.1, 0.72, 3.75 - BASE_TOP),
     0.22, sage, seg=4)
frame = core.obj_from_bm('frame', box_uv(bm, 'x'), weighted=False, angle=50)

# three seat cushions and three back cushions (puffy, slightly irregular)
bm = core.new_bm()
inner = W - 2 * ARM_W
cw = inner / 3
for i in range(3):
    x = -inner / 2 + cw * (i + 0.5)
    rbox(bm, V((x, 0.42, BASE_TOP + 0.24)), (cw - 0.06, D - 0.95, 0.5), 0.2, sage, seg=4,
         puff=0.08)
    rbox(bm, V((x, -D / 2 + 1.13, 3.38 + 0.03 * (i - 1))), (cw - 0.08, 0.72, 1.62), 0.26, sage,
         seg=4, puff=0.1, rot=(0, -11, 0))
cushions = core.obj_from_bm('cushions', box_uv(bm, 'x'), weighted=False, angle=60)

# throw pillows + a folded knitted throw over the left arm
bm = core.new_bm()
rbox(bm, V((-3.55, -0.35, 3.25)), (1.85, 0.45, 1.75), 0.28, mi('fabric_mustard'), seg=4,
     puff=0.18, rot=(18, -16, 8), axis='y')
rbox(bm, V((3.45, -0.38, 3.2)), (1.7, 0.45, 1.6), 0.26, mi('fabric_rust'), seg=4, puff=0.18,
     rot=(-20, -18, -6), axis='y')
# knitted throw draped over the left arm: a band swept along a U over the arm's rounded top
ax = -(W - ARM_W) / 2
x_in, x_out = ax + ARM_W / 2 + 0.04, -W / 2 - 0.04
path = [V((x_in, 0.55, ARM_TOP - 0.55)), V((x_in + 0.01, 0.55, ARM_TOP - 0.2))]
for k in range(1, 8):  # over the top: half-round of the arm's 0.3 fillet, widened
    a = math.pi * k / 8
    path.append(V((ax + (x_in - ax) * math.cos(a), 0.55, ARM_TOP - 0.2 + 0.25 * math.sin(a))))
path += [V((x_out - 0.01, 0.57, ARM_TOP - 0.2)), V((x_out - 0.04, 0.6, ARM_TOP - 0.9))]
sweep(bm, path, 1.65, 0.07, mi('knit_cream'), r=0.03)
throws = core.obj_from_bm('throws', box_uv(bm, 'x'), weighted=False, angle=60)

# tall tapered oak legs, splayed outwards
bm = core.new_bm()
for sx in (-1, 1):
    for sy in (-1, 1):
        leg(bm, (sx * 4.95, sy * 1.75, 0), CLEAR + 0.02, 0.085, 0.14, mi('oak'), n=14,
            splay=(sx * 0.12, sy * 0.08))
legs = core.obj_from_bm('legs', box_uv(bm, 'z'), weighted=False, angle=60)

done('sofa', [base, frame, cushions, throws, legs], budget=HERO, car=(1.0, 0.4, 180))
