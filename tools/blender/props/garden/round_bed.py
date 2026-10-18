"""Round raised bed (kit, central lawn): a ring of stone edging 10 u across and 1.2 u tall,
soil at 1.0, a small Japanese maple with copper-red foliage, lavender and daisies around it
(catalog: garden_round_bed; one cylinder collider, the canopy is out of reach).

    blender -b -P tools/blender/props/garden/round_bed.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

RO, T, H = 5.0, 0.6, 1.2
MATS['maple'] = dict(base='#A8432C', rough=0.7)
MATS['maple_dark'] = dict(base='#7E2F24', rough=0.7)
start('stone', 'soil', 'bark', 'maple', 'maple_dark', 'leaf_blue', 'stem', 'flower_purple',
      'leaf', 'flower_white', 'flower_yellow')
rng = random.Random(51)
bm = core.new_bm()
n = 20
for k in range(n):  # edging blocks around the ring, slightly irregular
    a0, a1 = 2 * math.pi * k / n, 2 * math.pi * (k + 1) / n - 0.02
    pts = [V(((RO - T) * math.cos(a0), (RO - T) * math.sin(a0))), V((RO * math.cos(a0), RO * math.sin(a0))),
           V((RO * math.cos(a1), RO * math.sin(a1))), V(((RO - T) * math.cos(a1), (RO - T) * math.sin(a1)))]
    geo.prism(bm, pts, V((0, 0, 0)), V((0, 0, H + rng.uniform(-0.04, 0.03))), mi('stone'))
box_uv(bm, scale=0.6)
ring = part('ring', bm, bevel=0.04, segments=1)
soil = core.new_bm()
revolve(soil, [(0, 1.0), (RO - T + 0.02, 1.0), (RO - T + 0.02, 0.9), (0, 0.9)][::-1], mi('soil'), n=26)
box_uv(soil, scale=0.5)
soil_ob = part('soil', soil)
pl = core.new_bm()
geo.tube(pl, [V((0, 0, 0.9)), V((0.2, 0.1, 3.0)), V((0.4, -0.1, 4.6))], 0.32, mi('bark'), n=8,
         r_end=0.18)
for k in range(5):
    a = 2 * math.pi * k / 5
    geo.tube(pl, [V((0.3, 0, 3.6)), V((math.cos(a) * 2.2, math.sin(a) * 2.2, 5.6))], 0.14, mi('bark'),
             n=6, r_end=0.06)
    foliage_ball(pl, V((math.cos(a) * 2.4, math.sin(a) * 2.4, 6.2)), 2.0,
                 mi('maple' if k % 2 else 'maple_dark'), rng, n=3, detail=1, squash=0.55)
foliage_ball(pl, V((0.3, 0, 7.0)), 2.2, mi('maple'), rng, n=4, detail=1, squash=0.6)
for k in range(5):
    a = 2 * math.pi * (k + 0.5) / 5
    p = V((math.cos(a) * 3.0, math.sin(a) * 3.0, 1.0))
    if k % 2:
        lavender(pl, p, 0.55, 1.2, rng)
    else:
        tuft(pl, p, 0.7, 26, 0.6, mi('leaf'), rng, width=0.09)
        for _ in range(5):
            q = p + V((rng.uniform(-0.6, 0.6), rng.uniform(-0.6, 0.6), rng.uniform(0.8, 1.2)))
            geo.cylinder(pl, q - V((0, 0, 0.8)), q, 0.025, mi('stem'), n=3)
            blossom(pl, q, 0.2, mi('flower_white'), mi('flower_yellow'), petals=8)
plants = smooth('plants', pl, angle=70)
finish('round_bed', [ring, soil_ob, plants], car=(RO + 1.8, -2.0, 20.0))
