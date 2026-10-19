"""Volt GT — Elite mid-engine hypercar (P1-style): yellow body over black carbon, low pointed
nose with swept "smile" headlights and a black mouth, teardrop bubble canopy that tapers into a
black roof spine with a snorkel and runs back over the engine cover, cinched waist with huge
side scoops ahead of tall rear haunches, skeletal black mesh tail outlined by thin C-shaped
taillights, centre exhaust, big finned diffuser, active wing on pylons. Black side sweep, white
number plate 17. Dark grey 10-spoke rims, orange calipers, low-profile tyres.

    blender -b -P tools/blender/cars/volt_gt.py -- [--preview DIR]
"""
import math
import os
import sys

import bpy

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'volt_gt'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#F2B705', accent='#1E1E22', rim='#5E6268', rim_metallic=0.5,
                   metal='#8A8E94')
_b = bpy.data.materials['paint_accent'].node_tree.nodes['Principled BSDF']
_b.inputs['Roughness'].default_value = 0.6  # matte carbon: the dark canopy stays dark
_b.inputs['Coat Weight'].default_value = 0.0
core.material('caliper', '#F26A0F', 0.0, 0.35, coat=0.6)
core.material('mesh', '#0E0E10', 0.3, 0.6)
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']
acc, plastic, mesh, paint = mi('paint_accent'), mi('plastic'), mi('mesh'), mi('paint')

# --- shell: low nose, fender humps, cinched scooped waist, bubble canopy, haunches, cut tail ----
prof = shell.Profile([  # hg >= arch x_inner over the wells, so the fender tops lift cleanly
    (-HL, dict(hw=0.535, z0=0.13, zb=0.30, hg=0.42, zg=0.305, hr=0.09, zt=0.28, crown=0.006,
               rs=0.03, rg=0.03, bulge=0.0)),
    (-0.95, dict(hw=0.56, z0=0.11, zb=0.35, hg=0.41, zg=0.35, hr=0.10, zt=0.31)),
    (yR, dict(hw=HW, z0=0.074, zb=0.40, hg=0.40, zg=0.40, hr=0.12, zt=0.365, crown=0.008,
              bulge=0.0)),
    (-0.38, dict(hw=0.54, zb=0.36, hg=0.41, zg=0.36, hr=0.18, zt=0.40, bulge=-0.015)),
    (-0.22, dict(bulge=-0.035)),
    (-0.08, dict(hw=0.51, z0=CLR + 0.002, zb=0.31, hg=0.39, zg=0.32, hr=0.22, zt=0.43,
                 crown=0.012, bulge=-0.012, rr=0.08, rg=0.05)),
    (0.12, dict(zt=0.41, hr=0.23, bulge=0.0)),
    (0.38, dict(hw=0.53, zb=0.335, hg=0.41, zg=0.34, hr=0.24, zt=0.37, crown=0.008, rr=0.04,
                rg=0.03)),
    (yF, dict(hw=0.555, z0=0.075, zb=0.34, hg=0.41, zg=0.34, hr=0.17, zt=0.335, crown=0.006,
              bulge=0.0)),
    (0.92, dict(hw=0.51, z0=0.082, zb=0.26, hg=0.40, zg=0.26, hr=0.12, zt=0.26)),
    (HL, dict(hw=0.40, z0=0.085, zb=0.215, hg=0.30, zg=0.22, hr=0.08, zt=0.215, crown=0.004,
              bulge=0.0)),
])
bm = core.new_bm()
fs = shell.shell(bm, prof, -HL, HL, arches=shell.arches(D, clear=0.012), ends=(0.0, 0.08),
                 end_scale=0.72, step=0.05, roof=(-HL, HL, 'paint_accent'), cap=HH - 0.005,
                 min_thick=0.012)
