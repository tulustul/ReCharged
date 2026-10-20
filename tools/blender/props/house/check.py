#!/usr/bin/env python3
"""Validate the family-house props against assets/data/props/house.ron (no Blender needed).

Per kind: the glb exists and parses (glTF 2.0 binary), triangles within budget (hero 15 k,
kit 5 k), ground contact (lowest vertex at y = 0 +- 0.03), every collider primitive inside the
model's bounds (+ 0.12 u), at least one material; dynamic props have a mass. Also checks that
every model of maps/sofa_circuit/decor.ron exists. Prints a triangle table.

    python3 tools/blender/props/house/check.py
"""
import json
import os
import struct
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..', '..'))
sys.path.insert(0, os.path.join(ROOT, 'tools', 'blender'))
from lib import toy_ron  # noqa: E402  (shared minimal RON reader)

HERO = {'house_sofa', 'house_coffee_table', 'house_book_ramp', 'house_dining_table',
        'house_staircase', 'house_sideboard'}
TOL = 0.12
HANGING = {'house_pendant'}  # hangs from the ceiling: no ground contact


def read_glb(path):
    data = open(path, 'rb').read()
    magic, version, length = struct.unpack_from('<4sII', data, 0)
    if magic != b'glTF' or version != 2 or length != len(data):
        raise ValueError('not a glTF 2.0 binary')
    n, kind = struct.unpack_from('<II', data, 12)
    if kind != 0x4E4F534A:
        raise ValueError('first chunk is not JSON')
    return json.loads(data[20:20 + n])


def model_stats(g):
    lo, hi, tris = [1e9] * 3, [-1e9] * 3, 0
    for m in g['meshes']:
        for prim in m['primitives']:
            acc = g['accessors'][prim['attributes']['POSITION']]
            lo = [min(a, b) for a, b in zip(lo, acc['min'])]
            hi = [max(a, b) for a, b in zip(hi, acc['max'])]
            tris += (g['accessors'][prim['indices']]['count'] if 'indices' in prim
                     else acc['count']) // 3
    for node in g['nodes']:  # the bake leaves identity nodes; a transform would break bounds
        if any(k in node for k in ('rotation', 'scale')) or any(
                abs(v) > 1e-4 for v in node.get('translation', [0, 0, 0])):
            raise ValueError(f'node {node.get("name")} has a transform')
    return lo, hi, tris


def aabb(prim):
    """Prop-space AABB of an unrotated collider primitive."""
    name = prim['_name']
    off = prim.get('offset', [0, 0, 0])
    if any(abs(v) > 1e-6 for v in prim.get('rot', [0, 0, 0])):
        return None  # rotated: not checked
    if name in ('Box', 'Wedge'):
        h = prim['half']
    elif name == 'Cylinder':
        h = [prim['radius'], prim['half_height'], prim['radius']]
    else:
        h = [prim['radius']] * 3
    return [o - e for o, e in zip(off, h)], [o + e for o, e in zip(off, h)]


def main():
    cat = toy_ron.loads(open(os.path.join(ROOT, 'assets', 'data', 'props', 'house.ron')).read())
    errors, rows = [], []
    for kind, d in sorted(cat['props'].items()):
        path = os.path.join(ROOT, 'assets', d['model'])
        if not os.path.exists(path):
            errors.append(f'{kind}: missing {d["model"]}')
            continue
        try:
            g = read_glb(path)
            lo, hi, tris = model_stats(g)
        except ValueError as e:
            errors.append(f'{kind}: {e}')
            continue
        budget = 15000 if kind in HERO else 5000
        if tris > budget:
            errors.append(f'{kind}: {tris} triangles > {budget}')
        if abs(lo[1]) > 0.03 and kind not in HANGING:
            errors.append(f'{kind}: lowest point at y = {lo[1]:.3f}, expected 0 (ground contact)')
        if not g.get('materials'):
            errors.append(f'{kind}: no materials')
        for m in g.get('materials', []):  # Bevy rejects texCoord -1 (texture on a mesh w/o UVs)
            tex = m.get('pbrMetallicRoughness', {}).get('baseColorTexture')
            if tex is not None and tex.get('texCoord', 0) < 0:
                errors.append(f'{kind}: material {m.get("name")} has texCoord {tex["texCoord"]}')
            if 'KHR_materials_clearcoat' in m.get('extensions', {}):  # renders washed out on props
                errors.append(f'{kind}: material {m.get("name")} uses clearcoat')
        if d.get('dynamic') and not d.get('mass'):
            errors.append(f'{kind}: dynamic without mass')
        for i, prim in enumerate(d['colliders']):
            box = aabb(prim)
            if box is None:
                continue
            for k in range(3):
                if box[0][k] < lo[k] - TOL or box[1][k] > hi[k] + TOL:
                    errors.append(f'{kind}: collider {i} ({prim["_name"]}) leaves the model on '
                                  f'{"xyz"[k]}: {box[0][k]:.2f}..{box[1][k]:.2f} vs model '
                                  f'{lo[k]:.2f}..{hi[k]:.2f}')
                    break
        rows.append((kind, tris, budget, [h - l for h, l in zip(hi, lo)],
                     'dynamic' if d.get('dynamic') else 'static'))
    decor = os.path.join(ROOT, 'assets', 'maps', 'sofa_circuit', 'decor.ron')
    if os.path.exists(decor):
        for item in toy_ron.loads(open(decor).read())['decor']:
            if not os.path.exists(os.path.join(ROOT, 'assets', item['model'])):
                errors.append(f'decor: missing {item["model"]}')
    for kind, tris, budget, size, kind_dyn in rows:
        print(f'{kind:24s} {tris:6d} / {budget:5d} tris  size {size[0]:6.2f} x {size[1]:5.2f} x '
              f'{size[2]:6.2f} u  {kind_dyn}')
    print(f'{len(rows)} props, {sum(r[1] for r in rows)} triangles total')
    for e in errors:
        print(f'  ERROR {e}')
    sys.exit(1 if errors else 0)


if __name__ == '__main__':
    main()
