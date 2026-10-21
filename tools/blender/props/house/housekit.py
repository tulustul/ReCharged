"""Family-house (Sofa Circuit) prop kit: palette, materials, procedural textures, shapes and an
indoor preview stage (art bible §3.1 + §3.3). Prop scripts put this directory on sys.path and
`from housekit import *`; that also makes tools/blender/lib importable as `lib`.

Conventions (as lib/core.py): 1 Blender unit = 1 world unit (22.5 cm), real size x 4.44, Z up,
front = +Y, origin at the ground-contact centre. Bake / budget / export / previews go through the
shared lib/propkit.finish (models land in assets/models/props/house/).

    blender -b -P tools/blender/props/house/<prop>.py -- [--preview DIR]
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, '..', '..')))

import bmesh  # noqa: E402
import bpy  # noqa: E402
import numpy as np  # noqa: E402
from mathutils import Vector  # noqa: E402

from lib import core, geo, propkit  # noqa: E402,F401

V = Vector
ENV = 'house'
KIT, HERO = propkit.KIT_TRIS, propkit.HERO_TRIS

# ---------------------------------------------------------------------------------------------
# palette (art bible §3.3) + supporting colours, and the material table

OAK_FLOOR, RUG, SAGE, MUSTARD = '#A9784A', '#B5553C', '#7E9278', '#D9A532'
TILE, WALL = '#E6DFCF', '#CFC5B4'

# name: (base sRGB, metallic, roughness, texture kind or None, extra)
MATERIALS = {
    'oak': ('#B98D5E', 0.0, 0.42, 'grain', {}),            # light varnished oak furniture
    'oak_end': ('#9C7449', 0.0, 0.55, None, {}),           # end grain / undersides
    'walnut': ('#6E4B34', 0.0, 0.40, 'grain', {}),
    'plywood': ('#CDAA74', 0.0, 0.70, 'grain', {}),
    'plywood_edge': ('#B48E5A', 0.0, 0.75, None, {}),
    'paint_white': ('#EAE6DD', 0.0, 0.50, None, {}),       # skirting, risers, cabinets
    'paint_navy': ('#34465E', 0.0, 0.50, None, {}),        # kitchen island
    'plaster': (WALL, 0.0, 0.85, None, {}),
    'fabric_sage': (SAGE, 0.0, 0.95, 'fabric', {}),
    'fabric_mustard': (MUSTARD, 0.0, 0.95, 'fabric', {}),
    'fabric_rust': (RUG, 0.0, 0.95, 'fabric', {}),
    'fabric_cream': ('#DCD3C1', 0.0, 0.95, 'fabric', {}),
    'fabric_blue': ('#5A7290', 0.0, 0.95, 'fabric', {}),
    'fabric_grey': ('#8C8A86', 0.0, 0.95, 'fabric', {}),
    'fabric_dark': ('#2F2D2A', 0.0, 0.98, None, {}),        # sofa dust cover (tunnel ceiling)
    'knit_cream': ('#E3DACA', 0.0, 0.98, 'fabric', {}),
    'leather': ('#7A4E33', 0.0, 0.55, None, {}),
    'metal_black': ('#2B2B2D', 0.0, 0.45, None, {}),       # powder-coated steel
    'shade_black': ('#2B2B2D', 0.0, 0.45, None, {'two_sided': True}),  # open lamp shade
    'brass': ('#C8A15A', 1.0, 0.32, None, {}),
    'steel': ('#BFC3C7', 1.0, 0.35, None, {}),
    'ceramic': ('#ECE7DD', 0.0, 0.15, None, {}),
    'terracotta': ('#B8653F', 0.0, 0.75, None, {}),
    'leaf': ('#4E7A3C', 0.0, 0.65, None, {'two_sided': True}),
    'leaf_dark': ('#3B5F31', 0.0, 0.65, None, {'two_sided': True}),
    'soil': ('#3B2C21', 0.0, 0.95, None, {}),
    'pages': ('#EEE7D4', 0.0, 0.85, None, {}),
    'book_red': ('#9A4636', 0.0, 0.80, None, {}),
    'book_blue': ('#3F5B7A', 0.0, 0.80, None, {}),
    'book_green': ('#5D7552', 0.0, 0.80, None, {}),
    'book_cream': ('#D9CCB1', 0.0, 0.80, None, {}),
    'book_mustard': ('#C39737', 0.0, 0.80, None, {}),
    'book_grey': ('#6F6D69', 0.0, 0.80, None, {}),
    'book_black': ('#2F2E2D', 0.0, 0.80, None, {}),
    'book_teal': ('#3E7A78', 0.0, 0.80, None, {}),
    'screen': ('#0D0F13', 0.0, 0.06, None, {}),             # no clearcoat on props (renders washed out)
    'plastic_black': ('#232325', 0.0, 0.45, None, {}),
    'plastic_white': ('#E4E2DC', 0.0, 0.40, None, {}),
    'rubber': ('#262628', 0.0, 0.85, None, {}),
    'worktop': ('#DCD6CB', 0.0, 0.25, None, {}),           # pale quartz
    'tile_wall': (TILE, 0.0, 0.2, 'tile', {}),               # glazed subway tiles (backsplash)
    'lamp_shade': ('#EFE6D2', 0.0, 0.90, 'fabric', {'two_sided': True}),
    'bulb': ('#FFF3DC', 0.0, 0.30, None, {'emission': (1.0, 0.80, 0.55), 'strength': 12.0}),
    'glass': ('#1E2124', 0.0, 0.05, None, {}),
    'mirror': ('#E9ECEE', 1.0, 0.04, None, {}),
}
BOOK_COLOURS = ['book_red', 'book_blue', 'book_green', 'book_cream', 'book_mustard', 'book_grey',
                'book_black', 'book_teal']


def materials(*names):
    """Create the named materials (MATERIALS table) in this slot order. Textured ones share one
    neutral texture per kind per model (grain, fabric, tile), tinted by the base colour through
    a Multiply node (glTF baseColorFactor x baseColorTexture): few, small textures per glb."""
    for n in names:
        base, metal, rough, tex, extra = MATERIALS[n]
        m = core.material(n, base, metal, rough,
                          emission=extra.get('emission'), strength=extra.get('strength', 0.0),
                          two_sided=extra.get('two_sided', False))
        if tex:
            nt = m.node_tree
            t = nt.nodes.new('ShaderNodeTexImage')
            t.image = shared_texture(tex)
            mix = nt.nodes.new('ShaderNodeMix')
            mix.data_type, mix.blend_type = 'RGBA', 'MULTIPLY'
            mix.inputs['Factor'].default_value = 1.0
            mix.inputs[7].default_value = (*core.srgb(base), 1.0)  # colour B
            nt.links.new(t.outputs['Color'], mix.inputs[6])  # colour A
            nt.links.new(mix.outputs[2], nt.nodes['Principled BSDF'].inputs['Base Color'])


def mi(name):
    return core.mi(name)


# ---------------------------------------------------------------------------------------------
# procedural textures: tileable, neutral grey (tinted per material), saved as JPEG.
# Grain and tiles 512 px per 2 u (256 px/u, props near the track), fabric 256 px per 2 u.

TEX_PX, TEX_U = 512, 2.0


def _srgb01(hexc):
    h = hexc.lstrip('#')
    return np.array([int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)])


def _periodic(rng, u, v, fu, fv, amp):
    out = np.zeros_like(u)
    for a in range(0, fu + 1):
        for b in range(0, fv + 1):
            if a == 0 and b == 0:
                continue
            out += rng.normal() * amp / (a + b) * np.sin(2 * np.pi * (a * u + b * v)
                                                         + rng.uniform(0, 2 * np.pi))
    return out


def _image(name, arr):
    """Packed JPEG image from a grey (n, n) or RGB (n, n, 3) array in 0..1 (rows = v). Saved to
    a temporary JPEG first: generated images otherwise pack (and export) as large PNGs."""
    import tempfile
    n = arr.shape[0]
    rgb = np.repeat(arr[..., None], 3, axis=-1) if arr.ndim == 2 else arr
    img = bpy.data.images.new(name, n, n, alpha=False)
    img.pixels.foreach_set(np.concatenate([np.clip(rgb, 0, 1), np.ones((n, n, 1))], axis=-1)
                           .ravel().astype(np.float32))
    path = os.path.join(tempfile.mkdtemp(prefix='housekit_'), f'{name}.jpg')
    img.filepath_raw = path
    img.file_format = 'JPEG'
    img.save(filepath=path, quality=88)
    bpy.data.images.remove(img)
    img = bpy.data.images.load(path)
    img.name = name
    img.pack()
    return img


def shared_texture(kind):
    """The model's one neutral texture of `kind` ('grain', 'fabric', 'tile')."""
    name = f'tex_{kind}'
    img = bpy.data.images.get(name)
    if img is None:
        img = _image(name, {'grain': grain_shade, 'fabric': fabric_shade, 'tile': tile_shade}[kind]())
    return img


