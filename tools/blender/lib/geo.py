"""bmesh primitives. Every solid primitive appends closed, outward-facing geometry to `bm` with
one material index and returns its faces. Coordinates are Blender world space (Z up, +Y front).
"""
import math

import bmesh
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

V = Vector


# ---------------------------------------------------------------------------------------------
# helpers

def _bridge(bm, rings, mat, cap0=True, cap1=True):
    """Bridge equal-length vertex rings (lists of Vector) into quads; optional n-gon end caps."""
    vr = [[bm.verts.new(p) for p in r] for r in rings]
    n = len(rings[0])
    out = []
    for a, b in zip(vr, vr[1:]):
        for j in range(n):
            k = (j + 1) % n
            out.append(bm.faces.new((a[j], a[k], b[k], b[j])))
    if cap0:
        out.append(bm.faces.new(vr[0]))
    if cap1:
        out.append(bm.faces.new(vr[-1]))
    for f in out:
        f.material_index = mat
    return out


def _finish(bm, fs, orient=True):
    """Triangulate n-gons (> 4 sides), make normals of the closed island point outward."""
    ng = [f for f in fs if len(f.verts) > 4]
    keep = [f for f in fs if len(f.verts) <= 4]
    if ng:
        keep += bmesh.ops.triangulate(bm, faces=ng, quad_method='BEAUTY',
                                      ngon_method='BEAUTY')['faces']
    if orient:
        bmesh.ops.recalc_face_normals(bm, faces=keep)
    return keep


def frame(z_axis, x_hint=V((1, 0, 0))):
    """Orthonormal basis (x, y, z) with z along `z_axis`."""
    z = V(z_axis).normalized()
    x = V(x_hint)
    if abs(x.dot(z)) > 0.95:
        x = V((0, 1, 0)) if abs(z.y) < 0.95 else V((0, 0, 1))
    x = (x - z * x.dot(z)).normalized()
    return x, z.cross(x), z


def set_mat(faces, mat):
    for f in faces:
        f.material_index = mat
    return faces


# ---------------------------------------------------------------------------------------------
# 2D shapes

def rounded_poly(pts, radii, seg=4, closed=True):
    """Fillet the corners of a 2D polyline. Every corner yields seg+1 points (fixed count, even
    for radius 0) so rings built from it stay loft-compatible. `seg` may be a per-corner list."""
    pts = [V(p) for p in pts]
    n = len(pts)
    out = []
    segs = seg
    for i in range(n):
        p1 = pts[i]
        seg = segs[i] if hasattr(segs, '__len__') else segs
        if not closed and i in (0, n - 1):
            out.append(p1.copy())
            continue
        p0, p2 = pts[i - 1], pts[(i + 1) % n]
        r = radii[i] if hasattr(radii, '__len__') else radii
        a, b = p0 - p1, p2 - p1
        la, lb = a.length, b.length
        if r <= 1e-6 or la < 1e-7 or lb < 1e-7:
            out += [p1.copy() for _ in range(seg + 1)]
            continue
        a.normalize()
        b.normalize()
        if a.dot(b) < -0.99995 or a.dot(b) > 0.99995:  # straight or spike: no fillet
            out += [p1.copy() for _ in range(seg + 1)]
            continue
        ang = math.acos(min(1.0, a.dot(b)))
        d = min(r / math.tan(ang / 2), la * 0.5, lb * 0.5)
        r_eff = d * math.tan(ang / 2)
        t0, t1 = p1 + a * d, p1 + b * d
        c = p1 + (a + b).normalized() * (r_eff / math.sin(ang / 2))
        v0, v1 = t0 - c, t1 - c
        a0, a1 = math.atan2(v0.y, v0.x), math.atan2(v1.y, v1.x)
        da = (a1 - a0 + math.pi) % (2 * math.pi) - math.pi
        for s in range(seg + 1):
            t = a0 + da * s / seg
            out.append(c + V((math.cos(t), math.sin(t))) * r_eff)
    return out


def rrect(w, h, r, seg=4, cx=0.0, cy=0.0):
    """Rounded rectangle centred at (cx, cy), CCW."""
    x, y = w / 2, h / 2
    return [p + V((cx, cy)) for p in rounded_poly([(x, -y), (x, y), (-x, y), (-x, -y)], r, seg)]


