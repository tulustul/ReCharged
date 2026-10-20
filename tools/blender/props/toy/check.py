#!/usr/bin/env python3
"""Validate the Toy Town props without Blender (re-import check, like tools/blender/check_cars.py).

Per catalog kind (assets/data/props/toy.ron): the glb parses (glTF 2.0 binary), triangle budget
(kit 5 k, hero 15 k), named materials, no texture without UVs (texCoord -1), ground contact
(lowest point at 0 except the support pillar), every collider primitive inside the visual
bounds (+2 cm), dynamic kinds have a mass. Also: decor models (assets/maps/toy_town/decor.ron)
parse, the track places only known kinds and at most 40 dynamic bodies, no orphan glb.

    python3 tools/blender/props/toy/check.py
"""
import json
import math
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, '..', '..')))
from lib import toy_ron  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
HERO = {'toy_dollhouse', 'toy_castle', 'toy_shelf_a', 'toy_shelf_b', 'toy_rocking_horse'}
BELOW = {'toy_track_support': 8.0}   # may reach below its origin (u)
TOL = 0.02


def read_glb(path):
    data = open(path, 'rb').read()
    magic, version, length = struct.unpack_from('<4sII', data, 0)
    if magic != b'glTF' or version != 2 or length != len(data):
        raise ValueError('not a glTF 2.0 binary')
    n, kind = struct.unpack_from('<II', data, 12)
    if kind != 0x4E4F534A:
        raise ValueError('first chunk is not JSON')
    return json.loads(data[20:20 + n])


def mesh_info(g):
    """(min xyz, max xyz, triangles) over all primitives (prop glbs have untransformed nodes)."""
    lo, hi, tris = [1e9] * 3, [-1e9] * 3, 0
    for node in g['nodes']:
        if 'mesh' not in node:
            continue
        if any(k in node for k in ('rotation', 'scale', 'translation', 'matrix')):
            raise ValueError(f"node {node.get('name')} has a transform")
        for prim in g['meshes'][node['mesh']]['primitives']:
            acc = g['accessors'][prim['attributes']['POSITION']]
            lo = [min(a, b) for a, b in zip(lo, acc['min'])]
            hi = [max(a, b) for a, b in zip(hi, acc['max'])]
            idx = prim.get('indices')
            tris += (g['accessors'][idx]['count'] if idx is not None else acc['count']) // 3
    return lo, hi, tris


def collider_box(c):
    """Axis-aligned bounds (min, max) of a collider primitive in prop space (yaw-only rotations
    rotate the footprint; other rotations are bounded by the enclosing sphere)."""
    o = c.get('offset', [0, 0, 0])
    rot = c.get('rot', [0, 0, 0])
    if c['_name'] in ('Box', 'Wedge'):
        h = list(c['half'])
        if rot[0] == 0 and rot[2] == 0 and rot[1] % 90 == 0:
            if rot[1] % 180:
                h[0], h[2] = h[2], h[0]
        elif any(rot):
            r = math.sqrt(sum(v * v for v in h))
            h = [r, r, r]
    elif c['_name'] == 'Cylinder':
        h = [c['radius'], c['half_height'], c['radius']]
    else:
        h = [c['radius']] * 3
    return [o[i] - h[i] for i in range(3)], [o[i] + h[i] for i in range(3)]


def check_kind(kind, d):
    errs = []
    path = os.path.join(ROOT, 'assets', d['model'])
    if not os.path.exists(path):
        return [f'missing {d["model"]}'], 0
    try:
        g = read_glb(path)
        lo, hi, tris = mesh_info(g)
    except (ValueError, KeyError) as e:
        return [f'{d["model"]}: {e}'], 0
    budget = 15000 if kind in HERO else 5000
    if tris > budget:
        errs.append(f'{tris} triangles > {budget}')
    for m in g.get('materials', []):
        if not m.get('name'):
            errs.append('unnamed material')
        tex = m.get('pbrMetallicRoughness', {}).get('baseColorTexture')
        if tex and tex.get('texCoord', 0) < 0:
            errs.append(f'material {m["name"]}: texture without UVs')
    below = BELOW.get(kind, 0.0)
    if lo[1] < -below - TOL or (kind not in BELOW and abs(lo[1]) > TOL):
        errs.append(f'lowest point {lo[1]:.3f} (ground contact must be 0)')
    for i, c in enumerate(d['colliders']):
        cl, ch = collider_box(c)
        if any(cl[a] < lo[a] - TOL or ch[a] > hi[a] + TOL for a in range(3)):
            errs.append(f'collider {i} ({c["_name"]}) leaves the visual bounds '
                        f'{[round(v, 2) for v in lo]}..{[round(v, 2) for v in hi]}')
    if d.get('dynamic') and not d.get('mass', 1.0) > 0:
        errs.append('dynamic without mass')
    print(f'{kind:20s} tris {tris:6d}  size {hi[0] - lo[0]:5.2f} x {hi[1] - lo[1]:5.2f} x '
          f'{hi[2] - lo[2]:5.2f}  {"dynamic " if d.get("dynamic") else ""}'
          f'{"OK" if not errs else "FAIL"}')
    return errs, tris


def main():
    cat = toy_ron.load(os.path.join(ROOT, 'assets', 'data', 'props', 'toy.ron'))['props']
    failed = False
    used = set()
    for kind in sorted(cat):
        errs, _ = check_kind(kind, cat[kind])
        used.add(os.path.normpath(cat[kind]['model']))
        for e in errs:
            print(f'  {kind}: {e}')
        failed |= bool(errs)
    decor = os.path.join(ROOT, 'assets', 'maps', 'toy_town', 'decor.ron')
    for item in toy_ron.load(decor)['decor'] if os.path.exists(decor) else []:
        used.add(os.path.normpath(item['model']))
        try:
            read_glb(os.path.join(ROOT, 'assets', item['model']))
        except (OSError, ValueError) as e:
            print(f'  decor {item["model"]}: {e}')
            failed = True
    track = toy_ron.load(os.path.join(ROOT, 'assets', 'maps', 'toy_town', 'track.ron'))
    kinds = [p['kind'] for p in track['props']]
    unknown = sorted({k for k in kinds if k.startswith('toy_') and k not in cat})
    dynamic = sum(1 for k in kinds if cat.get(k, {}).get('dynamic'))
    print(f'toy_town: {len(kinds)} props, {dynamic} dynamic (max 40)')
    if unknown or dynamic > 40:
        print(f'  unknown kinds {unknown}' if unknown else '  too many dynamic props')
        failed = True
    folder = os.path.join(ROOT, 'assets', 'models', 'props', 'toy')
    orphans = sorted(f for f in os.listdir(folder)
                     if os.path.normpath(f'models/props/toy/{f}') not in used)
    if orphans:
        print(f'  orphan models: {orphans}')
        failed = True
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
