"""Scene, units, named PBR materials and part objects (bevel + weighted normals).

Conventions (art bible §2.4): 1 Blender unit = 1 world unit (22.5 cm), Z up, car front = +Y,
origin at the ground-contact centre. Geometry is built with bmesh into *parts*; every part
object gets the full material list in the same slot order so face material indices stay valid
when parts are joined.
"""
import math

import bmesh
import bpy

MATS = []  # material names in slot order; index = bmesh face.material_index


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    s = bpy.context.scene
    s.unit_settings.system = 'METRIC'
    s.unit_settings.scale_length = 1.0
    MATS.clear()


def srgb(hex_or_rgb):
    """'#RRGGBB' or sRGB 0..1 tuple -> linear RGB tuple (Blender colour sockets are linear)."""
    if isinstance(hex_or_rgb, str):
        h = hex_or_rgb.lstrip('#')
        hex_or_rgb = tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    return tuple(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in hex_or_rgb)


def material(name, base, metallic=0.0, roughness=0.5, coat=0.0, coat_rough=0.04,
             emission=None, strength=0.0, two_sided=False):
    """Principled BSDF material that exports to glTF metallic-roughness (+clearcoat/emissive).
    One-sided (the game culls back faces) unless `two_sided`: leaves, grass and other open
    shells seen from both sides."""
    m = bpy.data.materials.new(name)
    m.use_backface_culling = not two_sided
    b = m.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*srgb(base), 1.0)
    b.inputs['Metallic'].default_value = metallic
    b.inputs['Roughness'].default_value = roughness
    b.inputs['Coat Weight'].default_value = coat
    if coat > 0:  # otherwise the exporter writes an empty clearcoat extension
        b.inputs['Coat Roughness'].default_value = coat_rough
    if emission is not None:  # linear colour (art bible §1.2)
        b.inputs['Emission Color'].default_value = (*emission, 1.0)
        b.inputs['Emission Strength'].default_value = strength
    m.diffuse_color = (*srgb(base), 1.0)
    MATS.append(name)
    return m


def car_materials(paint, accent, rim='#EDEDED', metal='#D9A441', rim_metallic=0.0,
                  paint_metallic=0.0, paint_rough=0.34, accent_metallic=None, glow=None):
    """The fixed car material set (art bible §2.3). Colours are sRGB hex strings."""
    material('paint', paint, paint_metallic, paint_rough, coat=1.0, coat_rough=0.04)
    material('paint_accent', accent, paint_metallic if accent_metallic is None else accent_metallic,
             paint_rough, coat=1.0, coat_rough=0.05)
    material('decal', '#EEEEEA', 0.0, 0.35, coat=1.0, coat_rough=0.05)
    material('light_head', '#FFF4E0', 0.0, 0.2, emission=(1.0, 0.92, 0.78), strength=6.0)
    material('light_tail', '#B01010', 0.0, 0.25, emission=(1.0, 0.06, 0.04), strength=2.5)
    material('light_brake', '#C01010', 0.0, 0.25, emission=(1.0, 0.05, 0.03), strength=10.0)
    material('glass', '#121820', 0.0, 0.05, coat=1.0, coat_rough=0.02)
    material('rubber', '#1F1F21', 0.0, 0.86)
    material('rim', rim, rim_metallic, 0.3 if rim_metallic else 0.38,
             coat=0.0 if rim_metallic else 0.6)
    material('plastic', '#262729', 0.0, 0.52)
    material('metal', metal, 1.0, 0.32)
    material('chrome', '#F5F5F5', 1.0, 0.07)
    if glow is not None:
        material('glow', '#202020', 0.0, 0.3, emission=srgb(glow), strength=5.0)


def mi(name):
    """Material slot index by name."""
    return MATS.index(name)


def new_bm():
    return bmesh.new()


def obj_from_bm(name, bm, bevel=0.0, segments=2, angle=35.0, weighted=True,
                location=(0.0, 0.0, 0.0), parent=None):
    """Create a mesh object from a bmesh; optional angle-limited bevel + weighted normals.

    Faces are smooth-shaded with sharp edges above `angle` so flat panels stay flat and fillets
    stay round.
    """
    me = bpy.data.meshes.new(name)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-6)
    bm.normal_update()
    bm.to_mesh(me)
    bm.free()
    for m in MATS:
        me.materials.append(bpy.data.materials[m])
    me.shade_smooth()
    me.set_sharp_from_angle(angle=math.radians(angle))
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    ob.location = location
    if parent is not None:
        ob.parent = parent
    if bevel > 0:
        b = ob.modifiers.new('bevel', 'BEVEL')
        b.width = bevel
        b.segments = segments
        b.limit_method = 'ANGLE'
        b.angle_limit = math.radians(angle)
        b.use_clamp_overlap = True
        b.harden_normals = weighted
        b.miter_outer = 'MITER_ARC'
    if weighted:
        w = ob.modifiers.new('wn', 'WEIGHTED_NORMAL')
        w.keep_sharp = True
        w.mode = 'FACE_AREA'
        w.weight = 50
    return ob


def bake(objects, name, location=(0.0, 0.0, 0.0)):
    """Apply modifiers of `objects`, join them into one object `name` with origin `location`."""
    objects = [o for o in objects if o is not None]
    bpy.ops.object.select_all(action='DESELECT')
    for o in objects:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.object.convert(target='MESH')
    if len(objects) > 1:
        bpy.ops.object.join()
    ob = bpy.context.view_layer.objects.active
    ob.name = name
    ob.data.name = name
    # move the origin without moving geometry
    import mathutils
    off = mathutils.Vector(location) - ob.location
    ob.data.transform(mathutils.Matrix.Translation(-off))
    ob.location = location
    ob.select_set(False)
    return ob


def tris(ob):
    me = ob.data
    me.calc_loop_triangles()
    return len(me.loop_triangles)