def circle2(r, n=24, cx=0.0, cy=0.0):
    return [V((cx + r * math.cos(2 * math.pi * i / n), cy + r * math.sin(2 * math.pi * i / n)))
            for i in range(n)]


def inset2(poly, d):
    """Inset a convex CCW polygon by d (edge offset)."""
    n = len(poly)
    lines = []
    for i in range(n):
        a, b = V(poly[i]), V(poly[(i + 1) % n])
        t = (b - a).normalized()
        nrm = V((-t.y, t.x))  # left normal = inward for CCW
        lines.append((a + nrm * d, t))
    out = []
    for i in range(n):
        (p, t), (q, s) = lines[i - 1], lines[i]
        den = t.x * s.y - t.y * s.x
        if abs(den) < 1e-9:
            out.append(q.copy())
            continue
        u = ((q.x - p.x) * s.y - (q.y - p.y) * s.x) / den
        out.append(p + t * u)
    return out


def ccw(poly):
    area = sum(a.x * b.y - b.x * a.y for a, b in zip(poly, poly[1:] + poly[:1]))
    return poly if area > 0 else poly[::-1]


# ---------------------------------------------------------------------------------------------
# 1D profiles

class Curve:
    """Monotone cubic (PCHIP) interpolation through (x, y) keys; flat beyond the ends."""

    def __init__(self, keys):
        k = sorted(keys)
        xs, ys = [a for a, _ in k], [b for _, b in k]
        n = len(xs)
        d = [(ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]) for i in range(n - 1)]
        m = [0.0] * n
        for i in range(1, n - 1):
            if d[i - 1] * d[i] > 0:
                w1 = 2 * (xs[i + 1] - xs[i]) + (xs[i] - xs[i - 1])
                w2 = (xs[i + 1] - xs[i]) + 2 * (xs[i] - xs[i - 1])
                m[i] = (w1 + w2) / (w1 / d[i - 1] + w2 / d[i])
        if n == 2:
            m = [d[0], d[0]]
        self.xs, self.ys, self.m = xs, ys, m

    def __call__(self, x):
        xs, ys, m = self.xs, self.ys, self.m
        if x <= xs[0]:
            return ys[0]
        if x >= xs[-1]:
            return ys[-1]
        i = 0
        while xs[i + 1] < x:
            i += 1
        h = xs[i + 1] - xs[i]
        t = (x - xs[i]) / h
        return ((2 * t ** 3 - 3 * t ** 2 + 1) * ys[i] + (t ** 3 - 2 * t ** 2 + t) * h * m[i]
                + (-2 * t ** 3 + 3 * t ** 2) * ys[i + 1] + (t ** 3 - t ** 2) * h * m[i + 1])


# ---------------------------------------------------------------------------------------------
# solids

def box(bm, center, size, mat):
    """Axis-aligned box centred at `center`."""
    sx, sy, sz = (s / 2 for s in size)
    m = Matrix.Translation(center)
    vs = [bm.verts.new(m @ V((x, y, z))) for z in (-sz, sz) for y in (-sy, sy) for x in (-sx, sx)]
    idx = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    return _finish(bm, set_mat([bm.faces.new([vs[i] for i in q]) for q in idx], mat))


def prism(bm, poly2d, p0, p1, mat, up=V((0, 0, 1)), scale1=1.0):
    """Extrude a 2D polygon (x = right of the axis, y = toward `up`) from p0 to p1."""
    p0, p1 = V(p0), V(p1)
    ax = (p1 - p0).normalized()
    x = ax.cross(V(up))
    if x.length < 1e-6:
        x = ax.cross(V((1, 0, 0)))
    x.normalize()
    y = x.cross(ax)
    poly = [V(q) for q in poly2d]
    rings = [[p0 + x * q.x + y * q.y for q in poly], [p1 + (x * q.x + y * q.y) * scale1 for q in poly]]
    return _finish(bm, _bridge(bm, rings, mat))


