"""Parametric RC wheel: tyre with tread variants + rim styles.

Built around the origin for the right-hand side (rim face toward +X) and mirrored for the left,
so every wheel node has identity rotation and spins about local X. The tyre's outermost
surface (knob tops) sits exactly at `radius`, its width is exactly `width`.
"""
import math

from mathutils import Vector

from . import core, geo

V = Vector

# tread: knob height as a fraction of the radius, shoulder radius as a fraction of the width
TREADS = {
    'knobby': dict(kh=0.10, shoulder=0.30),
    'block': dict(kh=0.07, shoulder=0.22),
    'chevron': dict(kh=0.11, shoulder=0.28),
    'street': dict(kh=0.0, shoulder=0.16),
    'slick': dict(kh=0.0, shoulder=0.22),
}


def _tyre(bm, R, W, rb, tread, n, rows, letter_ring=None):
    t = TREADS[tread]
    kh = t['kh'] * R
    Rc = R - kh
    h = Rc - rb
    sr = min(t['shoulder'] * W, 0.45 * h)
    x0, x1 = -W / 2, W / 2
    bead = 0.12 * W
    mid = rb + 0.42 * h
    pts = [(x0 + bead, rb), (x0 - 0.02 * W, mid), (x0, Rc), (x1, Rc), (x1 + 0.02 * W, mid),
           (x1 - bead, rb)]
    radii = [0, 0.35 * h, sr, sr, 0.35 * h, 0]
    prof = geo.rounded_poly(pts, radii, seg=[0, 1, 3, 3, 1, 0], closed=False)
    if tread == 'street':  # two circumferential grooves
        top = [p for p in prof if abs(p.y - Rc) < 1e-6]
        gw, gd = 0.06 * W, 0.035 * R
        i0 = prof.index(top[0])
        groove = []
        for gx in (0.0,):
            groove += [V((gx - gw, Rc)), V((gx - gw * 0.7, Rc - gd)), V((gx + gw * 0.7, Rc - gd)),
                       V((gx + gw, Rc))]
        prof = prof[:i0 + 1] + groove + prof[i0 + 1:]
    prof = [p for i, p in enumerate(prof) if i == 0 or (p - prof[i - 1]).length > 1e-7]
    rub = core.mi('rubber')
    fs = geo.revolve_x(bm, [(p.x, p.y) for p in prof], rub, n=n)
    if letter_ring:  # raised sidewall ring (white letters / coloured stripe) on the outer side
        r0, r1 = rb + 0.18 * h, rb + 0.42 * h
        xr = x1 + 0.02 * W + 0.002
        geo.revolve_x(bm, [(xr - 0.004, r1), (xr, r1), (xr, r0), (xr - 0.004, r0), (xr - 0.004, r1)],
                      core.mi(letter_ring), n=n)
    if kh > 0:
        _knobs(bm, R, Rc, W, tread, rows, rub)
    return fs


def _knob(bm, foot, R, Rc, mat, ch):
    """Frustum knob on the tread: convex footprint in (x, s); s = arc length on the carcass."""
    foot = geo.ccw([V(p) for p in foot])
    cen = sum(foot, V((0, 0))) / len(foot)
    upper = [cen + (p - cen) * 0.94 for p in foot]
    top = geo.inset2(upper, ch)

    def p3(q, r):
        a = q.y / Rc
        return V((q.x, r * math.cos(a), r * math.sin(a)))

    rings = [[p3(q, Rc - 0.008) for q in foot], [p3(q, R - ch) for q in upper],
             [p3(q, R) for q in top]]
    fs = geo._bridge(bm, rings, mat, cap0=False, cap1=True)
    fs = geo._finish(bm, fs, orient=False)
    c3 = sum((v.co for f in fs for v in f.verts), V()) / sum(len(f.verts) for f in fs)
    for f in fs:
        f.normal_update()
        if f.normal.dot(f.calc_center_median() - c3) < 0:
            f.normal_flip()


def _knobs(bm, R, Rc, W, tread, rows, mat):
    circ = 2 * math.pi * Rc
    pitch = circ / rows
    hw = W / 2
    for k in range(rows):
        s = k * pitch
        if tread == 'knobby':  # staggered square pins, shoulder pins on alternate rows
            q = 0.20 * W
            xs = (-0.27 * W, 0.27 * W) if k % 2 == 0 else (-0.42 * W, 0.0, 0.42 * W)
            for x in xs:
                qx = q * (0.9 if abs(x) > 0.4 * W else 1.0)
                _knob(bm, geo.rrect(qx, q, 0.0, 1, x, s)[::2], R, Rc, mat, 0.25 * q)
        elif tread == 'block':  # rally gravel blocks: 3 columns, staggered centre
            bw, bl = 0.26 * W, 0.55 * pitch
            off = 0.25 * pitch if k % 2 else -0.25 * pitch
            for x, ds in ((-0.34 * W, 0.0), (0.0, off), (0.34 * W, 0.0)):
                _knob(bm, geo.rrect(bw, bl, 0.0, 1, x, s + ds)[::2], R, Rc, mat, 0.18 * bw)
        elif tread == 'chevron':  # monster-truck V lugs: two angled arms + shoulder block
            aw = 0.15 * pitch * 2.2
            sweep = 0.9 * pitch
            for sgn in (-1, 1):
                a = V((0.0, s))
                b = V((sgn * 0.40 * W, s + sweep))
                d = (b - a).normalized()
                nrm = V((-d.y, d.x)) * aw / 2
                arm = [a - nrm, b - nrm, b + nrm, a + nrm]
                _knob(bm, arm, R, Rc, mat, 0.12 * aw)
                sb = V((sgn * 0.47 * W, s + sweep + 0.18 * pitch))
                _knob(bm, geo.rrect(0.06 * W, 0.32 * pitch, 0.0, 1, sb.x, sb.y)[::2], R, Rc, mat,
                      0.1 * aw)


