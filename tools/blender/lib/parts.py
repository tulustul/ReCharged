"""Composite car parts: suspension, lights, antenna, wings, racing numbers.

All functions take a bmesh and world-space points (Blender axes, front +Y) and use the fixed
material names from core.car_materials().
"""
import math

from mathutils import Matrix, Vector

from . import core, geo

V = Vector


def bar(bm, a, b, w, t, mat, up=V((0, 0, 1)), taper=1.0):
    """Flat rounded bar from a to b (w across, t thick along `up`)."""
    return geo.prism(bm, geo.rrect(w, t, min(w, t) * 0.35, 1), a, b, mat, up=up, scale1=taper)


def a_arm(bm, pivot_f, pivot_r, ball, w=0.032, t=0.022, mat='plastic'):
    """Wishbone: two bars from the chassis pivots to the hub ball joint, plus a web bar."""
    m = core.mi(mat)
    pivot_f, pivot_r, ball = V(pivot_f), V(pivot_r), V(ball)
    bar(bm, pivot_f, ball, w, t, m, taper=0.8)
    bar(bm, pivot_r, ball, w, t, m, taper=0.8)
    mid_f, mid_r = pivot_f.lerp(ball, 0.45), pivot_r.lerp(ball, 0.45)
    bar(bm, mid_f, mid_r, w * 0.8, t * 0.8, m)
    for p in (pivot_f, pivot_r):  # pivot bushings
        geo.cylinder(bm, p - V((0, 0.025, 0)), p + V((0, 0.025, 0)), t * 0.7, m, n=8)


def shock(bm, bottom, top, r=0.022, spring='paint_accent', coils=5, body='metal'):
    """Coil-over shock: chrome shaft, anodised body, spring, eyelets."""
    bottom, top = V(bottom), V(top)
    ax = (top - bottom).normalized()
    ln = (top - bottom).length
    chrome, metal, plastic = core.mi('chrome'), core.mi(body), core.mi('plastic')
    split = bottom + ax * ln * 0.42
    geo.cylinder(bm, bottom + ax * 0.02, split, r * 0.32, chrome, n=8)
    geo.cylinder(bm, split, top - ax * 0.02, r, metal, n=10)
    geo.cylinder(bm, top - ax * 0.05, top - ax * 0.015, r * 1.12, metal, n=10)  # cap
    for p in (bottom, top):  # eyelets
        geo.cylinder(bm, p - V((0, 0.012, 0)), p + V((0, 0.012, 0)), r * 0.55, plastic, n=8)
    geo.cylinder(bm, bottom + ax * 0.06, bottom + ax * 0.075, r * 1.25, metal, n=10)  # collar
    if spring:
        geo.helix(bm, bottom + ax * 0.075, top - ax * 0.07, r * 1.3, r * 0.2, coils,
                  core.mi(spring), n=4, steps_per_turn=7)


def light_pod(bm, center, direction, r, depth, lens='light_head', housing='plastic'):
    """Round off-road lamp: housing cup, chrome bezel ring, emissive lens."""
    c, d = V(center), V(direction).normalized()
    geo.cylinder(bm, c - d * depth, c, r, core.mi(housing), n=12, r1=r * 1.05)
    geo.cylinder(bm, c - d * 0.002, c + d * 0.006, r * 1.06, core.mi('chrome'), n=12)
    geo.cylinder(bm, c, c + d * 0.009, r * 0.82, core.mi(lens), n=12)


def antenna(base, height, lean=V((0, -0.12, 1.0)), r0=0.007, r1=0.003, name='antenna'):
    """Whip antenna object with its pivot at `base` (parent it to `body`)."""
    bm = core.new_bm()
    d = lean.normalized()
    b = V(base)
    geo.cylinder(bm, b - V((0, 0, 0.01)), b + d * 0.035, r0 * 2.0, core.mi('plastic'), n=10)
    pts = [b + d * 0.03]
    for i in range(1, 7):  # slight droop toward the tip
        t = i / 6
        pts.append(b + d * (height * t) + V((0, -0.04 * t * t, 0)))
    geo.tube(bm, pts, r0, core.mi('plastic'), n=6, r_end=r1)
    tip = pts[-1]
    geo.cylinder(bm, tip - d * 0.012, tip + d * 0.022, r1 * 3.2, core.mi('paint_accent'), n=8)
    return core.obj_from_bm(name, bm, weighted=False, angle=50)