def extrude_x(bm, poly_yz, x0, x1, mat):
    """Extrude a polygon in the (y, z) plane along X (wings, fins, flaps)."""
    rings = [[V((x0, p[0], p[1])) for p in poly_yz], [V((x1, p[0], p[1])) for p in poly_yz]]
    return _finish(bm, _bridge(bm, rings, mat))


def extrude_y(bm, poly_xz, y0, y1, mat):
    """Extrude a polygon in the (x, z) plane along Y (bumpers, light bars)."""
    rings = [[V((p[0], y0, p[1])) for p in poly_xz], [V((p[0], y1, p[1])) for p in poly_xz]]
    return _finish(bm, _bridge(bm, rings, mat))


def cylinder(bm, p0, p1, r, mat, n=16, r1=None):
    """Capped cylinder (cone frustum with r1) from p0 to p1."""
    p0, p1 = V(p0), V(p1)
    x, y, _ = frame(p1 - p0)
    r1 = r if r1 is None else r1
    rings = [[p + (x * math.cos(a) + y * math.sin(a)) * rr
              for a in (2 * math.pi * i / n for i in range(n))] for p, rr in ((p0, r), (p1, r1))]
    return _finish(bm, _bridge(bm, rings, mat))


def sphere(bm, c, r, mat, n=16, rings=8, scale=(1, 1, 1)):
    out = bmesh.ops.create_uvsphere(bm, u_segments=n, v_segments=rings, radius=r,
                                    matrix=Matrix.Translation(c) @ Matrix.Diagonal((*scale, 1)))
    return set_mat(list({f for v in out['verts'] for f in v.link_faces}), mat)


def tube(bm, path, r, mat, n=8, corner=0.0, cseg=4, r_end=None):
    """Sweep a circle along a polyline with filleted corners (parallel transport, no twist).
    `r_end` tapers the radius linearly along the path."""
    pts = [V(p) for p in path]
    if corner > 0 and len(pts) > 2:
        pts = _fillet3d(pts, corner, cseg)
    tan = [(pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]).normalized() for i in range(len(pts))]
    x, _, _ = frame(tan[0])
    rings = []
    for i, p in enumerate(pts):
        t = tan[i]
        x = (x - t * x.dot(t)).normalized()
        y = t.cross(x)
        rr = r if r_end is None else r + (r_end - r) * i / (len(pts) - 1)
        rings.append([p + (x * math.cos(a) + y * math.sin(a)) * rr
                      for a in (2 * math.pi * k / n for k in range(n))])
    return _finish(bm, _bridge(bm, rings, mat))


def _fillet3d(pts, r, seg):
    out = [pts[0]]
    for i in range(1, len(pts) - 1):
        p0, p1, p2 = pts[i - 1], pts[i], pts[(i + 1) % len(pts)]
        a, b = p0 - p1, p2 - p1
        la, lb = a.length, b.length
        a.normalize()
        b.normalize()
        if abs(a.dot(b)) > 0.99995:  # straight or spike: keep the point
            out.append(p1)
            continue
        ang = math.acos(min(1.0, a.dot(b)))
        d = min(r / math.tan(ang / 2), la * 0.45, lb * 0.45)
        t0, t1 = p1 + a * d, p1 + b * d
        for s in range(seg + 1):  # quadratic Bezier ~ arc
            t = s / seg
            out.append(t0 * (1 - t) ** 2 + p1 * 2 * t * (1 - t) + t1 * t * t)
    out.append(pts[-1])
    return out


def revolve_x(bm, profile, mat, n=32):
    """Revolve an open (x, radius) polyline around the X axis.

    Winding: walking along the profile, the surface faces to the side given by
    normal = dx * e_r - dr * e_x, i.e. a profile walking toward +x faces outward (away from the
    axis) and one walking down toward the axis on the +x side faces +x. Points on the axis
    (radius 0) are welded. Returns the faces (normals already final, no recalculation)."""
    vr = []
    for k in range(n):
        a = 2 * math.pi * k / n
        c, s = math.cos(a), math.sin(a)
        vr.append([bm.verts.new(V((p[0], p[1] * c, p[1] * s))) for p in profile])
    fs = []
    for i in range(n):
        a, b = vr[i], vr[(i + 1) % n]
        for j in range(len(profile) - 1):
            fs.append(bm.faces.new((a[j], b[j], b[j + 1], a[j + 1])))
    set_mat(fs, mat)
    axis = [v for r in vr for v in r if v.co.y ** 2 + v.co.z ** 2 < 1e-12]
    if axis:
        bmesh.ops.remove_doubles(bm, verts=axis, dist=1e-6)
    return [f for f in fs if f.is_valid]


