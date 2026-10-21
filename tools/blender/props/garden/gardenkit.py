"""Backyard Bash (back garden) prop kit: palette, PBR material table, tileable textures, UV
mapping and plant builders shared by the garden prop scripts (art bible §3.4). Prop scripts put
this directory on sys.path and `from gardenkit import *`; it makes tools/blender/lib importable
as `lib` and reuses lib.propkit (bake / budget / export / previews) and lib.street_props (outdoor
preview stage, material tables).

    blender -b -P tools/blender/props/garden/<prop>.py -- [--preview DIR]
"""
import math
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, '..', '..')))

import bmesh  # noqa: E402
import bpy  # noqa: E402
import numpy as np  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

from lib import core, geo, propkit, street_props  # noqa: E402,F401

V = Vector
ENV = 'garden'
KIT, HERO = propkit.KIT_TRIS, propkit.HERO_TRIS

# art bible §3.4 palette (sRGB)
LAWN, SOIL, TERRACOTTA, PATIO = '#5E9A3A', '#6B4A2E', '#C0663C', '#B9B1A3'
DECK_WOOD, HOSE, PINK, YELLOW = '#8A6142', '#3FA34D', '#E84A6F', '#F5C542'

# material table: name -> Principled parameters (art bible §1.1 ranges); `tex` = texture kind.
# No clearcoat on props for now (orchestrator: it renders washed out in-game) — gloss comes from
# roughness and metallic.
MATS = {
    'deck_wood': dict(base=DECK_WOOD, rough=0.6, tex='wood'),
    'bench_wood': dict(base='#9C7048', rough=0.6, tex='wood'),
    'pine': dict(base='#B48A5A', rough=0.7, tex='wood'),           # sandbox, sleepers, shed
    'pine_dark': dict(base='#6E5238', rough=0.75, tex='wood'),     # stained shed boards
    'plywood': dict(base='#C8A46E', rough=0.7, tex='ply'),
    'terracotta': dict(base=TERRACOTTA, rough=0.75, tex='clay'),
    'terracotta_dark': dict(base='#9E5232', rough=0.8, tex='clay'),
    'soil': dict(base=SOIL, rough=0.95, tex='grit'),
    'sand': dict(base='#CDAE76', rough=0.95, tex='grit'),
    'stone': dict(base='#A8A196', rough=0.85, tex='stone'),
    'concrete': dict(base='#9D9A93', rough=0.9, tex='stone'),
    'brick': dict(base='#9A4E36', rough=0.85, tex='stone'),
    'edging_brick': dict(base='#B86A4F', rough=0.9, tex='stone'),
    'house_brick': dict(base='#A35A40', rough=0.88, tex='bricks'),
    'fence_wood': dict(base='#A57452', rough=0.8, tex='wood'),
    'roof_tile': dict(base='#5A4A44', rough=0.8, tex='stone'),
    'upvc': dict(base='#E9E7E1', rough=0.4),
    'hose_green': dict(base=HOSE, rough=0.45),
    'plastic_black': dict(base='#2A2B2E', rough=0.55),
    'plastic_green': dict(base='#3E7B3A', rough=0.45),
    'plastic_red': dict(base='#C23B2E', rough=0.4),
    'plastic_blue': dict(base='#2F6DB8', rough=0.4),
    'plastic_yellow': dict(base='#E9B92E', rough=0.4),
    'steel': dict(base='#A7ABAF', metallic=1.0, rough=0.4),
    'steel_dark': dict(base='#4A4D51', metallic=0.8, rough=0.5),
    'enamel_black': dict(base='#232427', rough=0.28),
    'rubber': dict(base='#232326', rough=0.88),
    'brass': dict(base='#C9A45A', metallic=1.0, rough=0.35),
    'felt': dict(base='#3B3D40', rough=0.95),
    'leaf': dict(base='#4C8A2E', rough=0.7, two_sided=True),
    'leaf_dark': dict(base='#2F5E2A', rough=0.7, two_sided=True),
    'leaf_light': dict(base='#7AAE45', rough=0.7, two_sided=True),
    'leaf_blue': dict(base='#5E8C6A', rough=0.65, two_sided=True),
    'grass': dict(base='#5E9A3A', rough=0.8, two_sided=True),
    'grass_dry': dict(base='#9A9A4E', rough=0.85, two_sided=True),
    'stem': dict(base='#5B7A32', rough=0.8, two_sided=True),
    'flower_pink': dict(base=PINK, rough=0.6, two_sided=True),
    'flower_yellow': dict(base=YELLOW, rough=0.6, two_sided=True),
    'flower_red': dict(base='#D2382F', rough=0.6, two_sided=True),
    'flower_white': dict(base='#ECE8DE', rough=0.6, two_sided=True),
    'flower_purple': dict(base='#8C78B4', rough=0.7, two_sided=True),
    'bark': dict(base='#5B4838', rough=0.92, tex='grit'),
    'glass': dict(base='#1A222A', rough=0.06),
    'paint_white': dict(base='#E6E3DA', rough=0.5),
    'paint_green': dict(base='#3D5E43', rough=0.5),
    'paint_blue': dict(base='#4F6E8C', rough=0.5),
    'roof_felt': dict(base='#3A3B3C', rough=0.95, tex='grit'),
    'ceramic_red': dict(base='#C8352B', rough=0.22),
    'ceramic_blue': dict(base='#2F5FA8', rough=0.22),
    'ceramic_skin': dict(base='#E9B99A', rough=0.28),
    'ceramic_white': dict(base='#ECE8E0', rough=0.22),
    'leather_brown': dict(base='#6B4428', rough=0.6),
    'lamp_warm': dict(base='#FFF1D8', rough=0.3, emission=(1.0, 0.80, 0.55), strength=12.0),
    'lamp_glass': dict(base='#E8D9B8', rough=0.15, emission=(1.0, 0.78, 0.5), strength=5.0),
    'lamp_window': dict(base='#3A3226', rough=0.1, emission=(1.0, 0.72, 0.42), strength=1.6),
    'football_white': dict(base='#ECEAE4', rough=0.4),
    'football_black': dict(base='#26272A', rough=0.45),
    'tennis_felt': dict(base='#D4E048', rough=0.95),
}


