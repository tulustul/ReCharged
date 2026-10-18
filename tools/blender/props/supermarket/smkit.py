"""Supermarket (Aisle Rush) prop kit: palette, material table, the packaging / signage atlases,
product builders and the shelf pieces shared by the prop scripts (art bible §3.5). Prop scripts
put this directory on sys.path and `from smkit import *`; it makes tools/blender/lib importable
as `lib` and catalog.py as `C` (the dimensions and colliders).

Conventions (lib/core.py): 1 Blender unit = 1 u, Z up, front = +Y, origin at the ground-contact
centre; exported +Y up so the front faces Bevy -Z. Textures: tex/packaging.png and
tex/signs.png (run textures.py first), embedded into each glb.
"""
import json
import math
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, '..', '..')))
sys.path.insert(0, HERE)

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import catalog as C  # noqa: E402
from lib import core, geo, propkit  # noqa: E402,F401

V = Vector
ENV = 'supermarket'
KIT, HERO = propkit.KIT_TRIS, propkit.HERO_TRIS
TEX = os.path.join(HERE, 'tex')
ATLAS = json.load(open(os.path.join(TEX, 'atlas.json'), encoding='utf-8'))

# art bible §3.5 palette (sRGB)
VINYL, SHELF_GREY, FREEZER_CYAN = '#D9D6CC', '#8E959C', '#7FD8F0'
PRICE_YELLOW, PACK_RED, PACK_GREEN, BELT_BLACK = '#F4D03F', '#D7352B', '#3E9B4F', '#232427'

# material table: name -> Principled parameters (art bible §1.1 ranges); 'tex' = atlas image,
# 'emit_tex' = the texture also drives the emission
MATS = {
    'shelf_steel': dict(base=SHELF_GREY, rough=0.4),
    'shelf_light': dict(base='#C9CDD1', rough=0.42),
    'steel_dark': dict(base='#3A3E44', rough=0.45),
    'kick_plate': dict(base='#8E959C', metallic=0.2, rough=0.45),
    'pegboard': dict(base='#B8BDC2', rough=0.55),
    'chrome': dict(base='#F2F2F2', metallic=1.0, rough=0.08),
    'brushed': dict(base='#B9BDC1', metallic=1.0, rough=0.35),
    'black_plastic': dict(base='#26272A', rough=0.5),
    'white_plastic': dict(base='#E6E4DE', rough=0.35),
    'red_plastic': dict(base='#C8322A', rough=0.35),
    'blue_plastic': dict(base='#2F5FB0', rough=0.38),
    'yellow_plastic': dict(base='#F2C230', rough=0.35),
    'green_plastic': dict(base='#3E8B4F', rough=0.4),
    'rubber_belt': dict(base=BELT_BLACK, rough=0.7),
    'rubber': dict(base='#2A2B2E', rough=0.88),
    'glass': dict(base='#DDEFF4', rough=0.04, alpha=0.18),
    'glass_frost': dict(base='#E8F4F8', rough=0.25, alpha=0.45),
    'freezer_glow': dict(base='#E4F6FB', rough=0.3, emission=(0.62, 0.9, 1.0), strength=2.2),
    'cooler_glow': dict(base='#EEF6FA', rough=0.3, emission=(0.85, 0.95, 1.0), strength=3.0),
    'freezer_inner': dict(base='#DCE6EA', rough=0.5, emission=(0.8, 0.93, 1.0), strength=1.1),
    'frost': dict(base='#EEF5F8', rough=0.6),
    'cardboard': dict(base='#B58B5A', rough=0.9),
    'cardboard_dark': dict(base='#8E6A42', rough=0.92),
    'tape': dict(base='#C9A879', rough=0.45),
    'pallet_wood': dict(base='#B79466', rough=0.8),
    'pallet_wood_dark': dict(base='#8C6E48', rough=0.85),
    'wrap': dict(base='#F2F6F8', rough=0.1, alpha=0.32),
    'screen': dict(base='#111418', rough=0.2, emission=(0.35, 0.6, 1.0), strength=1.2),
    'lamp_tube': dict(base='#F4F8FA', rough=0.3, emission=(0.92, 0.97, 1.0), strength=8.0),
    'packaging': dict(base='#FFFFFF', rough=0.5, tex='packaging.png'),
    'packaging_gloss': dict(base='#FFFFFF', rough=0.22, tex='packaging.png'),
    'can_label': dict(base='#FFFFFF', rough=0.35, tex='packaging.png'),
    'signs': dict(base='#FFFFFF', rough=0.45, tex='signs.png'),
    'signs_glow': dict(base='#FFFFFF', rough=0.4, tex='signs.png', emit_tex=True, strength=2.0),
    'fruit_orange': dict(base='#E8821E', rough=0.45),
    'fruit_red': dict(base='#B8282A', rough=0.35),
    'fruit_green': dict(base='#7DAA2E', rough=0.4),
    'fruit_yellow': dict(base='#E8C030', rough=0.42),
    'melon': dict(base='#6E8E3A', rough=0.55),
    'leafy': dict(base='#4E8A32', rough=0.7),
    'leafy_dark': dict(base='#33662A', rough=0.75),
}
_IMAGES = {}