def helix(bm, p0, p1, r_coil, r_wire, turns, mat, n=5, steps_per_turn=10):
    """Coil spring from p0 to p1."""
    p0, p1 = V(p0), V(p1)
    x, y, z = frame(p1 - p0)
    length = (p1 - p0).length
    steps = max(2, int(turns * steps_per_turn))
    path = [p0 + z * (length * i / steps)
            + (x * math.cos(2 * math.pi * turns * i / steps)
               + y * math.sin(2 * math.pi * turns * i / steps)) * r_coil for i in range(steps + 1)]
    return tube(bm, path, r_wire, mat, n=n)


def loft(bm, rings, mat):
    """Closed loft through rings (lists of Vector, same length), capped at both ends."""
    return _finish(bm, _bridge(bm, rings, mat))


def slab(bm, keys, mat, seg=3, cx=0.0):
    """Loft of rounded rectangles along Y centred at x = cx. keys: [(y, half_width, z_bottom,
    z_top, radius)] (chassis tubs, bumpers, pods, light bars)."""
    rings = []
    for y, hw, z0, z1, r in sorted(keys):
        rr = rrect(2 * hw, z1 - z0, min(r, hw * 0.99, (z1 - z0) / 2 * 0.99), seg, cx, (z0 + z1) / 2)
        rings.append([V((q.x, y, q.y)) for q in rr])
    return loft(bm, rings, mat)


# ---------------------------------------------------------------------------------------------
# surface-conforming decals

class Surface:
    """Ray-cast target made of objects (evaluated mesh, world space)."""

    def __init__(self, objects):
        import bpy
        dg = bpy.context.evaluated_depsgraph_get()
        self.trees = []
        for o in objects:
            ev = o.evaluated_get(dg)
            tmp = bmesh.new()
            tmp.from_mesh(ev.to_mesh())
            tmp.transform(o.matrix_world)
            self.trees.append(BVHTree.FromBMesh(tmp))
            tmp.free()
            ev.to_mesh_clear()

    def project(self, p, d, offset, back=0.25):
        """First hit of a ray from p - d*back along d; pushed out along the surface normal."""
        best = None
        for t in self.trees:
            hit, nrm, _, dist = t.ray_cast(V(p) - V(d) * back, V(d), 3.0)
            if hit is not None and (best is None or dist < best[2]):
                best = (hit, nrm, dist)
        if best is None:
            return None, None
        return best[0] + best[1] * offset, best[1]


def _outline(poly2d, h):
    """Clean (no duplicates) outline resampled so no edge is longer than h."""
    poly = []
    for p in map(V, poly2d):
        if not poly or (p - poly[-1]).length > 1e-5:
            poly.append(p)
    if (poly[0] - poly[-1]).length <= 1e-5:
        poly.pop()
    out = []
    for i, a in enumerate(poly):
        b = poly[(i + 1) % len(poly)]
        k = max(1, int(math.ceil((b - a).length / h)))
        out += [a.lerp(b, j / k) for j in range(k)]
    return out


def _inside(p, poly):
    c = False
    for a, b in zip(poly, poly[1:] + poly[:1]):
        if (a.y > p.y) != (b.y > p.y) and p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y):
            c = not c
    return c


def _seg_dist(p, a, b):
    ab = b - a
    t = max(0.0, min(1.0, (p - a).dot(ab) / max(ab.dot(ab), 1e-12)))
    return (a + ab * t - p).length


