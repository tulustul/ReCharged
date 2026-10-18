"""Vegetable bed (kit, along the south fence): a timber raised bed 2.5 × 1 m (11 × 4.4 u), 1.1 u
tall, soil at 0.95, two rows of lettuces and a bean-cane wigwam with runner beans climbing it
(catalog: garden_veg_bed; long side along x).

    blender -b -P tools/blender/props/garden/veg_bed.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

LX, LY, H, T = 11.0, 4.4, 1.1, 0.2
start('pine', 'soil', 'leaf_light', 'leaf', 'stem', 'bark', 'flower_red')
rng = random.Random(61)
bm = core.new_bm()
k = 0
for sy in (-1, 1):
    for z0 in (0.0, 0.55):
        k += 1
        board(bm, (0, sy * (LY / 2 - T / 2), z0 + 0.27), (LX, T, 0.53), mi('pine'), axis=0, seed=k)
for sx in (-1, 1):
    for z0 in (0.0, 0.55):
        k += 1
        board(bm, (sx * (LX / 2 - T / 2), 0, z0 + 0.27), (T, LY - 2 * T, 0.53), mi('pine'), axis=1,
              seed=k)
frame = part('frame', bm, bevel=0.02, segments=1)
soil = core.new_bm()
geo.box(soil, V((0, 0, 0.9)), (LX - 2 * T, LY - 2 * T, 0.1), mi('soil'))
box_uv(soil, scale=0.5)
soil_ob = part('soil', soil)
pl = core.new_bm()
for row in (-0.85, 0.85):          # lettuces: rosettes of round leaves
    for i in range(6):
        c = V((-4.2 + i * 1.4, row, 0.95))
        for j in range(9):
            a = 2 * math.pi * j / 9 + rng.uniform(-0.2, 0.2)
            round_leaf(pl, c + V((math.cos(a) * 0.3, math.sin(a) * 0.3, 0.2)),
                       V((math.cos(a), math.sin(a), 1.4)), 0.33, mi('leaf_light'), rng, cup=0.4, n=7)
        round_leaf(pl, c + V((0, 0, 0.35)), V((0, 0, 1)), 0.25, mi('leaf_light'), rng, cup=0.6, n=7)
# bean-cane wigwam at the east end
top = V((4.0, 0.0, 8.0))
for j in range(6):
    a = 2 * math.pi * j / 6
    foot = V((4.0 + math.cos(a) * 1.3, math.sin(a) * 1.3, 0.9))
    geo.cylinder(pl, foot, top + (foot - top).normalized() * -0.4, 0.06, mi('bark'), n=5)
    for s in range(1, 9):         # bean leaves and the odd red flower up the cane
        p = foot.lerp(top, s / 10)
        round_leaf(pl, p + V((rng.uniform(-0.2, 0.2), rng.uniform(-0.2, 0.2), 0)),
                   V((math.cos(a), math.sin(a), 0.5)), 0.32, mi('leaf'), rng, n=7)
        if rng.random() < 0.3:
            geo.sphere(pl, p + V((math.cos(a) * 0.3, math.sin(a) * 0.3, 0.1)), 0.09,
                       mi('flower_red'), n=6, rings=3)
plants = smooth('plants', pl, angle=70)
finish('veg_bed', [frame, soil_ob, plants], car=(0.0, -LY / 2 - 2.0, 0.0))
