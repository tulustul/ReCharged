"""Street environment (T16) additions to lib/propkit.py, which keeps bake / budget / export /
previews: a material-table helper and an outdoor preview stage (physical sky, sun, asphalt).

    from lib import propkit, street_props
    street_props.use_outdoor()            # propkit previews now render outdoors
    street_props.materials({...})
    propkit.finish(env, name, parts, budget=..., car=(x, y, yaw))
"""
import math

import bmesh
import bpy
from mathutils import Vector

from . import core, propkit

V = Vector


def materials(spec):
    """Create materials from {name: dict(base=hex, metallic, rough, coat, coat_rough, emission,
    strength, two_sided)}; the dict order is the slot order (core.MATS)."""
    for name, m in spec.items():
        core.material(name, m['base'], m.get('metallic', 0.0), m.get('rough', 0.5),
                      coat=m.get('coat', 0.0), coat_rough=m.get('coat_rough', 0.04),
                      emission=m.get('emission'), strength=m.get('strength', 0.0),
                      two_sided=m.get('two_sided', False))


def outdoor_stage(ground='#4A4C50', sun_elev=40.0, sun_az=-35.0, size=400.0, hole=None):
    """Morning sun + multiple-scattering sky + an asphalt-grey ground (AgX view); `hole`
    = (x0, x1, y0, y1) leaves a rectangular opening for sunken props."""
    s = bpy.context.scene
    s.render.engine = 'BLENDER_EEVEE'
    s.eevee.taa_render_samples = 48
    s.eevee.use_raytracing = True
    s.eevee.use_shadows = True
    s.eevee.use_fast_gi = True
    s.view_settings.view_transform = 'AgX'
    s.view_settings.look = 'AgX - Medium High Contrast'
    s.view_settings.exposure = -0.4
    s.render.image_settings.file_format = 'PNG'
    w = bpy.data.worlds.new('sky')
    s.world = w
    nt = w.node_tree
    sky = nt.nodes.new('ShaderNodeTexSky')
    sky.sky_type = 'MULTIPLE_SCATTERING'
    sky.sun_elevation = math.radians(sun_elev)
    sky.sun_rotation = math.radians(sun_az)
    sky.sun_disc = False
    nt.links.new(sky.outputs['Color'], nt.nodes['Background'].inputs['Color'])
    nt.nodes['Background'].inputs['Strength'].default_value = 0.12
    li = bpy.data.lights.new('sun', 'SUN')
    li.energy = 5.0
    li.angle = math.radians(1.5)
    li.color = (1.0, 0.95, 0.88)
    sun = bpy.data.objects.new('sun', li)
    s.collection.objects.link(sun)
    e, a = math.radians(sun_elev), math.radians(sun_az)
    to_sun = V((math.cos(e) * math.sin(a), math.cos(e) * math.cos(a), math.sin(e)))
    sun.rotation_euler = to_sun.to_track_quat('Z', 'Y').to_euler()  # light shines along -Z
    me = bpy.data.meshes.new('ground')
    bm = bmesh.new()
    if hole is None:
        bmesh.ops.create_circle(bm, cap_ends=True, radius=size, segments=64)
    else:
        x0, x1, y0, y1 = hole
        outer = [bm.verts.new(V(p)) for p in ((-size, -size, 0), (size, -size, 0),
                                               (size, size, 0), (-size, size, 0))]
        inner = [bm.verts.new(V(p)) for p in ((x0, y0, 0), (x1, y0, 0), (x1, y1, 0),
                                               (x0, y1, 0))]
        for i in range(4):
            j = (i + 1) % 4
            bm.faces.new((outer[i], outer[j], inner[j], inner[i]))
    bm.to_mesh(me)
    bm.free()
    m = bpy.data.materials.new('stage_ground')
    b = m.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*core.srgb(ground), 1)
    b.inputs['Roughness'].default_value = 0.9
    me.materials.append(m)
    g = bpy.data.objects.new('ground', me)
    g.location.z = -0.002
    s.collection.objects.link(g)
    return g


def use_outdoor(**stage_kw):
    """Make propkit.preview() render on the outdoor stage."""
    propkit.stage = lambda: outdoor_stage(**stage_kw)


def finish(env, name, parts, budget=propkit.KIT_TRIS, below=0.0, car=(None, None, 0.0),
           cams=None, res=(1280, 800)):
    """propkit.finish (bake, checks, export); with `--preview DIR` and `cams` =
    [(shot, location, target, lens)], renders those cameras on the outdoor stage instead of
    propkit's auto-framed shots (which crop tall or flat props)."""
    import subprocess
    import sys
    pdir = propkit.preview_dir()
    argv = sys.argv[:]
    if pdir and cams:
        i = sys.argv.index('--preview')
        del sys.argv[i:i + 2]
    try:
        ob = propkit.finish(env, name, parts, budget, below, car=car)
    finally:
        sys.argv[:] = argv
    if not (pdir and cams):
        return ob
    import os
    os.makedirs(pdir, exist_ok=True)
    propkit.stage()
    lo, hi = propkit.render._bounds([ob])
    cx, cy, cyaw = car
    if cx is None:
        cx, cy = hi.x + 1.6, lo.y + 1.2
    propkit.ref_car(cx, cy, cyaw)
    s = bpy.context.scene
    s.render.resolution_x, s.render.resolution_y = res
    paths = []
    for shot, loc, tgt, lens in cams:
        cam = propkit.render._camera(V(loc), V(tgt), lens)
        cam.data.clip_end = 3000
        p = os.path.join(pdir, f'{name}_{shot}.png')
        s.render.filepath = p
        bpy.ops.render.render(write_still=True)
        bpy.data.objects.remove(cam)
        paths.append(p)
    sheet = os.path.join(pdir, f'{name}_sheet.png')
    subprocess.run(['magick', 'montage', *paths, '-tile', f'{min(3, len(paths))}x',
                    '-geometry', '640x400+4+4', '-background', '#202020', sheet], check=True)
    for p in paths + [sheet]:
        print(f'[{name}] preview {p}')
    return ob
