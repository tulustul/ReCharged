#!/usr/bin/env python3
"""Validate the car assets against docs/design/car-dimensions.md (no Blender needed).

Checks per car: glb loads (glTF 2.0 binary), nodes body / wheel_fl/fr/rl/rr (+ antenna under
body), wheel nodes translation-only at the table's layout (±0.01 u) and equal to
<car>.wheels.ron, wheel mesh radius/width, required materials, ≤ 25 k triangles, body envelope
(length/height/width, clearance) and a 512² RGBA thumbnail.

    python3 tools/blender/check_cars.py [car ...]
"""
import json
import os
import re
import struct
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
CARS = ['sparky', 'dumpster', 'zip', 'torque', 'hornet', 'phantom', 'volt_gt', 'bolt']
WHEELS = ['wheel_fl', 'wheel_fr', 'wheel_rl', 'wheel_rr']
MATERIALS = {'paint', 'paint_accent', 'light_head', 'light_tail', 'light_brake', 'rubber',
             'plastic', 'metal', 'glass', 'chrome'}
TOL = 0.01
# flexible parts allowed below the chassis clearance: hornet's rally mud flaps (signature)
LOW_PARTS = {'hornet': 0.09}
# car-dimensions.md footnote: phantom's rear wing (on its stands) may rise to 0.56
WING_TOP = {'phantom': 0.56}


def dims():
    out = {}
    for line in open(os.path.join(ROOT, 'docs', 'design', 'car-dimensions.md'), encoding='utf-8'):
        c = [x.strip() for x in line.strip().strip('|').split('|')]
        if len(c) == 10 and c[0] in CARS:
            pair = [tuple(float(v) for v in s.split('/')) for s in c[6:9]]
            out[c[0]] = dict(length=float(c[2]), width=float(c[3]), height=float(c[4]),
                             wheelbase=float(c[5]), track=pair[0], radius=pair[1],
                             wheel_width=pair[2], clearance=float(c[9]))
    return out


def read_glb(path):
    data = open(path, 'rb').read()
    magic, version, length = struct.unpack_from('<4sII', data, 0)
    if magic != b'glTF' or version != 2 or length != len(data):
        raise ValueError('not a glTF 2.0 binary')
    n, kind = struct.unpack_from('<II', data, 12)
    if kind != 0x4E4F534A:
        raise ValueError('first chunk is not JSON')
    return json.loads(data[20:20 + n])


def read_ron(path):
    pat = re.compile(r'name:\s*"(\w+)",\s*center:\s*\(([^)]*)\),\s*radius:\s*([\d.]+),'
                     r'\s*width:\s*([\d.]+)')
    return {m[1]: ([float(v) for v in m[2].split(',')], float(m[3]), float(m[4]))
            for m in pat.finditer(open(path, encoding='utf-8').read())}


def png_info(path):
    with open(path, 'rb') as f:
        head = f.read(26)
    w, h, depth, color = struct.unpack('>IIBB', head[16:26])
    return w, h, color  # colour type 6 = RGBA


def mesh_bounds(g, mesh_index):
    lo, hi = [1e9] * 3, [-1e9] * 3
    tris = 0
    for prim in g['meshes'][mesh_index]['primitives']:
        acc = g['accessors'][prim['attributes']['POSITION']]
        lo = [min(a, b) for a, b in zip(lo, acc['min'])]
        hi = [max(a, b) for a, b in zip(hi, acc['max'])]
        tris += g['accessors'][prim['indices']]['count'] // 3 if 'indices' in prim else acc['count'] // 3
    return lo, hi, tris


