#!/usr/bin/env python3
"""Validate the garden assets (no Blender needed): every glb in assets/models/props/garden loads
as glTF 2.0 binary, has no invalid texCoord, no clearcoat, triangle budget (kit 5 k; the art
bible's hero props and the deck, house and trees 15 k), sits on its origin (ground contact);
every model the catalog (props/garden.ron) or the decor file names exists; every catalog kind is
placed by the track or unused on purpose.

    python3 tools/blender/props/garden/check.py
"""
import json
import os
import re
import struct
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..', '..'))
MODELS = os.path.join(ROOT, 'assets', 'models', 'props', 'garden')
HERO = {'bench', 'hose_reel', 'sandbox', 'pot_cluster', 'shed', 'bbq', 'deck', 'house',
        'tree_oak', 'tree_birch', 'conifer'}
BELOW = {'kicker': 0.1, 'deck_ramp': 0.12, 'hose_spiral': 0.06, 'tree_oak': 0.8,
         'tree_birch': 0.8, 'conifer': 0.8, 'stepping_stone': 0.07, 'pebbles': 0.07,
         'leaves': 0.07}


def glb(path):
    data = open(path, 'rb').read()
    magic, version, length = struct.unpack_from('<4sII', data, 0)
    if magic != b'glTF' or version != 2 or length != len(data):
        raise ValueError('not a glTF 2.0 binary')
    n, kind = struct.unpack_from('<II', data, 12)
    if kind != 0x4E4F534A:
        raise ValueError('first chunk is not JSON')
    return json.loads(data[20:20 + n])


def check(path):
    name = os.path.basename(path)[:-4]
    g = glb(path)
    errs = []
    if any('clearcoat' in u for u in g.get('extensionsUsed', [])):
        errs.append('uses clearcoat')
    for m in g.get('materials', []):
        tex = m.get('pbrMetallicRoughness', {}).get('baseColorTexture')
        if tex is not None and tex.get('texCoord', 0) < 0:
            errs.append(f'material {m["name"]}: texCoord {tex["texCoord"]}')
    tris, lo = 0, 1e9
    for mesh in g['meshes']:
        for prim in mesh['primitives']:
            acc = g['accessors'][prim['attributes']['POSITION']]
            lo = min(lo, acc['min'][1])
            idx = prim.get('indices')
            tris += (g['accessors'][idx]['count'] if idx is not None else acc['count']) // 3
            if 'TEXCOORD_0' not in prim['attributes'] and any(
                    'baseColorTexture' in g['materials'][prim['material']].get('pbrMetallicRoughness', {})
                    for _ in [0] if 'material' in prim):
                errs.append('textured primitive without TEXCOORD_0')
    budget = 15000 if name in HERO else 5000
    if tris > budget:
        errs.append(f'{tris} triangles > {budget}')
    if lo < -BELOW.get(name, 0.0) - 0.021:
        errs.append(f'reaches {lo:.3f} below the ground contact')
    return name, tris, errs


def main():
    failed = False
    names = set()
    for f in sorted(os.listdir(MODELS)):
        if f.endswith('.glb'):
            name, tris, errs = check(os.path.join(MODELS, f))
            names.add(name)
            print(f'{name:18s} {tris:6d} tris  {"OK" if not errs else "FAIL: " + "; ".join(errs)}')
            failed |= bool(errs)
    refs = set()
    for rel in ('assets/data/props/garden.ron', 'assets/data/decor/backyard_bash.ron'):
        refs |= set(re.findall(r'models/props/garden/(\w+)\.glb', open(os.path.join(ROOT, rel)).read()))
    for missing in sorted(refs - names):
        print(f'missing model: {missing}.glb')
        failed = True
    kinds = set(re.findall(r'"(garden_\w+)": \(', open(os.path.join(ROOT, 'assets/data/props/garden.ron')).read()))
    placed = set(re.findall(r'kind: "(garden_\w+)"', open(os.path.join(ROOT, 'assets/data/tracks/backyard_bash.ron')).read()))
    for k in sorted(kinds - placed):
        print(f'note: catalog kind {k} is not placed by backyard_bash')
    for k in sorted(placed - kinds):
        print(f'placed kind {k} is not in the catalog')
        failed = True
    print(f'{len(names)} models, {len(kinds)} catalog kinds, {len(placed)} placed')
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
