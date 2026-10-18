"""Start/finish gate of the giant toy track: two red-and-white striped posts (stand outside
the barriers, 13.6 u apart) carrying a checkered banner 5.4 u above the floor and two pennants.
Spans the track along X; front +Y. Static.

    blender -b -P tools/blender/props/toy/start_arch.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

X, R, TOP = 6.8, 0.38, 7.2     # post offset, radius, post height
B0, B1 = 5.4, 6.9              # banner bottom / top
core.reset()
plastic('red', RED)
plastic('white', '#F2F0EA')
plastic('black', '#1D1D20', 0.3)
plastic('yellow', YELLOW)
mi = core.mi
posts, banner = core.new_bm(), core.new_bm()
for s in (-1, 1):
    geo.cylinder(posts, V((s * X, 0, 0)), V((s * X, 0, 0.25)), R + 0.25, mi('white'), n=20)
    z = 0.25
    k = 0
    while z < TOP - 0.01:  # stripes
        z1 = min(z + 0.6, TOP)
        geo.cylinder(posts, V((s * X, 0, z)), V((s * X, 0, z1)), R, mi('red' if k % 2 else 'white'),
                     n=20)
        z, k = z1, k + 1
    geo.sphere(posts, V((s * X, 0, TOP + 0.2)), 0.42, mi('yellow'), n=16, rings=8)
    geo.cylinder(posts, V((s * X, 0, TOP + 0.5)), V((s * X, 0, TOP + 2.0)), 0.05, mi('black'), n=6)
    geo.loft(posts, [[V((s * X, -0.03, TOP + 1.95)), V((s * X, -0.03, TOP + 1.4)),
                      V((s * X + s * 1.1, -0.03, TOP + 1.68))],
                     [V((s * X, 0.03, TOP + 1.95)), V((s * X, 0.03, TOP + 1.4)),
                      V((s * X + s * 1.1, 0.03, TOP + 1.68))]], mi('red'))
# banner: a frame with two rows of checks on both faces
geo.box(banner, V((0, 0, (B0 + B1) / 2)), (2 * X - 2 * R, 0.16, B1 - B0), mi('white'))
n = 18
w = (2 * X - 2 * R - 0.3) / n
for row in range(2):
    for i in range(n):
        if (i + row) % 2 == 0:
            cx = -X + R + 0.15 + (i + 0.5) * w
            cz = B0 + 0.15 + (row + 0.5) * (B1 - B0 - 0.3) / 2
            geo.box(banner, V((cx, 0, cz)), (w, 0.2, (B1 - B0 - 0.3) / 2), mi('black'))
propkit.finish(ENV, 'start_arch', [smooth('posts', posts),
                                   part('banner', banner, bevel=0.02, segments=1)],
               car=(2.0, 3.5, 200))
