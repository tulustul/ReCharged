#!/usr/bin/env python3
"""Validate the workshop props (no Blender needed), like tools/blender/check_cars.py:

    python3 tools/blender/props/workshop/check.py

For every `workshop_*` kind in assets/data/props/workshop.ron and every model in
assets/data/decor/wrench_works.ron: the glb parses (glTF 2.0 binary), triangles within budget
(hero 15 k, kit 5 k), ground contact (nothing below y = -0.02 except `BELOW`), every collider
primitive inside the visual bounds (+0.06 u), embedded textures <= 1024 px; the track file only
places catalogued kinds and <= 40 dynamic bodies.
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
HEROES = {'workshop_lift_a', 'workshop_lift_b', 'workshop_workbench', 'workshop_tool_chest',
          'workshop_paint_booth', 'workshop_wash_bay'}
BELOW = {}            # kind/model -> allowed depth below the origin
TOL = 0.06            # collider may poke this far out of the visual bounds (bevels)


def read_glb(path):
    data = open(path, 'rb').read()
    magic, version, length = struct.unpack_from('<4sII', data, 0)
    if magic != b'glTF' or version != 2 or length != len(data):
        raise ValueError('not a glTF 2.0 binary')
    n, kind = struct.unpack_from('<II', data, 12)
    if kind != 0x4E4F534A:
        raise ValueError('first chunk is not JSON')
    g = json.loads(data[20:20 + n])
    binary = data[20 + n + 8:] if len(data) > 20 + n else b''
    return g, binary


def png_size(blob):
    if blob[:8] != b'\x89PNG\r\n\x1a\n':
        return None
    return struct.unpack('>II', blob[16:24])


def stats(path):
    """(triangles, (min xyz), (max xyz), [texture sizes]) of a single-node prop glb."""
    g, binary = read_glb(path)
    tris, lo, hi = 0, [1e9] * 3, [-1e9] * 3
    for node in g.get('nodes', []):
        if 'mesh' not in node:
            continue
        t = node.get('translation', [0, 0, 0])
        if node.get('rotation', [0, 0, 0, 1]) != [0, 0, 0, 1] or node.get('scale', [1, 1, 1]) != [1, 1, 1]:
            raise ValueError('prop node has a rotation/scale (transforms must be applied)')
        for prim in g['meshes'][node['mesh']]['primitives']:
            acc = g['accessors'][prim['attributes']['POSITION']]
            lo = [min(a, b + c) for a, b, c in zip(lo, acc['min'], t)]
            hi = [max(a, b + c) for a, b, c in zip(hi, acc['max'], t)]
            idx = g['accessors'][prim['indices']]['count'] if 'indices' in prim else acc['count']
            tris += idx // 3
    sizes = []
    for img in g.get('images', []):
        if 'bufferView' in img:
            bv = g['bufferViews'][img['bufferView']]
            blob = binary[bv.get('byteOffset', 0):bv.get('byteOffset', 0) + bv['byteLength']]
            sizes.append(png_size(blob))
    return tris, lo, hi, sizes


def rot_yxz(v, rot):
    """Rotate v by degrees (pitch x, yaw y, roll z) applied yaw -> pitch -> roll (Bevy YXZ)."""
    rx, ry, rz = (math.radians(a) for a in rot)
    x, y, z = v
    x, y = x * math.cos(rz) - y * math.sin(rz), x * math.sin(rz) + y * math.cos(rz)   # roll
    y, z = y * math.cos(rx) - z * math.sin(rx), y * math.sin(rx) + z * math.cos(rx)   # pitch
    x, z = x * math.cos(ry) + z * math.sin(ry), -x * math.sin(ry) + z * math.cos(ry)  # yaw
    return x, y, z


def collider_aabb(c):
    name = c.get('_name')
    off = c.get('offset', [0, 0, 0])
    rot = c.get('rot', [0, 0, 0])
    if name == 'Ball':
        r = c['radius']
        return [o - r for o in off], [o + r for o in off]
    if name == 'Cylinder':
        r, h = c['radius'], c['half_height']
        pts = [(sx * r, sy * h, sz * r) for sx in (-1, 1) for sy in (-1, 1) for sz in (-1, 1)]
    else:  # Box / Wedge
        hx, hy, hz = c['half']
        pts = [(sx * hx, sy * hy, sz * hz) for sx in (-1, 1) for sy in (-1, 1) for sz in (-1, 1)]
    pts = [rot_yxz(p, rot) for p in pts]
    lo = [min(p[i] for p in pts) + off[i] for i in range(3)]
    hi = [max(p[i] for p in pts) + off[i] for i in range(3)]
    return lo, hi


def main():
    errs, rows = [], []
    cat = toy_ron.load(os.path.join(ROOT, 'assets', 'data', 'props', 'workshop.ron'))['props']
    for kind, d in sorted(cat.items()):
        path = os.path.join(ROOT, 'assets', d['model'])
        if not os.path.isfile(path):
            errs.append(f'{kind}: missing {d["model"]}')
            continue
        try:
            tris, lo, hi, sizes = stats(path)
        except ValueError as e:
            errs.append(f'{kind}: {e}')
            continue
        budget = 15000 if kind in HEROES else 5000
        if tris > budget:
            errs.append(f'{kind}: {tris} triangles > {budget}')
        if lo[1] < -BELOW.get(kind, 0.0) - 0.02:
            errs.append(f'{kind}: geometry reaches y = {lo[1]:.3f}')
        for s in sizes:
            if s and max(s) > 1024:
                errs.append(f'{kind}: texture {s} > 1024')
        for i, c in enumerate(d['colliders']):
            clo, chi = collider_aabb(c)
            if any(a < b - TOL for a, b in zip(clo, lo)) or any(a > b + TOL for a, b in zip(chi, hi)):
                errs.append(f'{kind}: collider {i} ({c["_name"]}) leaves the visual bounds: '
                            f'{[round(v, 2) for v in clo]}..{[round(v, 2) for v in chi]} vs '
                            f'{[round(v, 2) for v in lo]}..{[round(v, 2) for v in hi]}')
        rows.append((kind, tris, 'dynamic' if d.get('dynamic') else 'static'))
    decor = toy_ron.load(os.path.join(ROOT, 'assets', 'data', 'decor', 'wrench_works.ron'))['decor']
    for model in sorted({d['model'] for d in decor}):
        path = os.path.join(ROOT, 'assets', model)
        if not os.path.isfile(path):
            errs.append(f'decor: missing {model}')
            continue
        tris, lo, hi, _ = stats(path)
        if tris > 5000:
            errs.append(f'decor {model}: {tris} triangles > 5000')
        rows.append((os.path.basename(model), tris, 'decor'))
    track = toy_ron.load(os.path.join(ROOT, 'assets', 'data', 'tracks', 'wrench_works.ron'))
    kinds = [p['kind'] for p in track['props']]
    unknown = sorted({k for k in kinds if k not in cat and not k.startswith(('crate_', 'tyre_stack',
                                                                              'ramp_'))})
    if unknown:
        errs.append(f'wrench_works places unknown kinds {unknown}')
    dyn = sum(1 for k in kinds if cat.get(k, {}).get('dynamic'))
    if dyn > 40:
        errs.append(f'wrench_works places {dyn} dynamic props (> 40)')
    for name, tris, what in rows:
        print(f'{name:30s} {tris:6d}  {what}')
    print(f'{len(rows)} models, {len(kinds)} placed props ({dyn} dynamic), {len(decor)} decor items')
    for e in errs:
        print('ERROR', e)
    return 1 if errs else 0


if __name__ == '__main__':
    sys.exit(main())
