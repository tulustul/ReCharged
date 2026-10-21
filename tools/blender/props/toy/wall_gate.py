"""Light-up gateway wall: a 24 u toy-store wall panel (section as toy_wall) with a round-topped
archway the track drives through: opening x -7.5..7.5, straight jambs to 5 u, a semicircle of
radius 7.5 (top 12.5 u). A painted rainbow archivolt rings the opening on both faces, studded
with coloured bulbs (`lamp_*`, dimmed by the time of day; rainbow order mirrored about the
apex). Static.

    blender -b -P tools/blender/props/toy/wall_gate.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

HALF, R, SPRING, ARC = 12.0, 7.5, 5.0, 32   # half length, opening radius, jamb top, segments
BAND, BASE = 0.3, 0.6                        # archivolt band width, its foot (skirting top)
core.reset()
wall_materials()
LAMPS = [('lamp_red', (1.0, 0.03, 0.0)), ('lamp_orange', (1.0, 0.28, 0.0)),
         ('lamp_yellow', (1.0, 0.65, 0.0)), ('lamp_green', (0.05, 1.0, 0.08)),
         ('lamp_blue', (0.05, 0.25, 1.0)), ('lamp_purple', (0.55, 0.05, 1.0))]
for (name, glow), base in zip(LAMPS, RAINBOW):
    core.material(name, base, 0.0, 0.25, emission=glow, strength=30.0)
mi = core.mi


def arch(r, n=ARC):
    """Opening outline at radius r from the right foot over the top to the left foot (x, z)."""
    return [V((r, BASE))] + [V((r * math.cos(math.pi * i / n), SPRING + r * math.sin(math.pi * i / n)))
                             for i in range(n + 1)] + [V((-r, BASE))]


wall = core.new_bm()
for s in (-1, 1):  # the two side piers (full section), stripes start beyond the archivolt
    pier = [(s * R, 0), (s * HALF, 0)]
    wall_sweep(wall, pier, [l for l in WALL_LAYERS if not l[3].startswith(('stripe', 'crown'))])
    wall_sweep(wall, [(s * 10.0, 0), (s * HALF, 0)],
               [l for l in WALL_LAYERS if l[3].startswith('stripe')])
wall_sweep(wall, [(-HALF, 0), (HALF, 0)], [l for l in WALL_LAYERS if l[3] == 'crown'])
top = arch(R)[1:-1] + [V((-R, WALL_H - 0.01)), V((R, WALL_H - 0.01))]   # spandrel over the arch
extrude(wall, 'y', top[::-1], -0.4, 0.4, mi('plaster'))
box_uv(wall, 0.5)

bands = core.new_bm()
for k, c in enumerate(RAINBOW):  # archivolt: innermost red
    r0, r1 = R + BAND * k, R + BAND * (k + 1)
    ring = arch(r1) + arch(r0)[::-1]
    for y0, y1 in ((0.4, 0.43), (-0.43, -0.4)):
        extrude(bands, 'y', ring, y0, y1, mi(f'stripe{k}'))

bulbs = core.new_bm()
RB = R + 6 * BAND + 0.35
apex = math.pi * RB / 2
for face in (-1, 1):
    n_arc = 24
    spots = [(RB, z) for z in (1.2, 2.4, 3.6)]
    spots += [(RB * math.cos(math.pi * i / n_arc), SPRING + RB * math.sin(math.pi * i / n_arc))
              for i in range(n_arc + 1)]
    spots += [(-RB, z) for z in (3.6, 2.4, 1.2)]
    for j, (x, z) in enumerate(spots):
        k = abs(j - (len(spots) - 1) // 2) % len(LAMPS)   # mirrored about the apex
        geo.sphere(bulbs, V((x, face * 0.6, z)), 0.22, mi(LAMPS[k][0]), n=8, rings=5)

propkit.finish(ENV, 'wall_gate', [smooth('wall', wall, angle=30), smooth('bands', bands, angle=30),
                                  smooth('bulbs', bulbs)],
               budget=propkit.HERO_TRIS, car=(0.0, -6.0, 0))