def _rim(bm, R, W, rb, style):
    """Rim lip + face. style keys: spokes, spoke_w, skew, depth, holes, bolts, nut."""
    rim, dark, chrome, metal = (core.mi(m) for m in ('rim', 'plastic', 'chrome', 'metal'))
    xf = W / 2 - 0.10 * W          # face plane (slightly recessed into the tyre)
    lipw = 0.11 * rb
    # lip ring: closed profile, clockwise in (x, r), outer corners filleted so they catch light
    lip = geo.rounded_poly([(xf - 0.04 * W, rb + 0.004), (xf + 0.004, rb + 0.004),
                            (xf + 0.004, rb - lipw), (xf - 0.04 * W, rb - lipw)],
                           [0.0, 0.006, 0.005, 0.0], seg=[0, 1, 1, 0])
    lip = [p for i, p in enumerate(lip) if i == 0 or (p - lip[i - 1]).length > 1e-7]
    geo.revolve_x(bm, [(p.x, p.y) for p in lip + lip[:1]], rim, n=32)
    depth = style.get('depth', 0.28) * W
    back_mat = rim if style.get('spokes', 0) == 0 else dark
    # back plate / inner barrel closes the wheel (seen between spokes and from the car side)
    xb = xf - depth
    xi = -W / 2 + 0.12 * W
    geo.revolve_x(bm, [(xi, 0.0), (xi, rb - lipw * 0.5), (xb, rb - lipw * 0.5), (xb, 0.0)],
                  back_mat, n=32)
    rh = style.get('hub', 0.30) * rb
    n_sp = style.get('spokes', 0)
    if n_sp:
        sw = style.get('spoke_w', 0.22) * rb
        skew = style.get('skew', 0.0)
        for i in range(n_sp):
            a = 2 * math.pi * i / n_sp + math.pi / 2
            a1 = a + skew
            p0 = V((xf - 0.004, rh * 0.8 * math.cos(a), rh * 0.8 * math.sin(a)))
            p1 = V((xf - 0.02 * W, (rb - lipw * 0.6) * math.cos(a1), (rb - lipw * 0.6) * math.sin(a1)))
            dz = style.get('spoke_d', 0.5) * depth
            prof = geo.rrect(sw, dz, min(sw, dz) * 0.3, 1, 0.0, -dz / 2)
            geo.prism(bm, prof, p0, p1, rim, up=V((1, 0, 0)), scale1=style.get('taper', 0.75))
    holes = style.get('holes', 0)
    if holes:  # dish wheel: round dark pockets on the face
        for i in range(holes):
            a = 2 * math.pi * (i + 0.5) / holes + math.pi / 2
            rr = (rh + rb - lipw) / 2
            c = V((xb + 0.003, rr * math.cos(a), rr * math.sin(a)))
            geo.cylinder(bm, c, c + V((0.006, 0, 0)), 0.32 * (rb - lipw - rh), dark, n=12)
    # hub boss
    geo.cylinder(bm, V((xb, 0, 0)), V((xf + 0.006, 0, 0)), rh, rim, n=20, r1=rh * 0.92)
    bolts = style.get('bolts', 0)
    for i in range(bolts):  # beadlock ring bolts
        a = 2 * math.pi * i / bolts
        c = V((xf + 0.004, (rb - lipw * 0.45) * math.cos(a), (rb - lipw * 0.45) * math.sin(a)))
        geo.cylinder(bm, c, c + V((0.008, 0, 0)), 0.012 * R / 0.3, chrome, n=6)
    # hex nut + axle tip (anodised RC nut)
    nut = style.get('nut', 0.42) * rh
    geo.cylinder(bm, V((xf + 0.005, 0, 0)), V((xf + 0.005 + 0.06 * W, 0, 0)), nut,
                 metal if style.get('nut_metal', True) else chrome, n=6)
    geo.cylinder(bm, V((xf + 0.005 + 0.06 * W, 0, 0)), V((xf + 0.012 + 0.06 * W, 0, 0)), nut * 0.5,
                 chrome, n=10)


def wheel(name, center, radius, width, side, tread='knobby', rim=None, rim_ratio=0.6, n=None,
          rows=None, letter_ring=None):
    """Build one wheel object named `name` at `center` (world). side: +1 right, -1 left."""
    rim = rim or dict(spokes=6)
    rows = rows or {'knobby': 18, 'block': 16, 'chevron': 15}.get(tread, 0)
    n = n or (32 if TREADS[tread]['kh'] > 0 else 44)
    rb = rim_ratio * radius
    bm_t = core.new_bm()
    _tyre(bm_t, radius, width, rb, tread, n, rows, letter_ring)
    bm_r = core.new_bm()
    _rim(bm_r, radius, width, rb, rim)
    objs = []
    for bm, weighted in ((bm_t, False), (bm_r, True)):
        if side < 0:
            import bmesh
            bmesh.ops.scale(bm, vec=(-1, 1, 1), verts=bm.verts)
            bmesh.ops.reverse_faces(bm, faces=bm.faces)
        bmesh_translate(bm, center)
        objs.append(core.obj_from_bm(name + '_part', bm, angle=35, weighted=weighted))
    return core.bake(objs, name, location=center)


def bmesh_translate(bm, offset):
    import bmesh
    bmesh.ops.translate(bm, vec=V(offset), verts=bm.verts)
