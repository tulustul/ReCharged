"""Phantom — Elite widebody sports coupe (GT86 time-attack build): long hood with a black louvred
bonnet, slim angular headlights, wide mesh mouth with splitter + canards, bolted-on box
overfenders, side skirts, 2+2 fastback greenhouse, black tail panel with wide lamps, finned
diffuser, tall swan-neck GT wing. Black slash livery, number 13. Gunmetal 10-spoke rims, red
calipers, low-profile street tyres.

    blender -b -P tools/blender/cars/phantom.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'phantom'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#1F5FE0', accent='#1D1D20', paint_metallic=0.35, accent_metallic=0.0,
                   rim='#5C6068', rim_metallic=0.6, metal='#B8BCC2')
core.material('caliper', '#D42A22', 0.0, 0.35, coat=0.6)
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']
HL, HW = D['length'] / 2, D['width'] / 2
WING_TOP = 0.64  # car-dimensions.md footnote: only the rear wing may rise above the 0.58 body

# --- shell: long hood, upright 2+2 greenhouse, fastback, box overfenders (hw steps) ----------
prof = shell.Profile([
    (-HL, dict(hw=0.52, z0=0.20, zb=0.30, hg=0.47, zg=0.385, hr=0.40, zt=0.395, crown=0.006,
               rs=0.03)),
    (-0.93, dict(hw=HW - 0.005, z0=0.10, zb=0.35, hg=0.49, zg=0.405, hr=0.38, zt=0.418)),
    (yR, dict(hw=HW, zb=0.40, hg=0.49, zg=0.42)),
    (yR + 0.21, dict(hw=HW - 0.004)),
    (-0.82, dict(hr=0.34, zt=0.425)),
    (-0.72, dict(hr=0.30, zt=0.442)),
    (-0.56, dict(hr=0.285, zt=0.50)),
    (yR + 0.25, dict(hw=0.52, zb=0.37, hg=0.45, zg=0.41)),
    (-0.38, dict(hr=0.29, zt=0.548)),
    (-0.16, dict(hr=0.30, zt=0.562, crown=0.018)),
    (0.00, dict(hr=0.31, zt=0.548)),
    (0.03, dict(hw=0.52, zb=0.37, hg=0.45, zg=0.405)),
    (0.21, dict(hr=0.40, zt=0.418, crown=0.014)),
    (yF - 0.25, dict(hw=0.52, zb=0.37, hg=0.47, zg=0.40)),
    (yF - 0.21, dict(hw=HW - 0.004)),
    (yF, dict(hw=HW, zb=0.40, hg=0.48, zg=0.415, hr=0.41, zt=0.398, crown=0.015)),
    (0.90, dict(hw=HW - 0.01, z0=0.085, zb=0.34, hg=0.48, zg=0.36, hr=0.40, zt=0.365)),
    (HL, dict(hw=0.52, z0=0.10, zb=0.23, hg=0.47, zg=0.28, hr=0.41, zt=0.29, crown=0.01,
              rs=0.03)),
    (0.0, dict(z0=CLR + 0.01)),
])
bm = core.new_bm()
shell.shell(bm, prof, -HL, HL, arches=shell.arches(D, clear=0.01), ends=(0.05, 0.07),
            end_scale=0.7, step=0.06, min_thick=0.014)
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- decals: glass, bonnet, lamps, grille, tail panel, livery, number --------------------------
bm = core.new_bm()
acc, dark, white = mi('paint_accent'), mi('plastic'), mi('decal')
shell.glass(bm, surf, prof, windshield=(0.015, 0.20), side=(-0.13, 0.19), rear=(-0.71, -0.40))
shell.glass(bm, surf, prof, side=(-0.54, -0.18))  # rear quarter glass behind the B-pillar
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
front = (V((0, 3.0, 0)), V((-1, 0, 0)), V((0, 0, 1)))
rear = (V((0, -3.0, 0)), V((1, 0, 0)), V((0, 0, 1)))
# black vented bonnet centre with white flanking slashes
geo.decal(bm, surf, [(-0.30, 0.27), (0.30, 0.27), (0.25, 0.95), (-0.25, 0.95)], *top, acc)
for s in (-1, 1):
    geo.decal(bm, surf, geo.ccw([V((s * 0.31, 0.27)), V((s * 0.36, 0.27)), V((s * 0.27, 0.94)),
                                 V((s * 0.25, 0.94))]), *top, white)
    geo.solid_decal(bm, surf, geo.rrect(0.17, 0.22, 0.03, 2, s * 0.14, 0.58), *top, dark,
                    height=0.012)
    for k in range(4):  # louvre slats over the dark vent
        geo.decal(bm, surf, geo.rrect(0.13, 0.016, 0.006, 2, s * 0.14, 0.51 + k * 0.047), *top,
                  mi('rubber'), offset=0.0135, h=0.03)
# slim angular headlights: dark housing, LED strip, projector
house = [(-0.10, -0.03), (0.07, -0.022), (0.105, 0.025), (-0.08, 0.016)]
parts.lamps(bm, surf, house, 0.33, 0.30, True, 'plastic', height=0.002)
parts.lamps(bm, surf, [(-0.085, -0.016), (0.0, -0.012), (0.0, -0.004), (-0.075, -0.006)], 0.33,
            0.30, True, 'light_head', height=0.004)
parts.lamps(bm, surf, geo.circle2(0.018, 12), 0.36, 0.302, True, 'light_head', height=0.005,
            bezel='chrome', bezel_w=0.006)
# upper grille slot, wide lower mesh mouth, corner intakes
geo.decal(bm, surf, geo.rrect(0.34, 0.035, 0.012, 2, 0, 0.255), *front, dark)
mouth = geo.rounded_poly([(-0.32, 0.12), (0.32, 0.12), (0.27, 0.215), (-0.27, 0.215)], 0.02, 2)
geo.decal(bm, surf, mouth, *front, mi('rubber'))
for z in (0.145, 0.17, 0.195):
    geo.decal(bm, surf, geo.rrect(0.60, 0.006, 0.0, 1, 0, z), *front, dark, offset=0.005, h=0.04)
for x in (-0.24, -0.16, -0.08, 0.0, 0.08, 0.16, 0.24):
    geo.decal(bm, surf, geo.rrect(0.006, 0.085, 0.0, 1, x, 0.168), *front, dark, offset=0.005, h=0.03)
for s in (-1, 1):
    geo.decal(bm, surf, geo.ccw([V((s * 0.37, 0.12)), V((s * 0.47, 0.125)), V((s * 0.47, 0.19)),
                                 V((s * 0.36, 0.20))]), *front, mi('rubber'))
# rear: black tail panel, wide lamps with brake cores, mesh panel, black lower bumper
geo.decal(bm, surf, geo.rrect(0.78, 0.10, 0.03, 3, 0, 0.215), *rear, mi('plastic'))
geo.decal(bm, surf, geo.rrect(0.52, 0.05, 0.01, 2, 0, 0.215), *rear, mi('rubber'), offset=0.005)
geo.decal(bm, surf, geo.rrect(0.94, 0.11, 0.03, 3, 0, 0.335), *rear, acc)
tail = geo.rounded_poly([(-0.10, -0.035), (0.09, -0.042), (0.115, 0.035), (-0.095, 0.035)],
                        [0.015, 0.035, 0.015, 0.015], 2)
parts.lamps(bm, surf, tail, 0.335, 0.34, False, 'light_tail', bezel='plastic', bezel_w=0.008)
for x in (0.30, 0.375):  # round brake cores (quad lamps)
    parts.lamps(bm, surf, geo.circle2(0.024, 12), x, 0.34, False, 'light_brake', height=0.006)
geo.decal(bm, surf, geo.rounded_poly([(-0.15, 0.31), (0.15, 0.31), (0.17, 0.365), (-0.17, 0.365)],
                                     0.01, 2), *rear, mi('rubber'), offset=0.005)
# livery: black lower sides with forward-pointing slashes, white pinstripes, number plate
for s in (-1, 1):
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))

    def P(pts, s=s):
        return geo.ccw([V((s * y, z)) for y, z in pts])

    geo.decal(bm, surf, P([(0.40, 0.115), (-0.42, 0.115), (-0.42, 0.335), (-0.10, 0.285),
                           (0.42, 0.215), (0.15, 0.215), (0.42, 0.165)]), *side, acc, h=0.03)
    geo.decal(bm, surf, P([(0.44, 0.235), (0.05, 0.27), (-0.38, 0.355), (0.0, 0.29)]),
              *side, white, offset=0.005, h=0.03)
    geo.decal(bm, surf, P([(0.40, 0.33), (0.0, 0.335), (-0.36, 0.37), (-0.05, 0.35)]),
              *side, mi('plastic'), h=0.03)
    geo.decal(bm, surf, P([(0.36, 0.14), (-0.05, 0.17), (-0.36, 0.22), (-0.08, 0.19)]),
              *side, mi('paint'), offset=0.005, h=0.03)
    parts.number(bm, surf, '13', V((s * 2.0, -0.10, 0.27)), side[1], side[2], 0.075,
                 plate=(0.15, 0.11), offset=0.006)
for s in (-1, 1):  # overfender seams: dark lines where the bolt-on flares meet the body
    for y0, y1 in ((yF - 0.22, yF + 0.30), (yR - 0.30, yR + 0.22)):
        geo.strip(bm, surf, [(s * 0.505, y0), (s * 0.505, y1)], 0.008, *top, dark, offset=0.004)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: wing + swan necks, splitter, canards, skirts, diffuser, mirrors, calipers ------
bm = core.new_bm()
WY, WZ = -0.905, WING_TOP - 0.068
parts.wing(bm, V((0, WY, WZ)), 1.08, 0.22, thick=0.14, angle=-10, mat='paint_accent',
           plate=(0.17, 0.22), plate_mat='paint_accent')
neck = geo.rounded_poly([(-0.76, 0.40), (-0.78, WZ - 0.06), (-0.83, WZ + 0.031), (-0.93, WZ + 0.034),
                         (-0.93, WZ + 0.016), (-0.86, WZ + 0.011), (-0.83, WZ - 0.06), (-0.815, 0.40)],
                        [0, 0.04, 0.03, 0, 0, 0.02, 0.04, 0], 3)
for s in (-1, 1):
    geo.extrude_x(bm, [(p.x, p.y) for p in neck], s * 0.23, s * 0.255, mi('plastic'))
geo.slab(bm, [(0.84, 0.545, CLR, CLR + 0.02, 0.01), (1.03, 0.52, CLR, CLR + 0.02, 0.01)],
         mi('plastic'))  # splitter
for s in (-1, 1):
    for z, dy in ((0.205, 0.0), (0.16, 0.012)):  # canard pairs
        geo.extrude_x(bm, [(0.985 - dy, z), (0.88, z + 0.03), (0.88, z + 0.042), (0.995 - dy, z + 0.01)],
                      s * 0.44, s * 0.545, mi('plastic'))
    geo.slab(bm, [(-0.40, 0.025, CLR, CLR + 0.05, 0.01), (0.40, 0.025, CLR, CLR + 0.05, 0.01)],
             mi('plastic'), cx=s * 0.525)  # side skirts
    geo.slab(bm, [(0.10, 0.022, 0.43, 0.462, 0.012), (0.14, 0.028, 0.428, 0.47, 0.015),
                  (0.17, 0.026, 0.432, 0.466, 0.012)], mi('paint'), cx=s * 0.49)  # mirror
    geo.box(bm, V((s * 0.49, 0.099, 0.446)), (0.036, 0.003, 0.024), mi('chrome'))
    parts.bar(bm, V((s * 0.43, 0.15, 0.41)), V((s * 0.47, 0.145, 0.44)), 0.02, 0.01,
              mi('plastic'))
    for y, z, x, w in ((yF, zF, 0.49 + 0.03, 0.022), (yR, zR, 0.50 + 0.036, 0.025)):  # calipers
        geo.box(bm, V((s * x, y + 0.015, z + 0.10)), (w, 0.09, 0.05), mi('caliper'))
# diffuser: ramp + fins + side walls
geo.extrude_x(bm, [(-0.76, 0.085), (-1.03, 0.19), (-1.03, 0.21), (-0.76, 0.105)],
              -0.40, 0.40, mi('plastic'))
for x in (-0.40, -0.24, -0.08, 0.08, 0.24, 0.40):
    geo.extrude_x(bm, [(-0.80, 0.09), (-1.03, 0.085), (-1.03, 0.20)], x - 0.008, x + 0.008,
                  mi('plastic'))
hard = core.obj_from_bm('hard', bm, bevel=0.004, segments=1, angle=40)

# --- round parts: overfender bolts, suspension -------------------------------------------------
bm = core.new_bm()
for s in (-1, 1):
    for y, z in ((yF, zF), (yR, zR)):
        for a in (-0.75, -0.25, 0.25, 0.75):
            p = V((s * 2.0, y + 0.26 * math.sin(a * 1.6), z + 0.26 * math.cos(a * 1.6)))
            hit, nrm = surf.project(p, V((-s, 0, 0)), 0.0, back=0.1)
            if hit:
                geo.cylinder(bm, hit - nrm * 0.004, hit + nrm * 0.006, 0.009, mi('chrome'), n=6)
        parts.a_arm(bm, (s * 0.15, y - 0.07, 0.12), (s * 0.15, y + 0.07, 0.12),
                    (s * 0.37, y, z - 0.03), w=0.028, t=0.018)
        geo.box(bm, V((s * 0.375, y, z)), (0.03, 0.07, 0.07), mi('plastic'))
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='street', rim=dict(spokes=10, spoke_w=0.15, depth=0.34,
                                                             spoke_d=0.4, taper=0.6),
                                     rim_ratio=0.70, n=36))
ant_base = V((-0.32, -0.60, 0.43))
ant = parts.antenna(ant_base, 0.40)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
