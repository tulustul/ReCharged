"""Car dimensions, wheel placement, glTF export, wheel metadata and the build driver.

Usage from a car script:  blender -b -P tools/blender/cars/<id>.py -- [--preview DIR]
"""
import os
import sys

import bpy
from mathutils import Vector

from . import core, render, wheel

V = Vector
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
MAX_TRIS = 25000


def dims(car_id):
    """The car's row of docs/design/car-dimensions.md (the single source of truth)."""
    path = os.path.join(ROOT, 'docs', 'design', 'car-dimensions.md')
    for line in open(path, encoding='utf-8'):
        cells = [c.strip() for c in line.strip().strip('|').split('|')]
        if cells[0] == car_id:
            n = cells[2:]

            def pair(s):
                a, b = s.split('/')
                return float(a), float(b)

            return dict(length=float(n[0]), width=float(n[1]), height=float(n[2]),
                        wheelbase=float(n[3]), track=pair(n[4]), radius=pair(n[5]),
                        wheel_width=pair(n[6]), clearance=float(n[7]))
    raise SystemExit(f'{car_id}: no row in {path}')


def wheel_centers(D):
    """Blender-space wheel centres in order fl, fr, rl, rr (front = +Y, left = -X)."""
    hb = D['wheelbase'] / 2
    (tf, tr), (rf, rr) = D['track'], D['radius']
    return {'wheel_fl': V((-tf / 2, hb, rf)), 'wheel_fr': V((tf / 2, hb, rf)),
            'wheel_rl': V((-tr / 2, -hb, rr)), 'wheel_rr': V((tr / 2, -hb, rr))}


def wheels(D, front, rear=None):
    """Build the four wheels; `front`/`rear` are kwargs for wheel.wheel (tread, rim, ...)."""
    rear = rear or front
    out = []
    for name, c in wheel_centers(D).items():
        f = name[6] == 'f'
        i = 0 if f else 1
        side = -1 if name[7] == 'l' else 1
        out.append(wheel.wheel(name, c, D['radius'][i], D['wheel_width'][i], side,
                               **(front if f else rear)))
    return out


def _bevy(v):
    return (v.x + 0.0, v.z + 0.0, -v.y + 0.0)


def write_wheels_ron(car_id, D, wheel_objs, path):
    lines = ['(', f'    car: "{car_id}",',
             '    wheels: [ // order: fl, fr, rl, rr; centre in car space (Bevy axes), radius, width']
    for o in wheel_objs:
        i = 0 if o.name[6] == 'f' else 1
        x, y, z = _bevy(o.location)
        lines.append(f'        (name: "{o.name}", center: ({x:.3f}, {y:.3f}, {z:.3f}), '
                     f'radius: {D["radius"][i]:.3f}, width: {D["wheel_width"][i]:.3f}),')
    lines += ['    ],', ')', '']
    with open(path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))


def _args():
    argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
    preview = argv[argv.index('--preview') + 1] if '--preview' in argv else None
    return preview


def finish(car_id, D, body_parts, wheel_objs, antenna_obj, antenna_base):
    """Bake, check budgets, export glb + wheels.ron + thumbnail (+ previews with --preview)."""
    dg = bpy.context.evaluated_depsgraph_get()
    per_part = {}
    for o in body_parts:
        me = o.evaluated_get(dg).to_mesh()
        me.calc_loop_triangles()
        per_part[o.name] = len(me.loop_triangles)
        o.evaluated_get(dg).to_mesh_clear()
    print(f'[{car_id}] body parts: {per_part}')
    body = core.bake(body_parts, 'body')
    ant = core.bake([antenna_obj], 'antenna', location=antenna_base)
    ant.parent = body
    objs = [body, ant] + list(wheel_objs)
    counts = {o.name: core.tris(o) for o in objs}
    total = sum(counts.values())
    print(f'[{car_id}] triangles: {counts} total={total}')
    lo, hi = render._bounds([body] + list(wheel_objs))
    print(f'[{car_id}] bounds x {lo.x:.3f}..{hi.x:.3f}  y {lo.y:.3f}..{hi.y:.3f}  '
          f'z {lo.z:.3f}..{hi.z:.3f}  (antenna top {render._bounds([ant])[1].z:.3f})')
    if total > MAX_TRIS:
        raise SystemExit(f'{car_id}: {total} triangles > budget {MAX_TRIS}')
    out_dir = os.path.join(ROOT, 'assets', 'models', 'cars')
    os.makedirs(out_dir, exist_ok=True)
    bpy.ops.object.select_all(action='DESELECT')
    for o in objs:
        o.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=os.path.join(out_dir, f'{car_id}.glb'), export_format='GLB',
        use_selection=True, export_yup=True, export_apply=True, export_texcoords=False,
        export_normals=True, export_materials='EXPORT', export_cameras=False,
        export_lights=False, export_animations=False, export_extras=False, export_skins=False,
        export_morph=False)
    write_wheels_ron(car_id, D, wheel_objs, os.path.join(out_dir, f'{car_id}.wheels.ron'))
    ground = render.setup_studio()
    ui_dir = os.path.join(ROOT, 'assets', 'ui', 'cars')
    os.makedirs(ui_dir, exist_ok=True)
    render.thumbnail(objs, os.path.join(ui_dir, f'{car_id}.png'), ground)
    preview = _args()
    if preview:
        for p in render.preview(objs, preview, car_id):
            print(f'[{car_id}] preview {p}')
    return total
