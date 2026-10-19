"""Hornet — Pro lifted hot rod: early-'30s three-window coupe on a lifted off-road chassis. Tall
chrome grille shell, stalk-mounted chrome headlamps, louvred hood, flowing black pontoon fenders
running into running boards, chopped upright cabin, tilted spare on the rear deck, chrome nerf
bars, solid axles with red coil-overs, chunky all-terrain tyres on grey steel dish wheels.
Number 5.

    blender -b -P tools/blender/cars/hornet.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

import bmesh  # noqa: E402
from mathutils import Matrix, Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'hornet'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#EAA80A', accent='#1D1D20', rim='#8E939A', metal='#C8331E')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zW = D['radius'][0]
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']
BELT, ROOF = 0.56, HH - 0.007
SILL = 0.31  # body underside = running-board top

# --- body: hood, cabin, turtle deck (narrow, between the wheels) -------------------------------
prof = shell.Profile([
    (-0.80, dict(hw=0.25, z0=SILL + 0.03, zb=0.40, hg=0.22, zg=0.42, hr=0.12, zt=0.44)),
    (-0.70, dict(hw=0.29, z0=SILL, zb=0.48, hg=0.26, zg=0.51, hr=0.16, zt=0.53)),
    (-0.58, dict(hw=0.30, zb=0.535, hg=0.275, zg=0.55, hr=0.20, zt=0.58)),
    (-0.52, dict(hw=0.305, zb=BELT, hg=0.28, zg=BELT + 0.01, hr=0.215, zt=0.61)),
    (-0.45, dict(hr=0.24, zt=0.675)),
    (-0.40, dict(hr=0.25, zt=ROOF - 0.004)),
    (-0.05, dict(hw=0.305, zb=BELT, hg=0.28, zg=BELT + 0.01, hr=0.25, zt=ROOF, crown=0.007,
                 rr=0.05)),
    (0.00, dict(zt=ROOF - 0.01)),
    (0.055, dict(hw=0.30, zb=0.552, hg=0.27, zg=0.558, hr=0.19, zt=0.568, rr=0.04)),
    (0.40, dict(hw=0.23, zb=0.542, hg=0.21, zg=0.547, hr=0.12, zt=0.557)),
    (0.78, dict(hw=0.13, z0=SILL + 0.03, zb=0.528, hg=0.115, zg=0.531, hr=0.06, zt=0.54)),
])
bm = core.new_bm()
shell.shell(bm, prof, -0.80, 0.78, ends=(0.10, 0.02), end_scale=0.6, step=0.065,
            extra=(0.01, 0.02, 0.03, 0.04))
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])


# --- pontoon fenders + running boards: one sweep per side, nose to tail ------------------------
def sweep_fender(bm, zc, sec, side, mat, ds=0.045, end=0.07, end_scale=0.45):
    """Sweep the fender section along the underside line z = zc(y) (front to rear, in the
    y-z plane); sec(y) -> (x_in, x_out, thick, droop, top fillet). The section's 'up' is the
    path normal."""
    ys = [0.885 - i * 0.002 for i in range(int(1.77 / 0.002) + 1)]
    pts = [V((y, zc(y))) for y in ys]
    acc = [0.0]
    for a, b in zip(pts, pts[1:]):
        acc.append(acc[-1] + (b - a).length)
    n = int(acc[-1] / ds)
    rings, j = [], 0
    for k in range(n + 1):
        s = acc[-1] * k / n
        while j < len(acc) - 2 and acc[j + 1] < s:
            j += 1
        p = pts[j].lerp(pts[j + 1], (s - acc[j]) / max(acc[j + 1] - acc[j], 1e-9))
        t = (pts[min(j + 1, len(pts) - 1)] - pts[max(j - 1, 0)]).normalized()
        nrm = V((t.y, -t.x))  # (y, z) tangent turned -90 deg: up while heading rearward
        x_in, x_out, th, droop, rt = sec(p.x)
        poly = geo.rounded_poly([(x_in, 0.0), (x_out, -droop), (x_out + 0.003, th * 0.45),
                                 (x_out - 0.02, th), (x_in + 0.02, th), (x_in, th * 0.5)],
                                [0, 0.012, 0.02, rt, rt, 0.015], [0, 1, 1, 3, 3, 1])
        d = min(s, acc[-1] - s)  # round the nose and tail: shrink the section toward its middle
        if d < end:
            f = end_scale + (1 - end_scale) * math.sqrt(1 - (1 - d / end) ** 2)
            xm = (x_in + x_out) / 2
            poly = [V((xm + (q.x - xm) * f, th / 2 + (q.y - th / 2) * f)) for q in poly]
        rings.append([V((side * q.x, p.x + nrm.x * q.y, p.y + nrm.y * q.y)) for q in poly])
    return geo.loft(bm, rings, mat)


def arc(yc, r, deg):
    a = math.radians(deg)
    return (yc + r * math.cos(a), zW + r * math.sin(a))


RF, RFF = 0.262, 0.254  # rear / front fender underside radius around the wheel centre
z_line = geo.Curve([(0.885, 0.33), (0.85, 0.385)]
                   + [arc(yF, RFF, d) for d in (42, 70, 90, 112, 138)]
                   + [(0.30, 0.345), (0.22, 0.30), (0.15, SILL - 0.03), (-0.15, SILL - 0.03),
                      (-0.21, 0.30), (-0.29, 0.345)]
                   + [arc(yR, RF, d) for d in (42, 68, 90, 112, 138)]
                   + [(-0.85, 0.37), (-0.885, 0.31)])
c_in = geo.Curve([(0.885, 0.34), (0.6, 0.31), (0.36, 0.30), (0.2, 0.27), (-0.2, 0.27),
                  (-0.5, 0.275), (-0.72, 0.25), (-0.885, 0.26)])
c_out = geo.Curve([(0.885, 0.47), (0.78, 0.52), (0.6, 0.545), (0.4, 0.53), (0.25, 0.505),
                   (-0.2, 0.505), (-0.4, 0.545), (-0.75, 0.545), (-0.885, 0.50)])
c_th = geo.Curve([(0.885, 0.03), (0.78, 0.044), (0.36, 0.044), (0.2, 0.03), (-0.2, 0.03),
                  (-0.36, 0.055), (-0.78, 0.055), (-0.885, 0.035)])
c_dr = geo.Curve([(0.885, 0.004), (0.6, 0.016), (0.25, 0.004), (-0.25, 0.004), (-0.6, 0.03),
                  (-0.8, 0.03), (-0.885, 0.01)])
c_rt = geo.Curve([(0.885, 0.05), (0.36, 0.07), (0.2, 0.01), (-0.2, 0.01), (-0.36, 0.08),
                  (-0.885, 0.06)])
bm = core.new_bm()
for s in (-1, 1):
    sweep_fender(bm, z_line, lambda y: (c_in(y), c_out(y), c_th(y), c_dr(y), c_rt(y)), s,
                 mi('paint_accent'))
fenders = core.obj_from_bm('fenders', bm, weighted=False, angle=50)

# --- grille shell ------------------------------------------------------------------------------
G_OUT = geo.rounded_poly([(0, 0.30), (0.075, 0.33), (0.115, 0.43), (0.112, 0.565), (-0.112, 0.565),
                          (-0.115, 0.43), (-0.075, 0.33)],
                         [0.03, 0.05, 0.06, 0.035, 0.035, 0.06, 0.05], 3)
G_IN = [V((p.x * 0.80, 0.45 + (p.y - 0.45) * 0.86)) for p in G_OUT]


def z_span(poly, x):
    """(min z, max z) where the vertical line at x crosses the closed polygon."""
    zs = []
    for a, b in zip(poly, poly[1:] + poly[:1]):
        if (a.x - x) * (b.x - x) <= 0 and abs(b.x - a.x) > 1e-9:
            zs.append(a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x))
    return min(zs), max(zs)


bm = core.new_bm()
geo.extrude_y(bm, [(p.x, p.y) for p in G_OUT], 0.77, 0.845, mi('chrome'))
geo.extrude_y(bm, [(p.x, p.y) for p in G_IN], 0.845, 0.849, mi('plastic'))
for i in range(11):
    x = -0.075 + i * 0.015
    z0, z1 = z_span(G_IN, x)
    geo.box(bm, V((x, 0.851, (z0 + z1) / 2)), (0.006, 0.006, z1 - z0 - 0.01), mi('chrome'))
geo.cylinder(bm, V((0, 0.80, 0.56)), V((0, 0.80, 0.585)), 0.018, mi('chrome'), n=10, r1=0.012)
grille = core.obj_from_bm('grille', bm, weighted=False, angle=40)

# --- hard parts: frame rails, bumper brackets, running-board treads, knuckles ---------------
bm = core.new_bm()
for s in (-1, 1):
    geo.box(bm, V((s * 0.21, 0, 0.29)), (0.05, 1.70, 0.04), mi('plastic'))  # frame rails
    geo.box(bm, V((s * 0.20, 0.87, 0.31)), (0.04, 0.07, 0.03), mi('plastic'))  # bumper brackets
    geo.box(bm, V((s * 0.20, -0.87, 0.31)), (0.04, 0.07, 0.03), mi('plastic'))
    for k in range(3):  # ribbed rubber treads on the running boards
        geo.box(bm, V((s * (0.33 + k * 0.055), 0.0, SILL + 0.003)), (0.03, 0.30, 0.008),
                mi('rubber'))
    geo.box(bm, V((s * 0.37, yF, zW)), (0.04, 0.08, 0.10), mi('plastic'))  # knuckles
    geo.box(bm, V((s * 0.37, yR, zW)), (0.04, 0.08, 0.10), mi('plastic'))
hard = core.obj_from_bm('hard', bm, bevel=0.005, segments=1, angle=40)

# --- round parts: headlamps, nerf bars, tail lamps, axles, coil-overs, links -----------------
def coilover(bm, a, b, r=0.024):
    """Cheap coil-over (parts.shock is ~3x the triangles): chrome body, shaft, red spring."""
    ax = (b - a).normalized()
    mid = a.lerp(b, 0.45)
    geo.cylinder(bm, a, mid, r * 0.35, chrome, n=6)
    geo.cylinder(bm, mid, b, r * 0.8, chrome, n=8)
    geo.helix(bm, a + ax * 0.03, b - ax * 0.03, r * 1.25, r * 0.22, 4, mi('metal'), n=4,
              steps_per_turn=6)


bm = core.new_bm()
chrome, plastic = mi('chrome'), mi('plastic')
for s in (-1, 1):
    c = V((s * 0.215, 0.79, 0.50))
    geo.sphere(bm, c, 0.064, chrome, n=14, rings=8, scale=(1, 0.75, 1))
    parts.light_pod(bm, c + V((0, 0.05, 0)), V((0, 1, 0)), 0.056, 0.03, housing='chrome')
    geo.cylinder(bm, c - V((0, 0, 0.03)), V((s * 0.21, 0.78, 0.31)), 0.011, chrome, n=8)
    tl = V((s * 0.41, -0.80, 0.50))
    parts.light_pod(bm, tl, V((0, -1, 0)), 0.03, 0.04, lens='light_tail', housing='chrome')
    geo.cylinder(bm, tl - V((0, 0.009, 0)), tl - V((0, 0.012, 0)), 0.013, mi('light_brake'), n=10)
for y in (0.90, -0.90):
    sg = 1 if y > 0 else -1
    geo.tube(bm, [(-0.50, y - sg * 0.07, 0.315), (-0.45, y, 0.315), (0.45, y, 0.315),
                  (0.50, y - sg * 0.07, 0.315)], 0.024, chrome, n=10, corner=0.05)
    for s in (-1, 1):  # bumperettes
        geo.cylinder(bm, V((s * 0.30, y, 0.28)), V((s * 0.30, y, 0.35)), 0.03, chrome, n=10)
for y in (yF, yR):
    sg = 1 if y > 0 else -1
    geo.cylinder(bm, V((-0.36, y, zW)), V((0.36, y, zW)), 0.026, plastic, n=10)  # axle tube
    geo.sphere(bm, V((0.06 * sg, y, zW)), 0.065, plastic, n=10, rings=6, scale=(0.85, 1, 1))
    geo.cylinder(bm, V((0.06 * sg, y - sg * 0.05, zW)), V((0.06 * sg, y - sg * 0.065, zW)), 0.04,
                 chrome, n=12)  # diff cover
    for s in (-1, 1):
        coilover(bm, V((s * 0.30, y + sg * 0.02, zW + 0.03)), V((s * 0.27, y - sg * 0.02, 0.50)))
        parts.bar(bm, V((s * 0.26, y, zW - 0.02)), V((s * 0.21, y - sg * 0.34, 0.27)), 0.03, 0.02,
                  plastic, up=V((1, 0, 0)))  # trailing links
    parts.bar(bm, V((0.0, y - sg * 0.03, zW + 0.03)), V((0.12, y - sg * 0.30, 0.29)), 0.026, 0.018,
              plastic, up=V((1, 0, 0)))  # upper link
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)


# --- spare wheel on the turtle deck (cheap: revolved tyre + blocks, dish rim) ----------------
def spare(r=0.18, w=0.11, rb=0.105):
    bm = core.new_bm()
    prof = geo.rounded_poly([(-w / 2 + 0.012, rb), (-w / 2, rb + 0.03), (-w / 2, r - 0.02),
                             (w / 2, r - 0.02), (w / 2, rb + 0.03), (w / 2 - 0.012, rb)],
                            [0, 0.02, 0.022, 0.022, 0.02, 0], [0, 1, 2, 2, 1, 0], closed=False)
    prof = [p for i, p in enumerate(prof) if i == 0 or (p - prof[i - 1]).length > 1e-7]
    geo.revolve_x(bm, [(p.x, p.y) for p in prof], mi('rubber'), n=20)
    for k in range(20):  # chunky tread blocks, staggered
        a = 2 * math.pi * k / 20
        rad, tan = V((0, math.cos(a), math.sin(a))), V((0, -math.sin(a), math.cos(a)))
        cx = 0.022 if k % 2 else -0.022
        c = rad * (r - 0.02)
        geo.prism(bm, geo.rrect(0.05, 0.04, 0.006, 1, cx, 0.0), c - tan * 0.017, c + tan * 0.017,
                  mi('rubber'), up=rad)
    geo.revolve_x(bm, [(-0.035, 0.0), (-0.035, rb + 0.004), (0.03, rb + 0.004), (0.03, 0.07),
                       (0.042, 0.04), (0.042, 0.0)], mi('rim'), n=20)
    geo.cylinder(bm, V((0.04, 0, 0)), V((0.056, 0, 0)), 0.032, mi('chrome'), n=12, r1=0.026)
    for k in range(5):
        a = 2 * math.pi * k / 5
        p = V((0.036, 0.075 * math.cos(a), 0.075 * math.sin(a)))
        geo.cylinder(bm, p, p + V((0.008, 0, 0)), 0.011, plastic, n=8)
    geo.cylinder(bm, V((-0.035, 0, 0)), V((-0.10, 0, 0)), 0.03, plastic, n=10)  # mount post
    tilt = math.radians(32)
    bmesh.ops.transform(bm, matrix=Matrix.Rotation(-tilt, 4, 'X') @ Matrix.Rotation(
        -math.pi / 2, 4, 'Z'), verts=bm.verts)
    bmesh.ops.translate(bm, vec=V((0, -0.78, 0.49)), verts=bm.verts)
    return core.obj_from_bm('spare', bm, weighted=False, angle=40)


spare_ = spare()

# --- decals: glass, louvres, chrome trim, door numbers ---------------------------------------
bm = core.new_bm()
geo.decal(bm, surf, geo.rrect(0.44, 0.09, 0.02, 3, 0.0, 0.628), V((0, 1.2, 0)), V((-1, 0, 0)),
          V((0, 0, 1)), mi('glass'), back=0.2)  # windshield
geo.decal(bm, surf, geo.rrect(0.26, 0.05, 0.018, 3, 0.0, 0.615), V((0, -1.2, 0)), V((1, 0, 0)),
          V((0, 0, 1)), mi('glass'), back=0.2)  # rear window
for s in (-1, 1):  # door windows (three-window coupe), thin pillars all round
    win = geo.rrect(0.31, 0.088, 0.022, 3, -0.19, 0.622)
    geo.decal(bm, surf, geo.ccw([V((s * p.x, p.y)) for p in win]), V((s * 2.0, 0, 0)),
              V((0, s, 0)), V((0, 0, 1)), mi('glass'), back=0.2)
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
geo.strip(bm, surf, [(0, 0.77), (0, 0.07)], 0.012, *top, chrome)  # hood centre trim
lid = geo.rrect(0.36, 0.20, 0.06, 3, 0.0, -0.64)  # deck lid shut line
geo.strip(bm, surf, lid + lid[:1], 0.005, *top, plastic, across=1)
for s in (-1, 1):
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
    for k in range(8):  # hood side louvres, raked back
        y = 0.12 + k * 0.03
        sl = [V((s * (y + dy), z)) for dy, z in ((-0.006, 0.43), (0.006, 0.43), (0.021, 0.51),
                                                   (0.009, 0.51))]
        geo.decal(bm, surf, geo.ccw(sl), *side, plastic, back=0.2)
    geo.strip(bm, surf, [(s * -0.66, BELT - 0.012), (s * 0.62, BELT - 0.025)], 0.01, *side,
              chrome, back=0.2)  # belt moulding
    for y in (-0.375, -0.005):  # door shut lines + handle
        geo.strip(bm, surf, [(s * y, 0.335), (s * y, BELT - 0.03)], 0.005, *side, plastic,
                  back=0.2, across=1)
    geo.strip(bm, surf, [(s * -0.08, 0.52), (s * -0.03, 0.52)], 0.009, *side, chrome, back=0.2,
              across=1)
    parts.number(bm, surf, '5', V((s * 2.0, -0.20, 0.45)), side[1], side[2], 0.075,
                 roundel=0.058)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

wheels = export.wheels(D, front=dict(tread='block', rim=dict(spokes=0, holes=8, depth=0.2,
                                                             hub=0.34), rim_ratio=0.56, rows=14))
ant_base = V((-0.24, -0.60, 0.53))
ant = parts.antenna(ant_base, 0.42)
export.finish(CAR, D, [body_shell, fenders, grille, hard, round_, spare_, decals], wheels, ant,
              ant_base)
