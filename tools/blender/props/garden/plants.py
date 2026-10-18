"""Decorative planting (physics-free, placed by decor.py in assets/data/decor/backyard_bash.ron),
one glb each: long-grass tufts (three sizes) for track edges, wall feet and fence lines; bed
plants — hosta, daisies, fern, hydrangea shrub, ornamental grass; fallen leaves; a stepping
stone; a pebble patch.

    blender -b -P tools/blender/props/garden/plants.py -- [--preview DIR] [--only NAME]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
only = argv[argv.index('--only') + 1] if '--only' in argv else None
PLANT_MATS = ('grass', 'grass_dry', 'leaf', 'leaf_light', 'leaf_dark', 'leaf_blue', 'stem',
              'flower_white', 'flower_yellow', 'flower_pink', 'flower_purple', 'stone', 'soil')


def tufts(radius, n, height, seed):
    def build():
        rng = random.Random(seed)
        bm = core.new_bm()
        tuft(bm, (0, 0, 0), radius, n, height, mi('grass'), rng, width=0.08, lean=0.45)
        tuft(bm, (radius * 0.3, -radius * 0.2, 0), radius * 0.6, n // 4, height * 0.8,
             mi('grass_dry'), rng, width=0.07, lean=0.6)
        return [smooth('tuft', bm, angle=80)]
    return build


def hosta():
    rng = random.Random(7)
    bm = core.new_bm()
    for _ in range(18):
        a = rng.uniform(0, 2 * math.pi)
        d = V((math.cos(a), math.sin(a), rng.uniform(0.5, 1.1)))
        leaf(bm, V((0, 0, 0.05)), d, rng.uniform(1.2, 1.6), rng.uniform(0.55, 0.75),
             mi('leaf_blue'), droop=0.45, segs=5)
    return [smooth('hosta', bm, angle=70)]


def daisies():
    rng = random.Random(8)
    bm = core.new_bm()
    tuft(bm, (0, 0, 0), 0.8, 60, 0.7, mi('leaf'), rng, width=0.1, lean=0.5)
    for _ in range(16):
        a, r = rng.uniform(0, 2 * math.pi), rng.uniform(0, 0.8)
        base = V((math.cos(a) * r, math.sin(a) * r, 0))
        tip = base + V((rng.uniform(-0.2, 0.2), rng.uniform(-0.2, 0.2), rng.uniform(0.9, 1.4)))
        geo.cylinder(bm, base, tip, 0.025, mi('stem'), n=3)
        blossom(bm, tip, 0.2, mi('flower_white'), mi('flower_yellow'), petals=8)
    return [smooth('daisies', bm, angle=70)]


def fern():
    rng = random.Random(9)
    bm = core.new_bm()
    for _ in range(11):
        a = rng.uniform(0, 2 * math.pi)
        d = V((math.cos(a), math.sin(a), rng.uniform(0.7, 1.4)))
        length = rng.uniform(1.6, 2.2)
        leaf(bm, V((0, 0, 0)), d, length, 0.16, mi('stem'), droop=0.35, segs=5)
        dn = V((d.x, d.y, 0)).normalized()
        side = V((-dn.y, dn.x, 0))
        for k in range(1, 9):  # pinnae along the frond
            t = k / 9
            p = V((0, 0, 0)) + d.normalized() * length * t - V((0, 0, 1)) * 0.35 * length * t * t
            for s in (-1, 1):
                leaf(bm, p, side * s + dn * 0.6 + V((0, 0, 0.2)), 0.55 * (1 - t * 0.7), 0.16,
                     mi('leaf_light'), droop=0.2, segs=2)
    return [smooth('fern', bm, angle=70)]


def hydrangea():
    rng = random.Random(10)
    bm = core.new_bm()
    c = V((0, 0, 1.1))
    foliage_ball(bm, c, 1.3, mi('leaf_dark'), rng, n=8, detail=2, squash=0.7)  # dark core
    for _ in range(170):
        a, e = rng.uniform(0, 2 * math.pi), rng.uniform(-0.1, 1.4)
        d = V((math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e)))
        p = c + V((d.x * 1.8, d.y * 1.8, d.z * 1.35)) * rng.uniform(0.85, 1.0)
        round_leaf(bm, p, d + V((0, 0, 0.4)), rng.uniform(0.3, 0.42),
                   mi(rng.choice(('leaf', 'leaf', 'leaf_light'))), rng, n=7)
    for _ in range(16):
        a, e = rng.uniform(0, 2 * math.pi), rng.uniform(0.05, 1.3)
        d = V((math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e)))
        umbel(bm, c + V((d.x * 1.9, d.y * 1.9, d.z * 1.45)), 0.4,
              mi('flower_pink' if rng.random() > 0.4 else 'flower_purple'), rng, n=16)
    return [smooth('hydrangea', bm, angle=70)]


def ornamental_grass():
    rng = random.Random(11)
    bm = core.new_bm()
    tuft(bm, (0, 0, 0), 0.5, 120, 2.6, mi('grass_dry'), rng, width=0.07, lean=0.5)
    tuft(bm, (0, 0, 0), 0.4, 60, 2.0, mi('leaf_light'), rng, width=0.07, lean=0.4)
    return [smooth('grass', bm, angle=80)]


def leaves():
    rng = random.Random(12)
    bm = core.new_bm()
    cols = ('leaf_light', 'grass_dry', 'soil')
    for _ in range(14):
        a, r = rng.uniform(0, 2 * math.pi), rng.uniform(0, 1.6)
        round_leaf(bm, V((math.cos(a) * r, math.sin(a) * r, 0.03)),
                   V((rng.uniform(-0.2, 0.2), rng.uniform(-0.2, 0.2), 1)), rng.uniform(0.18, 0.28),
                   mi(rng.choice(cols)), rng, cup=0.15, n=7)
    return [smooth('leaves', bm, angle=70)]


def stepping_stone():
    rng = random.Random(13)
    bm = core.new_bm()
    pts = [V((1.0 * (1 + 0.08 * math.sin(3 * a + 1)) * math.cos(a),
              0.85 * (1 + 0.08 * math.sin(3 * a + 1)) * math.sin(a))) for a in
           (2 * math.pi * k / 16 for k in range(16))]
    geo.prism(bm, pts, V((0, 0, -0.06)), V((0, 0, 0.07)), mi('stone'))
    box_uv(bm, scale=0.6)
    return [part('stone', bm, bevel=0.03, segments=2)]


def pebbles():
    rng = random.Random(14)
    bm = core.new_bm()
    for _ in range(26):
        a, r = rng.uniform(0, 2 * math.pi), 1.2 * math.sqrt(rng.random())
        s = rng.uniform(0.08, 0.16)
        geo.sphere(bm, V((math.cos(a) * r, math.sin(a) * r, s * 0.35)), s, mi('stone'), n=6, rings=3,
                   scale=(rng.uniform(1, 1.5), 1, 0.6))
    return [smooth('pebbles', bm, angle=60)]


def clover():
    """A low patch of clover with a few white heads: breaks up the open lawn."""
    rng = random.Random(15)
    bm = core.new_bm()
    for _ in range(26):
        a, r = rng.uniform(0, 2 * math.pi), 1.1 * math.sqrt(rng.random())
        c = V((math.cos(a) * r, math.sin(a) * r, rng.uniform(0.05, 0.16)))
        for k in range(3):
            d = 2 * math.pi * k / 3 + rng.uniform(-0.2, 0.2)
            round_leaf(bm, c + V((math.cos(d) * 0.1, math.sin(d) * 0.1, 0)),
                       V((math.cos(d) * 0.3, math.sin(d) * 0.3, 1)), 0.1,
                       mi(rng.choice(('leaf', 'leaf_light'))), rng, cup=0.2, n=6)
    for _ in range(5):
        a, r = rng.uniform(0, 2 * math.pi), rng.uniform(0, 0.9)
        p = V((math.cos(a) * r, math.sin(a) * r, 0.3))
        geo.cylinder(bm, p - V((0, 0, 0.3)), p, 0.02, mi('stem'), n=3)
        umbel(bm, p, 0.06, mi('flower_white'), rng, n=6)
    return [smooth('clover', bm, angle=70)]


BUILD = {
    'tuft_small': tufts(0.35, 26, 0.45, 1), 'tuft': tufts(0.6, 48, 0.75, 2),
    'tuft_large': tufts(0.9, 80, 1.1, 3), 'hosta': hosta, 'daisies': daisies, 'fern': fern,
    'hydrangea': hydrangea, 'ornamental_grass': ornamental_grass, 'leaves': leaves,
    'stepping_stone': stepping_stone, 'pebbles': pebbles, 'clover': clover,
}
for name, fn in BUILD.items():
    if only and name != only:
        continue
    start(*PLANT_MATS)
    finish(name, fn(), below=0.07, car=(2.6, -1.0, 20.0))
