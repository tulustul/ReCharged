"""Hornet — Pro rally hatchback: tall yellow hatch with black roof and bonnet stripe, box
flares, roof scoop, roof-edge spoiler, white mud flaps behind every wheel, 4-lamp rally pod,
skid plate, gravel block tyres on white multi-spoke rims. Number 5.

    blender -b -P tools/blender/cars/hornet.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'hornet'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#EAA80A', accent='#1D1D20', rim='#F2F2F0', metal='#D9452B')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zW = D['radius'][0]
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']
ROOF = 0.648

# --- shell -----------------------------------------------------------------------------------
prof = shell.Profile([
    (-HL, dict(hw=0.50, z0=0.24, zb=0.43, hg=0.47, zg=0.435, hr=0.40, zt=0.46, rs=0.03)),
    (-0.87, dict(hw=0.53, z0=0.20, zb=0.48, hg=0.49, zg=0.485, hr=0.40, zt=0.50)),
    (-0.80, dict(zt=0.55)),
    (-0.70, dict(zt=0.61)),
    (-0.62, dict(zt=ROOF - 0.008)),
    (yR, dict(hw=HW, zb=0.565, hg=0.50, zg=0.535, hr=0.405, zt=ROOF - 0.004, bulge=0.005)),
    (-0.40, dict(hw=0.54, zb=0.505, hg=0.475, zg=0.51, hr=0.40)),
    (0.0, dict(hw=0.535, zb=0.495, hg=0.47, zg=0.50, hr=0.405, zt=ROOF, bulge=0.012)),
    (0.10, dict(zt=ROOF)),
    (0.42, dict(hw=0.54, zb=0.515, hg=0.48, zg=0.52, hr=0.40, zt=0.535)),
    (yF, dict(hw=HW, zb=0.565, hg=0.495, zg=0.54, hr=0.31, zt=0.528, bulge=0.005)),
    (0.82, dict(hw=0.53, z0=0.20, zb=0.485, hg=0.48, zg=0.48, hr=0.38, zt=0.478)),
    (HL, dict(hw=0.49, z0=0.24, zb=0.395, hg=0.45, zg=0.40, hr=0.38, zt=0.412, rs=0.03)),
    (0.3, dict(z0=CLR + 0.01)),
])
bm = core.new_bm()
shell.shell(bm, prof, -HL, HL, arches=shell.arches(D), ends=(0.06, 0.07), end_scale=0.7,
            step=0.05, roof=(-0.63, 0.13, 'paint_accent'))
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- glass, lamps, decals ------------------------------------------------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.14, 0.40), side=(-0.60, 0.36), rear=(-0.84, -0.645))
front = (V((-1, 0, 0)), V((0, 0, 1)))
rear = (V((1, 0, 0)), V((0, 0, 1)))
acc = mi('paint_accent')
geo.decal(bm, surf, geo.rrect(0.42, 0.075, 0.02, 3), V((0, 3, 0.355)), *front, mi('plastic'))
parts.lamps(bm, surf, geo.rrect(0.14, 0.07, 0.018, 3), 0.33, 0.36, True, 'light_head',
            bezel='chrome')
parts.lamps(bm, surf, geo.rrect(0.07, 0.11, 0.015, 3), 0.43, 0.43, False, 'light_tail')
parts.lamps(bm, surf, geo.rrect(0.07, 0.05, 0.015, 3), 0.43, 0.335, False, 'light_brake')
geo.decal(bm, surf, geo.rrect(0.62, 0.06, 0.02, 3), V((0, -3, 0.39)), *rear, acc)  # hatch band
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
geo.strip(bm, surf, [(0, 0.92), (0, 0.43)], 0.26, *top, acc, across=4)  # bonnet stripe
for x in (-0.21, 0.21):  # bonnet vents
    geo.decal(bm, surf, geo.rrect(0.07, 0.12, 0.02, 3, x, 0.58), *top, mi('plastic'))
for s in (-1, 1):
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
    parts.number(bm, surf, '5', V((s * 2.0, -0.05, 0.36)), side[1], side[2], 0.10,
                 plate=(0.14, 0.13))
    geo.strip(bm, surf, [(s * -0.33, 0.235), (s * 0.35, 0.235)], 0.05, *side, acc, back=0.2)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: roof scoop, spoiler, bumpers, skid plate, flaps, light pod, mirrors ---------
bm = core.new_bm()
geo.slab(bm, [(-0.22, 0.10, ROOF - 0.01, ROOF + 0.025, 0.02), (-0.01, 0.11, ROOF - 0.01,
                                                                 HH - 0.003, 0.025)], acc)
geo.box(bm, V((0, -0.005, HH - 0.026)), (0.17, 0.008, 0.035), mi('plastic'))  # scoop mouth
parts.wing(bm, V((0, -0.70, ROOF + 0.015)), 1.0, 0.16, thick=0.09, angle=-6, mat='paint_accent',
           plate=(0.07, 0.17), plate_mat='paint_accent')
for s in (-1, 1):
    parts.bar(bm, V((s * 0.30, -0.62, ROOF - 0.01)), V((s * 0.30, -0.68, ROOF + 0.008)), 0.04,
              0.014, mi('plastic'), up=V((1, 0, 0)))  # spoiler mounts
geo.slab(bm, [(0.86, 0.50, 0.19, 0.29, 0.03), (0.925, 0.47, 0.20, 0.28, 0.03)], mi('plastic'))
geo.slab(bm, [(-0.925, 0.48, 0.21, 0.30, 0.03), (-0.87, 0.51, 0.20, 0.30, 0.03)], mi('plastic'))
geo.extrude_x(bm, [(0.60, CLR + 0.005), (0.86, CLR + 0.03), (0.90, 0.21), (0.62, 0.20)],
              -0.30, 0.30, mi('metal'))  # front skid plate
geo.slab(bm, [(0.872, 0.30, 0.425, 0.465, 0.012), (0.892, 0.30, 0.425, 0.465, 0.012)],
         mi('plastic'))  # light pod bar
for s in (-1, 1):
    for y in (yF - 0.27, yR - 0.27):  # mud flaps behind every wheel
        geo.box(bm, V((s * 0.46, y, 0.19)), (0.17, 0.012, 0.18), mi('decal'))
        geo.box(bm, V((s * 0.46, y - 0.008, 0.125)), (0.17, 0.006, 0.03), mi('plastic'))
    geo.box(bm, V((s * 0.53, 0.37, 0.555)), (0.035, 0.07, 0.05), mi('paint_accent'))  # mirror
    parts.bar(bm, V((s * 0.48, 0.37, 0.54)), V((s * 0.515, 0.37, 0.552)), 0.018, 0.012,
              mi('plastic'))
hard = core.obj_from_bm('hard', bm, bevel=0.004, segments=1, angle=40)

# --- round parts: rally lamps, exhaust, suspension -------------------------------------------
bm = core.new_bm()
for x in (-0.24, -0.08, 0.08, 0.24):
    parts.light_pod(bm, V((x, 0.912, 0.445)), V((0, 1, 0)), 0.042, 0.04)
geo.cylinder(bm, V((0.30, -0.85, 0.22)), V((0.30, -0.922, 0.22)), 0.028, mi('chrome'), n=12)
geo.cylinder(bm, V((0.30, -0.89, 0.22)), V((0.30, -0.923, 0.22)), 0.02, mi('plastic'), n=12)
for s in (-1, 1):
    for y in (yF, yR):
        parts.a_arm(bm, (s * 0.15, y - 0.07, 0.19), (s * 0.15, y + 0.07, 0.19),
                    (s * 0.37, y, zW - 0.02), w=0.03, t=0.02)
        parts.shock(bm, (s * 0.33, y + 0.03, zW - 0.01), (s * 0.25, y + 0.05, 0.47), r=0.022,
                    spring=None)
        geo.box(bm, V((s * 0.38, y, zW)), (0.03, 0.07, 0.08), mi('plastic'))
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='block', rim=dict(spokes=8, spoke_w=0.15, depth=0.26),
                                     rim_ratio=0.6))
ant_base = V((-0.28, -0.50, ROOF))
ant = parts.antenna(ant_base, 0.40)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