# ponytail: every glb embeds its own copy of the atlases (~250 KB, 4 MB VRAM each); share one
# external texture (glb post-processing to an image URI) if memory ever matters.
def image(name):
    if name not in _IMAGES:
        img = bpy.data.images.load(os.path.join(TEX, name))
        img.colorspace_settings.name = 'sRGB'
        _IMAGES[name] = img
    return _IMAGES[name]


def mats(*names):
    """Create the named materials (slot order = argument order)."""
    for name in names:
        m = MATS[name]
        mat = core.material(name, m['base'], m.get('metallic', 0.0), m.get('rough', 0.5),
                            coat=m.get('coat', 0.0), coat_rough=m.get('coat_rough', 0.04),
                            emission=m.get('emission'), strength=m.get('strength', 0.0))
        nt = mat.node_tree
        bsdf = nt.nodes['Principled BSDF']
        if 'tex' in m:
            tex = nt.nodes.new('ShaderNodeTexImage')
            tex.image = image(m['tex'])
            tex.interpolation = 'Linear'
            nt.links.new(tex.outputs['Color'], bsdf.inputs['Base Color'])
            if m.get('emit_tex'):
                nt.links.new(tex.outputs['Color'], bsdf.inputs['Emission Color'])
                bsdf.inputs['Emission Strength'].default_value = m.get('strength', 1.0)
        if 'alpha' in m:
            bsdf.inputs['Alpha'].default_value = m['alpha']
            mat.surface_render_method = 'BLENDED'
            mat.use_backface_culling = False


def start(*names):
    """New scene, store previews, materials."""
    core.reset()
    _IMAGES.clear()
    propkit.stage = store_stage
    mats(*names)


mi = core.mi


# ---------------------------------------------------------------------------------------------
# UV helpers (atlas pixels -> Blender UV; v flipped because image rows run top-down)

def _uvl(bm):
    return bm.loops.layers.uv.verify()


def atlas_uv(sheet, x, y):
    size = ATLAS[sheet]['size']
    return x / size, 1.0 - y / size


def cell(name):
    """Packaging cell rect [x, y, w, h] (px)."""
    return ATLAS['packaging']['cells'][name]['rect']


def side(name):
    return ATLAS['packaging']['cells'][name]['side']


def region(name):
    return ATLAS['signs']['regions'][name]


def map_face(bm, face, rect, sheet='packaging', up=V((0, 0, 1)), inset=0.0, flip_u=False):
    """Planar-map `face` onto the atlas rectangle (px), viewed from outside with `up` as the
    image's up (falls back to +Y for horizontal faces)."""
    uv = _uvl(bm)
    n = face.normal
    u_axis = up if abs(n.dot(up)) < 0.9 else V((0, 1, 0))
    right = u_axis.cross(n).normalized()
    upv = n.cross(right).normalized()
    pts = [lp.vert.co for lp in face.loops]
    a = [p.dot(right) for p in pts]
    b = [p.dot(upv) for p in pts]
    a0, a1, b0, b1 = min(a), max(a), min(b), max(b)
    x, y, w, h = rect
    x, y, w, h = x + inset, y + inset, w - 2 * inset, h - 2 * inset
    for lp, pa, pb in zip(face.loops, a, b):
        s = (pa - a0) / max(a1 - a0, 1e-9)
        t = (pb - b0) / max(b1 - b0, 1e-9)
        if flip_u:
            s = 1 - s
        lp[uv].uv = atlas_uv(sheet, x + s * w, y + (1 - t) * h)


