"""Dumpster — Rookie monster truck: red/black short-bed pickup sitting high on a long-travel
4x4 chassis. Tall boxy cab with window nets, LED headlights over a mesh grille, black bumpers
with a light bar and skid plates, black fender flares and rockers, sharp black/grey livery
slashes, ribbed bed cover, black A-arms with red coil-overs angled to the towers, huge chevron
tyres on black multi-spoke rims with a red ring.

    blender -b -P tools/blender/cars/dumpster.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

import bmesh  # noqa: E402
from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'dumpster'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#BE1621', accent='#1D1D20', rim='#1A1A1D', metal='#C8202A')
core.material('trim', '#9A9DA3', 0.55, 0.38)  # silver livery slashes, window nets, grille mesh
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zW = D['radius'][0]
CLR = D['clearance']
HL, HW = D['length'] / 2, D['width'] / 2
YN, YT = 1.05, -1.05  # shell nose / tail (bumpers reach the full length)
ARCHES = shell.arches(D, clear=0.025)
R_ARCH = ARCHES[0][2]
plastic, acc, trim = mi('plastic'), mi('paint_accent'), mi('trim')

# --- body shell: bed + tall boxy cab + flat hood --------------------------------------------------
BED = dict(hw=0.64, hb=0.625, zb=0.865, hg=0.62, zg=0.875, hr=0.61, zt=0.885, crown=0.0,
           rs=0.02, rg=0.012, rr=0.015, rb=0.03, rm=0.05)
prof = shell.Profile([
    (YT, dict(BED, z0=0.67)),
    (-0.21, dict(BED)),
    (-0.25, dict(z0=0.56)),
    (0.25, dict(z0=0.56)),
    (-0.165, dict(zb=0.885, hg=0.60, zg=0.90, hr=0.52, zt=1.085, crown=0.012, rr=0.04)),
    (0.13, dict(zt=1.085, hr=0.52, crown=0.012)),
    (0.45, dict(zb=0.875, hg=0.62, zg=0.885, hr=0.59, zt=0.90, crown=0.012, rr=0.03)),
    (YN, dict(hw=0.64, hb=0.625, z0=0.70, zb=0.86, hg=0.62, zg=0.87, hr=0.59, zt=0.885,
              crown=0.01)),
])
bm = core.new_bm()
shell.shell(bm, prof, YT, YN, arches=ARCHES, ends=(0.03, 0.04), end_scale=0.85, step=0.085,
            seg=2,
            extra=(-0.20, -0.185, -0.165, -0.145))
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=45)
surf = geo.Surface([body_shell])

# --- glass, nets, lamps, livery (conformal decals) ------------------------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.16, 0.425), side=(-0.135, 0.40), rear=None, m=0.024)
FRONT = (V((0, 3, 0)), V((-1, 0, 0)), V((0, 0, 1)))
REAR = (V((0, -3, 0)), V((1, 0, 0)), V((0, 0, 1)))
TOP = (V((0, 0, 3)), V((1, 0, 0)), V((0, 1, 0)))


def arc(y_c, y_a, y_b, r=R_ARCH + 0.02, n=10):
    """Side-plane points (y, z) along an arch circle from y_a to y_b (decal edges under flares)."""
    ys = [y_a + (y_b - y_a) * i / n for i in range(n + 1)]
    return [(y, zW + math.sqrt(max(0.0, r * r - (y - y_c) ** 2))) for y in ys]


geo.decal(bm, surf, geo.rrect(0.74, 0.10, 0.025, 2, 0, 0.99), V((0, -1.2, 0)), *REAR[1:],
          mi('glass'), h=0.05)  # rear window
for s in (-1, 1):
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))

    def sp(pts):  # side-plane polygon from world (y, z)
        return [V((s * y, z)) for y, z in pts]

    geo.decal(bm, surf, sp([(-0.035, 0.90), (0.005, 0.90), (0.005, 1.08), (-0.035, 1.08)]),
              *side, mi('paint'), offset=0.005)  # B-pillar splits the side glass
    for y in (0.035, 0.08, 0.125, 0.17):  # window net (front door window)
        geo.strip(bm, surf, [(s * y, 0.925), (s * (y + 0.02), 1.04)], 0.007, *side, trim,
                  offset=0.0055, across=1)
    for z in (0.95, 0.99, 1.03):
        geo.strip(bm, surf, [(s * 0.02, z), (s * 0.20, z)], 0.007, *side,
                  trim, offset=0.006, across=1)
    # black lower door + bed side (bottom edge follows the rear arch under the flare)
    geo.decal(bm, surf, sp([(0.27, 0.60), (0.12, 0.665), (-0.05, 0.78), (-0.30, 0.85),
                            (YT + 0.02, 0.85)] + arc(yR, YT + 0.02, -0.246)
                           + [(-0.26, 0.60)]), *side, acc, h=0.05)
    geo.decal(bm, surf, sp([(-0.31, 0.853), (-0.80, 0.853), (-0.33, 0.77)]), *side,
              mi('paint'), offset=0.0045)  # red spike into the bed side
    geo.decal(bm, surf, sp([(0.76, 0.878), (YN - 0.02, 0.868)] + arc(yF, YN - 0.02, 0.76)),
              *side, acc)  # dark front fender corner
    geo.strip(bm, surf, [(s * 0.30, 0.665), (s * 0.12, 0.735), (s * -0.06, 0.84),
                         (s * -0.20, 0.885)], 0.022, *side, trim, offset=0.0045, across=1)
    geo.strip(bm, surf, [(s * -0.42, 0.80), (s * -0.98, 0.815)], 0.016, *side, trim,
              offset=0.0045, across=1)
    geo.strip(bm, surf, [(s * 0.30, 0.79), (s * 0.18, 0.835), (s * 0.05, 0.875)], 0.014, *side,
              acc, offset=0.0045, across=1)
# front: black fascia, grille mesh bars, LED headlights
geo.decal(bm, surf, [(0.56, 0.705), (0.56, 0.857), (-0.56, 0.857), (-0.56, 0.705)], *FRONT,
          plastic, h=0.06)
for z in (0.73, 0.758, 0.786):
    geo.decal(bm, surf, geo.rrect(0.56, 0.012, 0.005, 1, 0, z), *FRONT, trim, offset=0.005, h=0.1)
geo.decal(bm, surf, geo.rrect(0.10, 0.03, 0.01, 2, 0, 0.758), *FRONT, acc, offset=0.0065)
parts.lamps(bm, surf, [(-0.09, -0.01), (0.07, -0.03), (0.08, 0.02), (-0.08, 0.025)], 0.43,
            0.822, True, 'light_head', bezel='trim', bezel_w=0.008)
# rear: tail lights, tailgate stripes, handle
parts.lamps(bm, surf, geo.rrect(0.08, 0.17, 0.015, 2), 0.505, 0.775, False, 'light_tail',
            bezel='plastic', bezel_w=0.012)
parts.lamps(bm, surf, geo.rrect(0.06, 0.045, 0.01, 2), 0.505, 0.83, False, 'light_brake')
geo.decal(bm, surf, [(-0.42, 0.685), (0.42, 0.685), (0.42, 0.87), (-0.42, 0.87)], *REAR, acc,
          h=0.06)  # black tailgate
for x, w in ((-0.16, 0.05), (0.16, 0.05), (-0.27, 0.025), (0.27, 0.025)):
    lean = math.copysign(0.02, x)
    geo.decal(bm, surf, [(x - w / 2, 0.685), (x + w / 2, 0.685), (x + w / 2 + lean, 0.87),
                         (x - w / 2 + lean, 0.87)], *REAR, mi('paint'), offset=0.005,
              h=0.06)
geo.decal(bm, surf, geo.rrect(0.16, 0.03, 0.01, 2, 0, 0.83), *REAR, trim, offset=0.006)
# top: black roof panel, hood arrowhead with side spikes
geo.decal(bm, surf, geo.rrect(0.46, 0.27, 0.03, 2, 0, -0.015), *TOP, acc, h=0.06)
geo.decal(bm, surf, [(0, 0.98), (-0.13, 0.74), (-0.17, 0.46), (0.17, 0.46), (0.13, 0.74)],
          *TOP, acc, h=0.06)
for s in (-1, 1):
    geo.decal(bm, surf, [(s * 0.22, 0.46), (s * 0.31, 0.46), (s * 0.21, 0.92)], *TOP, acc, h=0.06)
    geo.decal(bm, surf, [(s * 0.33, 0.46), (s * 0.37, 0.46), (s * 0.30, 0.78)], *TOP, trim, h=0.06)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- fender flares: rounded bands following each arch ---------------------------------------------
bm = core.new_bm()


def flare(bm, y_c, a0, a1, x0=0.60, x1=HW, r0=R_ARCH - 0.004, th=0.036, n=14):
    """Band of rounded-rect section (x0..x1 across, th radial) around the arch from angle a0
    to a1 (degrees, 0 = toward +Y, 90 = up), both sides."""
    for s in (-1, 1):
        rings = []
        for k in range(n + 1):
            a = math.radians(a0 + (a1 - a0) * k / n)
            d = V((0, math.cos(a), math.sin(a)))
            sec = geo.rrect(x1 - x0, th, 0.016, 1, (x0 + x1) / 2, r0 + th / 2)
            rings.append([V((s * q.x, y_c, zW)) + d * q.y for q in sec])
        geo.loft(bm, rings, plastic)


def arch_angle(y_end, y_c):
    return math.degrees(math.acos(max(-1.0, min(1.0, (y_end - y_c) / (R_ARCH + 0.03)))))


z_door = math.degrees(math.asin((0.56 - zW) / R_ARCH))
flare(bm, yF, arch_angle(YN - 0.01, yF), 180 - z_door)
flare(bm, yR, z_door, arch_angle(YT + 0.01, yR))
flares = core.obj_from_bm('flares', bm, weighted=False, angle=50)

# --- panels (already rounded, no bevel): chassis tub, towers, bumpers, skids, rockers, bed cover --
pb = core.new_bm()
geo.slab(pb, [(-0.90, 0.13, CLR + 0.03, CLR + 0.07, 0.02), (-0.70, 0.17, CLR, CLR + 0.06, 0.02),
              (0.70, 0.17, CLR, CLR + 0.06, 0.02), (0.90, 0.13, CLR + 0.03, CLR + 0.07, 0.02)],
         plastic, seg=2)  # chassis tub
for y in (yF, yR):
    ty = y - 0.08 if y > 0 else y + 0.08
    geo.extrude_y(pb, geo.rounded_poly([(-0.12, 0.44), (0.12, 0.44), (0.35, 0.70), (0.35, 0.77),
                                        (-0.35, 0.77), (-0.35, 0.70)],
                                       [0, 0, 0.03, 0.02, 0.02, 0.03], 2),
                  ty - 0.012, ty + 0.012, plastic)  # shock tower
for y0, y1, sgn in ((0.99, HL, 1), (-0.99, -HL, -1)):  # bumpers: bar, wrap ends, mounts, skid
    yo = y1 - sgn * 0.005
    poly = geo.rounded_poly([(y0, 0.47), (yo, 0.49), (y1, 0.545), (yo, 0.60), (y0, 0.615)],
                            [0, 0.02, 0.02, 0.02, 0], 2)
    geo.extrude_x(pb, [(p.x, p.y) for p in poly], -0.30, 0.30, plastic)
    for s in (-1, 1):
        parts.bar(pb, V((s * 0.10, y0, 0.53)), V((s * 0.10, sgn * 0.80, 0.47)), 0.05, 0.03,
                  plastic, up=V((0, 0, 1)))
    parts.bar(pb, V((0, y1 - sgn * 0.03, 0.475)), V((0, sgn * 0.80, 0.415)), 0.36, 0.022,
              plastic)
for s in (-1, 1):
    geo.slab(pb, [(-0.235, 0.045, 0.55, 0.63, 0.015), (0.235, 0.045, 0.55, 0.63, 0.015)],
             plastic, seg=2, cx=s * 0.66)  # rocker panels between the flares
    parts.bar(pb, V((s * 0.61, 0.43, 0.915)), V((s * 0.65, 0.42, 0.94)), 0.025, 0.014, plastic)
geo.slab(pb, [(-1.03, 0.52, 0.87, 0.902, 0.012), (-0.23, 0.52, 0.87, 0.902, 0.012)],
         plastic, seg=2)  # bed cover
panels = core.obj_from_bm('panels', pb, weighted=False, angle=40)

# --- hard boxes (bevelled): gearboxes, hub carriers, light bar, hitch, mirrors, ribs ------------
bm = core.new_bm()
for y in (yF, yR):
    geo.box(bm, V((0, y, 0.48)), (0.26, 0.20, 0.15), plastic)  # diff / gearbox
    for s in (-1, 1):
        geo.box(bm, V((s * 0.46, y, 0.49)), (0.04, 0.08, 0.13), plastic)  # hub carrier
geo.box(bm, V((0, HL - 0.009, 0.545)), (0.42, 0.012, 0.05), plastic)  # light-bar housing
geo.box(bm, V((0, HL - 0.003, 0.545)), (0.38, 0.006, 0.026), mi('light_head'))
geo.box(bm, V((0, -1.075, 0.465)), (0.07, 0.05, 0.03), mi('metal'))  # tow hitch
for s in (-1, 1):  # red tow hooks
    geo.box(bm, V((s * 0.22, HL - 0.03, 0.475)), (0.035, 0.06, 0.03), mi('metal'))
for s in (-1, 1):
    geo.box(bm, V((s * 0.665, 0.41, 0.955)), (0.05, 0.08, 0.065), plastic)  # mirrors
for k in range(8):  # bed cover ribs
    geo.box(bm, V((-0.42 + k * 0.12, -0.63, 0.906)), (0.03, 0.74, 0.016), plastic)
geo.box(bm, V((0, -0.235, 0.925)), (0.92, 0.035, 0.05), plastic)  # bed headache rail
hard = core.obj_from_bm('hard', bm, bevel=0.006, segments=1, angle=40)

# --- round parts: A-arms, coil-overs, driveshafts -------------------------------------------------
bm = core.new_bm()
for y in (yF, yR):
    sgn = 1 if y > 0 else -1
    for s in (-1, 1):
        parts.a_arm(bm, (s * 0.13, y + 0.09, 0.45), (s * 0.13, y - 0.09, 0.45),
                    (s * 0.46, y, 0.435), w=0.042, t=0.026)
        parts.a_arm(bm, (s * 0.15, y + 0.06, 0.60), (s * 0.15, y - 0.06, 0.60),
                    (s * 0.45, y, 0.555), w=0.03, t=0.02)
        parts.shock(bm, (s * 0.44, y + sgn * 0.03, 0.465), (s * 0.22, y - sgn * 0.03, 0.76),
                    r=0.036, spring='metal', coils=4, body='metal')
        geo.cylinder(bm, V((s * 0.12, y, 0.48)), V((s * 0.46, y, 0.47)), 0.014, mi('chrome'), n=8)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

# --- wheels: chevron tyres, black 10-spoke rims + red ring ----------------------------------------
RIM_RATIO = 0.55
wheels = export.wheels(D, front=dict(tread='chevron', rim_ratio=RIM_RATIO, rows=13, n=28,
                                     rim=dict(spokes=6, spoke_w=0.24, depth=0.34, hub=0.26,
                                              taper=0.7, bolts=0)))
R, W = D['radius'][0], D['wheel_width'][0]
rb, xf = RIM_RATIO * R, W / 2 - 0.10 * W
for w in wheels:  # red anodised ring on the rim lip
    ring = core.new_bm()
    prof_r = [(xf - 0.002, rb + 0.006), (xf + 0.010, rb + 0.006), (xf + 0.010, rb - 0.028),
              (xf - 0.002, rb - 0.028)]
    geo.revolve_x(ring, prof_r + prof_r[:1], mi('metal'), n=28)
    if w.name[7] == 'l':
        bmesh.ops.scale(ring, vec=(-1, 1, 1), verts=ring.verts)
        bmesh.ops.reverse_faces(ring, faces=ring.faces)
    ring.from_mesh(w.data)
    ring.to_mesh(w.data)
    ring.free()

ant_base = V((-0.45, -0.97, 0.902))
ant = parts.antenna(ant_base, 0.45)
export.finish(CAR, D, [body_shell, decals, flares, panels, hard, round_], wheels, ant, ant_base)
