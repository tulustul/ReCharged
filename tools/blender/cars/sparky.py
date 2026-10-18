"""Sparky — Rookie off-road buggy: slim blue cowl, yellow roll cage with light bar, exposed
rear drivetrain, big rear wing, long-travel arms, knobbly tyres. Number 7.

    blender -b -P tools/blender/cars/sparky.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'sparky'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#1E5EE6', accent='#F2C318', rim='#F2F2F0', metal='#E8A93A')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']

# --- hard parts (bevelled): tub, guards, towers, gearbox, bumpers, wing, light housings -------
bm = core.new_bm()
geo.slab(bm, [(-0.84, 0.13, CLR, CLR + 0.05, 0.02), (-0.5, 0.17, CLR, CLR + 0.055, 0.02),
              (0.45, 0.17, CLR, CLR + 0.055, 0.02), (0.66, 0.14, CLR + 0.02, CLR + 0.07, 0.02),
              (0.88, 0.10, CLR + 0.09, CLR + 0.13, 0.02)], mi('plastic'))
for s in (-1, 1):  # side guards (nerf panels) protect the arms
    geo.box(bm, V((s * 0.27, -0.02, CLR + 0.035)), (0.17, 0.62, 0.03), mi('plastic'))
y, top, w = yR + 0.06, 0.56, 0.23  # rear shock tower plate (hidden behind the wing)
geo.extrude_y(bm, geo.rounded_poly([(-0.08, CLR + 0.04), (0.08, CLR + 0.04), (w, top),
                                    (w - 0.03, top + 0.02), (0.03 - w, top + 0.02), (-w, top)],
                                   [0, 0, 0.015, 0.02, 0.02, 0.015], 2),
              y - 0.012, y + 0.012, mi('plastic'))
for s in (-1, 1):  # front shock mounts: A-frame posts + crossbar, light and open
    parts.bar(bm, V((s * 0.07, yF - 0.04, CLR + 0.05)), V((s * 0.21, yF - 0.04, 0.50)), 0.035,
              0.02, mi('plastic'), up=V((0, 1, 0)))
parts.bar(bm, V((-0.22, yF - 0.04, 0.49)), V((0.22, yF - 0.04, 0.49)), 0.035, 0.02,
          mi('plastic'), up=V((0, 1, 0)))
geo.slab(bm, [(-0.76, 0.10, CLR + 0.03, CLR + 0.17, 0.04), (-0.56, 0.12, CLR + 0.03,
                                                            CLR + 0.19, 0.04)], mi('plastic'))
geo.box(bm, V((0, 0.90, 0.30)), (0.30, 0.10, 0.03), mi('plastic'))  # front skid
geo.slab(bm, [(-0.985, 0.17, 0.27, 0.33, 0.025), (-0.93, 0.18, 0.27, 0.33, 0.025)],
         mi('plastic'))  # rear bumper
for s in (-1, 1):
    geo.box(bm, V((s * 0.475, yF, zF)), (0.04, 0.08, 0.10), mi('plastic'))  # hub carriers
    geo.box(bm, V((s * 0.475, yR, zR)), (0.04, 0.08, 0.10), mi('plastic'))
    geo.box(bm, V((s * 0.19, 0.955, 0.375)), (0.05, 0.03, 0.03), mi('plastic'))  # lamp mounts
    parts.bar(bm, V((s * 0.07, -0.70, 0.43)), V((s * 0.09, -0.84, 0.645)), 0.03, 0.012,
              mi('plastic'), up=V((1, 0, 0)))  # wing stays
geo.slab(bm, [(-0.005, 0.16, 0.705, 0.735, 0.012), (0.03, 0.16, 0.705, 0.735, 0.012)],
         mi('plastic'))  # roof light bar
geo.slab(bm, [(-0.815, 0.15, 0.445, 0.48, 0.01), (-0.79, 0.15, 0.445, 0.48, 0.01)],
         mi('plastic'))  # rear light bar under the wing
geo.box(bm, V((0, -0.818, 0.4625)), (0.12, 0.012, 0.024), mi('light_brake'))
for s in (-1, 1):
    geo.box(bm, V((s * 0.11, -0.818, 0.4625)), (0.05, 0.012, 0.024), mi('light_tail'))
parts.wing(bm, V((0, -0.845, 0.655)), 0.76, 0.25, thick=0.10, angle=-9, mat='paint_accent',
           plate=(0.15, 0.27), plate_mat='paint')
hard = core.obj_from_bm('hard', bm, bevel=0.006, angle=40)

# --- round parts (no bevel): motor, bumper tube, lamps, driveshafts, suspension ---------------
bm = core.new_bm()
geo.cylinder(bm, V((0, -0.40, 0.335)), V((0, -0.55, 0.335)), 0.058, mi('metal'), n=16)
geo.cylinder(bm, V((0, -0.395, 0.335)), V((0, -0.41, 0.335)), 0.05, mi('plastic'), n=16)
for k in range(4):  # heat-sink fins
    y = -0.44 - k * 0.028
    geo.cylinder(bm, V((0, y, 0.335)), V((0, y - 0.01, 0.335)), 0.067, mi('plastic'), n=16)
geo.tube(bm, [(-0.27, 0.86, 0.33), (-0.20, 0.965, 0.33), (0.20, 0.965, 0.33),
              (0.27, 0.86, 0.33)], 0.028, mi('plastic'), n=8, corner=0.06)
for x in (-0.11, -0.037, 0.037, 0.11):
    parts.light_pod(bm, V((x, 0.045, 0.725)), V((0, 1, 0)), 0.024, 0.03)
for s in (-1, 1):
    parts.light_pod(bm, V((s * 0.19, 0.965, 0.395)), V((0, 1, 0)), 0.021, 0.025)
    for y, z, piv, tower_z, ty, w_t in ((yF, zF, (0.58, 0.73), 0.49, yF - 0.04, 0.21),
                                        (yR, zR, (-0.73, -0.57), 0.55, yR + 0.06, 0.23)):
        parts.a_arm(bm, (s * 0.13, piv[0], CLR + 0.06), (s * 0.13, piv[1], CLR + 0.06),
                    (s * 0.47, y, z - 0.01))
        parts.shock(bm, (s * 0.37, y + 0.02, z), (s * (w_t - 0.02), ty, tower_z), r=0.024)
        geo.cylinder(bm, V((s * 0.10, y, z)), V((s * 0.47, y, z)), 0.012, mi('metal'), n=8)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

# --- cowl body shell ---------------------------------------------------------------------------
prof = shell.Profile([
    (-0.44, dict(hw=0.22, z0=CLR + 0.05, zb=0.40, hg=0.17, zt=0.45, hr=0.13, crown=0.02)),
    (-0.25, dict(hw=0.27, zb=0.41, hg=0.24, zt=0.43, hr=0.21, crown=0.0)),
    (0.00, dict(hw=0.28, zb=0.41, hg=0.24, zt=0.43, hr=0.21, crown=0.0)),
    (0.14, dict(hw=0.27, zb=0.42, hg=0.22, zt=0.49, hr=0.14, crown=0.02)),
    (0.32, dict(hw=0.23, zb=0.41, hg=0.18, zt=0.45, hr=0.12, crown=0.02)),
    (0.60, dict(hw=0.16, z0=CLR + 0.05, zb=0.38, hg=0.12, zt=0.40, hr=0.08, crown=0.015)),
    (0.84, dict(hw=0.10, z0=CLR + 0.08, zb=0.345, hg=0.07, zt=0.36, hr=0.05, crown=0.01)),
])
bm = core.new_bm()
shell.shell(bm, prof, -0.44, 0.84, ends=(0.07, 0.06), end_scale=0.6, step=0.05)
cowl = core.obj_from_bm('cowl', bm, weighted=False, angle=50)

# --- roll cage + driver --------------------------------------------------------------------------
bm = core.new_bm()
acc = mi('paint_accent')
R_CAGE = 0.019
geo.tube(bm, [(-0.235, -0.27, 0.40), (-0.19, -0.25, 0.715), (0.19, -0.25, 0.715),
              (0.235, -0.27, 0.40)], R_CAGE, acc, n=8, corner=0.06)
geo.tube(bm, [(-0.22, 0.20, 0.44), (-0.165, 0.0, 0.715), (0.165, 0.0, 0.715),
              (0.22, 0.20, 0.44)], R_CAGE, acc, n=8, corner=0.06)
for s in (-1, 1):
    geo.tube(bm, [(s * 0.168, 0.01, 0.715), (s * 0.19, -0.255, 0.715)], R_CAGE, acc, n=8)
    geo.tube(bm, [(s * 0.19, -0.25, 0.70), (s * 0.15, -0.64, 0.40)], R_CAGE * 0.9, acc, n=8)
geo.tube(bm, [(-0.21, -0.26, 0.45), (0.185, -0.25, 0.69)], R_CAGE * 0.85, acc, n=8)
geo.sphere(bm, V((0, -0.09, 0.545)), 0.085, mi('decal'), n=16, rings=10)  # helmet
geo.sphere(bm, V((0, -0.045, 0.55)), 0.072, mi('glass'), n=16, rings=8, scale=(0.95, 0.62, 0.5))
geo.slab(bm, [(-0.16, 0.11, 0.42, 0.50, 0.04), (-0.04, 0.12, 0.42, 0.49, 0.04)], mi('plastic'))
geo.slab(bm, [(-0.215, 0.13, 0.42, 0.62, 0.04), (-0.18, 0.13, 0.42, 0.62, 0.04)], mi('plastic'))
cage = core.obj_from_bm('cage', bm, weighted=False, angle=45)

# --- decals: stripe, numbers ---------------------------------------------------------------------
surf = geo.Surface([cowl])
bm = core.new_bm()
geo.strip(bm, surf, [(0, 0.82), (0, 0.20)], 0.07, V((0, 0, 1.0)), V((1, 0, 0)), V((0, 1, 0)),
          acc, back=0.5)
for s in (-1, 1):  # side planes: u = (0, s, 0) so the normal points outward and text reads
    geo.strip(bm, surf, [(s * -0.42, 0.405), (s * 0.30, 0.405)], 0.016,
              V((s * 0.6, 0, 0)), V((0, s, 0)), V((0, 0, 1)), acc, back=0.3)
    parts.number(bm, surf, '7', V((s * 0.6, -0.12, 0.345)), V((0, s, 0)), V((0, 0, 1)), 0.075,
                 roundel=0.055)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

wheels = export.wheels(D, front=dict(tread='knobby', rim=dict(spokes=6), rim_ratio=0.56))
ant_base = V((-0.14, -0.50, CLR + 0.055))
ant = parts.antenna(ant_base, 0.55)
export.finish(CAR, D, [hard, round_, cowl, cage, decals], wheels, ant, ant_base)