def fill_uv(bm, faces, rect, sheet='packaging'):
    """Point every loop of `faces` at the middle of a (tiny) swatch rect: flat colour."""
    uv = _uvl(bm)
    x, y, w, h = rect
    c = atlas_uv(sheet, x + w / 2, y + h / 2)
    for f in faces:
        for lp in f.loops:
            lp[uv].uv = c


def strip_uv(bm, face, rect, sheet, along, length_u, repeat_u, offset=0.0):
    """Map a long thin face onto a strip region at `repeat_u` u per strip, starting `offset`
    (0-1) into it (no wrap in an atlas: the face spans at most the rest of the strip)."""
    uv = _uvl(bm)
    x, y, w, h = rect
    n = face.normal
    upv = n.cross(along).normalized()
    pts = [lp.vert.co for lp in face.loops]
    a = [p.dot(along) for p in pts]
    b = [p.dot(upv) for p in pts]
    a0, b0, b1 = min(a), min(b), max(b)
    span = min(1.0 - offset, length_u / repeat_u)
    for lp, pa, pb in zip(face.loops, a, b):
        s = offset + (pa - a0) / max(length_u, 1e-9) * span
        t = (pb - b0) / max(b1 - b0, 1e-9)
        lp[uv].uv = atlas_uv(sheet, x + s * w, y + (1 - t) * h)


# ---------------------------------------------------------------------------------------------
# products (all build into a bmesh, origin = bottom centre, front toward `front` (+-Y))

def product_box(bm, c, w, d, h, name, front=1, mat='packaging', bevel=0.0):
    """A facing of boxed products: the front shows the packaging cell, the rest its colour."""
    fs = geo.box(bm, V((c.x, c.y, c.z + h / 2)), (w, d, h), mi(mat))
    fr = V((0, front, 0))
    for f in fs:
        f.normal_update()
        if f.normal.dot(fr) > 0.9:
            map_face(bm, f, cell(name))
        elif f.normal.dot(fr) < -0.9:
            map_face(bm, f, cell(name))
        else:
            fill_uv(bm, [f], side(name))
    return fs


def product_bag(bm, c, w, d, h, name, front=1, mat='packaging_gloss'):
    """Crisps bag: a pillow (front/back bulge, crimped top and bottom)."""
    rows = []
    n = 4
    for i in range(n + 1):
        t = i / n
        z = c.z + h * t
        bulge = math.sin(math.pi * t) ** 0.6
        hw = w / 2 * (0.92 + 0.08 * bulge)
        hd = d / 2 * (0.15 + 0.85 * bulge)
        rows.append([V((c.x - hw, c.y - hd, z)), V((c.x + hw, c.y - hd, z)),
                     V((c.x + hw, c.y + hd, z)), V((c.x - hw, c.y + hd, z))])
    fs = geo._finish(bm, geo._bridge(bm, rows, mi(mat)))
    fr = V((0, front, 0))
    # front faces: map the whole front strip onto the cell by height
    for f in fs:
        f.normal_update()
        fill_uv(bm, [f], side(name))
    uv = _uvl(bm)
    x, y, cw, chh = cell(name)
    for f in fs:
        if f.normal.dot(fr) > 0.3:
            for lp in f.loops:
                p = lp.vert.co
                s = (p.x - (c.x - w / 2)) / w
                if front > 0:   # seen from +Y, image right = -X
                    s = 1 - s
                t = (p.z - c.z) / h
                lp[uv].uv = atlas_uv('packaging', x + s * cw, y + (1 - t) * chh)
    return fs