L = 64  # ring length: 2 * (1 + 7 * (seg + 1) + shell.SUB + 1) - 2 for seg=3
for i, f in enumerate(fs):
    c = f.calc_center_median()
    if f.normal.y < -0.9 and c.y < -HL + 0.002:  # skeletal tail: the flat rear cut is mesh
        f.material_index = mesh
    elif len(f.verts) == 4 and min(i % L, L - 1 - i % L) >= 22 and -0.46 < c.y < 0.37:
        f.material_index = acc  # the whole bubble canopy is black (windows go on top)
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- decals: glass, lamps, carbon lower body, scoops, livery -----------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, side=(-0.38, 0.33), m=0.014)
TOP = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
screen = [(0.0, 0.08), (0.20, 0.09), (0.25, 0.20), (0.31, 0.355), (0.0, 0.37)]
screen = screen + [(-x, y) for x, y in reversed(screen[1:-1])]
geo.decal(bm, surf, geo.rounded_poly(screen, 0.04, 3), *TOP, mi('glass'))  # wrap windscreen
FRONT = (V((0, 3, 0)), V((-1, 0, 0)), V((0, 0, 1)))
REAR = (V((0, -3, 0)), V((1, 0, 0)), V((0, 0, 1)))
for s in (-1, 1):
    def fp(pts):  # world (x, z) of the s-side lamp -> front-plane coordinates
        return [V((-s * x, z)) for x, z in pts]

    housing = fp([(0.27, 0.245), (0.35, 0.212), (0.44, 0.212), (0.49, 0.235), (0.505, 0.285),
                  (0.45, 0.262), (0.36, 0.258), (0.30, 0.262)])
    geo.decal(bm, surf, geo.rounded_poly(housing, 0.012, 2), *FRONT, mesh)  # swept housing
    geo.strip(bm, surf, fp([(0.29, 0.252), (0.37, 0.247), (0.44, 0.252), (0.49, 0.272)]),
              0.012, *FRONT, mi('light_head'), offset=0.006, across=1)  # LED smile
    for x in (0.36, 0.41):
        geo.decal(bm, surf, geo.rrect(0.024, 0.014, 0.006, 2, -s * x, 0.228), *FRONT,
                  mi('light_head'), offset=0.006)  # projector dots
    geo.decal(bm, surf, geo.rrect(0.05, 0.10, 0.02, 2, s * 0.075, 0.84), *TOP, mesh)  # nostrils
    geo.decal(bm, surf, geo.rrect(0.04, 0.16, 0.018, 2, s * 0.43, 0.62), *TOP, mesh)  # fender
    geo.decal(bm, surf, geo.rrect(0.17, 0.12, 0.03, 3, s * 0.47, yR), *TOP, mesh)  # haunch vent
mouth = [(-0.40, 0.105), (0.40, 0.105), (0.40, 0.17), (0.30, 0.19), (0.20, 0.175),
         (-0.20, 0.175), (-0.30, 0.19), (-0.40, 0.17)]
geo.decal(bm, surf, geo.rounded_poly(mouth, 0.02, 2), *FRONT, mesh)  # black lower mouth
cover = [(0.0, -1.045), (0.085, -1.04), (0.14, -0.95), (0.23, -0.80), (0.30, -0.62),
         (0.335, -0.45), (0.33, -0.40)]
cover = cover + [(-x, y) for x, y in reversed(cover[1:])]
geo.decal(bm, surf, geo.rounded_poly(cover, 0.02, 2), *TOP, acc)  # engine-cover teardrop
# tail: thin C-shaped taillight outline around the mesh + centre brake bar
for s in (-1, 1):
    path = [(s * 0.06, 0.258), (s * 0.24, 0.266), (s * 0.39, 0.275), (s * 0.475, 0.26),
            (s * 0.50, 0.215), (s * 0.47, 0.165)]
    geo.strip(bm, surf, path, 0.018, *REAR, mi('light_tail'), across=1)
geo.strip(bm, surf, [(-0.06, 0.258), (0.06, 0.258)], 0.018, *REAR, mi('light_brake'), across=1)
for s in (-1, 1):
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))

    def sp(pts):  # (y, z) -> side-plane coordinates
        return [V((s * y, z)) for y, z in pts]

    lower = sp([(0.405, 0.095), (0.405, 0.205), (0.10, 0.215), (-0.15, 0.24), (-0.30, 0.28),
                (-0.395, 0.315), (-0.395, 0.095)])
    geo.decal(bm, surf, geo.rounded_poly(lower, 0.03, 3), *side, acc, h=0.02)  # carbon lower
    scoop = sp([(-0.06, 0.235), (-0.20, 0.205), (-0.32, 0.225), (-0.385, 0.27), (-0.385, 0.305),
                (-0.30, 0.28), (-0.17, 0.25)])
    geo.decal(bm, surf, geo.rounded_poly(scoop, 0.02, 3), *side, mesh, offset=0.005, h=0.015)
    blade = sp([(0.38, 0.12), (0.05, 0.12), (-0.18, 0.15), (-0.31, 0.205), (-0.383, 0.30)])
    geo.decal(bm, surf, geo.rounded_poly(sp([(-0.925, 0.12), (-0.925, 0.29), (-1.045, 0.265),
                                             (-1.045, 0.155)]), 0.02, 2), *side, acc)  # tail
    geo.decal(bm, surf, geo.rounded_poly(sp([(0.905, 0.095), (0.905, 0.17), (0.99, 0.16),
                                             (0.99, 0.11)]), 0.015, 2), *side, acc)  # nose
    geo.strip(bm, surf, [V(p) for p in blade], 0.024, *side, paint, offset=0.006)
    parts.number(bm, surf, '17', V((s * 2.0, 0.17, 0.17)), side[1], side[2], 0.06,
                 plate=(0.13, 0.085), offset=0.007)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- wing (own part, no bevel on the thin foil) + hard parts: pylons, splitter, skirts, diffuser
