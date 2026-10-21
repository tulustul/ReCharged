"""Wrench Works (auto-repair workshop) prop kit: palette, material table, procedural textures,
an indoor preview stage and shared shapes (art bible §3.6). Prop scripts put this directory on
sys.path and `from wkit import *`; it makes tools/blender/lib importable as `lib`.

Previews: warm sodium work light from above against blue daylight from the side (the roller
door), on stained concrete, with the reference RC car (Sparky) for scale.
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, '..', '..')))
sys.path.insert(0, HERE)

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import catalog as C  # noqa: E402,F401
from lib import core, geo, propkit  # noqa: E402,F401
from lib import workshop_car as wc  # noqa: E402
from lib.workshop_car import place, revolve, tyre_profile  # noqa: E402,F401

V = Vector
ENV = 'workshop'
KIT, HERO = propkit.KIT_TRIS, propkit.HERO_TRIS
mi = core.mi

# art bible §3.6 palette + neutrals (sRGB)
CONCRETE, YELLOW, RED, BLUE = '#8D8A84', '#F2B705', '#B8322A', '#2C5E9E'
RUBBER, PLYWOOD, OIL = '#2A2A2A', '#C8A46E', '#1C1A18'

# name -> Principled parameters (art bible §1.1 ranges); `tex` names a texture generator.
MATS = {
    'rubber': dict(base='#262628', rough=0.86),
    'tyre': dict(base='#2A2A2C', rough=0.78, tex='tread'),
    'steel_black': dict(base='#2B2C2E', metallic=0.6, rough=0.5),
    'steel': dict(base='#A9ADB1', metallic=1.0, rough=0.42),
    'steel_dark': dict(base='#5E6266', metallic=1.0, rough=0.5),
    'galvanised': dict(base='#B9BCBD', metallic=1.0, rough=0.48),
    'chrome': dict(base='#F2F2F2', metallic=1.0, rough=0.08),
    'aluminium': dict(base='#C9CCCF', metallic=1.0, rough=0.32),
    'lift_blue': dict(base=BLUE, rough=0.45),
    'chest_red': dict(base=RED, rough=0.3),
    'safety_yellow': dict(base=YELLOW, rough=0.5),
    'hazard': dict(base=YELLOW, rough=0.55, tex='hazard'),
    'hazard_black': dict(base='#202022', rough=0.55),
    'plastic_black': dict(base='#252628', rough=0.55),
    'plastic_grey': dict(base='#6E7277', rough=0.5),
    'plastic_red': dict(base='#B3282A', rough=0.45),
    'plastic_orange': dict(base='#E2601B', rough=0.42),
    'plastic_blue': dict(base='#2F5EA8', rough=0.45),
    'plastic_white': dict(base='#E4E2DC', rough=0.45),
    'reflective': dict(base='#E8E8E2', rough=0.3),
    'plywood': dict(base=PLYWOOD, rough=0.7, tex='plywood'),
    'plywood_edge': dict(base='#B8925A', rough=0.75, tex='plyedge'),
    'timber': dict(base='#A88456', rough=0.75, tex='timber'),
    'butcher_block': dict(base='#B98A55', rough=0.5, tex='block'),
    'paint_grey': dict(base='#8E959C', rough=0.5),
    'paint_white': dict(base='#D9DBDA', rough=0.55),
    'filter_white': dict(base='#E2E2DC', rough=0.95),
    'tile_white': dict(base='#D8DAD6', rough=0.12, tex='tiles'),
    'glass_dark': dict(base='#1A2026', rough=0.06),
    'primer': dict(base='#9A9C9E', rough=0.62),
    'masking': dict(base='#C9B48C', rough=0.9),
    'light_panel': dict(base='#F4F6F8', rough=0.3, emission=(0.92, 0.96, 1.0), strength=4.0),
    'lamp_sodium': dict(base='#FFE2B0', rough=0.3, emission=(1.0, 0.72, 0.42), strength=12.0),
    'oil_dark': dict(base=OIL, rough=0.1),
    'amber_plastic': dict(base='#B8741E', rough=0.22),
    'cardboard': dict(base='#B4936A', rough=0.9),
    'rag_blue': dict(base='#3F5E8C', rough=0.95),
    'rag_red': dict(base='#9A3B30', rough=0.95),
    'copper': dict(base='#B87333', metallic=1.0, rough=0.35),
    'drum_blue': dict(base='#2D5B8C', metallic=0.3, rough=0.45),
    'drum_red': dict(base='#9C2E26', metallic=0.3, rough=0.45),
    'vinyl_black': dict(base='#232325', rough=0.5),
    'label_white': dict(base='#E6E4DC', rough=0.5),
    'door_slats': dict(base='#A8ACAE', metallic=0.7, rough=0.45, tex='slats'),
    'pegboard': dict(base='#C9B48C', rough=0.85, tex='pegs'),
    'light_tube': dict(base='#F2F4F6', rough=0.3, emission=(0.88, 0.94, 1.0), strength=6.0),
    'plastic_green': dict(base='#2F7D45', rough=0.45),
    'plastic_yellow': dict(base='#D9A520', rough=0.45),
    'floor_paint': dict(base='#D9A80F', rough=0.7),
    'grime': dict(base='#7A766F', rough=0.4),
    'screen': dict(base='#10161A', rough=0.2, emission=(0.35, 0.8, 0.6), strength=1.5),
}


# ---------------------------------------------------------------------------------------------
# textures (numpy -> packed image, embedded in the glb)

def _image(name, arr, non_color=False):
    import numpy as np
    h, w = arr.shape[:2]
    img = bpy.data.images.new(name, w, h, alpha=False, float_buffer=False)
    if non_color:  # before writing: changing it later regenerates (blanks) the image
        img.colorspace_settings.name = 'Non-Color'
    rgba = np.concatenate([arr, np.ones((h, w, 1))], axis=-1)
    img.pixels.foreach_set(rgba.ravel().astype(np.float32))
    img.pack()
    assert img.packed_file is not None, f'{name}: texture not packed'
    return img


def _srgb01(hexc):
    h = hexc.lstrip('#')
    return [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]


def _periodic(rng, u, v, fu, fv, amp):
    import numpy as np
    out = np.zeros_like(u)
    for a in range(0, fu + 1):
        for b in range(0, fv + 1):
            if a == 0 and b == 0:
                continue
            out += rng.normal() * amp / (a + b) * np.sin(2 * np.pi * (a * u + b * v)
                                                         + rng.uniform(0, 2 * np.pi))
    return out


def grain(base, size=1024, stripes=7, contrast=0.16, warp=0.08, seed=1, knots=0):
    """Tileable rotary-cut veneer: wavy growth bands along v, pores, optional knots."""
    import numpy as np
    rng = np.random.default_rng(seed)
    t = np.arange(size) / size
    u, v = np.meshgrid(t, t)
    phase = stripes * u + _periodic(rng, u, v, 3, 2, warp) * stripes + _periodic(
        rng, u, v, 1, 3, 0.35)
    ring = 0.5 + 0.5 * np.sin(2 * np.pi * phase)
    late = np.power(ring, 5)
    pores = np.repeat(rng.normal(size=(size // 16, size)), 16, axis=0)  # pores along v
    shade = 1.0 - contrast * (0.6 * late + 0.25 * ring) + 0.018 * pores
    shade += _periodic(rng, u, v, 2, 2, 0.05)
    for _ in range(knots):  # darker oval knots, tiled
        cu, cv = rng.uniform(0, 1, 2)
        du = (u - cu + 0.5) % 1 - 0.5
        dv = (v - cv + 0.5) % 1 - 0.5
        r = np.hypot(du / 0.018, dv / 0.045)
        shade -= 0.22 * np.exp(-r * r) + 0.06 * np.exp(-r * r / 6)
    rgb = np.clip(np.stack([shade * c for c in _srgb01(base)], axis=-1), 0, 1)
    return rgb


def block_strips(base, size=1024, strips=12, seed=3, stains=5):
    """Butcher-block top: glued strips along v with per-strip tone and grain, a few worn oil
    stains (tileable)."""
    import numpy as np
    rng = np.random.default_rng(seed)
    g = grain(base, size, stripes=19, contrast=0.1, warp=0.05, seed=seed)
    t = (np.arange(size) + 0.5) / size
    u, v = np.meshgrid(t, t)
    k = np.floor(u * strips).astype(int)
    tone = (1 + 0.045 * rng.normal(size=strips))[k]
    edge = np.abs((u * strips) % 1 - 0.5) > 0.485
    tone = np.where(edge, tone * 0.86, tone)
    for _ in range(stains):
        cu, cv = rng.uniform(0, 1, 2)
        du = (u - cu + 0.5) % 1 - 0.5
        dv = (v - cv + 0.5) % 1 - 0.5
        r = np.hypot(du / rng.uniform(0.03, 0.08), dv / rng.uniform(0.03, 0.08))
        tone = tone * (1 - 0.32 * np.exp(-r ** 3))
    return np.clip(g * tone[..., None], 0, 1)


def ply_edge(base, size=256, plies=7):
    """Plywood edge: alternating light and dark plies across v."""
    import numpy as np
    t = (np.arange(size) + 0.5) / size
    u, v = np.meshgrid(t, t)
    ply = np.floor(v * plies).astype(int)
    shade = np.where(ply % 2 == 0, 1.0, 0.78) * (1 - 0.06 * np.abs(np.sin(9 * np.pi * u)))
    glue = np.abs((v * plies) % 1 - 0.5) > 0.46
    shade = np.where(glue, shade * 0.7, shade)
    return np.clip(np.stack([shade * c for c in _srgb01(base)], axis=-1), 0, 1)


def normal_from_height(h, strength):
    """Tangent-space normal map (OpenGL / glTF convention) from a tileable height field."""
    import numpy as np
    du = (np.roll(h, -1, axis=1) - np.roll(h, 1, axis=1)) * 0.5
    dv = (np.roll(h, -1, axis=0) - np.roll(h, 1, axis=0)) * 0.5
    n = np.stack([-du * strength, -dv * strength, np.ones_like(h)], axis=-1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    return n * 0.5 + 0.5


def tread_height(w=512, h=256):
    """Tyre band height field: u around (one repeat = 1/12 of the circumference), v across the
    whole profile (0 = inner bead ... 1 = outer bead). Sidewalls carry rim-protector and
    lettering-like ridges; the tread (v 0.36-0.64) has 4 grooves and shoulder blocks."""
    import numpy as np
    t_u = (np.arange(w) + 0.5) / w
    t_v = (np.arange(h) + 0.5) / h
    u, v = np.meshgrid(t_u, t_v)
    hf = np.zeros_like(u)
    tv = (v - 0.36) / 0.28  # 0..1 across the tread
    in_tread = (tv > 0) & (tv < 1)
    for g in (0.2, 0.4, 0.6, 0.8):  # circumferential grooves
        hf -= np.where(in_tread, np.exp(-((tv - g) / 0.03) ** 2), 0)
    sh = in_tread & ((tv < 0.2) | (tv > 0.8))  # shoulder blocks: lateral grooves, 6 per repeat
    lat = np.exp(-(((u * 6 + 0.25 * tv) % 1 - 0.5) / 0.07) ** 2)
    hf -= np.where(sh, lat * 0.8, 0)
    mid = in_tread & (tv > 0.2) & (tv < 0.8)  # angled sipes in the centre ribs
    sipe = np.exp(-(((u * 9 + 1.5 * tv) % 1 - 0.5) / 0.04) ** 2)
    hf -= np.where(mid, sipe * 0.35, 0)
    for side in (v, 1 - v):  # sidewall: rim protector ring + faint lettering band
        hf += 0.6 * np.exp(-((side - 0.08) / 0.02) ** 2)
        band = (side > 0.17) & (side < 0.24)
        letters = (np.sin(2 * np.pi * u * 14) > 0.3) & (np.sin(2 * np.pi * side * 30) > -0.2)
        hf += np.where(band & letters, 0.25, 0)
    return hf


def hazard(size=256, stripes=4):
    """Diagonal safety-yellow / black hazard stripes (tileable at 45 deg)."""
    import numpy as np
    t = (np.arange(size) + 0.5) / size
    u, v = np.meshgrid(t, t)
    k = ((u + v) * stripes) % 1.0
    m = np.clip((np.abs(k - 0.5) - 0.25) / 0.02 + 0.5, 0, 1)[..., None]  # antialiased edge
    y, b = np.array(_srgb01(YELLOW)), np.array(_srgb01('#1E1E20'))
    rng = np.random.default_rng(9)
    wear = 1 - 0.06 * np.abs(_periodic(rng, u, v, 3, 3, 0.5))[..., None]
    return np.clip((y * m + b * (1 - m)) * wear, 0, 1)


def tiles(base, size=512, n=4, grout='#9A9C98', seed=4):
    """Glazed square tiles (n x n per texture) with grout lines and a slight per-tile tone."""
    import numpy as np
    rng = np.random.default_rng(seed)
    t = (np.arange(size) + 0.5) / size
    u, v = np.meshgrid(t, t)
    iu, iv = np.floor(u * n).astype(int), np.floor(v * n).astype(int)
    fu, fv = (u * n) % 1, (v * n) % 1
    tone = (1 + 0.025 * rng.normal(size=(n, n)))[iv, iu]
    g = ((fu < 0.035) | (fu > 0.965) | (fv < 0.035) | (fv > 0.965))[..., None]
    col = np.array(_srgb01(base)) * tone[..., None]
    return np.clip(np.where(g, np.array(_srgb01(grout)), col), 0, 1)


def slats(base, size=256, n=8):
    """Roller-door curtain: horizontal ribbed slats (shading only, tileable along u)."""
    import numpy as np
    t = (np.arange(size) + 0.5) / size
    u, v = np.meshgrid(t, t)
    f = (v * n) % 1
    shade = 0.82 + 0.18 * np.sin(np.pi * f) ** 0.6 - 0.25 * (f > 0.93)
    return np.clip(np.stack([shade * c for c in _srgb01(base)], axis=-1), 0, 1)


def pegs(base, size=512, per=12):
    """Pegboard: dark round holes on a square grid."""
    import numpy as np
    t = (np.arange(size) + 0.5) / size * per
    u, v = np.meshgrid(t, t)
    d = np.hypot(u % 1 - 0.5, v % 1 - 0.5)
    m = np.clip((d - 0.12) / 0.04 + 0.5, 0, 1)[..., None]
    return np.array(_srgb01('#3A332A')) * (1 - m) + np.array(_srgb01(base)) * m


TEXTURES = {
    'slats': lambda base: (slats(base), None),
    'pegs': lambda base: (pegs(base), None),
    'tiles': lambda base: (tiles(base), None),
    'hazard': lambda base: (hazard(), None),
    'plywood': lambda base: (grain(base, contrast=0.12, knots=3, seed=11), None),
    'plyedge': lambda base: (ply_edge(base), None),
    'timber': lambda base: (grain(base, size=512, stripes=11, contrast=0.2, seed=5, knots=1),
                            None),
    'block': lambda base: (block_strips(base), None),
    'tread': lambda base: (None, normal_from_height(tread_height(), 3.0)),
}


def material(name, spec):
    m = core.material(name, spec['base'], spec.get('metallic', 0.0), spec.get('rough', 0.5),
                      coat=spec.get('coat', 0.0), coat_rough=spec.get('coat_rough', 0.04),
                      emission=spec.get('emission'), strength=spec.get('strength', 0.0))
    if 'tex' in spec:
        albedo, normal = TEXTURES[spec['tex']](spec['base'])
        nt = m.node_tree
        b = nt.nodes['Principled BSDF']
        if albedo is not None:
            tex = nt.nodes.new('ShaderNodeTexImage')
            tex.image = _image(f'{name}_albedo', albedo)
            nt.links.new(tex.outputs['Color'], b.inputs['Base Color'])
        if normal is not None:
            tex = nt.nodes.new('ShaderNodeTexImage')
            tex.image = _image(f'{name}_normal', normal, non_color=True)
            nm = nt.nodes.new('ShaderNodeNormalMap')
            nt.links.new(tex.outputs['Color'], nm.inputs['Color'])
            nt.links.new(nm.outputs['Normal'], b.inputs['Normal'])
    return m


def start(*names, extra=None):
    """New scene, the workshop preview stage, materials in slot order (`extra` adds specs)."""
    core.reset()
    propkit.stage = stage
    spec = dict(MATS, **(extra or {}))
    for n in names:
        material(n, spec[n])


# ---------------------------------------------------------------------------------------------
# preview stage

def stage():
    """Indoor workshop: warm sodium key from above, cool daylight from the door side, concrete."""
    s = bpy.context.scene
    s.render.engine = 'BLENDER_EEVEE'
    s.eevee.taa_render_samples = 48
    s.eevee.use_raytracing = True
    s.eevee.use_shadows = True
    s.eevee.use_fast_gi = True
    s.view_settings.view_transform = 'AgX'
    s.view_settings.look = 'None'
    s.view_settings.exposure = 0.5
    s.render.image_settings.file_format = 'PNG'
    w = bpy.data.worlds.new('shop')
    s.world = w
    bg = w.node_tree.nodes['Background']
    bg.inputs['Color'].default_value = (*core.srgb('#7D7A76'), 1)
    bg.inputs['Strength'].default_value = 0.9
    for name, energy, col, d in (('key', 3.4, (1.0, 0.78, 0.52), V((0.35, -0.45, 1.0))),
                                 ('door', 1.6, (0.72, 0.84, 1.0), V((-1.0, 0.6, 0.45)))):
        li = bpy.data.lights.new(name, 'SUN')
        li.energy = energy
        li.angle = math.radians(8)
        li.color = col
        ob = bpy.data.objects.new(name, li)
        s.collection.objects.link(ob)
        ob.rotation_euler = d.to_track_quat('Z', 'Y').to_euler()
    me = bpy.data.meshes.new('floor')
    bm = bmesh.new()
    bmesh.ops.create_circle(bm, cap_ends=True, radius=400, segments=64)
    bm.to_mesh(me)
    bm.free()
    m = bpy.data.materials.new('stage_floor')
    b = m.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*core.srgb(CONCRETE), 1)
    b.inputs['Roughness'].default_value = 0.85
    me.materials.append(m)
    f = bpy.data.objects.new('floor', me)
    f.location.z = -0.002
    s.collection.objects.link(f)
    return f


# ---------------------------------------------------------------------------------------------
# UV helpers

def planar_uv(bm, faces, u_axis, v_axis, scale=0.25, origin=(0, 0, 0)):
    """Project face corners onto u_axis / v_axis (world units x scale = texture repeats)."""
    uv = bm.loops.layers.uv.verify()
    o, ua, va = V(origin), V(u_axis), V(v_axis)
    for f in faces:
        for lp in f.loops:
            d = lp.vert.co - o
            lp[uv].uv = (d.dot(ua) * scale, d.dot(va) * scale)


def box_uv(bm, faces, scale=0.25):
    """Tri-planar style: each face projected along its dominant normal axis."""
    uv = bm.loops.layers.uv.verify()
    for f in faces:
        n = f.normal
        ax = max(range(3), key=lambda i: abs(n[i]))
        a, b = [(1, 2), (0, 2), (0, 1)][ax]
        for lp in f.loops:
            lp[uv].uv = (lp.vert.co[a] * scale, lp.vert.co[b] * scale)


# ---------------------------------------------------------------------------------------------
# shapes

RIM_R = 0.203 * C.M  # 16" rim radius (0.90 u)


def tyre(bm, center=(0, 0, 0), axis=(0, 0, 1), r=None, w=None, rim_r=RIM_R, n=32, mat='tyre',
         inner=True, bottom=True, top=True, spin=0.0, seg=3):
    """Car tyre (catalog 205/55 R16 by default) about `axis`, lying flat unless told otherwise;
    the inner casing behind the bead hole is dark rubber."""
    return wc.tyre(bm, center, axis, r or C.TYRE['r'], w or C.TYRE['w'], rim_r, n=n, mat=mat,
                   inner_mat='rubber', inner=inner, bottom=bottom, top=top, spin=spin, seg=seg)


def steel_rim(bm, center=(0, 0, 0), axis=(0, 0, 1), rim_r=0.9, w=0.8, n=24, mat='steel_black',
              holes=8):
    """Pressed steel wheel (inside the tyre): barrel, dished centre with vent holes and 5
    studs; the face toward +axis."""
    hw = w / 2
    prof = [(rim_r * 0.99, -hw * 0.9), (rim_r, -hw * 0.85), (rim_r * 0.93, -hw * 0.6),
            (rim_r * 0.93, hw * 0.6), (rim_r, hw * 0.85), (rim_r * 0.99, hw * 0.9),
            (rim_r * 0.9, hw * 0.88), (rim_r * 0.72, hw * 0.45), (rim_r * 0.36, hw * 0.55),
            (rim_r * 0.3, hw * 0.62), (0.0, hw * 0.62),
            (0.0, -hw * 0.9), (rim_r * 0.99, -hw * 0.9)]
    fs, vs = revolve(bm, prof, mi(mat), n=n)  # closed by a flat back plate: a solid wheel
    for i in range(5):  # wheel studs + nuts
        a = 2 * math.pi * i / 5
        c = V((math.cos(a) * rim_r * 0.2, math.sin(a) * rim_r * 0.2, hw * 0.62))
        fs += geo.cylinder(bm, c, c + V((0, 0, 0.09)), 0.05, mi('steel'), n=6)
        vs += list({v for f in fs[-8:] for v in f.verts})
    for i in range(holes):  # dark vent holes on the dish
        a = 2 * math.pi * (i + 0.5) / holes
        rr = rim_r * 0.55
        c = V((math.cos(a) * rr, math.sin(a) * rr, hw * 0.505))
        hole = geo.cylinder(bm, c - V((0, 0, 0.01)), c + V((0, 0, 0.012)), rim_r * 0.1,
                            mi('rubber'), n=10)
        fs += hole
        vs += list({v for f in hole for v in f.verts})
    place(bm, list(set(vs)), center, axis)
    return fs


def tyre_stack(n, seed=1, name='stack', segs=32):
    """`n` used tyres lying flat, each nudged and spun a little."""
    import random
    rng = random.Random(seed)
    bm = core.new_bm()
    w = C.TYRE['w']
    for i in range(n):
        tyre(bm, (rng.uniform(-0.04, 0.04), rng.uniform(-0.04, 0.04), w * (i + 0.5)),
             spin=rng.uniform(0, 360), n=segs)
    return core.obj_from_bm(name, bm, weighted=False, angle=60)


def extra_shots(name, shots, res=(1280, 800)):
    """After propkit.finish(..., --preview): render custom cameras [(shot, eye, target, lens)]
    into the preview dir as <name>_<shot>.png (the stage and reference car are still there)."""
    from lib import render
    pdir = propkit.preview_dir()
    if not pdir:
        return []
    s = bpy.context.scene
    s.render.resolution_x, s.render.resolution_y = res
    out = []
    for shot, eye, target, lens in shots:
        cam = render._camera(V(eye), V(target), lens)
        cam.data.clip_end = 2000
        p = os.path.join(pdir, f'{name}_{shot}.png')
        s.render.filepath = p
        bpy.ops.render.render(write_still=True)
        bpy.data.objects.remove(cam)
        print(f'[{name}] preview {p}')
        out.append(p)
    return out


def unify_uvs(parts):
    """One UV map named 'UVMap' on every part, so joins keep the textured parts' UVs."""
    for o in parts:
        uvs = o.data.uv_layers
        if not uvs:
            uvs.new(name='UVMap')
        else:
            uvs[0].name = 'UVMap'
    return parts


def finish(name, parts, **kw):
    """propkit.finish with UV maps unified first (see unify_uvs)."""
    return propkit.finish(ENV, name, unify_uvs([p for p in parts if p is not None]), **kw)


def bake_scaled(parts, name):
    """Join scaled / moved parts into one object with an identity transform: modifiers are
    converted first (bevel widths stay relative to the part), then transforms applied."""
    unify_uvs(parts)
    bpy.ops.object.select_all(action='DESELECT')
    for o in parts:
        o.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.convert(target='MESH')
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    if len(parts) > 1:
        bpy.ops.object.join()
    ob = bpy.context.view_layer.objects.active
    ob.name = ob.data.name = name
    ob.select_set(False)
    return ob