def check(car, D):
    errs = []
    glb = os.path.join(ROOT, 'assets', 'models', 'cars', f'{car}.glb')
    ron = os.path.join(ROOT, 'assets', 'models', 'cars', f'{car}.wheels.ron')
    png = os.path.join(ROOT, 'assets', 'ui', 'cars', f'{car}.png')
    for p in (glb, ron, png):
        if not os.path.exists(p):
            return [f'missing {os.path.relpath(p, ROOT)}'], 0
    g = read_glb(glb)
    nodes = {n.get('name'): (i, n) for i, n in enumerate(g['nodes'])}
    roots = g['scenes'][g.get('scene', 0)]['nodes']
    for name in ['body'] + WHEELS:
        if name not in nodes:
            errs.append(f'node {name} missing')
        elif nodes[name][0] not in roots:
            errs.append(f'node {name} is not a scene root')
    if 'antenna' in nodes and nodes['antenna'][0] not in nodes['body'][1].get('children', []):
        errs.append('antenna is not a child of body')
    mats = {m['name'] for m in g['materials']}
    if MATERIALS - mats:
        errs.append(f'materials missing: {sorted(MATERIALS - mats)}')
    tris = 0
    for _, n in nodes.values():
        if 'mesh' in n:
            tris += mesh_bounds(g, n['mesh'])[2]
    if tris > 25000:
        errs.append(f'{tris} triangles > 25000')
    ron_w = read_ron(ron)
    hb = D['wheelbase'] / 2
    for name in WHEELS:
        if name not in nodes:
            continue
        n = nodes[name][1]
        i = 0 if name[6] == 'f' else 1
        side = -1 if name[7] == 'l' else 1
        want = [side * D['track'][i] / 2, D['radius'][i], -hb if i == 0 else hb]
        got = n.get('translation', [0, 0, 0])
        if any(abs(a - b) > TOL for a, b in zip(got, want)):
            errs.append(f'{name} at {got}, expected {want}')
        if any(abs(v) > 1e-6 for v in n.get('rotation', [0, 0, 0, 1])[:3]) or 'scale' in n:
            errs.append(f'{name} must have identity rotation and scale')
        r = ron_w.get(name)
        if r is None or any(abs(a - b) > TOL for a, b in zip(r[0], got)) \
                or abs(r[1] - D['radius'][i]) > TOL or abs(r[2] - D['wheel_width'][i]) > TOL:
            errs.append(f'{name}: wheels.ron {r} disagrees with glb/table')
        lo, hi, _ = mesh_bounds(g, n['mesh'])
        radius = (hi[1] - lo[1]) / 2
        if abs(radius - D['radius'][i]) > TOL:
            errs.append(f'{name} mesh radius {radius:.3f} != {D["radius"][i]}')
        width = hi[0] - lo[0]
        if not D['wheel_width'][i] - TOL <= width <= D['wheel_width'][i] * 1.06 + 0.012:
            errs.append(f'{name} mesh width {width:.3f} vs {D["wheel_width"][i]}')
    lo, hi, _ = mesh_bounds(g, nodes['body'][1]['mesh'])
    if hi[2] - lo[2] > D['length'] + 0.02:
        errs.append(f'body length {hi[2] - lo[2]:.3f} > {D["length"]}')
    top = max(D['height'], WING_TOP.get(car, 0.0))
    if hi[1] > top + TOL:
        errs.append(f'body height {hi[1]:.3f} > {top}')
    if hi[0] - lo[0] > D['width'] + 0.02:
        errs.append(f'body width {hi[0] - lo[0]:.3f} > {D["width"]}')
    if lo[1] < min(D['clearance'] - 0.015, LOW_PARTS.get(car, 1.0)):
        errs.append(f'lowest body point {lo[1]:.3f} < clearance {D["clearance"]}')
    w, h, color = png_info(png)
    if (w, h, color) != (512, 512, 6):
        errs.append(f'thumbnail is {w}x{h} colour type {color}, expected 512x512 RGBA')
    print(f'{car:9s} tris {tris:6d}  body L {hi[2] - lo[2]:.3f} W {hi[0] - lo[0]:.3f} '
          f'H {hi[1]:.3f} low {lo[1]:.3f}  {"OK" if not errs else "FAIL"}')
    return errs, tris


def main():
    table = dims()
    cars = sys.argv[1:] or CARS
    failed = False
    for car in cars:
        errs, _ = check(car, table[car])
        for e in errs:
            print(f'  {car}: {e}')
        failed |= bool(errs)
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