def grain_shade(seed=7, px=TEX_PX):
    """Oak-like grain along v (image rows): gently warped growth rings, darker latewood,
    fibres and pores; ~0.85..1."""
    rng = np.random.default_rng(seed)
    t = np.arange(px) / px
    u, v = np.meshgrid(t, t)
    warp = _periodic(rng, u, v, 1, 2, 0.006)  # gentle undulation only: oak is straight-grained
    phase = 26 * (u + warp) + _periodic(rng, u, v, 1, 1, 0.8)
    ring = 0.5 + 0.5 * np.sin(2 * np.pi * phase)
    late = ring ** 6
    streak = np.repeat(rng.normal(size=(1, px)), px, axis=0)  # fibres along v
    streak = 0.5 * (streak + np.roll(streak, 1, axis=1))
    pores = np.repeat(rng.normal(size=(px // 8, px)), 8, axis=0)
    pores = 0.5 * (pores + np.roll(pores, 4, axis=0))
    return (0.95 - 0.075 * late - 0.035 * ring + 0.022 * streak + 0.018 * pores
            + _periodic(rng, u, v, 1, 1, 0.02))


def fabric_shade(seed=3, px=256):
    """Woven upholstery: a plain weave of 0.06 u (above the 0.05 u detail floor) + slubs."""
    rng = np.random.default_rng(seed)
    t = np.arange(px) / px
    u, v = np.meshgrid(t, t)
    weave = np.sin(2 * np.pi * 32 * u) * np.sin(2 * np.pi * 32 * v)
    slub = rng.normal(size=(px, px // 8)).repeat(8, axis=1)
    slub = 0.5 * (slub + np.roll(slub, 4, axis=1))
    return 0.95 + 0.025 * weave + 0.015 * slub + _periodic(rng, u, v, 2, 2, 0.015)


def tile_shade(seed=9, px=TEX_PX):
    """Glazed subway tiles (7.5 x 15 cm = 0.33 x 0.67 u): staggered rows, darker grout, slight
    tone variation per tile. 2 u per texture: 6 rows x 3 tiles."""
    rng = np.random.default_rng(seed)
    t = np.arange(px) / px
    u, v = np.meshgrid(t, t)
    row = np.floor(v * 6).astype(int)
    uu = (u * 3 + 0.5 * (row % 2)) % 3
    col = np.floor(uu).astype(int)
    tone = 0.97 + 0.03 * rng.normal(size=(6, 3))[row % 6, col % 3]
    gu, gv = np.abs(uu % 1.0 - 0.5), np.abs((v * 6) % 1.0 - 0.5)
    grout = (gu > 0.485) | (gv > 0.47)
    edge = np.clip((np.maximum(gu / 0.5, gv / 0.5) - 0.85) * 4, 0, 1) * 0.06  # bevelled edges
    return np.where(grout, 0.7, tone - edge)


def box_uv(bm, axis='x', faces=None):
    """World-space box mapping: the grain (texture v) runs along `axis` on every face that
    contains it; faces facing along `axis` (end grain) get the two other axes."""
    uv = bm.loops.layers.uv.verify()
    ai = 'xyz'.index(axis)
    for f in (faces if faces is not None else bm.faces):
        n = [abs(c) for c in f.normal]
        k = n.index(max(n))  # projection axis
        if k == ai:
            a, b = [i for i in range(3) if i != k]
        else:
            b = ai
            a = [i for i in range(3) if i not in (k, ai)][0]
        for loop in f.loops:
            co = loop.vert.co
            loop[uv].uv = (co[a] / TEX_U, co[b] / TEX_U)
    return bm


# ---------------------------------------------------------------------------------------------
# shapes

def rbox(bm, center, size, r, mat, seg=3, puff=0.0, axis='x', rot=(0, 0, 0)):
    """Box with all twelve edges rounded by r (loft of rounded rectangles along `axis`, ends
    closed by quarter-round shoulders). `puff` domes the two big faces (cushions); `rot` =
    (yaw, pitch, roll) degrees about the centre."""
    c = V(center)
    R = frame_m((0, 0, 0), *rot).to_3x3()
    sx, sy, sz = size
    if axis == 'y':  # loft along y: swap roles of x and y
        sx, sy = sy, sx
    r = min(r, sx / 2 * 0.98, sy / 2 * 0.98, sz / 2 * 0.98)
    hx = sx / 2
    stations = []
    for i in range(seg + 1):  # rounded shoulder at -x
        t = math.pi / 2 * i / seg
        stations.append((-hx + r - r * math.cos(t), r - r * math.sin(t)))
    n_body = 4
    for i in range(1, n_body):
        stations.append((-hx + r + (sx - 2 * r) * i / n_body, 0.0))
    for i in range(seg, -1, -1):
        t = math.pi / 2 * i / seg
        stations.append((hx - r + r * math.cos(t), r - r * math.sin(t)))
    rings = []
    for x, shrink in stations:
        s = 1.0 + puff * max(0.0, 1.0 - (x / hx) ** 2)
        w, h = (sy - 2 * shrink) * (1 + 0.35 * puff * max(0.0, 1 - (x / hx) ** 2)), (sz - 2 * shrink) * s
        rr = max(0.0, min(r - shrink, w / 2 * 0.98, h / 2 * 0.98))
        ring = geo.rrect(max(w, 1e-4), max(h, 1e-4), rr, seg)
        if axis == 'y':
            rings.append([c + R @ V((q.x, x, q.y)) for q in ring])
        else:
            rings.append([c + R @ V((x, q.x, q.y)) for q in ring])
    return geo.loft(bm, rings, mat)


def sweep(bm, path, w, t, mat, r=0.03, side=(0, 1, 0), seg=2):
    """Flat band (w along `side`, thickness t) swept along a 3D polyline: blankets, straps,
    cables. Rounded-rectangle cross-section, capped ends."""
    pts = [V(p) for p in path]
    sd = V(side).normalized()
    rings = []
    prof = geo.rrect(w, t, min(r, t / 2 * 0.95, w / 2 * 0.95), seg)
    for i, p in enumerate(pts):
        tan = (pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]).normalized()
        thick = tan.cross(sd).normalized()
        rings.append([p + sd * q.x + thick * q.y for q in prof])
    return geo.loft(bm, rings, mat)


def leg(bm, base, height, r0, r1, mat, n=12, splay=(0.0, 0.0)):
    """Tapered round leg from the floor point `base` up `height` (r0 at the floor, r1 at the
    top); `splay` offsets the foot (x, y) for mid-century legs."""
    b = V(base)
    top = b + V((0, 0, height))
    foot = b + V((splay[0], splay[1], 0))
    return geo.cylinder(bm, foot, top, r0, mat, n=n, r1=r1)


def frame_m(pos, yaw=0.0, pitch=0.0, roll=0.0):
    """4x4 transform: rotate (roll about y, pitch about x, yaw about z; degrees), then move."""
    from mathutils import Euler, Matrix
    return Matrix.Translation(V(pos)) @ Euler((math.radians(pitch), math.radians(roll),
                                               math.radians(yaw)), 'YXZ').to_matrix().to_4x4()


def obox(bm, M, size, mat, center=(0, 0, 0)):
    """Box of `size` around local `center`, transformed by the 4x4 matrix M."""
    sx, sy, sz = (s / 2 for s in size)
    c = V(center)
    vs = [bm.verts.new(M @ (c + V((x, y, z)))) for z in (-sz, sz) for y in (-sy, sy)
          for x in (-sx, sx)]
    idx = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    return geo._finish(bm, geo.set_mat([bm.faces.new([vs[i] for i in q]) for q in idx], mat))


def flat_book(bm, M, size, cover):
    """Cheap book (one box) at transform M, size (width x, height y, thickness z): covers on
    ±z, spine on -x in `cover`; fore-edge (+x), head and tail (±y) in 'pages'."""
    fs = obox(bm, M, size, mi(cover))
    nz = (M.to_3x3() @ V((0, 0, 1))).normalized()
    nx = (M.to_3x3() @ V((1, 0, 0))).normalized()
    for f in fs:
        if abs(f.normal.dot(nz)) < 0.5 and f.normal.dot(nx) > -0.5:
            f.material_index = mi('pages')
    return fs


def book(bm, M, size, cover, board=0.02):
    """Closed hardcover book at transform M: size = (width x, height y, thickness z) with the
    spine on local -x; two cover boards + spine in `cover`, the page block (inset 0.03) on the
    three open sides in 'pages'."""
    w, h, t = size
    cov, pag = mi(cover), mi('pages')
    for s in (-1, 1):  # covers
        obox(bm, M, (w, h, board), cov, (0, 0, s * (t - board) / 2))
    obox(bm, M, (board, h, t), cov, (-(w - board) / 2, 0, 0))  # spine
    ins = 0.03
    obox(bm, M, (w - board - ins, h - 2 * ins, t - 2 * board + 0.002), pag,
         ((board - ins) / 2, 0, 0))


# ---------------------------------------------------------------------------------------------
# preview stage: oak floor, warm low "window" sun, cool sky fill (indoor afternoon)

def indoor_stage():
    s = bpy.context.scene
    s.render.engine = 'BLENDER_EEVEE'
    s.eevee.taa_render_samples = 48
    s.eevee.use_raytracing = True
    s.eevee.use_shadows = True
    s.eevee.use_fast_gi = True
    s.view_settings.view_transform = 'Khronos PBR Neutral'
    s.view_settings.exposure = -0.2
    s.render.image_settings.file_format = 'PNG'
    w = bpy.data.worlds.new('room')
    s.world = w
    bg = w.node_tree.nodes['Background']
    bg.inputs['Color'].default_value = (*core.srgb('#C9C2B6'), 1)
    bg.inputs['Strength'].default_value = 0.8
    sun = bpy.data.lights.new('window_sun', 'SUN')
    sun.energy = 3.4
    sun.angle = math.radians(4)
    sun.color = (1.0, 0.90, 0.76)
    so = bpy.data.objects.new('window_sun', sun)
    s.collection.objects.link(so)
    so.rotation_euler = V((0.7, 0.55, 0.62)).to_track_quat('Z', 'Y').to_euler()
    me = bpy.data.meshes.new('floor')
    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=300)
    uv = bm.loops.layers.uv.verify()
    for f in bm.faces:
        for loop in f.loops:
            loop[uv].uv = (loop.vert.co.x / 3.0, loop.vert.co.y / 3.0)  # 0.67 u planks
    bm.to_mesh(me)
    bm.free()
    m = bpy.data.materials.new('stage_oak')
    nt = m.node_tree
    b = nt.nodes['Principled BSDF']
    b.inputs['Roughness'].default_value = 0.38
    t = nt.nodes.new('ShaderNodeTexImage')
    t.image = _plank_floor()
    nt.links.new(t.outputs['Color'], b.inputs['Base Color'])
    me.materials.append(m)
    f = bpy.data.objects.new('floor', me)
    f.location.z = -0.002
    s.collection.objects.link(f)
    return f


def _plank_floor():
    """Oak boards 0.67 u wide (stage only): 3 u tile = 4.5 boards, staggered tones, seams."""
    rgb = grain_shade(seed=11)[..., None] * _srgb01(OAK_FLOOR)[None, None, :]
    rng = np.random.default_rng(5)
    t = np.arange(TEX_PX) / TEX_PX
    col = np.floor(t * 4.5).astype(int)
    for b in range(5):
        rgb[:, col == b] *= 1.0 + rng.normal() * 0.05
    rgb[:, np.abs((t * 4.5) % 1.0) < 0.012] *= 0.55
    return _image('stage_planks', rgb)


propkit.stage = indoor_stage  # propkit.preview() renders on the indoor stage


def done(name, parts, budget=KIT, car=(None, None, 0.0), below=0.0):
    """Bake, check, export assets/models/props/house/<name>.glb (+ previews with --preview).
    Parts without a UV map get box-mapped ones first: a textured material on a mesh without
    UVs exports as texCoord -1, which Bevy rejects."""
    for ob in parts:
        if ob is not None and not ob.data.uv_layers:
            bm = bmesh.new()
            bm.from_mesh(ob.data)
            bm.normal_update()
            box_uv(bm, 'x')
            bm.to_mesh(ob.data)
            bm.free()
    return propkit.finish(ENV, name, parts, budget=budget, below=below, car=car)
