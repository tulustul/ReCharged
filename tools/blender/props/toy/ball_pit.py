"""Inflatable ball pit (1.2 m pool): two stacked air rings in blue and yellow with a white
valve, heaped with ball-pit balls in five colours, a few spilled around it. Static.

    blender -b -P tools/blender/props/toy/ball_pit.py -- [--preview DIR]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

R, TUBE = 2.65, 0.36     # ring centre radius, tube radius
BALL = 0.19              # ball radius (8.5 cm)
core.reset()
plastic('ring_a', BLUE, 0.25)
plastic('ring_b', YELLOW, 0.25)
plastic('floor', '#E9E4D8', 0.4)
COLS = ['ball_r', 'ball_y', 'ball_b', 'ball_g', 'ball_p']
for name, c in zip(COLS, (RED, YELLOW, BLUE, GREEN, PINK)):
    plastic(name, c, 0.3)
mi = core.mi
rings, balls = core.new_bm(), core.new_bm()
for k, z in enumerate((TUBE, 3 * TUBE)):
    prof = [(R + TUBE * math.cos(a), z + TUBE * math.sin(a))
            for a in (2 * math.pi * j / 10 for j in range(11))]
    geo.revolve_x(rings, [(p[1], p[0]) for p in prof], mi('ring_a' if k == 0 else 'ring_b'),
                       n=30)
rings_rot = Matrix.Rotation(-math.pi / 2, 3, 'Y')
geo.bmesh.ops.rotate(rings, verts=rings.verts, cent=V((0, 0, 0)), matrix=rings_rot)
geo.cylinder(rings, V((0, 0, 0)), V((0, 0, 0.05)), R, mi('floor'), n=36)
geo.cylinder(rings, V((R + TUBE * 0.9, 0, 3 * TUBE)), V((R + TUBE * 1.15, 0, 3 * TUBE)), 0.08,
             mi('floor'), n=8)
rnd = random.Random(3)
heap = []
r_in = R - TUBE * 0.6
for _ in range(140):  # top layer of a gentle heap (rejection sampling)
    a, rr = rnd.uniform(0, 2 * math.pi), r_in * math.sqrt(rnd.random())
    x, y = rr * math.cos(a), rr * math.sin(a)
    if any((x - hx) ** 2 + (y - hy) ** 2 < (1.95 * BALL) ** 2 for hx, hy in heap):
        continue
    heap.append((x, y))
    z = 4 * TUBE - 0.25 + 0.35 * (1 - (rr / r_in) ** 2)
    geo.sphere(balls, V((x, y, z)), BALL, mi(rnd.choice(COLS)), n=7, rings=4)
for _ in range(14):  # spilled balls
    a = rnd.uniform(-0.9, 1.4)
    rr = R + TUBE + rnd.uniform(0.4, 2.2)
    geo.sphere(balls, V((rr * math.cos(a), rr * math.sin(a), BALL)), BALL, mi(rnd.choice(COLS)),
               n=7, rings=4)
propkit.finish(ENV, 'ball_pit', [smooth('rings', rings), smooth('balls', balls)],
               car=(4.5, 3.5, 200))
