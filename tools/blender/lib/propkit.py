"""Prop pipeline for environment generators (art bible §3, PRACTICES §7): bake, budget and
ground-contact checks, glTF export, and EEVEE previews on a stage with a reference car.

Conventions as for the cars (lib/core.py): 1 Blender unit = 1 world unit (22.5 cm), props are
real size x 4.44, Z up, front = +Y, origin at the ground-contact centre; exported "+Y up" so the
front ends up at Bevy -Z. Build part objects with lib.core / lib.geo, then call finish():

    blender -b -P tools/blender/props/<env>/<prop>.py -- [--preview DIR]
"""
import math
import os
import subprocess
import sys

import bmesh
import bpy
from mathutils import Vector

from . import core, render

V = Vector
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
KIT_TRIS, HERO_TRIS = 5000, 15000
REF_CAR = os.path.join(ROOT, 'assets', 'models', 'cars', 'sparky.glb')


def _argv():
    return sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []


def preview_dir():
    a = _argv()
    return a[a.index('--preview') + 1] if '--preview' in a else None


def finish(env, name, parts, budget=KIT_TRIS, below=0.0, car=(None, None, 0.0), shots=None,
           before_preview=None):
    """Bake `parts` into one object, check budget and ground contact, export
    assets/models/props/<env>/<name>.glb; with `--preview DIR` render previews.

    below: how far geometry may legitimately reach under the origin (supports sunk into the
    floor). car: (x, y, yaw_deg) of the reference car in the previews (None = beside the prop).
    before_preview(ob): optional hook run after the export to pose the prop or add context
    objects for the previews only. Returns the baked object."""
    parts = [p for p in parts if p is not None]
    if any(p.data.uv_layers for p in parts):  # one shared UV map, or glTF gets texCoord -1
        for p in parts:
            if not p.data.uv_layers:
                p.data.uv_layers.new(name='UVMap')
            p.data.uv_layers[0].name = 'UVMap'
    dg = bpy.context.evaluated_depsgraph_get()
    per = {}
    for p in parts:
        me = p.evaluated_get(dg).to_mesh()
        me.calc_loop_triangles()
        per[p.name] = len(me.loop_triangles)
        low = min((p.matrix_world @ v.co).z for v in me.vertices) if me.vertices else 0.0
        if low < -below - 0.02:
            print(f'[{name}] part {p.name} reaches z = {low:.3f}')
        p.evaluated_get(dg).to_mesh_clear()
    print(f'[{name}] parts {per}')
    ob = core.bake(parts, name)
    n = core.tris(ob)
    lo, hi = render._bounds([ob])
    print(f'[{name}] triangles {n} (budget {budget})  size x {hi.x - lo.x:.2f} y {hi.y - lo.y:.2f}'
          f' z {hi.z - lo.z:.2f}  bounds x {lo.x:.2f}..{hi.x:.2f} y {lo.y:.2f}..{hi.y:.2f} z '
          f'{lo.z:.2f}..{hi.z:.2f}')
    if n > budget:
        raise SystemExit(f'{name}: {n} triangles > budget {budget}')
    if lo.z < -below - 0.02:
        raise SystemExit(f'{name}: geometry reaches {lo.z:.3f} below the origin (allowed {below})')
    out = os.path.join(ROOT, 'assets', 'models', 'props', env)
    os.makedirs(out, exist_ok=True)
    bpy.ops.object.select_all(action='DESELECT')
    ob.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=os.path.join(out, f'{name}.glb'), export_format='GLB', use_selection=True,
        export_yup=True, export_apply=True, export_texcoords=bool(ob.data.uv_layers),
        export_normals=True,
        export_materials='EXPORT', export_cameras=False, export_lights=False,
        export_animations=False, export_extras=False, export_skins=False, export_morph=False)
    pdir = preview_dir()
    if pdir:
        if before_preview:
            before_preview(ob)
        for p in preview(ob, pdir, name, car, shots):
            print(f'[{name}] preview {p}')
    return ob


# ---------------------------------------------------------------------------------------------
# previews