# --- textures ------------------------------------------------------------------------------------

def srgb01(hexc):
    h = hexc.lstrip('#')
    return np.array([int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)])


def field(size, falloff, rng):
    """Tileable gaussian random field (FFT-filtered white noise), normalised to std 1."""
    f = np.fft.fftfreq(size)
    kx, ky = np.meshgrid(f, f)
    k = np.hypot(kx, ky) + 1e-6
    spec = np.fft.fft2(rng.normal(size=(size, size))) * k ** (-falloff)
    spec[0, 0] = 0
    out = np.real(np.fft.ifft2(spec))
    return out / (out.std() + 1e-9)


def _image(name, rgb):
    """Packed albedo image. It goes through a temporary JPEG file so the glTF exporter (AUTO)
    embeds JPEG: noisy albedo compresses ~5x better than PNG (the glbs were 17 MB)."""
    import tempfile
    size = rgb.shape[0]
    img = bpy.data.images.new(name, size, size, alpha=False)
    rgba = np.concatenate([np.clip(rgb, 0, 1), np.ones((size, size, 1))], axis=-1)
    img.pixels.foreach_set(rgba[::-1].ravel().astype(np.float32))  # Blender rows: bottom-up
    tmp = tempfile.mkdtemp(prefix='garden_tex_')
    path = os.path.join(tmp, f'{name}.jpg')
    img.filepath_raw = path
    img.file_format = 'JPEG'
    img.save()
    bpy.data.images.remove(img)
    img = bpy.data.images.load(path)
    img.name = name
    img.pack()  # the packed bytes live in the .blend data now: drop the temporary file
    os.remove(path)
    os.rmdir(tmp)
    return img


