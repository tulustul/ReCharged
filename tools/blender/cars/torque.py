"""Torque — Pro muscle car: candy-red coke-bottle fastback with rear haunches over fat rear
tyres, chrome supercharger + scoop through the long hood, quad headlights, chrome bumpers,
ducktail, twin pipes, white twin stripes, chrome 5-spoke rims on white-letter tyres. Number 88.

    blender -b -P tools/blender/cars/torque.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'torque'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#C8161E', accent='#F4F4F0', rim='#E9E9EA', rim_metallic=1.0,
                   metal='#C9CDD2')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']

# --- shell -----------------------------------------------------------------------------------
prof = shell.Profile([
    (-HL, dict(hw=0.52, z0=0.16, zb=0.44, hg=0.48, zg=0.445, hr=0.40, zt=0.462, rs=0.03)),
    (-0.95, dict(hw=0.555, z0=0.12, zb=0.50, hg=0.51, zg=0.505, hr=0.40, zt=0.50)),
    (yR, dict(hw=HW, zb=0.585, hg=0.49, zg=0.55, hr=0.29, zt=0.515, bulge=0.006)),
    (-0.45, dict(zt=0.522)),
    (-0.19, dict(hw=0.55, zb=0.47, hg=0.48, zg=0.475, hr=0.37, zt=0.585, crown=0.01)),
    (0.05, dict(hw=0.545, zb=0.445, hg=0.47, zg=0.45, hr=0.375, zt=HH - 0.014, bulge=0.012)),
    (0.15, dict(zt=HH - 0.016)),
    (0.36, dict(hw=0.55, zb=0.47, hg=0.49, zg=0.475, hr=0.40, zt=0.478)),
    (yF, dict(hw=0.565, zb=0.54, hg=0.50, zg=0.505, hr=0.30, zt=0.48, bulge=0.004)),
    (0.95, dict(hw=0.55, z0=0.12, zb=0.45, hg=0.49, zg=0.445, hr=0.42, zt=0.44)),
    (HL, dict(hw=0.50, z0=0.15, zb=0.34, hg=0.45, zg=0.345, hr=0.40, zt=0.355, rs=0.03)),
    (0.0, dict(z0=0.11)),
])
bm = core.new_bm()
shell.shell(bm, prof, -HL, HL, arches=shell.arches(D), ends=(0.07, 0.07),
            end_scale=0.66, step=0.055)
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- glass, lamps, decals ------------------------------------------------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.17, 0.34), side=(-0.30, 0.31), rear=(-0.43, -0.21))
front = (V((-1, 0, 0)), V((0, 0, 1)))
geo.decal(bm, surf, geo.rrect(0.86, 0.11, 0.035, 3), V((0, 3, 0.275)), *front, mi('plastic'))
for x in (0.20, 0.31):  # quad round headlights with chrome bezels
    parts.lamps(bm, surf, geo.circle2(0.034, 20), x, 0.275, True, 'light_head', bezel='chrome',
                bezel_w=0.007)
rear = (V((1, 0, 0)), V((0, 0, 1)))
geo.decal(bm, surf, geo.rrect(0.92, 0.075, 0.025, 3), V((0, -3, 0.385)), *rear, mi('plastic'))
parts.lamps(bm, surf, geo.rrect(0.15, 0.05, 0.018, 3), 0.33, 0.385, False, 'light_tail')
parts.lamps(bm, surf, geo.rrect(0.11, 0.05, 0.018, 3), 0.17, 0.385, False, 'light_brake')
acc = mi('paint_accent')
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
for x in (-0.075, 0.075):  # twin stripes: nose to tail, gaps over glass and the blower
    for y0, y1 in ((1.06, 0.71), (0.39, 0.355), (0.16, -0.205), (-0.44, -1.06)):
        geo.strip(bm, surf, [(x, y0), (x, y1)], 0.085, *top, acc, back=0.3)
for s in (-1, 1):
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
    parts.number(bm, surf, '88', V((s * 2.0, -0.02, 0.32)), side[1], side[2], 0.10,
                 roundel=0.085)
    geo.strip(bm, surf, [(s * -0.36, 0.17), (s * 0.38, 0.17)], 0.03, *side, acc, back=0.2)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: blower + scoop, bumpers, ducktail, mirrors -----------------------------------
bm = core.new_bm()
geo.slab(bm, [(0.40, 0.15, 0.45, 0.50, 0.02), (0.70, 0.15, 0.44, 0.49, 0.02)], mi('plastic'))
geo.slab(bm, [(0.42, 0.125, 0.48, 0.55, 0.03), (0.68, 0.125, 0.48, 0.55, 0.03)],
         mi('chrome'))  # supercharger body
for x in (-0.08, -0.027, 0.027, 0.08):  # blower ribs
    geo.box(bm, V((x, 0.55, 0.553)), (0.014, 0.24, 0.012), mi('chrome'))
geo.slab(bm, [(0.46, 0.11, 0.55, 0.598, 0.015), (0.63, 0.11, 0.55, 0.588, 0.015)],
         mi('plastic'))  # butterfly scoop
geo.box(bm, V((0, 0.632, 0.571)), (0.19, 0.006, 0.03), mi('chrome'))  # scoop throat
geo.slab(bm, [(1.03, 0.50, 0.16, 0.225, 0.03), (1.072, 0.48, 0.165, 0.22, 0.025)],
         mi('chrome'))  # front bumper
geo.slab(bm, [(-1.072, 0.48, 0.17, 0.235, 0.025), (-1.03, 0.50, 0.165, 0.24, 0.03)],
         mi('chrome'))  # rear bumper
geo.extrude_x(bm, [(-0.99, 0.455), (-1.068, 0.46), (-1.073, 0.49), (-1.035, 0.49)], -0.47, 0.47,
              acc)  # ducktail lip
for s in (-1, 1):
    geo.box(bm, V((s * 0.545, 0.30, 0.485)), (0.04, 0.05, 0.03), mi('chrome'))  # mirror
    parts.bar(bm, V((s * 0.49, 0.30, 0.462)), V((s * 0.53, 0.30, 0.48)), 0.016, 0.01,
              mi('chrome'))
hard = core.obj_from_bm('hard', bm, bevel=0.004, angle=40)

# --- round parts: exhausts, suspension in the arches -----------------------------------------
bm = core.new_bm()
for s in (-1, 1):
    geo.cylinder(bm, V((s * 0.30, -0.98, 0.14)), V((s * 0.30, -1.07, 0.14)), 0.032,
                 mi('chrome'), n=14)
    geo.cylinder(bm, V((s * 0.30, -1.04, 0.14)), V((s * 0.30, -1.071, 0.14)), 0.024,
                 mi('plastic'), n=14)
    for y, z in ((yF, zF), (yR, zR)):
        parts.a_arm(bm, (s * 0.16, y - 0.07, 0.15), (s * 0.16, y + 0.07, 0.15),
                    (s * 0.40, y, z - 0.03), w=0.03, t=0.02)
        geo.box(bm, V((s * 0.405, y, z)), (0.03, 0.07, 0.08), mi('plastic'))
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='street', rim=dict(spokes=5, spoke_w=0.26, depth=0.28),
                                     rim_ratio=0.62,
                                     letter_ring='decal'),
                       rear=dict(tread='street', rim=dict(spokes=5, spoke_w=0.26, depth=0.32),
                                 rim_ratio=0.62,
                                 letter_ring='decal'))
ant_base = V((-0.30, -0.80, 0.50))
ant = parts.antenna(ant_base, 0.45)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