def fill2d(poly2d, h=None):
    """Constrained-Delaunay fill of a 2D polygon (concave allowed): outline resampled at h plus
    an interior grid, so projected decals follow curved surfaces. Returns (outline, verts, tris)
    where verts[:len(outline)] is the outline."""
    from mathutils.geometry import delaunay_2d_cdt
    raw = [V(p) for p in poly2d]
    w = max(p.x for p in raw) - min(p.x for p in raw)
    ht = max(p.y for p in raw) - min(p.y for p in raw)
    h = h or max(0.01, min(0.035, min(w, ht) / 2.5))
    ol = _outline(raw, h)
    grid = []
    x0, y0 = min(p.x for p in ol), min(p.y for p in ol)
    nx, ny = int(w / h) + 2, int(ht / h) + 2
    for i in range(nx):
        for j in range(ny):
            q = V((x0 + (i + 0.5) * h, y0 + (j + 0.5) * h))
            if _inside(q, ol) and min(_seg_dist(q, a, b) for a, b in zip(ol, ol[1:] + ol[:1])) > 0.45 * h:
                grid.append(q)
    m = len(ol)
    vs, _, fs, *_ = delaunay_2d_cdt(ol + grid, [(i, (i + 1) % m) for i in range(m)],
                                    [list(range(m))], 1, 1e-7)
    return ol, [V(v) for v in vs], fs


def decal(bm, surf, poly2d, origin, u, v, mat, offset=0.0035, back=0.25, h=None):
    """Project a 2D outline (plane origin + x*u + y*v) onto `surf` along -(u x v); the fill is a
    constrained Delaunay mesh dense enough to hug curved surfaces. Faces point toward +(u x v);
    vertices that miss the surface stay in the plane (reported)."""
    origin, u, v = V(origin), V(u), V(v)
    nrm = u.cross(v).normalized()
    _, vs, tris = fill2d(poly2d, h)
    misses = []
    verts = []
    for p in vs:
        w = origin + u * p.x + v * p.y
        hit, _ = surf.project(w, -nrm, offset, back)
        if hit is None:
            misses.append(p)
        verts.append(bm.verts.new(hit if hit is not None else w))
    fs = []
    for t in sorted(tris, key=lambda t: sorted(t)):  # CDT face order varies between runs
        f = bm.faces.new([verts[i] for i in t])
        f.material_index = mat
        f.normal_update()
        if f.normal.dot(nrm) < 0:
            f.normal_flip()
        fs.append(f)
    if misses:
        print(f'decal: {len(misses)} vertices missed the surface (origin {tuple(origin)}, '
              f'first at plane {tuple(round(c, 3) for c in misses[0])})')
    return fs


def solid_decal(bm, surf, poly2d, origin, u, v, mat, height=0.004, side_mat=None, back=0.25,
                h=None):
    """A raised decal: projected top at `height` plus side walls down into the surface."""
    origin, u, v = V(origin), V(u), V(v)
    nrm = u.cross(v).normalized()
    ol, _, _ = fill2d(poly2d, h)
    top = decal(bm, surf, poly2d, origin, u, v, mat, offset=height, back=back, h=h)
    ring_t, ring_b = [], []
    for p in ol:
        w = origin + u * p.x + v * p.y
        h1, _ = surf.project(w, -nrm, height, back)
        h0, _ = surf.project(w, -nrm, -0.003, back)
        ring_t.append(h1 if h1 is not None else w)
        ring_b.append(h0 if h0 is not None else w - nrm * 0.006)
    vt = [bm.verts.new(p) for p in ring_t]
    vb = [bm.verts.new(p) for p in ring_b]
    n = len(vt)
    cen = sum(ring_t, V()) / n
    side = []
    for j in range(n):
        k = (j + 1) % n
        f = bm.faces.new((vb[j], vb[k], vt[k], vt[j]))
        f.material_index = mat if side_mat is None else side_mat
        f.normal_update()
        if f.normal.dot((vt[j].co + vt[k].co) / 2 - cen) < 0:
            f.normal_flip()
        side.append(f)
    return top + side


