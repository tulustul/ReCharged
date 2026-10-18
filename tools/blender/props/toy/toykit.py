"""Toy Town (toy store / playroom) prop kit: palette, materials and shared shapes (art bible
§3.2). Prop scripts do `from toykit import *` after putting this directory on sys.path; it also
makes tools/blender/lib importable as `lib`.
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, '..', '..')))

from mathutils import Matrix, Vector  # noqa: E402,F401 (re-exported to the prop scripts)

from lib import core, geo, propkit  # noqa: E402,F401

V = Vector
ENV = 'toy'

# art bible §3.2 palette (+ toy accents for small areas)
TEAL, ORANGE, RED, BLUE = '#2E9C9A', '#F07A1F', '#D8392B', '#2F6FD6'
YELLOW, WALL, BEECH = '#F2C230', '#EDE6DA', '#C99A63'
GREEN, PURPLE, PINK, CREAM = '#3E9B4F', '#7A4FC2', '#E8739E', '#F1E6CF'


def srgb01(hexc):
    h = hexc.lstrip('#')
    return [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]


def grain_image(name, base, size=512, contrast=0.11, stripes=9, seed=1):
    """Tileable wood-grain albedo tinted with `base` (sRGB hex): growth stripes along v with a
    gentle periodic warp, plus fine pores stretched along v. 512 px = 2 u at 256 px/u."""
    import bpy
    import numpy as np
    rng = np.random.default_rng(seed)
    t = np.arange(size) / size
    u, v = np.meshgrid(t, t)  # rows = v

    def periodic(fu, fv, amp):
        out = np.zeros_like(u)
        for a in range(1, fu + 1):
            for b in range(0, fv + 1):
                out += rng.normal() * amp / (a + b) * np.sin(2 * np.pi * (a * u + b * v)
                                                             + rng.uniform(0, 2 * np.pi))
        return out

    warp = periodic(3, 2, 0.05)
    phase = stripes * u + warp * stripes + periodic(2, 1, 0.3)
    ring = 0.5 + 0.5 * np.sin(2 * np.pi * phase)
    late = np.power(ring, 6)  # thin darker latewood lines
    pores = rng.normal(size=(size // 16, size))
    pores = np.repeat(pores, 16, axis=0)
    pores = (pores + np.roll(pores, 8, axis=0)) * 0.5
    shade = 1.0 - contrast * (0.55 * late + 0.25 * ring) + 0.02 * pores + periodic(1, 1, 0.03)
    rgb = np.clip(np.stack([shade * c for c in srgb01(base)], axis=-1), 0, 1)
    img = bpy.data.images.new(name, size, size, alpha=False)
    img.pixels.foreach_set(np.concatenate([rgb, np.ones((size, size, 1))], axis=-1).ravel()
                           .astype(np.float32))
    img.pack()
    return img


def blocks_image(name, base, size=512, rows=10, cols=5, mortar=0.05, jitter=0.07, seed=2):
    """Tileable staggered stone/brick blocks tinted with `base`, darker mortar lines and a
    per-block value jitter."""
    import bpy
    import numpy as np
    rng = np.random.default_rng(seed)
    t = (np.arange(size) + 0.5) / size
    u, v = np.meshgrid(t, t)
    row = np.floor(v * rows).astype(int)
    x = u * cols + 0.5 * (row % 2)
    col = np.floor(x).astype(int) % cols
    fu, fv = x - np.floor(x), v * rows - row
    val = rng.normal(size=(rows, cols))[row, col]
    joint = (fu < mortar) | (fu > 1 - mortar) | (fv < mortar * cols / rows * 2) | \
            (fv > 1 - mortar * cols / rows * 2)
    shade = np.where(joint, 0.8, 1.0 + jitter * val)
    shade *= 1.0 + 0.025 * rng.normal(size=(size, size))
    rgb = np.clip(np.stack([shade * c for c in srgb01(base)], axis=-1), 0, 1)
    img = bpy.data.images.new(name, size, size, alpha=False)
    img.pixels.foreach_set(np.concatenate([rgb, np.ones((size, size, 1))], axis=-1).ravel()
                           .astype(np.float32))
    img.pack()
    return img


def dots_image(name, base, hole='#3A3A3E', size=512, per_tile=8, radius=0.13):
    """Tileable pegboard: round holes on a square grid (per_tile holes per texture edge)."""
    import bpy
    import numpy as np
    t = (np.arange(size) + 0.5) / size * per_tile
    u, v = np.meshgrid(t, t)
    d = np.hypot(u - np.floor(u) - 0.5, v - np.floor(v) - 0.5)
    m = np.clip((d - radius) / 0.04 + 0.5, 0, 1)[..., None]  # 1 = board, 0 = hole (antialiased)
    rgb = np.array(srgb01(hole)) * (1 - m) + np.array(srgb01(base)) * m
    img = bpy.data.images.new(name, size, size, alpha=False)
    img.pixels.foreach_set(np.concatenate([rgb, np.ones((size, size, 1))], axis=-1).ravel()
                           .astype(np.float32))
    img.pack()
    return img


def cyl_uv(bm, faces, center, tiles_per_u=0.25):
    """Cylindrical mapping around a vertical axis through `center` (u = arc length, v = z),
    without a stretched face at the seam."""
    uv = bm.loops.layers.uv.verify()
    c = V(center)
    for f in faces:
        rows = []
        for lp in f.loops:
            d = lp.vert.co - c
            r = max(math.hypot(d.x, d.y), 1e-6)
            rows.append([math.atan2(d.y, d.x), r, lp])
        angles = [a for a, _, _ in rows]
        if max(angles) - min(angles) > math.pi:
            for e in rows:
                if e[0] < 0:
                    e[0] += 2 * math.pi
        for a, r, lp in rows:
            lp[uv].uv = (a * r * tiles_per_u, lp.vert.co.z * tiles_per_u)
    return bm


def textured(name, base, rough, image):
    """Principled material with `image` as base colour (glTF baseColorTexture)."""
    m = core.material(name, base, 0.0, rough)
    nt = m.node_tree
    tex = nt.nodes.new('ShaderNodeTexImage')
    tex.image = image
    nt.links.new(tex.outputs['Color'], nt.nodes['Principled BSDF'].inputs['Base Color'])
    return m


def wood(name='beech', base=BEECH, rough=0.45, contrast=0.11, seed=1):
    """Wood with grain (natural or stained: the tint is baked into the texture)."""
    return textured(name, base, rough, grain_image(name, base, contrast=contrast, seed=seed))


def box_uv(bm, tiles_per_u=0.5):
    """World-space box mapping (grain = v): vertical on side faces, along y on top faces."""
    uv = bm.loops.layers.uv.verify()
    bm.normal_update()
    for f in bm.faces:
        n = f.normal
        ax = max(range(3), key=lambda i: abs(n[i]))
        for lp in f.loops:
            c = lp.vert.co
            a, b = ((c.x, c.y), (c.y, c.z), (c.x, c.z))[(2, 0, 1).index(ax)]
            lp[uv].uv = (a * tiles_per_u, b * tiles_per_u)
    return bm


def plastic(name, color, rough=0.22):
    """Glossy toy plastic (art bible §1.1: rough 0.15-0.30)."""
    return core.material(name, color, 0.0, rough)


def painted(name, color, rough=0.4):
    """Lacquered / painted wood."""
    return core.material(name, color, 0.0, rough)


def beech(name='beech', rough=0.45):
    return wood(name, BEECH, rough)


def fabric(name, color, rough=0.95):
    return core.material(name, color, 0.0, rough)


def cardboard(name='cardboard', color='#C4A27A', rough=0.9):
    return core.material(name, color, 0.0, rough)


def part(name, bm, bevel=0.02, segments=2, angle=35.0):
    """Mesh object from a bmesh with the art-bible bevel (+ weighted normals)."""
    return core.obj_from_bm(name, bm, bevel=bevel, segments=segments, angle=angle)


def bevel_bm(bm, offset, segments=2, angle=35.0, mat=None):
    """Bevel every edge sharper than `angle` inside the bmesh (exact, unlike the modifier after
    insets). New faces take material `mat` (default: slot 0). Returns the bmesh."""
    import bmesh
    bm.normal_update()
    edges = [e for e in bm.edges if len(e.link_faces) == 2
             and e.calc_face_angle(0.0) > math.radians(angle)]
    bmesh.ops.bevel(bm, geom=edges, offset=offset, segments=segments, profile=0.5,
                    affect='EDGES', clamp_overlap=True, material=-1 if mat is None else mat)
    return bm


def smooth(name, bm, angle=60.0):
    """Round parts (revolves, tubes, spheres): smooth shading, no bevel."""
    return core.obj_from_bm(name, bm, bevel=0.0, angle=angle, weighted=False)


def rpoly(pts, radii, seg=4):
    """geo.rounded_poly without the duplicate points it emits at zero-radius corners (they break
    the triangulation of concave caps)."""
    out = []
    for p in geo.rounded_poly(pts, radii, seg):
        if not out or (p - out[-1]).length > 1e-6:
            out.append(p)
    if (out[0] - out[-1]).length <= 1e-6:
        out.pop()
    return out


def extrude(bm, axis, poly2d, a0, a1, mat):
    """Prism of a (possibly concave) 2D outline between a0 and a1 along axis 'x' (outline in
    (y, z)), 'y' (outline in (x, z)) or 'z' (outline in (x, y)). Caps are tessellated with
    mathutils (robust for concave outlines, unlike BEAUTY triangulation). Returns the faces."""
    from mathutils.geometry import tessellate_polygon
    pts = [V(p) for p in poly2d]

    def place(p, a):
        return {'x': V((a, p.x, p.y)), 'y': V((p.x, a, p.y)), 'z': V((p.x, p.y, a))}[axis]

    r0 = [bm.verts.new(place(p, a0)) for p in pts]
    r1 = [bm.verts.new(place(p, a1)) for p in pts]
    n = len(pts)
    fs = [bm.faces.new((r0[j], r0[(j + 1) % n], r1[(j + 1) % n], r1[j])) for j in range(n)]
    for tri in tessellate_polygon([[V((p.x, p.y, 0)) for p in pts]]):
        fs.append(bm.faces.new([r0[k] for k in tri]))
        fs.append(bm.faces.new([r1[k] for k in tri]))
    for f in fs:
        f.material_index = mat
    import bmesh
    bmesh.ops.recalc_face_normals(bm, faces=fs)
    return fs


# ---------------------------------------------------------------------------------------------
# 2D outlines for raised symbols (centred, unit size: fits a 1 x 1 box)

def star2(n=5, r_out=0.5, r_in=0.22, rot=90.0):
    return [V((math.cos(math.radians(rot + 180 * i / n)) * (r_out if i % 2 == 0 else r_in),
               math.sin(math.radians(rot + 180 * i / n)) * (r_out if i % 2 == 0 else r_in)))
            for i in range(2 * n)]


def heart2(k=28, s=0.5):
    pts = []
    for i in range(k):
        t = 2 * math.pi * i / k
        x = 16 * math.sin(t) ** 3
        y = 13 * math.cos(t) - 5 * math.cos(2 * t) - 2 * math.cos(3 * t) - math.cos(4 * t)
        pts.append(V((x / 17 * s, (y + 2.5) / 17 * s)))
    return geo.ccw(pts)


def relief(bm, poly2d, center, normal, up, size, height, mat):
    """Extrude a 2D outline (unit size, CCW) as raised relief on a face: centre, outward normal,
    in-plane `up` direction, edge size and height above the face."""
    n, u = V(normal).normalized(), V(up).normalized()
    r = u.cross(n)  # in-plane x (right when looking at the face)
    c = V(center)
    pts = [V((p.x * size, p.y * size)) for p in poly2d]
    # prism() maps 2D (x, y) to x * (ax x up) + y * (up'): build the rings by hand instead
    rings = [[c + n * h + r * p.x + u * p.y for p in pts] for h in (-0.004, height)]
    return geo.loft(bm, rings, mat)


def digit_bars(text, h):
    """Rounded-bar outlines (CCW) of a blocky number of height h, centred (no slant)."""
    return [geo.ccw(b) for b in geo.number_bars(text, h, t=0.2, slant=0.0)]