def product_can(bm, c, r, h, name, n=8, lid='brushed', label='can_label'):
    """Tin can: labelled side (the cell wraps once around), metal lid and rim."""
    uv = _uvl(bm)
    x, y, cw, chh = cell(name)
    rings = []
    for z, rr in ((c.z, r * 0.94), (c.z + h * 0.05, r), (c.z + h * 0.95, r), (c.z + h, r * 0.94)):
        rings.append([V((c.x + rr * math.cos(2 * math.pi * k / n),
                         c.y + rr * math.sin(2 * math.pi * k / n), z)) for k in range(n)])
    vr = [[bm.verts.new(p) for p in ring] for ring in rings]
    fs = []
    for ri in range(3):
        for k in range(n):
            k2 = (k + 1) % n
            f = bm.faces.new((vr[ri][k], vr[ri][k2], vr[ri + 1][k2], vr[ri + 1][k]))
            f.material_index = mi(label if ri == 1 else lid)
            if ri == 1:
                for lp, kk, rr in ((f.loops[0], k, 0), (f.loops[1], k + 1, 0),
                                   (f.loops[2], k + 1, 1), (f.loops[3], k, 1)):
                    lp[uv].uv = atlas_uv('packaging', x + cw * kk / n, y + chh * (1 - rr))
            fs.append(f)
    top = bm.faces.new(vr[3])
    top.material_index = mi(lid)
    fs.append(top)
    return fs


def product_bottle(bm, c, r, h, name, n=8, cap='white_plastic', body='packaging_gloss'):
    """PET bottle: body (label band from the cell), shoulder, neck, cap."""
    uv = _uvl(bm)
    x, y, cw, chh = cell(name)
    prof = [(0.0, r * 0.85), (0.04, r), (0.55, r), (0.72, r * 0.82), (0.84, r * 0.38),
            (0.9, r * 0.36)]
    rings = [[V((c.x + rr * math.cos(2 * math.pi * k / n), c.y + rr * math.sin(2 * math.pi * k / n),
                 c.z + h * zz)) for k in range(n)] for zz, rr in prof]
    vr = [[bm.verts.new(p) for p in ring] for ring in rings]
    fs = []
    for ri in range(len(prof) - 1):
        for k in range(n):
            k2 = (k + 1) % n
            f = bm.faces.new((vr[ri][k], vr[ri][k2], vr[ri + 1][k2], vr[ri + 1][k]))
            f.material_index = mi(body)
            if ri == 1:   # the label band
                for lp, kk, rr in ((f.loops[0], k, 0), (f.loops[1], k + 1, 0),
                                   (f.loops[2], k + 1, 1), (f.loops[3], k, 1)):
                    lp[uv].uv = atlas_uv('packaging', x + cw * kk / n, y + chh * (0.95 - 0.8 * rr))
            else:
                for lp in f.loops:
                    lp[uv].uv = atlas_uv('packaging', *[a + b / 2 for a, b in
                                                        zip(side(name)[:2], side(name)[2:])])
            fs.append(f)
    geo.cylinder(bm, V((c.x, c.y, c.z + h * 0.9)), V((c.x, c.y, c.z + h)), r * 0.42, mi(cap),
                 n=n)
    return fs


def product_carton(bm, c, w, d, h, name, front=1):
    """Gable-top carton (milk, juice): box with a ridge roof, the cell on front and back."""
    hb = h * 0.82
    fs = product_box(bm, c, w, d, hb, name, front=front)
    y0, y1, zb = c.y - d / 2, c.y + d / 2, c.z + hb
    v = [bm.verts.new(p) for p in (V((c.x - w / 2, y0, zb)), V((c.x + w / 2, y0, zb)),
                                   V((c.x + w / 2, y1, zb)), V((c.x - w / 2, y1, zb)),
                                   V((c.x, y0, c.z + h)), V((c.x, y1, c.z + h)))]
    rf = [bm.faces.new((v[0], v[1], v[4])), bm.faces.new((v[2], v[3], v[5])),
          bm.faces.new((v[1], v[2], v[5], v[4])), bm.faces.new((v[3], v[0], v[4], v[5]))]
    for f in rf:
        f.material_index = mi('packaging')
    geo._finish(bm, rf)
    fill_uv(bm, rf, side(name))
    return fs + rf


# ---------------------------------------------------------------------------------------------
# shelf filling

