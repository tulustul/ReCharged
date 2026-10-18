"""Terracotta pots (kit), one glb each: large (2.3 u tall, 1.3 u rim radius) with a red
geranium, medium (1.4 u, 0.8 u) with lavender, small (0.7 u, empty, knockable)
(catalog: garden_pot_large / _medium / _small).

    blender -b -P tools/blender/props/garden/pots.py -- [--preview DIR] [--only NAME]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

SPECS = {
    'pot_large': dict(h=2.3, r=1.3, plant='geranium'),
    'pot_medium': dict(h=1.4, r=0.8, plant='lavender'),
    'pot_small': dict(h=0.7, r=0.4, plant=None),
}
argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
only = argv[argv.index('--only') + 1] if '--only' in argv else None

for name, s in SPECS.items():
    if only and name != only:
        continue
    start('terracotta', 'soil', 'leaf', 'leaf_light', 'leaf_blue', 'leaf_dark', 'stem',
          'flower_red', 'flower_purple', 'flower_pink', 'flower_white', 'flower_yellow')
    rng = random.Random(len(name))
    bm = core.new_bm()
    soil_z = pot(bm, (0, 0, 0), s['h'], s['r'])
    body = smooth('pot', bm, angle=50)
    parts = [body]
    if s['plant']:
        pb = core.new_bm()
        if s['plant'] == 'geranium':
            geranium(pb, (0, 0, soil_z + 0.35), s['r'] * 1.05, rng)
        else:
            lavender(pb, (0, 0, soil_z), s['r'] * 0.95, s['h'] * 0.9, rng)
        parts.append(smooth('plant', pb, angle=70))
    finish(name, parts, car=(s['r'] + 1.6, -s['r'] - 0.6, 20.0))