bm = core.new_bm()
WY, WZ = -0.948, 0.425
parts.wing(bm, V((0, WY, WZ)), 1.08, 0.17, thick=0.14, angle=-12, mat='paint_accent',
           plate=(0.12, 0.19), plate_mat='paint_accent')
wing = core.obj_from_bm('wing', bm, weighted=False, angle=30)  # thin foil: no bevel
bm = core.new_bm()
for s in (-1, 1):
    parts.bar(bm, V((s * 0.20, -0.90, 0.29)), V((s * 0.20, WY - 0.01, WZ - 0.005)), 0.05, 0.014,
              acc, up=V((1, 0, 0)), taper=0.7)  # pylons
geo.slab(bm, [(0.86, 0.50, CLR, CLR + 0.018, 0.008), (1.045, 0.40, CLR, CLR + 0.018, 0.008)],
         acc)  # front splitter
for s in (-1, 1):
    geo.box(bm, V((s * 0.505, 0.0, CLR + 0.014)), (0.05, 0.78, 0.028), acc)  # side skirts
    geo.slab(bm, [(0.25, 0.022, 0.348, 0.372, 0.01), (0.30, 0.032, 0.342, 0.38, 0.014)], acc,
             cx=s * 0.505)  # mirrors on stalks
    parts.bar(bm, V((s * 0.43, 0.29, 0.335)), V((s * 0.49, 0.28, 0.36)), 0.016, 0.01, acc)
for x in (-0.38, -0.24, -0.10, 0.10, 0.24, 0.38):  # diffuser fins
    geo.extrude_x(bm, [(-0.62, 0.072), (-1.045, 0.075), (-1.045, 0.20), (-0.62, 0.085)],
                  x - 0.009, x + 0.009, plastic)
geo.slab(bm, [(-1.0, 0.075, 0.185, 0.25, 0.028), (-1.054, 0.075, 0.185, 0.25, 0.028)],
         mi('chrome'))  # centre exhaust
geo.slab(bm, [(-1.0, 0.06, 0.197, 0.238, 0.02), (-1.055, 0.06, 0.197, 0.238, 0.02)], mesh)
geo.slab(bm, [(-0.30, 0.04, 0.40, 0.43, 0.012), (-0.06, 0.055, 0.405, 0.456, 0.02)],
         acc)  # roof snorkel
hard = core.obj_from_bm('hard', bm, bevel=0.004, segments=1, angle=40)

# --- round parts: calipers, suspension -------------------------------------------------------------
bm = core.new_bm()
for i in range(5):  # tail mesh: grille of thin bars over the dark rear cut
    geo.box(bm, V((0, -1.0525, 0.150 + i * 0.022)), (0.84, 0.005, 0.006), plastic)
for i in range(11):
    geo.box(bm, V((-0.40 + i * 0.08, -1.0525, 0.194)), (0.006, 0.005, 0.10), plastic)


def caliper(c, s, r0, r1, x0, x1):
    """Arc-shaped brake caliper inside the rim (upper rear quadrant), on the body."""
    pts = []
    for r, a_s in ((r1, range(9)), (r0, range(8, -1, -1))):
        for i in a_s:
            a = math.radians(100 + 70 * i / 8)
            pts.append((c.y + r * math.cos(a), c.z + r * math.sin(a)))
    xa, xb = c.x + s * x0, c.x + s * x1
    geo.extrude_x(bm, pts, min(xa, xb), max(xa, xb), mi('caliper'))


for s in (-1, 1):
    for y, z, t, w in ((yF, zF, D['track'][0], D['wheel_width'][0]),
                       (yR, zR, D['track'][1], D['wheel_width'][1])):
        c = V((s * t / 2, y, z))
        caliper(c, s, 0.072, 0.118, -0.002, 0.16 * w)
        parts.a_arm(bm, (s * 0.15, y - 0.07, 0.11), (s * 0.15, y + 0.07, 0.11),
                    (s * (t / 2 - 0.12), y, z - 0.03), w=0.028, t=0.018)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='street', rim=dict(
    spokes=10, spoke_w=0.14, depth=0.42, spoke_d=0.45, hub=0.26, taper=0.7), rim_ratio=0.70))
hit, _ = surf.project(V((-0.30, -0.80, 1.0)), V((0, 0, -1)), 0.0, back=0.0)
ant_base = V((-0.30, -0.80, hit.z - 0.005))
ant = parts.antenna(ant_base, 0.40)
export.finish(CAR, D, [body_shell, decals, hard, wing, round_], wheels, ant, ant_base)