CATEGORIES = {
    # name: (builder, product cells, (w, d, h) ranges)
    'cereal': ('box', ['cereal_%d' % i for i in range(1, 9)], (0.9, 1.0), (1.15, 1.35)),
    'crackers': ('box', ['cracker_1', 'cracker_2', 'cookie_1', 'cookie_2'], (0.8, 1.0),
                 (0.55, 0.75)),
    'pasta': ('box', ['pasta_1', 'pasta_2', 'rice_1', 'flour_1', 'sugar_1', 'baking_1'],
              (0.6, 0.85), (0.8, 1.05)),
    'tea': ('box', ['tea_1', 'tea_2', 'coffee_1', 'sugar_1'], (0.55, 0.75), (0.45, 0.7)),
    'soap': ('box', ['soap_1', 'soap_2', 'soap_3', 'soap_4'], (1.1, 1.4), (0.95, 1.25)),
    'paper': ('box', ['tissue_1', 'nappy_1', 'pet_1', 'pet_2'], (1.0, 1.3), (0.7, 1.2)),
    'chips': ('bag', ['chips_1', 'chips_2', 'chips_3', 'chips_4'], (0.8, 1.0), (1.0, 1.25)),
    'cans': ('can', ['can_tomato', 'can_beans', 'can_corn', 'can_peach', 'can_fish',
                     'can_soup'], (0.33, 0.34), (0.47, 0.5)),
    'drinks': ('bottle', ['soda_cola', 'soda_lemon', 'soda_cherry', 'water', 'juice',
                          'energy'], (0.4, 0.42), (1.3, 1.45)),
    'jars': ('can', ['jar_1', 'jar_2'], (0.3, 0.32), (0.55, 0.6)),
    'milk': ('carton', ['milk_1', 'milk_2', 'milk_3', 'juice_carton'], (0.42, 0.46),
             (1.0, 1.1)),
    'yogurt': ('can', ['yogurt', 'cheese'], (0.3, 0.33), (0.4, 0.45)),
    'cheese': ('box', ['cheese', 'yogurt'], (0.7, 0.9), (0.3, 0.4)),
}


def product_size(name, wr, hr):
    """Every product has one size (seeded by its name), so runs of it line up."""
    r = random.Random(sum(map(ord, name)) * 97)
    return r.uniform(*wr), r.uniform(*hr)


def fill_shelf(bm, x0, x1, y_front, depth, z, max_h, category, rnd, front=1, gaps=0.07):
    """Fill [x0, x1] of a shelf (front edge at y_front, products `depth` deep toward the spine)
    with runs of facings of one category; a few facings are sold out (gaps) or pulled forward.
    Returns the facings count."""
    kind, names, wr, hr = CATEGORIES[category]
    x = x0 + 0.03
    count = 0
    while x < x1 - 0.2:
        name = rnd.choice(names)
        a, h = product_size(name, wr, hr)
        h = min(h, max_h)
        for _ in range(rnd.choice((1, 2, 2, 3, 3, 4))):
            if kind in ('box', 'bag', 'carton'):
                w = a
                if x + w > x1 - 0.03:
                    return count
                if rnd.random() < gaps:          # sold out: an empty slot
                    x += w
                    continue
                pull = rnd.uniform(0.0, 0.1)
                if kind == 'box':
                    yc = y_front - front * (depth / 2 + pull)
                    product_box(bm, V((x + w / 2, yc, z)), w - 0.04, depth, h, name, front=front)
                elif kind == 'carton':
                    yc = y_front - front * (0.2 + pull)
                    product_carton(bm, V((x + w / 2, yc, z)), w - 0.03, 0.4, h, name,
                                   front=front)
                else:
                    for k in range(2):          # two bags deep, the front one forward
                        yc = y_front - front * (0.18 + pull + k * 0.42)
                        product_bag(bm, V((x + w / 2, yc, z)), w - 0.05, 0.3, h - k * 0.05, name,
                                    front=front)
                x += w
            else:
                r = a
                if x + 2 * r > x1 - 0.03:
                    return count
                if rnd.random() < gaps:
                    x += 2 * r + 0.02
                    continue
                stack = max(1, min(3, int(max_h / h))) if kind == 'can' else 1
                yc = y_front - front * (r + 0.02 + rnd.uniform(0.0, 0.06))
                for sk in range(stack):
                    if kind == 'can':
                        product_can(bm, V((x + r, yc, z + sk * h)), r, h, name)
                    else:
                        product_bottle(bm, V((x + r, yc, z)), r, h, name)
                x += 2 * r + 0.02
            count += 1
    return count


