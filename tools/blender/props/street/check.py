#!/usr/bin/env python3
"""Validate the street props against their catalog (no Blender needed):

- every catalog kind (catalog.py + the plan's kerb arcs) has its glb, which parses as glTF 2.0
- triangle budget: kit 5 k, hero 15 k (art bible §3.1)
- every collider primitive lies inside the visual mesh's bounds (+0.3 u; parts below the
  model's lowest point are ignored: sunk into the ground)
- kinds with `lights` have a `lamp*` bulb material (rc_render dims it by day)
- every decor model exists

    python3 tools/blender/props/street/check.py
"""
import json
import math
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import catalog as C  # noqa: E402
import plan  # noqa: E402

TOL = 0.3


def read_glb(path):
    data = open(path, 'rb').read()
    magic, version, length = struct.unpack_from('<4sII', data, 0)
    if magic != b'glTF' or version != 2 or length != len(data):
        raise ValueError('not a glTF 2.0 binary')
    n, kind = struct.unpack_from('<II', data, 12)
    if kind != 0x4E4F534A:
        raise ValueError('first chunk is not JSON')
    return json.loads(data[20:20 + n])


def mesh_stats(g):
    lo, hi, tris = [1e9] * 3, [-1e9] * 3, 0
    for mesh in g.get('meshes', []):
        for prim in mesh['primitives']:
            acc = g['accessors'][prim['attributes']['POSITION']]
            lo = [min(a, b) for a, b in zip(lo, acc['min'])]
            hi = [max(a, b) for a, b in zip(hi, acc['max'])]
            idx = prim.get('indices')
            tris += (g['accessors'][idx]['count'] if idx is not None else acc['count']) // 3
    return lo, hi, tris


def rot_matrix(rot):
    """rc_track's Quat::from_euler(YXZ, y, x, z) for rot = (x, y, z) degrees."""
    x, y, z = (math.radians(v) for v in rot or (0, 0, 0))
    cy, sy, cx, sx, cz, sz = math.cos(y), math.sin(y), math.cos(x), math.sin(x), math.cos(z), \
        math.sin(z)
    ry = [[cy, 0, sy], [0, 1, 0], [-sy, 0, cy]]
    rx = [[1, 0, 0], [0, cx, -sx], [0, sx, cx]]
    rz = [[cz, -sz, 0], [sz, cz, 0], [0, 0, 1]]

    def mul(a, b):
        return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    return mul(mul(ry, rx), rz)


def collider_aabb(c):
    kind = c[0]
    if kind == 'Ball':  # only the cap above the ground matters (soft bumps)
        _, r, off = c
        rc = math.sqrt(max(0.0, r * r - off[1] * off[1])) if off[1] < 0 else r
        return [off[0] - rc, max(off[1] - r, 0.0), off[2] - rc], [off[0] + rc, off[1] + r,
                                                                  off[2] + rc]
    if kind in ('Box', 'Wedge'):
        _, half, off, rot = c
    else:
        _, r, hh, off, rot = c
        half = (r, hh, r)
    m = rot_matrix(rot)
    pts = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                p = (sx * half[0], sy * half[1], sz * half[2])
                pts.append([sum(m[i][k] * p[k] for k in range(3)) + off[i] for i in range(3)])
    return [min(p[i] for p in pts) for i in range(3)], [max(p[i] for p in pts) for i in range(3)]


def check_kind(kind, d):
    errs = []
    path = os.path.join(C.ROOT, 'assets', d['model'])
    if not os.path.exists(path):
        return [f'missing {d["model"]}'], 0
    g = read_glb(path)
    lo, hi, tris = mesh_stats(g)
    budget = 15000 if d.get('hero') else 5000
    if tris > budget:
        errs.append(f'{tris} triangles > {budget}')
    for c in d['colliders']:
        clo, chi = collider_aabb(c)
        clo[1] = max(clo[1], lo[1])  # below the model's lowest point: in the ground
        if any(clo[i] < lo[i] - TOL or chi[i] > hi[i] + TOL for i in range(3)):
            errs.append(f'{c[0]} collider {[round(v, 2) for v in clo]}..'
                        f'{[round(v, 2) for v in chi]} outside the mesh '
                        f'{[round(v, 2) for v in lo]}..{[round(v, 2) for v in hi]}')
    names = [m.get('name', '') for m in g.get('materials', [])]
    if d.get('lights') and not any(n.startswith('lamp') for n in names):
        errs.append('has lights but no lamp* bulb material')
    return errs, tris


def main():
    kinds = C.props()
    pl = plan.build()
    kinds.update({k: dict(model=f'models/props/street/{k[len("street_"):]}.glb',
                          colliders=C.kerb_arc_colliders(*v)) for k, v in pl.arcs.items()})
    failed = False
    total = 0
    for kind in sorted(kinds):
        errs, tris = check_kind(kind, kinds[kind])
        total += tris
        print(f'{kind:34s} {tris:6d} tris  {"OK" if not errs else "FAIL"}')
        for e in errs:
            print(f'    {e}')
        failed |= bool(errs)
    for model, *_ in plan.DECOR:
        if not os.path.exists(os.path.join(C.ROOT, 'assets', 'models', 'props', 'street',
                                           f'{model}.glb')):
            print(f'decor model missing: {model}')
            failed = True
    placed = {}
    for kind, *_ in pl.props:
        placed[kind] = placed.get(kind, 0) + 1
    unknown = sorted(k for k in placed if k not in kinds)
    if unknown:
        print(f'placed kinds missing from the catalog: {unknown}')
        failed = True
    dyn = sum(n for k, n in placed.items() if kinds.get(k, {}).get('dynamic'))
    print(f'{len(kinds)} kinds, {sum(placed.values())} placements ({dyn} dynamic, max 40), '
          f'{len(plan.DECOR)} decor')
    failed |= dyn > 40
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