def airfoil(chord, thick, camber=0.03, n=10):
    """Closed (y, z) airfoil, leading edge at +y (front), trailing edge at -y."""
    up, lo = [], []
    for i in range(n + 1):
        x = (1 - math.cos(math.pi * i / n)) / 2  # 0 = leading edge
        t = 5 * thick * (0.2969 * math.sqrt(x) - 0.126 * x - 0.3516 * x * x + 0.2843 * x ** 3
                         - 0.1036 * x ** 4)
        c = camber * 4 * x * (1 - x)
        up.append(V((chord / 2 - x * chord, (c + t) * chord)))
        lo.append(V((chord / 2 - x * chord, (c - t) * chord)))
    return lo[::-1][:-1] + up[:-1]


def wing(bm, center, span, chord, thick=0.12, angle=-8.0, mat='paint_accent', plate=None,
         plate_mat='paint_accent', camber=0.04):
    """Rear wing: cambered airfoil extruded across X, pitched by `angle` deg, with endplates
    `plate` = (height, length) or None."""
    c = V(center)
    rot = Matrix.Rotation(math.radians(angle), 2)
    prof = [rot @ p for p in airfoil(chord, thick, camber)]
    prof = [(p.x + c.y, p.y + c.z) for p in prof]
    geo.extrude_x(bm, prof, c.x - span / 2, c.x + span / 2, core.mi(mat))
    if plate:
        ph, pl = plate
        for sx in (-1, 1):
            x = c.x + sx * span / 2
            poly = geo.rrect(pl, ph, 0.03, 2, c.y - 0.01, c.z - ph * 0.25)
            geo.extrude_x(bm, [(p.x, p.y) for p in poly], x, x + 0.012 * sx, core.mi(plate_mat))


def number(bm, surf, text, origin, u, v, h, roundel=0.0, plate=None, digit_mat='plastic',
           offset=0.003, back_mat='decal'):
    """Racing number decal: optional roundel (radius) or plate (w, h) + blocky digits."""
    if roundel:
        geo.decal(bm, surf, geo.circle2(roundel, 32), origin, u, v, core.mi(back_mat), offset)
    if plate:
        geo.decal(bm, surf, geo.rrect(plate[0], plate[1], min(plate) * 0.25, 3), origin, u, v,
                  core.mi(back_mat), offset)
    for i, b in enumerate(geo.number_bars(text, h)):
        geo.decal(bm, surf, b, origin, u, v, core.mi(digit_mat), offset + 0.0012 + 0.0001 * i)


def lamps(bm, surf, poly2d, x, z, front, mat, height=0.004, bezel=None, bezel_w=0.008,
          y_plane=None):
    """Mirrored pair of raised lenses projected onto the nose (front) or tail (rear).
    poly2d is the right-hand lens outline in (x, z) around its own centre (outer side = +x)."""
    y = (3.0 if front else -3.0) if y_plane is None else y_plane
    for s in (-1, 1):
        # front faces +Y: u = (-1,0,0); rear faces -Y: u = (1,0,0); mirror for the left lamp
        u = V((-1, 0, 0)) if front else V((1, 0, 0))
        pts = [V((s * p.x * (-1 if front else 1), p.y)) for p in map(V, poly2d)]
        pts = geo.ccw(pts)
        org = V((s * x, y, z))
        if bezel:
            outer = geo.ccw([c * 1.0 for c in pts])
            cen = sum(outer, V((0, 0))) / len(outer)
            ring = [cen + (p - cen) * (1 + bezel_w / max(0.01, (p - cen).length)) for p in outer]
            geo.decal(bm, surf, ring, org, u, V((0, 0, 1)), core.mi(bezel), offset=0.003)
        geo.solid_decal(bm, surf, pts, org, u, V((0, 0, 1)), core.mi(mat), height=height,
                        side_mat=core.mi(bezel or 'plastic'))
