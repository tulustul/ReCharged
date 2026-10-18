"""Rocking horse (hero): a carved dapple-grey wooden horse on beech bow rockers, red saddle and
bridle, yarn mane and tail, brass handle. Real size (1 m long, 75 cm tall); faces +Y. Static.

    blender -b -P tools/blender/props/toy/rocking_horse.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()


def dapple_image(name, base, spot, size=512, seed=8):
    import bpy
    import numpy as np
    rng = np.random.default_rng(seed)
    t = (np.arange(size) + 0.5) / size
    u, v = np.meshgrid(t, t)
    m = np.zeros((size, size))
    for _ in range(26):  # soft round spots, wrapped for tiling
        cx, cy, r = rng.random(), rng.random(), rng.uniform(0.03, 0.07)
        dx = np.minimum(abs(u - cx), 1 - abs(u - cx))
        dy = np.minimum(abs(v - cy), 1 - abs(v - cy))
        m = np.maximum(m, np.clip(1.2 - np.hypot(dx, dy) / r, 0, 1))
    b, s = np.array(srgb01(base)), np.array(srgb01(spot))
    rgb = b * (1 - m[..., None] * 0.55) + s * m[..., None] * 0.55
    img = bpy.data.images.new(name, size, size, alpha=False)
    img.pixels.foreach_set(np.concatenate([rgb, np.ones((size, size, 1))], axis=-1).ravel()
                           .astype(np.float32))
    img.pack()
    return img


textured('coat', '#E8E4DA', 0.4, dapple_image('coat', '#E8E4DA', '#7E7C82'))
beech()
painted('saddle', '#B8302A', 0.35)
painted('trim', YELLOW, 0.3)
fabric('mane', '#2A2522', 0.9)
plastic('eye', '#151417', 0.1)
core.material('brass', '#D2A54A', 1.0, 0.3)
mi = core.mi
RR, RX = 3.6, 0.56          # rocker arc radius, rocker offset from the centre (x)


def rocker_z(y):
    """Bottom of the bow rockers at y (touching the floor at the middle)."""
    return RR - math.sqrt(RR * RR - y * y)


coat, bits, wood_bm = core.new_bm(), core.new_bm(), core.new_bm()
for s in (-1, 1):  # plank bow rockers (0.14 x 0.22 section) on a true arc
    rings = []
    for k in range(17):
        y = -2.05 + 4.1 * k / 16
        z = rocker_z(y)
        t = V((0, 1.0, y / math.sqrt(RR * RR - y * y))).normalized()
        n = V((0, -t.z, t.y))
        p = V((s * RX, y, z))
        rings.append([p + V((-0.07, 0, 0)), p + V((0.07, 0, 0)), p + V((0.07, 0, 0)) + n * 0.22,
                      p + V((-0.07, 0, 0)) + n * 0.22])
    geo.loft(wood_bm, rings, mi('beech'))
for y in (-1.5, 1.5):  # cross bars between the rockers
    z = rocker_z(y) + 0.12
    geo.cylinder(wood_bm, V((-RX, y, z)), V((RX, y, z)), 0.06, mi('beech'), n=8)
geo.sphere(coat, V((0, 0, 1.75)), 1.0, mi('coat'), n=24, rings=12, scale=(0.46, 1.12, 0.48))
geo.tube(coat, [V((0, 0.72, 1.92)), V((0, 1.0, 2.35)), V((0, 1.16, 2.7))], 0.3, mi('coat'), n=16,
         corner=0.25, cseg=4, r_end=0.24)
geo.sphere(coat, V((0, 1.28, 2.86)), 0.27, mi('coat'), n=18, rings=9, scale=(0.9, 1.15, 1.0))
geo.sphere(coat, V((0, 1.64, 2.66)), 0.22, mi('coat'), n=16, rings=8, scale=(0.85, 1.45, 0.9))
geo.sphere(bits, V((0, 1.93, 2.6)), 0.12, mi('mane'), n=10, rings=5, scale=(1.4, 0.6, 1.0))
for s in (-1, 1):
    geo.cylinder(coat, V((s * 0.1, 1.2, 3.05)), V((s * 0.13, 1.18, 3.32)), 0.08, mi('coat'), n=8,
                 r1=0.02)  # ears
    geo.sphere(bits, V((s * 0.21, 1.42, 2.93)), 0.055, mi('eye'), n=10, rings=5)
    for y0, y1 in ((0.62, 1.32), (-0.62, -1.32)):  # galloping legs landing on the rockers
        top = V((s * 0.28, y0, 1.5))
        foot = V((s * RX, y1, rocker_z(y1) + 0.3))
        geo.tube(coat, [top, foot], 0.13, mi('coat'), n=12, r_end=0.1)
        geo.cylinder(bits, foot - V((0, 0, 0.1)), foot + V((0, 0, 0.06)), 0.12, mi('eye'), n=10)
# saddle with girth, bridle and brass handle, yarn mane and tail
geo.sphere(bits, V((0, 0.0, 2.2)), 0.5, mi('saddle'), n=18, rings=8, scale=(0.95, 1.05, 0.3))
geo.box(bits, V((0, 0.0, 1.75)), (0.98, 0.18, 0.95), mi('trim'))
geo.tube(bits, [V((0.2, 1.68, 2.62)), V((0, 1.5, 3.02)), V((-0.2, 1.68, 2.62))], 0.035,
         mi('saddle'), n=6)
geo.cylinder(bits, V((-0.38, 1.3, 2.86)), V((0.38, 1.3, 2.86)), 0.04, mi('brass'), n=8)
for k in range(8):
    t = k / 7
    p = V((0, 0.62 + 0.55 * t, 2.2 + 0.85 * t))
    geo.sphere(bits, p + V((0, -0.14, 0.05)), 0.12, mi('mane'), n=8, rings=5, scale=(0.6, 1, 1.2))
geo.tube(bits, [V((0, -1.05, 1.9)), V((0, -1.3, 1.65)), V((0, -1.38, 1.2))], 0.12, mi('mane'),
         n=8, corner=0.2, r_end=0.05)
propkit.finish(ENV, 'rocking_horse', [smooth('coat', cyl_uv(coat, coat.faces, V((0, 0, 0)), 0.5)),
                                      smooth('wood', box_uv(wood_bm)), smooth('bits', bits)],
               budget=propkit.HERO_TRIS, car=(2.4, 2.8, 200))