def fruit_bed(bm, x0, x1, y0, y1, z, kinds, rnd, r=0.18, slope=0.0, mass_mat=None):
    """A crate's worth of fruit: a coloured 'mass' just under the top plus one layer of low-poly
    fruit (spheres); `slope` tilts the bed toward -Y (rise per u)."""
    mat = mass_mat or kinds[0]
    geo.box(bm, V(((x0 + x1) / 2, (y0 + y1) / 2, z - r * 0.6)), (x1 - x0, y1 - y0, 0.05), mi(mat))
    nx = max(1, int((x1 - x0) / (2 * r)))
    ny = max(1, int((y1 - y0) / (2 * r)))
    for i in range(nx):
        for j in range(ny):
            px = x0 + (x1 - x0) * (i + 0.5) / nx + rnd.uniform(-0.03, 0.03)
            py = y0 + (y1 - y0) * (j + 0.5) / ny + rnd.uniform(-0.03, 0.03)
            pz = z + slope * (py - y1) + rnd.uniform(-0.03, 0.02)
            geo.sphere(bm, V((px, py, pz)), r * rnd.uniform(0.92, 1.05), mi(rnd.choice(kinds)),
                       n=6, rings=4)


def pallet(bm, l, w, PH=C.PALLET_H):
    """EUR pallet: top deck boards, blocks, bottom runners."""
    for k in range(5):
        x = -l / 2 + 0.3 + k * (l - 0.6) / 4
        geo.box(bm, V((x, 0, PH - 0.05)), (0.55 if k % 2 == 0 else 0.4, w, 0.1), mi('pallet_wood'))
    for x in (-l / 2 + 0.45, 0.0, l / 2 - 0.45):
        for y in (-w / 2 + 0.4, 0.0, w / 2 - 0.4):
            geo.box(bm, V((x, y, PH / 2)), (0.55, 0.55, PH - 0.2), mi('pallet_wood_dark'))
        geo.box(bm, V((x, 0, 0.05)), (0.55, w, 0.1), mi('pallet_wood'))


# ---------------------------------------------------------------------------------------------
# store preview stage

def store_stage():
    """Supermarket after closing: vinyl floor, a cool fluorescent overhead area light, a warm
    low 'sunset' key from one side and a dim blue ambient (EEVEE)."""
    s = bpy.context.scene
    s.render.engine = 'BLENDER_EEVEE'
    s.eevee.taa_render_samples = 48
    s.eevee.use_raytracing = True
    s.eevee.use_shadows = True
    s.eevee.use_fast_gi = True
    s.view_settings.view_transform = 'AgX'
    s.view_settings.look = 'AgX - Medium High Contrast'
    s.view_settings.exposure = 0.0
    s.render.image_settings.file_format = 'PNG'
    w = bpy.data.worlds.new('store')
    s.world = w
    bg = w.node_tree.nodes['Background']
    bg.inputs['Color'].default_value = (*core.srgb('#3A4250'), 1)
    bg.inputs['Strength'].default_value = 0.35
    for name, loc, energy, size, color in (
            ('tubes', (0, 0, 22), 9000, 30, (0.86, 0.93, 1.0)),
            ('sunset', (40, 30, 6), 30000, 6, (1.0, 0.62, 0.36))):
        li = bpy.data.lights.new(name, 'AREA')
        li.energy = energy
        li.size = size
        li.color = color
        ob = bpy.data.objects.new(name, li)
        s.collection.objects.link(ob)
        ob.location = loc
        ob.rotation_euler = (V((0, 0, 3)) - V(loc)).to_track_quat('-Z', 'Y').to_euler()
    me = bpy.data.meshes.new('floor')
    bm = bmesh.new()
    bmesh.ops.create_circle(bm, cap_ends=True, radius=400, segments=64)
    bm.to_mesh(me)
    bm.free()
    m = bpy.data.materials.new('stage_floor')
    b = m.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*core.srgb(VINYL), 1)
    b.inputs['Roughness'].default_value = 0.18
    me.materials.append(m)
    f = bpy.data.objects.new('floor', me)
    f.location.z = -0.002
    s.collection.objects.link(f)
    return f


def part(name, bm, bevel=0.0, segments=2, angle=35.0):
    return core.obj_from_bm(name, bm, bevel=bevel, segments=segments, angle=angle)


def smooth(name, bm, angle=60.0):
    return core.obj_from_bm(name, bm, bevel=0.0, angle=angle, weighted=False)


def rng(seed):
    return random.Random(seed)
