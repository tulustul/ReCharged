"""Maple Street (suburban street & front lawns) prop kit: palette, material table and the street
cross-section shared by the props (art bible §3.7). Prop scripts put this directory on sys.path
and `from streetkit import *`; it makes tools/blender/lib importable as `lib`.

Street heights (u, Blender Z = Bevy Y): road 0, gutter paint level 0.025, kerb top 0.65,
lawns 0.6, sidewalks and driveways 0.62. Kerb: 0.6 u deep, vertical face toward the road.
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, '..', '..')))

from mathutils import Matrix, Vector  # noqa: E402

from lib import core, geo, propkit, street_props  # noqa: E402,F401

V = Vector
ENV = 'street'
KIT, HERO = propkit.KIT_TRIS, propkit.HERO_TRIS
if '--draft' in propkit._argv():  # iterate on looks before optimising (no budget check)
    KIT = HERO = 10 ** 7

# street cross-section (u)
KERB_H = 0.65      # kerb top above the road
KERB_D = 0.6       # kerb depth (face to back)
LAWN_H = 0.6       # lawns, planting strips
WALK_H = 0.62      # sidewalks, driveways
GUTTER_W = 1.6     # concrete gutter pan in front of the kerb
GUTTER_H = 0.025   # gutter top above the road (no z-fighting with the road polygon)

# art bible §3.7 palette + neutrals (sRGB)
ASPHALT, PAINT_WHITE, PAINT_YELLOW = '#4A4C50', '#E8E6DF', '#E3B341'
SIDEWALK, LAWN, MAPLE_RED, MAPLE_ORANGE = '#B7B2A8', '#5C8F3C', '#C2452D', '#E07B2E'
TRIM_WHITE = '#E6E3DA'

# material table: name -> Principled parameters (art bible §1.1 ranges). Clearcoat only where
# the art bible asks for it (the parked cars' paint and glass, parked_suv.py).
MATS = {
    'concrete': dict(base=SIDEWALK, rough=0.85),
    'kerb': dict(base='#A9A499', rough=0.8),
    'gutter': dict(base='#9C988F', rough=0.88),
    'asphalt': dict(base=ASPHALT, rough=0.9),
    'paint_white': dict(base=PAINT_WHITE, rough=0.62),
    'paint_yellow': dict(base=PAINT_YELLOW, rough=0.62),
    'iron': dict(base='#3B3734', metallic=0.45, rough=0.62),
    'void': dict(base='#1C1C1E', rough=0.95),
    'rubber': dict(base='#232326', rough=0.88),
    'plastic_black': dict(base='#2A2B2E', rough=0.55),
    'chrome': dict(base='#F2F2F2', metallic=1.0, rough=0.08),
    'steel': dict(base='#A7ABAF', metallic=1.0, rough=0.4),
    'glass': dict(base='#141A22', rough=0.05),
    'lawn': dict(base=LAWN, rough=0.85),
    'hedge': dict(base='#3D6A2C', rough=0.9),
    'hedge_light': dict(base='#46743A', rough=0.9),
    'leaf_red': dict(base=MAPLE_RED, rough=0.75),
    'leaf_orange': dict(base=MAPLE_ORANGE, rough=0.75),
    'leaf_yellow': dict(base='#D7A23A', rough=0.75),
    'leaf_brown': dict(base='#8A5A32', rough=0.8),
    'bark': dict(base='#6E5F52', rough=0.92),
    'soil': dict(base='#5E4532', rough=0.95),
    'mulch': dict(base='#6E4A33', rough=0.95),
    'plywood': dict(base='#C8A46E', rough=0.7),
    'plywood_edge': dict(base='#B08A55', rough=0.75),
    'cinder': dict(base='#8F8C86', rough=0.92),
    'brick': dict(base='#8E4B3A', rough=0.85),
    'stone_wall': dict(base='#9C9284', rough=0.88),
    'trim_white': dict(base=TRIM_WHITE, rough=0.5),
    'paint_red': dict(base='#B3322A', rough=0.35),
    'paint_yellow_gloss': dict(base='#E2B12E', rough=0.35),
    'paint_green': dict(base='#2F5A3A', rough=0.45),
    'paint_black': dict(base='#232427', rough=0.45),
    'pole_grey': dict(base='#6B6F73', metallic=0.6, rough=0.45),
    # bulbs are named lamp*: rc_render dims them by day and adds the catalog `lights` at night
    'lamp_sodium': dict(base='#FFE2B0', rough=0.3, emission=(1.0, 0.70, 0.38), strength=14.0),
    'lamp_warm': dict(base='#FFF1D8', rough=0.3, emission=(1.0, 0.80, 0.55), strength=10.0),
    'lamp_amber': dict(base='#E8A040', rough=0.3, emission=(1.0, 0.45, 0.05), strength=4.0),
    'paper': dict(base='#D9D6CC', rough=0.9),
    'sleeve': dict(base='#3D6FB0', rough=0.25),
    'paint_teal': dict(base='#1E8E8C', rough=0.3),
    'orange_plastic': dict(base='#E8641E', rough=0.45),
    'reflective_white': dict(base='#EDEDE8', rough=0.35),
}


def mats(*names):
    """Create the named materials (slot order = argument order)."""
    street_props.materials({n: MATS[n] for n in names})


def start(*names):
    """New scene, outdoor previews, materials."""
    core.reset()
    street_props.use_outdoor()
    mats(*names)


mi = core.mi


# ---------------------------------------------------------------------------------------------
# shared shapes

def frames_line(length, step=1e9):
    """Face-line samples (x, y, nx, ny) of a straight kerb along X, road toward +Y."""
    n = max(1, math.ceil(length / step))
    return [(-length / 2 + length * i / n, 0.0, 0.0, 1.0) for i in range(n + 1)]


def frames_arc(radius, span_deg, face, step=1.0):
    """Face-line samples of an arc kerb in prop space (Blender: front +Y), origin at the arc
    middle. face 'out': road outside the circle (centre behind, -Y); 'in': road inside."""
    n = max(2, math.ceil(math.radians(span_deg) * radius / step))
    half = math.radians(span_deg) / 2
    out = []
    for i in range(n + 1):
        a = -half + 2 * half * i / n
        if face == 'out':
            out.append((radius * math.sin(a), -radius + radius * math.cos(a), math.sin(a),
                        math.cos(a)))
        else:
            out.append((radius * math.sin(a), radius - radius * math.cos(a), -math.sin(a),
                        math.cos(a)))
    return out


def sweep(bm, frames, profile, mat, closed_profile=True, caps=True):
    """Sweep a 2D profile [(d, z)] (d along the frame normal) through frames
    [(x, y, nx, ny)]; returns the faces (outward normals)."""
    rings = [[V((x + nx * d, y + ny * d, z)) for d, z in profile] for x, y, nx, ny in frames]
    if not closed_profile:
        vr = [[bm.verts.new(p) for p in r] for r in rings]
        fs = []
        for a, b in zip(vr, vr[1:]):
            for j in range(len(profile) - 1):
                fs.append(bm.faces.new((a[j], a[j + 1], b[j + 1], b[j])))
        for f in fs:
            f.material_index = mat
        return fs
    fs = geo._bridge(bm, rings, mat, cap0=caps, cap1=caps)
    return geo._finish(bm, fs)


def stones(frames, length, stone=4.0, gap=0.025, sub=1.0):
    """Split dense face-line frames (spanning `length`) into stone chunks with joint gaps."""
    n = max(1, round(length / stone))
    pts = []
    s = 0.0
    acc = [0.0]
    for a, b in zip(frames, frames[1:]):
        s += math.hypot(b[0] - a[0], b[1] - a[1])
        acc.append(s)
    total = acc[-1]

    def at(t):
        for i in range(len(acc) - 1):
            if acc[i + 1] >= t:
                u = (t - acc[i]) / max(acc[i + 1] - acc[i], 1e-9)
                a, b = frames[i], frames[i + 1]
                nx, ny = a[2] + (b[2] - a[2]) * u, a[3] + (b[3] - a[3]) * u
                ln = math.hypot(nx, ny)
                return (a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, nx / ln, ny / ln)
        return frames[-1]
    for k in range(n):
        t0, t1 = total * k / n + gap / 2, total * (k + 1) / n - gap / 2
        m = max(1, math.ceil((t1 - t0) / sub))
        pts.append([at(t0 + (t1 - t0) * i / m) for i in range(m + 1)])
    return pts


# ---------------------------------------------------------------------------------------------
# full-size bodies (lib/geo decals and lib/shell glass assume RC-car scale: 3 u rays, origins at
# 2 u), adapted for props tens of units across

class BigSurface(geo.Surface):
    """geo.Surface with long rays (works with geo.decal / strip / solid_decal)."""

    def project(self, p, d, offset, back=0.25):
        best = None
        for t in self.trees:
            hit, nrm, _, dist = t.ray_cast(V(p) - V(d) * back, V(d), 60.0)
            if hit is not None and (best is None or dist < best[2]):
                best = (hit, nrm, dist)
        if best is None:
            return None, None
        return best[0] + best[1] * offset, best[1]


def big_glass(bm, surf, prof, windshield=None, side=None, m=0.2, r=0.25, top=20.0, out=20.0):
    """shell.glass for full-size shells: windshield projected from above (between the roof-edge
    lines), side windows from outside (between the belt and the roof edge)."""
    mat = core.mi('glass')
    if windshield:
        ys = [windshield[0] + (windshield[1] - windshield[0]) * i / 10 for i in range(11)]
        right = [V((prof.at(y)['hr'] - m, y)) for y in ys]
        poly = right + [V((-p.x, p.y)) for p in reversed(right)]
        poly = geo.rounded_poly(geo.ccw(poly), r, 2)
        geo.decal(bm, surf, poly, V((0, 0, top)), V((1, 0, 0)), V((0, 1, 0)), mat,
                  offset=0.012, back=0.0, h=0.4)
    if side:
        ys = [side[0] + (side[1] - side[0]) * i / 16 for i in range(17)]
        bot = [V((y, prof.at(y)['zg'] + m * 0.7)) for y in ys]
        top_ = [V((y, prof.at(y)['zt'] - prof.at(y)['rr'] * 0.6 - m * 0.5)) for y in ys]
        ok = [t.y - b.y >= 0.12 for t, b in zip(top_, bot)]
        if True in ok:
            i0 = ok.index(True)
            i1 = len(ok) - ok[::-1].index(True)
            bot, top_ = bot[i0:i1], top_[i0:i1]
            for s in (-1, 1):
                poly = [V((s * p.x, p.y)) for p in bot + top_[::-1]]
                poly = geo.rounded_poly(geo.ccw(poly), r, 2)
                geo.decal(bm, surf, poly, V((s * out, 0, 0)), V((0, s, 0)), V((0, 0, 1)), mat,
                          offset=0.012, back=0.0, h=0.4)


def road_wheel(name, center, radius, width, side, spokes=5, rim_ratio=0.58, n=24):
    """Full-size road-car wheel (spin axis X, rim face toward `side`): tyre with two tread
    grooves and rounded shoulders, alloy rim with spokes, lip and centre cap. Needs materials
    rubber, rim, metal."""
    import bmesh
    R, W = radius, width
    rb = rim_ratio * R
    h = R - rb
    x0, x1 = -W / 2, W / 2
    sh = 0.22 * W
    prof = [(x0 + 0.08 * W, rb), (x0 - 0.03 * W, rb + 0.45 * h), (x0, R - sh),
            (x0 + 0.35 * sh, R - 0.1 * sh), (x0 + sh, R), (-0.17 * W, R), (-0.15 * W, R - 0.04 * R),
            (-0.09 * W, R - 0.04 * R), (-0.07 * W, R), (0.07 * W, R), (0.09 * W, R - 0.04 * R),
            (0.15 * W, R - 0.04 * R), (0.17 * W, R), (x1 - sh, R), (x1 - 0.35 * sh, R - 0.1 * sh),
            (x1, R - sh), (x1 + 0.03 * W, rb + 0.45 * h), (x1 - 0.08 * W, rb)]
    bm_t = bmesh.new()
    geo.revolve_x(bm_t, prof, mi('rubber'), n=n)
    bm_r = bmesh.new()
    xf = x1 - 0.12 * W                     # face plane
    lip = [(xf - 0.05 * W, rb + 0.01), (xf + 0.01, rb + 0.01), (xf + 0.01, rb - 0.1 * rb),
           (xf - 0.05 * W, rb - 0.1 * rb), (xf - 0.05 * W, rb + 0.01)]
    geo.revolve_x(bm_r, lip, mi('rim'), n=n)
    xb = xf - 0.3 * W                      # dark barrel / back plate
    geo.revolve_x(bm_r, [(x0 + 0.1 * W, 0.0), (x0 + 0.1 * W, rb * 0.95), (xb, rb * 0.95),
                         (xb, 0.0)], mi('metal'), n=n)
    hub = 0.28 * rb
    for i in range(spokes):
        a = 2 * math.pi * i / spokes + math.pi / 2
        p0 = V((xf - 0.02 * W, hub * 0.8 * math.cos(a), hub * 0.8 * math.sin(a)))
        p1 = V((xf - 0.06 * W, rb * 0.92 * math.cos(a), rb * 0.92 * math.sin(a)))
        sw = 0.26 * rb
        geo.prism(bm_r, [(-sw / 2, -0.12 * W), (sw / 2, -0.12 * W), (sw * 0.35, 0.0),
                         (-sw * 0.35, 0.0)], p0, p1, mi('rim'), up=V((1, 0, 0)), scale1=0.7)
    geo.cylinder(bm_r, V((xb, 0, 0)), V((xf + 0.02 * W, 0, 0)), hub, mi('rim'), n=12, r1=hub * 0.9)
    geo.cylinder(bm_r, V((xf + 0.02 * W, 0, 0)), V((xf + 0.045 * W, 0, 0)), hub * 0.55,
                 mi('metal'), n=12)
    objs = []
    for bm, weighted in ((bm_t, False), (bm_r, True)):
        if side < 0:
            bmesh.ops.scale(bm, vec=(-1, 1, 1), verts=bm.verts)
            bmesh.ops.reverse_faces(bm, faces=bm.faces)
        bmesh.ops.translate(bm, vec=V(center), verts=bm.verts)
        objs.append(core.obj_from_bm(name + '_part', bm, angle=35, weighted=weighted))
    return core.bake(objs, name, location=center)


# ---------------------------------------------------------------------------------------------
# block lettering for signs: strokes on a 1 x 2 grid drawn as rounded bars (no font files)
_LETTERS = {
    'A': [((0, 0), (0, 2)), ((0, 2), (1, 2)), ((1, 2), (1, 0)), ((0, 1), (1, 1))],
    'B': [((0, 0), (0, 2)), ((0, 2), (0.8, 2)), ((0.8, 2), (0.8, 1)), ((0, 1), (1, 1)),
          ((1, 1), (1, 0)), ((1, 0), (0, 0))],
    'C': [((1, 2), (0, 2)), ((0, 2), (0, 0)), ((0, 0), (1, 0))],
    'E': [((1, 2), (0, 2)), ((0, 2), (0, 0)), ((0, 0), (1, 0)), ((0, 1), (0.75, 1))],
    'H': [((0, 0), (0, 2)), ((1, 0), (1, 2)), ((0, 1), (1, 1))],
    'I': [((0.5, 0), (0.5, 2)), ((0.15, 2), (0.85, 2)), ((0.15, 0), (0.85, 0))],
    'L': [((0, 2), (0, 0)), ((0, 0), (1, 0))],
    'M': [((0, 0), (0, 2)), ((0, 2), (0.5, 1.1)), ((0.5, 1.1), (1, 2)), ((1, 2), (1, 0))],
    'N': [((0, 0), (0, 2)), ((0, 2), (1, 0)), ((1, 0), (1, 2))],
    'O': [((0, 0), (1, 0)), ((1, 0), (1, 2)), ((1, 2), (0, 2)), ((0, 2), (0, 0))],
    'P': [((0, 0), (0, 2)), ((0, 2), (1, 2)), ((1, 2), (1, 1)), ((1, 1), (0, 1))],
    'R': [((0, 0), (0, 2)), ((0, 2), (1, 2)), ((1, 2), (1, 1)), ((1, 1), (0, 1)),
          ((0.35, 1), (1, 0))],
    'S': [((1, 2), (0, 2)), ((0, 2), (0, 1)), ((0, 1), (1, 1)), ((1, 1), (1, 0)), ((1, 0), (0, 0))],
    'T': [((0, 2), (1, 2)), ((0.5, 2), (0.5, 0))],
}


def text_bars(text, h=1.0, t=0.2, gap=0.3):
    """Convex 2D bars (centred) spelling `text` in block capitals of height h."""
    s = h / 2.0
    tt = t * h
    adv = s + gap * h
    x0 = -(len(text) * adv - gap * h) / 2
    bars = []
    for ci, ch in enumerate(text):
        ox = x0 + ci * adv
        for (ax, ay), (bx, by) in _LETTERS.get(ch, []):
            a = V((ox + ax * (s - tt) + tt / 2, ay * (h - tt) / 2 + tt / 2 - h / 2))
            b = V((ox + bx * (s - tt) + tt / 2, by * (h - tt) / 2 + tt / 2 - h / 2))
            d = b - a
            ang = math.atan2(d.y, d.x)
            rot = Matrix.Rotation(ang, 2)
            bars.append([(a + b) / 2 + rot @ p for p in geo.rrect(d.length + tt, tt, tt * 0.2, 1)])
    return bars


def raised_text(bm, text, origin, u, v, h, mat, depth=0.02, t=0.2):
    """Text bars as thin prisms on the plane origin + x*u + y*v, raised along u x v (pick u so
    that u x v is the face's outward normal: then the text reads left to right from outside)."""
    origin, u, v = V(origin), V(u), V(v)
    n = u.cross(v).normalized()
    for bar in text_bars(text, h, t):
        pts = [origin + u * p.x + v * p.y for p in bar]
        bot = [bm.verts.new(p) for p in pts]
        top = [bm.verts.new(p + n * depth) for p in pts]
        fs = [bm.faces.new(top), bm.faces.new(bot[::-1])]
        for i in range(len(pts)):
            j = (i + 1) % len(pts)
            fs.append(bm.faces.new((bot[i], bot[j], top[j], top[i])))
        geo._finish(bm, geo.set_mat(fs, mat))