def stage(floor='#2E9C9A', floor_rough=0.95):
    """Daylight stage: warm sun (window), cool sky fill, a big floor (default: play-mat teal)."""
    s = bpy.context.scene
    s.render.engine = 'BLENDER_EEVEE'
    s.eevee.taa_render_samples = 48
    s.eevee.use_raytracing = True
    s.eevee.use_shadows = True
    s.eevee.use_fast_gi = True
    s.view_settings.view_transform = 'Khronos PBR Neutral'
    s.view_settings.exposure = -0.3
    s.render.image_settings.file_format = 'PNG'
    w = bpy.data.worlds.new('sky')
    s.world = w
    bg = w.node_tree.nodes['Background']
    bg.inputs['Color'].default_value = (*core.srgb('#B9C8D8'), 1)
    bg.inputs['Strength'].default_value = 0.9
    sun = bpy.data.lights.new('sun', 'SUN')
    sun.energy = 3.2
    sun.angle = math.radians(6)
    sun.color = (1.0, 0.93, 0.82)
    so = bpy.data.objects.new('sun', sun)
    s.collection.objects.link(so)
    so.rotation_euler = V((0.55, 0.75, 1.0)).to_track_quat('Z', 'Y').to_euler()
    me = bpy.data.meshes.new('floor')
    bm = bmesh.new()
    bmesh.ops.create_circle(bm, cap_ends=True, radius=400, segments=64)
    bm.to_mesh(me)
    bm.free()
    m = bpy.data.materials.new('stage_floor')
    b = m.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*core.srgb(floor), 1)
    b.inputs['Roughness'].default_value = floor_rough
    me.materials.append(m)
    f = bpy.data.objects.new('floor', me)
    f.location.z = -0.002
    s.collection.objects.link(f)
    return f


def ref_car(x, y, yaw=0.0):
    """Import the reference car (Sparky, 2 u long) at (x, y) facing yaw degrees from +Y."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=REF_CAR)
    new = [o for o in bpy.data.objects if o not in before]
    root = bpy.data.objects.new('ref_car', None)
    bpy.context.scene.collection.objects.link(root)
    for o in new:
        if o.parent is None:
            o.parent = root
    root.location = (x, y, 0)
    root.rotation_euler = (0, 0, math.radians(yaw))
    bpy.context.view_layer.update()  # children's matrix_world is stale until now
    return root, new


# (shot, direction from the target or None for the RC eye-level shot, lens mm)
SHOTS = [
    ('hero', V((1.0, 1.3, 0.62)), 50),
    ('front', V((0.12, 1.0, 0.28)), 50),
    ('back', V((-0.7, -1.0, 0.45)), 50),
    ('top', V((0.0001, -0.25, 1.0)), 50),
    ('rc', None, 24),
]


def preview(ob, out_dir, name, car=(None, None, 0.0), shots=None, res=(1280, 800)):
    """Render the shots of `ob` (+ reference car) into out_dir/<name>_<shot>.png and a sheet."""
    os.makedirs(out_dir, exist_ok=True)
    stage()
    lo, hi = render._bounds([ob])
    cx, cy, cyaw = car
    if cx is None:
        cx, cy = hi.x + 1.6, lo.y + 1.2
    _, car_objs = ref_car(cx, cy, cyaw)
    s = bpy.context.scene
    s.render.resolution_x, s.render.resolution_y = res
    s.render.film_transparent = False
    paths = []
    for shot, d, lens in shots or SHOTS:
        if d is None:  # RC eye level: low camera in front of the prop
            c = (lo + hi) / 2
            r = max((hi - lo).length / 2, 3.0)
            eye = V((c.x + r * 0.55, c.y + r * 1.9, 1.0))
            cam = render._camera(eye, V((c.x, c.y, min(hi.z * 0.45, 3.0))), lens)
        else:
            loc, tgt = render._fit([ob] + [o for o in car_objs if o.type == 'MESH'], d, lens,
                                   res[0] / res[1])
            cam = render._camera(loc, tgt, lens)
        cam.data.clip_end = 2000
        p = os.path.join(out_dir, f'{name}_{shot}.png')
        s.render.filepath = p
        bpy.ops.render.render(write_still=True)
        bpy.data.objects.remove(cam)
        paths.append(p)
    sheet = os.path.join(out_dir, f'{name}_sheet.png')
    subprocess.run(['magick', 'montage', *paths, '-tile', '3x2', '-geometry', '640x400+4+4',
                    '-background', '#202020', sheet], check=True)
    return paths + [sheet]
