"""Preview renderer (EEVEE, studio lighting, 4 angles + 3/4 hero, contact sheet) and UI
thumbnail renderer (512², transparent background)."""
import math
import os
import subprocess

import bpy
from mathutils import Vector

from . import core

V = Vector

# (name, direction from the target, lens mm or None for the game chase camera)
SHOTS = [
    ('hero', V((1.0, 1.25, 0.42)), 50),
    ('side', V((1.0, 0.0, 0.08)), 70),
    ('front', V((0.25, 1.0, 0.22)), 70),
    ('chase', None, None),
    ('top', V((0.0001, -0.18, 1.0)), 70),
]


def _world(strength=1.0):
    """Product-shot gradient sky: dark floor bounce, soft horizon, bright top (for reflections)."""
    w = bpy.data.worlds.new('studio')
    bpy.context.scene.world = w
    nt = w.node_tree
    bg = nt.nodes['Background']
    tc = nt.nodes.new('ShaderNodeTexCoord')
    sep = nt.nodes.new('ShaderNodeSeparateXYZ')
    ramp = nt.nodes.new('ShaderNodeValToRGB')
    nt.links.new(tc.outputs['Generated'], sep.inputs[0])
    mr = nt.nodes.new('ShaderNodeMapRange')
    mr.inputs['From Min'].default_value = -0.4
    mr.inputs['From Max'].default_value = 1.0
    nt.links.new(sep.outputs['Z'], mr.inputs['Value'])
    nt.links.new(mr.outputs['Result'], ramp.inputs['Fac'])
    els = ramp.color_ramp.elements
    els[0].position, els[0].color = 0.0, (0.16, 0.16, 0.17, 1)
    els[1].position, els[1].color = 1.0, (0.9, 0.9, 0.92, 1)
    for pos, c in ((0.28, (0.3, 0.31, 0.33)), (0.45, (0.55, 0.57, 0.6)), (0.75, (0.8, 0.82, 0.85))):
        e = els.new(pos)
        e.color = (*c, 1)
    nt.links.new(ramp.outputs['Color'], bg.inputs['Color'])
    bg.inputs['Strength'].default_value = strength


def _area(name, loc, target, energy, size, color=(1, 1, 1)):
    li = bpy.data.lights.new(name, 'AREA')
    li.energy = energy
    li.size = size
    li.color = color
    ob = bpy.data.objects.new(name, li)
    bpy.context.scene.collection.objects.link(ob)
    ob.location = loc
    ob.rotation_euler = (V(target) - V(loc)).to_track_quat('-Z', 'Y').to_euler()
    return ob


def setup_studio():
    s = bpy.context.scene
    s.render.engine = 'BLENDER_EEVEE'
    s.eevee.taa_render_samples = 64
    s.eevee.use_raytracing = True
    s.eevee.use_shadows = True
    s.eevee.use_fast_gi = True
    s.view_settings.view_transform = 'Khronos PBR Neutral'  # shows base colours faithfully
    s.view_settings.exposure = -0.5
    s.render.image_settings.file_format = 'PNG'
    _world(1.0)
    _area('key', (2.6, 2.4, 3.6), (0, 0, 0.2), 900, 2.5, (1.0, 0.97, 0.92))
    _area('fill', (-3.5, 1.0, 1.6), (0, 0, 0.3), 260, 4.0, (0.85, 0.9, 1.0))
    _area('rim', (-1.0, -3.6, 2.2), (0, 0, 0.3), 700, 2.0)
    me = bpy.data.meshes.new('ground')
    import bmesh
    bm = bmesh.new()
    bmesh.ops.create_circle(bm, cap_ends=True, radius=200, segments=64)
    bm.to_mesh(me)
    bm.free()
    mat = bpy.data.materials.new('ground')
    b = mat.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*core.srgb('#7A7D84'), 1)
    b.inputs['Roughness'].default_value = 0.6
    me.materials.append(mat)
    g = bpy.data.objects.new('ground', me)
    s.collection.objects.link(g)
    return g


def _camera(loc, target, lens=None, fov_h=None):
    cam = bpy.data.cameras.new('cam')
    if lens:
        cam.lens = lens
    if fov_h:
        cam.sensor_fit = 'HORIZONTAL'
        cam.angle = math.radians(fov_h)
    cam.clip_start = 0.02
    ob = bpy.data.objects.new('cam', cam)
    bpy.context.scene.collection.objects.link(ob)
    ob.location = loc
    ob.rotation_euler = (V(target) - V(loc)).to_track_quat('-Z', 'Y').to_euler()
    bpy.context.scene.camera = ob
    return ob


def _bounds(objs):
    pts = [o.matrix_world @ V(c) for o in objs for c in o.bound_box]
    lo = V((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
    hi = V((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
    return lo, hi


def _fit(objs, direction, lens, aspect, margin=0.86):
    lo, hi = _bounds(objs)
    c = (lo + hi) / 2
    r = (hi - lo).length / 2
    fov = 2 * math.atan(18 / lens)  # 36 mm sensor width
    if aspect < 1:
        fov *= aspect
    dist = r / math.sin(fov / 2) * margin
    return c + direction.normalized() * dist, c


def preview(objs, out_dir, car_id, res=(1280, 800)):
    """Render the preview shots of `objs` into out_dir/<car_id>_<shot>.png + a contact sheet."""
    os.makedirs(out_dir, exist_ok=True)
    s = bpy.context.scene
    s.render.resolution_x, s.render.resolution_y = res
    s.render.film_transparent = False
    paths = []
    for name, d, lens in SHOTS:
        if d is None:  # low game-like chase camera: 3.2 u behind, 0.95 u up, 70 deg horiz. FOV
            cam = _camera(V((0, -3.2, 0.95)), V((0, 1.2, 0.3)), fov_h=70)
        else:
            loc, tgt = _fit(objs, d, lens, res[0] / res[1])
            cam = _camera(loc, tgt, lens)
        p = os.path.join(out_dir, f'{car_id}_{name}.png')
        s.render.filepath = p
        bpy.ops.render.render(write_still=True)
        bpy.data.objects.remove(cam)
        paths.append(p)
    sheet = os.path.join(out_dir, f'{car_id}_sheet.png')
    subprocess.run(['magick', 'montage', *paths, '-tile', '3x2', '-geometry', '640x400+4+4',
                    '-background', '#202020', sheet], check=True)
    return paths + [sheet]


def thumbnail(objs, path, ground=None):
    """512² transparent 3/4 view for the car-select UI."""
    s = bpy.context.scene
    s.render.resolution_x = s.render.resolution_y = 512
    s.render.film_transparent = True
    s.render.image_settings.color_mode = 'RGBA'
    if ground is not None:
        ground.hide_render = True
    loc, tgt = _fit(objs, V((1.0, 1.15, 0.5)), 60, 1.0, margin=0.95)
    cam = _camera(loc, tgt, 60)
    s.render.filepath = path
    bpy.ops.render.render(write_still=True)
    bpy.data.objects.remove(cam)
    if ground is not None:
        ground.hide_render = False
    s.render.film_transparent = False
    return path