def strip(bm, surf, path2d, width, origin, u, v, mat, offset=0.0035, step=0.02, across=2,
          back=0.25):
    """Band of `width` along a 2D polyline in the plane origin + x*u + y*v, projected onto
    `surf` along -(u x v). For stripes, glow strips and trims."""
    origin, u, v = V(origin), V(u), V(v)
    nrm = u.cross(v).normalized()
    path = [V(p) for p in path2d]
    pts = []
    for a, b in zip(path, path[1:]):
        k = max(1, int(math.ceil((b - a).length / step)))
        pts += [a.lerp(b, i / k) for i in range(k)]
    pts.append(path[-1])
    rows = []
    misses = 0
    for i, p in enumerate(pts):
        t = (pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]).normalized()
        side = V((-t.y, t.x))
        row = []
        for k in range(across + 1):
            q = p + side * (width * (k / across - 0.5))
            wp = origin + u * q.x + v * q.y
            hit, _ = surf.project(wp, -nrm, offset, back)
            misses += hit is None
            row.append(bm.verts.new(hit if hit is not None else wp))
        rows.append(row)
    if misses:
        print(f'strip: {misses} vertices missed the surface (origin {tuple(origin)})')
    fs = [bm.faces.new((a[k], a[k + 1], b[k + 1], b[k]))
          for a, b in zip(rows, rows[1:]) for k in range(across)]
    for f in fs:
        f.material_index = mat
        f.normal_update()
        if f.normal.dot(nrm) < 0:
            f.normal_flip()
    return fs


# ---------------------------------------------------------------------------------------------
# in-house blocky racing digits: strokes on a 1 x 2 grid, drawn as rounded bars (no font files)

_STROKES = {
    '0': [((0, 0), (1, 0)), ((1, 0), (1, 2)), ((1, 2), (0, 2)), ((0, 2), (0, 0))],
    '1': [((0.55, 0), (0.55, 2)), ((0.55, 2), (0.15, 1.6)), ((0.15, 0), (0.95, 0))],
    '2': [((0, 2), (1, 2)), ((1, 2), (1, 1)), ((1, 1), (0, 1)), ((0, 1), (0, 0)), ((0, 0), (1, 0))],
    '3': [((0, 2), (1, 2)), ((1, 2), (1, 0)), ((0.3, 1), (1, 1)), ((0, 0), (1, 0))],
    '4': [((0, 2), (0, 0.8)), ((0, 0.8), (1, 0.8)), ((0.8, 1.5), (0.8, 0))],
    '5': [((1, 2), (0, 2)), ((0, 2), (0, 1)), ((0, 1), (1, 1)), ((1, 1), (1, 0)), ((1, 0), (0, 0))],
    '6': [((1, 2), (0, 2)), ((0, 2), (0, 0)), ((0, 0), (1, 0)), ((1, 0), (1, 1)), ((1, 1), (0, 1))],
    '7': [((0, 2), (1, 2)), ((1, 2), (0.4, 0))],
    '8': [((0, 0), (1, 0)), ((1, 0), (1, 2)), ((1, 2), (0, 2)), ((0, 2), (0, 0)), ((0, 1), (1, 1))],
    '9': [((0, 0), (1, 0)), ((1, 0), (1, 2)), ((1, 2), (0, 2)), ((0, 2), (0, 1)), ((0, 1), (1, 1))],
}


def number_bars(text, h=1.0, t=0.22, gap=0.25, slant=0.12):
    """Outlines (convex 2D polygons, centred) for a racing number of height h."""
    s = h / 2.0
    tt = t * h
    bars = []
    adv = s + gap * h
    x0 = -(len(text) * adv - gap * h) / 2
    for ci, ch in enumerate(text):
        ox = x0 + ci * adv
        for (ax, ay), (bx, by) in _STROKES[ch]:
            a = V((ox + ax * (s - tt) + tt / 2, ay * (h - tt) / 2 + tt / 2 - h / 2))
            b = V((ox + bx * (s - tt) + tt / 2, by * (h - tt) / 2 + tt / 2 - h / 2))
            d = b - a
            ln = d.length
            ang = math.atan2(d.y, d.x)
            rot = Matrix.Rotation(ang, 2)
            rect = rrect(ln + tt, tt, tt * 0.28, 2)
            pts = [(a + b) / 2 + rot @ p for p in rect]
            bars.append([V((p.x + p.y * slant, p.y)) for p in pts])
    return bars
