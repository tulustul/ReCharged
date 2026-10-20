#!/usr/bin/env python3
"""Validate the supermarket props (no Blender needed), like tools/blender/check_cars.py:

- every `supermarket_*` kind of catalog.py and every model of the decor file has a glb that
  loads (glTF 2.0 binary) with embedded textures only;
- triangles within budget (hero kinds 15 k, the rest 5 k);
- ground contact: the mesh starts at y = 0 (wall/ceiling items excepted) and colliders stay
  inside the mesh bounds (+ tolerance).

    python3 tools/blender/props/supermarket/check.py
"""
import json
import math
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import catalog as C  # noqa: E402

ROOT = C.ROOT
ASSETS = os.path.join(ROOT, 'assets')
HERO = ('gondola_', 'endcap_', 'freezer', 'cooler', 'checkout_', 'box_jump', 'trolley')
AIRBORNE = ('exit_sign', 'hanging_sign', 'ceiling_light', 'wall_sign', 'topstock')
TOL = 0.15


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
    for node in g.get('nodes', []):
        if 'mesh' not in node:
            continue
        t = node.get('translation', [0, 0, 0])
        for prim in g['meshes'][node['mesh']]['primitives']:
            acc = g['accessors'][prim['attributes']['POSITION']]
            lo = [min(a, b + c) for a, b, c in zip(lo, acc['min'], t)]
            hi = [max(a, b + c) for a, b, c in zip(hi, acc['max'], t)]
            idx = prim.get('indices')
            tris += (g['accessors'][idx]['count'] if idx is not None else acc['count']) // 3
    return lo, hi, tris


def collider_box(p):
    """AABB (lo, hi) of a collider primitive in prop space."""
    kind = p[0]
    if kind in ('Box', 'Wedge'):
        _, half, off, rot = p
        yaw = math.radians((rot or (0, 0, 0))[1])
        c, s = abs(math.cos(yaw)), abs(math.sin(yaw))
        hx, hz = half[0] * c + half[2] * s, half[0] * s + half[2] * c
        h = (hx, half[1], hz)
    else:   # Cylinder
        _, r, hh, off, _ = p
        h = (r, hh, r)
    return [o - e for o, e in zip(off, h)], [o + e for o, e in zip(off, h)]


def check_model(rel, budget, colliders=(), airborne=False):
    errs = []
    path = os.path.join(ASSETS, rel)
    if not os.path.isfile(path):
        return [f'{rel}: missing'], None
    try:
        g = read_glb(path)
    except ValueError as e:
        return [f'{rel}: {e}'], None
    lo, hi, tris = mesh_stats(g)
    if tris > budget:
        errs.append(f'{rel}: {tris} triangles > {budget}')
    if not airborne and abs(lo[1]) > 0.03:
        errs.append(f'{rel}: mesh starts at y = {lo[1]:.3f}, not on the ground')
    for img in g.get('images', []):
        if 'uri' in img:
            errs.append(f'{rel}: external image {img["uri"]}')
    for p in colliders:
        clo, chi = collider_box(p)
        if any(a < b - TOL for a, b in zip(clo, lo)) or any(a > b + TOL for a, b in zip(chi, hi)):
            errs.append(f'{rel}: collider {p[0]} {[round(v, 2) for v in clo]}..'
                        f'{[round(v, 2) for v in chi]} outside the mesh '
                        f'{[round(v, 2) for v in lo]}..{[round(v, 2) for v in hi]}')
    return errs, tris


def main():
    errs, rows = [], []
    seen = set()
    for kind, d in sorted(C.kinds().items()):
        rel = d['model']
        name = kind[len('supermarket_'):]
        budget = 15000 if name.startswith(HERO) else 5000
        if rel in seen:   # e.g. freezer_dark shares the freezer glb
            continue
        seen.add(rel)
        e, tris = check_model(rel, budget, d['colliders'], name.startswith(AIRBORNE))
        errs += e
        rows.append((rel, tris, budget))
    decor = os.path.join(ASSETS, 'maps', 'aisle_rush', 'decor.ron')
    if os.path.isfile(decor):
        import re
        models = sorted(set(re.findall(r'model: "([^"]+)"', open(decor, encoding='utf-8').read())))
        for rel in models:
            e, tris = check_model(rel, 5000, (), True)
            errs += e
            rows.append((rel, tris, 5000))
    for rel, tris, budget in rows:
        print(f'{rel:55s} {tris if tris is not None else "-":>6} / {budget}')
    for e in errs:
        print('ERROR', e)
    print(f'{len(rows)} models checked, {len(errs)} errors')
    return 1 if errs else 0


if __name__ == '__main__':
    sys.exit(main())
