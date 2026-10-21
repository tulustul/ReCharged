"""Carnival marquee light arch across the track: two red-and-white striped posts 14.8 u apart
(outside the barriers), 7 u tall, carrying a curved red marquee board that peaks at 8.5 u,
studded on both faces with coloured bulbs (`lamp_*`, dimmed by the time of day; the catalog's
`lights` add real light) and a yellow star on top. Clearance under the board ≥ 6.5 u between
x = ±6. Spans the track along X. Static.

    blender -b -P tools/blender/props/toy/light_arch.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

X, R, TOP, PEAK, BOARD = 7.4, 0.4, 7.0, 8.5, 0.9   # post x, radius, height; arch peak; board h
core.reset()
plastic('red', RED)
plastic('white', '#F2F0EA')
plastic('yellow', YELLOW)
BULBS = {'lamp_red': ('#FF3B30', (1.0, 0.03, 0.0)), 'lamp_yellow': ('#FFD60A', (1.0, 0.65, 0.0)),
         'lamp_green': ('#34C759', (0.05, 1.0, 0.08)), 'lamp_blue': ('#2F7BFF', (0.05, 0.25, 1.0)),
         'lamp_pink': ('#FF4FB8', (1.0, 0.1, 0.55))}
for name, (base, glow) in BULBS.items():
    core.material(name, base, 0.0, 0.25, emission=glow, strength=30.0)
mi = core.mi


def mid(x):
    """Board centre height at x: from the post tops up to the peak."""
    return TOP + 0.35 + (PEAK - TOP - 0.35 - BOARD / 2) * (1 - (x / X) ** 2)


posts = core.new_bm()
for s in (-1, 1):
    geo.cylinder(posts, V((s * X, 0, 0)), V((s * X, 0, 0.25)), R + 0.25, mi('white'), n=12)
    z, k = 0.25, 0
    while z < TOP - 0.01:  # stripes
        z1 = min(z + 0.6, TOP)
        geo.cylinder(posts, V((s * X, 0, z)), V((s * X, 0, z1)), R,
                     mi('red' if k % 2 else 'white'), n=12)
        z, k = z1, k + 1
    geo.sphere(posts, V((s * X, 0, TOP + 1.3)), 0.42, mi('yellow'), n=16, rings=8)

board = core.new_bm()
xs = [-X + 2 * X * i / 16 for i in range(17)]
rings = [[V((x, -0.22, mid(x) - BOARD / 2)), V((x, 0.22, mid(x) - BOARD / 2)),
          V((x, 0.22, mid(x) + BOARD / 2)), V((x, -0.22, mid(x) + BOARD / 2))] for x in xs]
geo.loft(board, rings, mi('red'))
for s in (-1, 1):  # yellow trim rails along the top and bottom edges
    edge = [V((x, 0, mid(x) + s * (BOARD / 2 + 0.06))) for x in xs]
    geo.tube(board, edge, 0.08, mi('yellow'), n=6)
star = core.new_bm()
extrude(star, 'y', [V((p.x * 1.6, p.y * 1.6 + mid(0) + BOARD / 2 + 0.75)) for p in star2()],
        -0.15, 0.15, mi('yellow'))

bulbs = core.new_bm()
names = list(BULBS)
n = 15
for face in (-1, 1):
    for i in range(n):
        x = -X + 0.8 + (2 * X - 1.6) * i / (n - 1)
        geo.sphere(bulbs, V((x, face * 0.3, mid(x))), 0.17, mi(names[i % len(names)]), n=8, rings=5)

propkit.finish(ENV, 'light_arch', [smooth('posts', posts), part('board', board, bevel=0.03,
                                                                 segments=1),
                                   part('star', star, bevel=0.03, segments=1),
                                   smooth('bulbs', bulbs)],
               car=(2.0, 3.5, 200))
