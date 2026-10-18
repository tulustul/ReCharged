"""Dumpster — Rookie monster truck: dumpster-green pickup cab with a dumpster-bin bed (black
lid, hazard chevrons), roof light bar, chrome bull bar and stacks, tubular frame, portal solid
axles with diffs, 4-links and eight long-travel coil-overs, huge chevron tyres on chrome
beadlock rims. Number 13.

    blender -b -P tools/blender/cars/dumpster.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'dumpster'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#257A34', accent='#F2BE12', rim='#DCDFE2', rim_metallic=1.0,
                   metal='#F2BE12')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zW = D['radius'][0]
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']
Z0 = 0.62  # body shell underside
ROOF = 1.066

# --- body shell: cab + dumpster bed --------------------------------------------------------------
prof = shell.Profile([
    (-HL, dict(hw=0.69, z0=Z0 + 0.02, zb=0.92, hg=0.66, zg=0.925, hr=0.62, zt=0.935, rs=0.03,
               crown=0.0)),
    (-1.0, dict(hw=0.715, zb=0.95, hg=0.69, zg=0.955, hr=0.64, zt=0.962)),
    (-0.14, dict(hw=0.715, zb=0.95, hg=0.69, zg=0.955, hr=0.64, zt=0.962, crown=0.0)),
    (-0.10, dict(hw=0.70, zb=0.90, hg=0.66, zg=0.905, hr=0.56, zt=1.00, crown=0.01)),
    (-0.06, dict(zt=ROOF)),
    (0.30, dict(hw=0.70, zb=0.88, hg=0.65, zg=0.885, hr=0.56, zt=ROOF, crown=0.012)),
    (0.40, dict(zt=ROOF - 0.01)),
    (0.58, dict(hw=0.705, zb=0.90, hg=0.66, zg=0.905, hr=0.58, zt=0.925)),
    (yF, dict(hw=HW, zb=0.92, hg=0.67, zg=0.925, hr=0.58, zt=0.92)),
    (0.98, dict(hw=0.70, zb=0.88, hg=0.66, zg=0.88, hr=0.58, zt=0.885)),
    (HL, dict(hw=0.66, z0=Z0 + 0.03, zb=0.80, hg=0.62, zg=0.805, hr=0.56, zt=0.815, rs=0.03)),
    (0.0, dict(z0=Z0, rb=0.035)),
])
bm = core.new_bm()
shell.shell(bm, prof, -HL, HL, arches=shell.arches(D, clear=0.035), ends=(0.05, 0.06),
            end_scale=0.75, step=0.075)
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- glass, lamps, decals -----------------------------------------------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.39, 0.56), side=(-0.06, 0.52), rear=None, m=0.025)
front = (V((-1, 0, 0)), V((0, 0, 1)))
rear = (V((1, 0, 0)), V((0, 0, 1)))
side_planes = [(V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1))) for s in (-1, 1)]
acc, plastic = mi('paint_accent'), mi('plastic')
geo.decal(bm, surf, geo.rrect(0.62, 0.11, 0.03, 3), V((0, 3, 0.735)), *front, plastic)  # grille
for x in (-0.22, -0.11, 0.0, 0.11, 0.22):  # grille bars
    geo.decal(bm, surf, geo.rrect(0.025, 0.09, 0.01, 2, x, 0.735), V((0, 3, 0)), *front,
              mi('chrome'), offset=0.005)
parts.lamps(bm, surf, geo.circle2(0.055, 20), 0.48, 0.74, True, 'light_head', bezel='chrome',
            bezel_w=0.01)
parts.lamps(bm, surf, geo.rrect(0.08, 0.10, 0.02, 3), 0.60, 0.80, False, 'light_tail')
parts.lamps(bm, surf, geo.rrect(0.08, 0.06, 0.02, 3), 0.60, 0.69, False, 'light_brake')
for k in range(6):  # hazard chevrons across the bin's back
    x = -0.45 + k * 0.18
    geo.decal(bm, surf, [(x - 0.05, 0.66), (x + 0.03, 0.66), (x + 0.08, 0.74), (x, 0.74)],
              V((0, -3, 0)), *rear, acc)
for (org, u, v), s in zip(side_planes, (-1, 1)):
    geo.decal(bm, surf, geo.rrect(0.82, 0.07, 0.02, 3, s * -0.58, 0.885), org, u, v, acc)
    for k in range(5):  # hazard chevrons on the bin sides
        y = -0.95 + k * 0.17
        geo.decal(bm, surf, [(s * (y - 0.03), 0.855), (s * (y + 0.03), 0.855),
                             (s * (y + 0.07), 0.915), (s * (y + 0.01), 0.915)], org, u, v,
                  plastic, offset=0.0055)
    parts.number(bm, surf, '13', org + V((0, 0.22, 0.79)), u, v, 0.10, roundel=0.085)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: bin lid, roof light bar, frame plates, bumpers, mirrors ----------------------
bm = core.new_bm()
geo.slab(bm, [(-1.085, 0.70, 0.96, 0.985, 0.012), (-0.15, 0.70, 0.96, 0.985, 0.012)],
         plastic)  # dumpster lid
for x in (-0.30, 0.30):  # lid handles
    geo.box(bm, V((x, -1.09, 0.95)), (0.12, 0.02, 0.02), plastic)
geo.slab(bm, [(0.355, 0.40, ROOF - 0.02, ROOF + 0.004, 0.01), (0.385, 0.40, ROOF - 0.02,
                                                                  ROOF + 0.004, 0.01)], plastic)  # light bar
geo.slab(bm, [(-0.98, 0.32, 0.47, 0.50, 0.012), (0.98, 0.32, 0.47, 0.50, 0.012)],
         plastic)  # skid / chassis plate
for s in (-1, 1):
    geo.box(bm, V((s * 0.705, 0.47, 0.95)), (0.035, 0.06, 0.10), plastic)  # truck mirrors
    parts.bar(bm, V((s * 0.66, 0.47, 0.905)), V((s * 0.69, 0.47, 0.92)), 0.02, 0.012, plastic)
hard = core.obj_from_bm('hard', bm, bevel=0.006, segments=1, angle=40)

# --- round parts: bull bar, stacks, light pods, frame, axles, links, shocks -----------------------
bm = core.new_bm()
chrome, metal = mi('chrome'), mi('metal')
geo.tube(bm, [(-0.50, 1.02, 0.62), (-0.50, 1.075, 0.66), (0.50, 1.075, 0.66),
              (0.50, 1.02, 0.62)], 0.022, chrome, n=8, corner=0.05)  # bull bar
geo.tube(bm, [(-0.30, 1.075, 0.66), (-0.30, 1.08, 0.80)], 0.02, chrome, n=8)
geo.tube(bm, [(0.30, 1.075, 0.66), (0.30, 1.08, 0.80)], 0.02, chrome, n=8)
geo.tube(bm, [(-0.30, 1.08, 0.80), (0.30, 1.08, 0.80)], 0.02, chrome, n=8)
for s in (-1, 1):  # exhaust stacks behind the cab
    geo.tube(bm, [(s * 0.55, -0.16, 0.90), (s * 0.55, -0.16, 1.03), (s * 0.55, -0.20, 1.065)],
             0.028, chrome, n=10, corner=0.04)
for x in (-0.32, -0.16, 0.0, 0.16, 0.32):
    parts.light_pod(bm, V((x, 0.41, ROOF + 0.01)), V((0, 1, 0)), 0.022, 0.03)
for s in (-1, 1):
    geo.tube(bm, [(s * 0.26, -0.98, 0.53), (s * 0.26, 0.98, 0.53)], 0.026, plastic, n=8)
for y in (yF, yR):
    geo.cylinder(bm, V((-0.52, y, 0.50)), V((0.52, y, 0.50)), 0.032, metal, n=12)  # portal axle
    geo.sphere(bm, V((0.10, y, 0.50)), 0.072, metal, n=14, rings=8)  # diff
    for s in (-1, 1):
        geo.box(bm, V((s * 0.47, y, 0.47)), (0.05, 0.11, 0.13), plastic)  # portal box
        dy = 0.06 if y > 0 else -0.06
        parts.shock(bm, (s * 0.43, y + dy, 0.47), (s * 0.30, y + dy * 4.0, 0.615), r=0.032,
                    spring='paint_accent', coils=4, body='chrome')
        tgt = y * 0.45
        parts.bar(bm, V((s * 0.30, y, 0.50)), V((s * 0.26, tgt, 0.53)), 0.035, 0.022, plastic)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='chevron', rim=dict(spokes=5, spoke_w=0.22, depth=0.36,
                                                                bolts=10, hub=0.26),
                                     rim_ratio=0.5, rows=14))
ant_base = V((-0.45, -0.30, 0.97))
ant = parts.antenna(ant_base, 0.45)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