def tex_image(name, kind, base, seed=1, size=512):
    """Tileable albedo for a texture kind, tinted with `base` (sRGB hex). 512 px; the UV
    helpers map 2 u per repeat (256 px/u, art bible §3.1)."""
    rng = np.random.default_rng(seed)
    t = np.arange(size) / size
    u, v = np.meshgrid(t, t)  # rows = v
    if kind in ('wood', 'ply'):
        # straight grain along v: growth lines across u, gently warped, plus long pores
        warp = field(size, 2.0, rng) * (0.012 if kind == 'wood' else 0.03)
        stripes = 22 if kind == 'wood' else 6
        phase = stripes * (u + warp)
        ring = 0.5 + 0.5 * np.sin(2 * np.pi * phase)
        late = ring ** 6
        streak = field(size, 1.0, rng)
        streak = np.repeat(streak[:, ::1].mean(axis=0, keepdims=True), size, axis=0) * 0.5 \
            + 0.5 * np.roll(streak, size // 3, axis=0) * 0.15
        pores = np.repeat(rng.normal(size=(size // 64, size)), 64, axis=0)
        pores = 0.5 * (pores + np.roll(pores, 32, axis=0))
        macro = field(size, 2.2, rng)
        shade = 1 - 0.07 * late - 0.03 * ring + 0.035 * streak + 0.02 * pores + 0.04 * macro
    elif kind == 'bricks':  # stretcher bond, 4 bricks x 12 courses per tile (tile = 4 u)
        rows, cols = 12, 4
        row = np.floor(v * rows).astype(int)
        x = u * cols + 0.5 * (row % 2)
        col = np.floor(x).astype(int) % cols
        fu, fv = x - np.floor(x), v * rows - row
        val = rng.normal(size=(rows, cols + 1))[row, col]
        joint = (fu < 0.025) | (fu > 0.975) | (fv < 0.07) | (fv > 0.93)
        shade = np.where(joint, 0.78, 1.0 + 0.09 * val) + 0.03 * field(size, 1.0, rng)
        rgb = shade[..., None] * srgb01(base)
        rgb = np.where(joint[..., None], srgb01('#B5ADA0') * (0.9 + 0.05 * field(size, 1.0, rng))[..., None], rgb)
        return _image(name, rgb)
    elif kind == 'clay':
        shade = 1 + 0.05 * field(size, 1.5, rng) + 0.025 * field(size, 0.6, rng)
        shade -= 0.06 * np.clip(field(size, 2.2, rng) - 1.2, 0, None)  # weathered patches
    elif kind == 'stone':
        shade = 1 + 0.07 * field(size, 1.4, rng) + 0.05 * field(size, 0.4, rng)
    else:  # grit: soil, sand, bark, roofing felt
        shade = 1 + 0.06 * field(size, 1.2, rng) + 0.08 * field(size, 0.15, rng)
    return _image(name, shade[..., None] * srgb01(base))


# --- materials -----------------------------------------------------------------------------------

def material(name, seed=None):
    """Create the named material of MATS (textured kinds get a packed albedo image)."""
    m = MATS[name]
    mat = core.material(name, m['base'], m.get('metallic', 0.0), m.get('rough', 0.5),
                        coat=m.get('coat', 0.0), coat_rough=m.get('coat_rough', 0.04),
                        emission=m.get('emission'), strength=m.get('strength', 0.0),
                        two_sided=m.get('two_sided', False))
    if 'tex' in m:
        nt = mat.node_tree
        node = nt.nodes.new('ShaderNodeTexImage')
        node.image = tex_image(f'{name}_albedo', m['tex'], m['base'],
                               seed=seed if seed is not None else len(core.MATS))
        nt.links.new(node.outputs['Color'], nt.nodes['Principled BSDF'].inputs['Base Color'])
    return mat


def start(*names):
    """New scene, outdoor previews on a lawn, the named materials (slot order = argument order)."""
    core.reset()
    street_props.use_outdoor(ground=LAWN, sun_elev=38.0, sun_az=-40.0)
    for n in names:
        material(n)


mi = core.mi


# --- UV mapping ----------------------------------------------------------------------------------

def box_uv(bm, faces=None, scale=0.5, along=None):
    """World box mapping, 1 / `scale` u per texture repeat. `along` = axis index (0 x, 1 y,
    2 z) the texture's v (wood grain) follows on every face where it lies in the face plane."""
    uv = bm.loops.layers.uv.verify()
    bm.normal_update()
    for f in faces if faces is not None else bm.faces:
        n = f.normal
        ax = max(range(3), key=lambda i: abs(n[i]))
        a, b = [i for i in range(3) if i != ax]
        if along is not None and along == a:
            a, b = b, a
        for lp in f.loops:
            c = lp.vert.co
            lp[uv].uv = (c[a] * scale, c[b] * scale)
    return bm


def board_uv(bm, faces, start, axis, scale=0.5, seed=0):
    """Wood UVs for one board along unit `axis` from `start` (v = grain along the board), with a
    per-board random offset so neighbouring boards differ."""
    uv = bm.loops.layers.uv.verify()
    r = random.Random(seed)
    ou, ov = r.random(), r.random()
    axis = V(axis).normalized()
    s = V(start)
    for f in faces:
        f.normal_update()
        n = f.normal
        side = axis.cross(n)
        if side.length < 1e-3:  # end grain
            side = n.orthogonal()
        side.normalize()
        for lp in f.loops:
            d = lp.vert.co - s
            lp[uv].uv = (d.dot(side) * scale + ou, d.dot(axis) * scale + ov)


def cyl_uv(bm, faces, centre, scale=0.5):
    """Cylindrical mapping around a vertical axis through `centre` (no seam stretching)."""
    uv = bm.loops.layers.uv.verify()
    c = V(centre)
    for f in faces:
        rows = []
        for lp in f.loops:
            d = lp.vert.co - c
            rows.append([math.atan2(d.y, d.x), max(math.hypot(d.x, d.y), 1e-6), lp])
        angles = [a for a, _, _ in rows]
        if max(angles) - min(angles) > math.pi:
            for e in rows:
                if e[0] < 0:
                    e[0] += 2 * math.pi
        for a, r, lp in rows:
            lp[uv].uv = (a * r * scale, lp.vert.co.z * scale)


# --- parts ---------------------------------------------------------------------------------------

def _uv(bm):
    """Every exported mesh needs UVs: a textured material on a mesh without them exports an
    invalid glTF (texCoord -1) that Bevy rejects."""
    if not bm.loops.layers.uv:
        box_uv(bm, scale=0.5)
    return bm


def part(name, bm, bevel=0.0, segments=2, angle=35.0):
    """Mesh object (+ angle-limited bevel and weighted normals, art bible §3.1)."""
    return core.obj_from_bm(name, _uv(bm), bevel=bevel, segments=segments, angle=angle)


def smooth(name, bm, angle=60.0):
    """Round parts (revolves, tubes, foliage): smooth shading, no bevel."""
    return core.obj_from_bm(name, _uv(bm), bevel=0.0, angle=angle, weighted=False)


def board(bm, centre, size, mat, axis=0, seed=0, uv=True):
    """Box board of `size` centred at `centre`; wood grain along `axis` (0 x, 1 y, 2 z)."""
    fs = geo.box(bm, V(centre), size, mat)
    if uv:
        a = [V((1, 0, 0)), V((0, 1, 0)), V((0, 0, 1))][axis]
        board_uv(bm, fs, V(centre) - a * size[axis] / 2, a, seed=seed)
    return fs


def revolve(bm, profile, mat, n=24, centre=(0, 0, 0)):
    """Solid of revolution around the vertical axis: `profile` = [(radius, z), ...] from the
    bottom centre to the top centre (outward-facing when the profile walks up the outside)."""
    c = V(centre)
    rings = []
    for r, z in profile:
        rings.append([c + V((r * math.cos(2 * math.pi * k / n), r * math.sin(2 * math.pi * k / n),
                             z)) for k in range(n)])
    vr = [[bm.verts.new(p) for p in ring] for ring in rings]
    fs = []
    for a, b in zip(vr, vr[1:]):
        for k in range(n):
            j = (k + 1) % n
            fs.append(bm.faces.new((a[k], a[j], b[j], b[k])))
    for f in fs:
        f.material_index = mat
    pole = [v for ring in vr for v in ring if (v.co - c).xy.length < 1e-6]
    if pole:
        bmesh.ops.remove_doubles(bm, verts=pole, dist=1e-6)
    fs = [f for f in fs if f.is_valid]
    bmesh.ops.recalc_face_normals(bm, faces=fs)
    return fs


# --- plants --------------------------------------------------------------------------------------

def leaf(bm, base, direction, length, width, mat, droop=0.25, bend_axis=None, segs=4):
    """A single leaf blade: a tapered, slightly folded strip from `base` along `direction`
    that droops under gravity. Two-sided by construction (a thin closed sliver)."""
    d = V(direction).normalized()
    side = (bend_axis or d.cross(V((0, 0, 1)))).normalized() if abs(d.z) < 0.99 else V((1, 0, 0))
    up = side.cross(d).normalized()
    pts = []
    for k in range(segs + 1):
        t = k / segs
        p = V(base) + d * length * t - V((0, 0, 1)) * droop * length * t * t
        w = width * math.sin(math.pi * min(1.0, 0.15 + 0.85 * t)) * (1 - 0.85 * t ** 3)
        pts.append((p, w))
    top, bot = [], []
    for p, w in pts:
        top.append([bm.verts.new(p - side * w / 2), bm.verts.new(p + up * w * 0.12),
                    bm.verts.new(p + side * w / 2)])
        bot.append(bm.verts.new(p - up * w * 0.05))
    fs = []
    for a, b, ca, cb in zip(top, top[1:], bot, bot[1:]):
        fs.append(bm.faces.new((a[0], b[0], b[1], a[1])))
        fs.append(bm.faces.new((a[1], b[1], b[2], a[2])))
        fs.append(bm.faces.new((a[2], b[2], cb, ca)))
        fs.append(bm.faces.new((ca, cb, b[0], a[0])))
    for f in fs:
        f.material_index = mat
    return fs


def blade(bm, base, height, width, lean, mat, rng):
    """A grass blade: a tapered triangle pair leaning by `lean` (radians), random heading."""
    a = rng.uniform(0, 2 * math.pi)
    d = V((math.cos(a), math.sin(a), 0))
    side = V((-d.y, d.x, 0))
    b = V(base)
    mid = b + d * math.sin(lean) * height * 0.45 + V((0, 0, height * 0.55))
    tip = b + d * math.sin(lean) * height * 1.1 + V((0, 0, height * math.cos(lean)))
    v0, v1 = bm.verts.new(b - side * width / 2), bm.verts.new(b + side * width / 2)
    m0, m1 = bm.verts.new(mid - side * width * 0.32), bm.verts.new(mid + side * width * 0.32)
    t = bm.verts.new(tip)
    fs = [bm.faces.new((v0, v1, m1, m0)), bm.faces.new((m0, m1, t))]
    for f in fs:
        f.material_index = mat
    return fs


def tuft(bm, centre, radius, n, height, mat, rng, width=0.07, lean=0.35):
    """A clump of grass blades scattered in a disc."""
    c = V(centre)
    for _ in range(n):
        r = radius * math.sqrt(rng.random())
        a = rng.uniform(0, 2 * math.pi)
        blade(bm, c + V((r * math.cos(a), r * math.sin(a), 0)),
              height * rng.uniform(0.6, 1.15), width * rng.uniform(0.8, 1.2),
              lean * rng.uniform(0.3, 1.4), mat, rng)


def blossom(bm, centre, radius, mat, centre_mat=None, petals=5, n=None):
    """A flat flower head: petal ring (one material) + a raised centre."""
    c = V(centre)
    fs = []
    k = n or petals * 2
    ring = [c + V((radius * (1.0 if i % 2 == 0 else 0.55) * math.cos(2 * math.pi * i / k),
                   radius * (1.0 if i % 2 == 0 else 0.55) * math.sin(2 * math.pi * i / k), 0))
            for i in range(k)]
    vc = bm.verts.new(c + V((0, 0, radius * 0.08)))
    vr = [bm.verts.new(p) for p in ring]
    for i in range(k):
        f = bm.faces.new((vc, vr[i], vr[(i + 1) % k]))
        f.material_index = mat
        fs.append(f)
    if centre_mat is not None:
        fs += geo.sphere(bm, c + V((0, 0, radius * 0.1)), radius * 0.28, centre_mat, n=6, rings=3,
                         scale=(1, 1, 0.6))
    return fs


def foliage_ball(bm, centre, radius, mat, rng, n=10, detail=1, squash=0.85):
    """A clipped shrub / leafy mass: overlapping lumpy spheres."""
    c = V(centre)
    for k in range(n):
        a = rng.uniform(0, 2 * math.pi)
        e = rng.uniform(-0.3, 1.0)
        off = V((math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e) * squash))
        r = radius * rng.uniform(0.45, 0.7)
        p = c + off * radius * (1 - r / radius) * 0.9
        out = bmesh.ops.create_icosphere(bm, subdivisions=detail, radius=r,
                                         matrix=Matrix.Translation(p))
        for v in out['verts']:
            v.co = p + (v.co - p) * rng.uniform(0.88, 1.1)
        for f in {f for v in out['verts'] for f in v.link_faces}:
            f.material_index = mat


def finish(name, parts, hero=False, **kw):
    """propkit.finish for the garden environment (kit or hero budget)."""
    return propkit.finish(ENV, name, parts, budget=HERO if hero else KIT, **kw)


# --- pots and potted plants ----------------------------------------------------------------------

def pot(bm, centre, height, rim_r, body='terracotta', soil='soil', n=28):
    """Terracotta flower pot: tapered body, rolled rim band, inner wall and a soil disc 0.25 of
    the rim radius below the rim. Returns the soil height (absolute z)."""
    c = V(centre)
    base_r = rim_r * 0.7
    band = height * 0.15
    t = max(0.08, rim_r * 0.09)
    top = c.z + height
    soil_z = top - rim_r * 0.25
    prof = [(0.0, c.z), (base_r - 0.04, c.z), (base_r, c.z + 0.04),
            (rim_r * 0.9, top - band), (rim_r, top - band + 0.02), (rim_r + 0.02, top - 0.03),
            (rim_r - 0.02, top), (rim_r - t, top - 0.01), (rim_r - t - 0.02, soil_z - 0.02),
            (0.0, soil_z - 0.02)]
    fs = revolve(bm, prof, mi(body), n=n, centre=(c.x, c.y, 0))
    cyl_uv(bm, fs, c, scale=0.5)
    dirt = revolve(bm, [(0.0, soil_z), (rim_r - t - 0.01, soil_z), (rim_r - t - 0.01, soil_z - 0.05),
                        (0.0, soil_z - 0.05)][::-1], mi(soil), n=n // 2, centre=(c.x, c.y, 0))
    box_uv(bm, dirt, scale=0.5)
    return soil_z


def round_leaf(bm, centre, normal, radius, mat, rng, cup=0.25, n=9):
    """A roundish, scalloped, slightly cupped leaf (geranium, petunia) facing `normal`."""
    nrm = V(normal).normalized()
    x, y, _ = geo.frame(nrm, V((rng.uniform(-1, 1), rng.uniform(-1, 1), 0.3)))
    c = V(centre)
    vc = bm.verts.new(c - nrm * radius * cup * 0.5)
    ring = []
    for k in range(n):
        a = 2 * math.pi * k / n
        r = radius * (1 - 0.14 * (k % 2))
        ring.append(bm.verts.new(c + (x * math.cos(a) + y * math.sin(a)) * r
                                 + nrm * radius * cup * 0.3))
    fs = []
    for k in range(n):
        f = bm.faces.new((vc, ring[k], ring[(k + 1) % n]))
        f.material_index = mat
        fs.append(f)
    return fs  # materials export double-sided (no backface culling), so one side suffices


def umbel(bm, tip, radius, mat, rng, n=11):
    """A ball-shaped flower head of small five-petal blossoms (geranium, hydrangea)."""
    for _ in range(n):
        a = rng.uniform(0, 2 * math.pi)
        e = rng.uniform(-0.2, 1.3)
        d = V((math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e)))
        p = V(tip) + d * radius
        nrm = d
        x, y, _ = geo.frame(nrm)
        k = 10
        vc = bm.verts.new(p + nrm * radius * 0.05)
        ring = [bm.verts.new(p + (x * math.cos(2 * math.pi * i / k) + y * math.sin(2 * math.pi * i / k))
                             * radius * (0.62 if i % 2 == 0 else 0.4)) for i in range(k)]
        for i in range(k):
            f = bm.faces.new((vc, ring[i], ring[(i + 1) % k]))
            f.material_index = mat


def geranium(bm, centre, radius, rng, flower='flower_red'):
    """A pelargonium mound: a dense dome of scalloped leaves, umbels on stalks above it."""
    c = V(centre)
    S, F = mi('stem'), mi(flower)
    leaves = (mi('leaf'), mi('leaf'), mi('leaf_light'))
    for _ in range(int(52 * radius)):
        a = rng.uniform(0, 2 * math.pi)
        e = rng.uniform(0.05, 1.35)
        d = V((math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e)))
        p = c + V((d.x * radius, d.y * radius, d.z * radius * 0.62)) * rng.uniform(0.82, 1.0)
        round_leaf(bm, p, d + V((0, 0, 0.7)), radius * rng.uniform(0.17, 0.24), rng.choice(leaves),
                   rng)
    for _ in range(max(3, int(4.5 * radius))):
        a = rng.uniform(0, 2 * math.pi)
        r = radius * rng.uniform(0.0, 0.65)
        base = c + V((math.cos(a) * r, math.sin(a) * r, radius * 0.35))
        tip = base + V((rng.uniform(-0.25, 0.25), rng.uniform(-0.25, 0.25),
                        radius * rng.uniform(0.55, 0.8)))
        geo.cylinder(bm, base, tip, 0.035, S, n=4)
        umbel(bm, tip, radius * 0.17, F, rng)


def lavender(bm, centre, radius, height, rng):
    """A lavender bush: a mound of narrow grey-green leaves, a fountain of thin stems with slim
    purple spikes."""
    c = V(centre)
    S, P, LB = mi('stem'), mi('flower_purple'), mi('leaf_blue')
    tuft(bm, c, radius * 0.75, int(150 * radius), height * 0.42, LB, rng, width=0.07, lean=0.55)
    for _ in range(int(70 * radius)):
        a = rng.uniform(0, 2 * math.pi)
        r = radius * math.sqrt(rng.random()) * 0.45
        base = c + V((math.cos(a) * r, math.sin(a) * r, height * 0.15))
        out = V((math.cos(a), math.sin(a), 0)) * rng.uniform(0.25, 0.65) * height * (r / radius + 0.4)
        tip = base + out + V((0, 0, height * rng.uniform(0.7, 0.95)))
        geo.cylinder(bm, base, tip, 0.02, S, n=3)
        d = (tip - base).normalized()
        geo.cylinder(bm, tip - d * 0.03, tip + d * height * 0.22, 0.045, P, n=5, r1=0.02)


def box_ball(bm, centre, radius, rng):
    """A clipped box (Buxus) ball."""
    foliage_ball(bm, V(centre), radius, mi('leaf_dark'), rng, n=14, detail=2, squash=0.95)


def trailing(bm, centre, rim_r, rng, flower='flower_pink'):
    """Petunias spilling over a pot rim: small leaves and trumpet blossoms hanging down."""
    c = V(centre)
    L, F, W = mi('leaf_light'), mi(flower), mi('flower_white')
    for _ in range(int(30 * rim_r)):
        a = rng.uniform(0, 2 * math.pi)
        r = rim_r * rng.uniform(0.3, 1.15)
        drop = max(0.0, r - rim_r) * 1.6
        p = c + V((math.cos(a) * r, math.sin(a) * r, 0.25 - drop))
        round_leaf(bm, p, V((math.cos(a), math.sin(a), 1.2)), 0.18, L, rng, n=5)
    for _ in range(int(14 * rim_r)):
        a = rng.uniform(0, 2 * math.pi)
        r = rim_r * rng.uniform(0.4, 1.1)
        p = c + V((math.cos(a) * r, math.sin(a) * r, 0.35 - max(0.0, r - rim_r) * 1.4))
        blossom(bm, p, 0.22, F if rng.random() > 0.2 else W, mi('flower_yellow'), petals=5)
