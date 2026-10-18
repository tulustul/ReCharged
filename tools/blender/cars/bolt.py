"""Bolt — secret futuristic open-wheel formula: hot-magenta needle-nose monocoque with a glass
canopy and airbox, coke-bottle sidepods, tall shark fin, wide front wing, twin-fin rear wing,
exposed wishbones, flat floor, cyan glow accents (material `glow`), slicks with glowing
sidewall rings on black turbine rims. Number 0.

    blender -b -P tools/blender/cars/bolt.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'bolt'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#E01E78', accent='#17181B', rim='#1C1D20', rim_metallic=0.0,
                   metal='#2EE6FF', glow='#2EE6FF')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']
acc, plastic, glow = mi('paint_accent'), mi('plastic'), mi('glow')

# --- monocoque ------------------------------------------------------------------------------
prof = shell.Profile([
    (-1.0, dict(hw=0.10, z0=0.10, zb=0.20, hg=0.08, zg=0.205, hr=0.06, zt=0.215, rs=0.03)),
    (-0.80, dict(hw=0.14, z0=0.075, zb=0.23, hg=0.11, zg=0.235, hr=0.08, zt=0.26)),
    (-0.40, dict(hw=0.20, zb=0.25, hg=0.16, zg=0.255, hr=0.10, zt=0.34)),
    (-0.12, dict(hw=0.20, zb=0.26, hg=0.15, zg=0.265, hr=0.075, zt=HH - 0.012, crown=0.008)),
    (0.02, dict(zt=0.395)),
    (0.10, dict(hw=0.19, zb=0.27, hg=0.15, zg=0.275, hr=0.10, zt=0.39, crown=0.01)),
    (0.34, dict(hw=0.17, zb=0.27, hg=0.13, zg=0.275, hr=0.09, zt=0.30)),
    (0.60, dict(hw=0.13, z0=0.08, zb=0.25, hg=0.10, zg=0.255, hr=0.07, zt=0.27)),
    (0.90, dict(hw=0.08, z0=0.11, zb=0.21, hg=0.06, zg=0.215, hr=0.04, zt=0.225)),
    (HL - 0.05, dict(hw=0.05, z0=0.13, zb=0.17, hg=0.04, zg=0.175, hr=0.03, zt=0.18, rs=0.02)),
    (-0.6, dict(z0=0.075, rb=0.02)),
])
bm = core.new_bm()
shell.shell(bm, prof, -1.0, HL - 0.05, ends=(0.05, 0.05), end_scale=0.7, step=0.055,
            roof=(-0.36, -0.05, 'paint_accent'))
mono = core.obj_from_bm('mono', bm, weighted=False, angle=48)

# --- sidepods + floor ----------------------------------------------------------------------------
bm = core.new_bm()
for s in (-1, 1):
    geo.slab(bm, [(-0.62, 0.05, 0.10, 0.17, 0.03), (-0.40, 0.09, 0.08, 0.22, 0.05),
                  (0.0, 0.115, 0.075, 0.25, 0.06), (0.28, 0.11, 0.08, 0.24, 0.06),
                  (0.34, 0.10, 0.09, 0.23, 0.05)], mi('paint'), cx=s * 0.30)
pods = core.obj_from_bm('pods', bm, weighted=False, angle=48)
surf = geo.Surface([mono, pods])

# --- glass, lamps, glow, decals ----------------------------------------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.06, 0.32), side=(-0.08, 0.30), rear=None, m=0.012)
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
front = (V((-1, 0, 0)), V((0, 0, 1)))
for s in (-1, 1):
    geo.decal(bm, surf, geo.rrect(0.16, 0.12, 0.05, 3), V((s * 0.30, 3, 0.165)), *front, plastic)
    geo.strip(bm, surf, [(s * 0.30, 0.30), (s * 0.30, -0.50)], 0.022, *top, glow, across=1)
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
    parts.number(bm, surf, '0', V((s * 2.0, 0.62, 0.20)), side[1], side[2], 0.06, roundel=0.045)
geo.strip(bm, surf, [(0, 1.04), (0, 0.40)], 0.05, *top, acc, step=0.02)  # nose stripe
parts.number(bm, surf, '0', V((0, 0.55, 2.0)), V((1, 0, 0)), V((0, 1, 0)), 0.07, roundel=0.05,
             offset=0.0045)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: wings, fin, floor, mirrors, lamp housings -----------------------------------------
bm = core.new_bm()
parts.wing(bm, V((0, 0.99, 0.12)), 2 * HW - 0.03, 0.17, thick=0.09, angle=4, mat='paint_accent',
           plate=(0.08, 0.22), plate_mat='paint')  # front wing
parts.wing(bm, V((0, 0.95, 0.163)), 2 * HW - 0.10, 0.08, thick=0.08, angle=-14, mat='paint')
for s in (-1, 1):
    parts.bar(bm, V((s * 0.06, 0.92, 0.18)), V((s * 0.08, 0.98, 0.135)), 0.02, 0.012, plastic,
              up=V((1, 0, 0)))  # front wing pylons
parts.wing(bm, V((0, -0.93, 0.36)), 0.86, 0.20, thick=0.11, angle=-16, mat='paint_accent',
           plate=(0.18, 0.28), plate_mat='paint')  # rear wing with tall fins
parts.wing(bm, V((0, -1.03, 0.392)), 0.86, 0.07, thick=0.10, angle=-24, mat='paint_accent')
geo.extrude_x(bm, [(-0.14, 0.40), (-0.30, 0.41), (-0.90, 0.405), (-0.94, 0.37), (-0.86, 0.30),
                   (-0.40, 0.30)], -0.006, 0.006, acc)  # shark fin
geo.slab(bm, [(-0.62, 0.42, CLR, CLR + 0.014, 0.006), (0.38, 0.42, CLR, CLR + 0.014, 0.006)],
         plastic)  # flat floor
geo.box(bm, V((0, -0.99, 0.12)), (0.10, 0.02, 0.03), mi('light_brake'))  # rain light
for s in (-1, 1):
    geo.box(bm, V((s * 0.43, -0.98, 0.33)), (0.012, 0.03, 0.06), mi('light_tail'))
    geo.box(bm, V((s * 0.572, 1.02, 0.125)), (0.012, 0.05, 0.022), mi('light_head'))
    geo.box(bm, V((s * 0.34, 0.12, 0.27)), (0.04, 0.06, 0.025), mi('paint'))  # mirror
    parts.bar(bm, V((s * 0.30, 0.12, 0.235)), V((s * 0.33, 0.12, 0.262)), 0.012, 0.012, plastic)
hard = core.obj_from_bm('hard', bm, bevel=0.004, segments=1, angle=40)

bm = core.new_bm()  # glow edge along the fin top
geo.extrude_x(bm, [(-0.15, 0.407), (-0.30, 0.417), (-0.88, 0.412), (-0.88, 0.402),
                   (-0.30, 0.407), (-0.15, 0.397)], -0.0075, 0.0075, glow)
fin_glow = core.obj_from_bm('fin_glow', bm, weighted=False, angle=40)

# --- round parts: wishbones, push-rods, exhaust --------------------------------------------------
bm = core.new_bm()
for s in (-1, 1):
    for y, z, xin in ((yF, zF, 0.07), (yR, zR, 0.14)):
        for zz, dz in ((z - 0.035, 0.0), (z + 0.035, 0.06)):
            parts.a_arm(bm, (s * xin, y - 0.09, zz + dz * 0.3), (s * xin, y + 0.09, zz + dz * 0.3),
                        (s * 0.44, y, zz), w=0.02, t=0.014, mat='paint_accent')
        parts.bar(bm, V((s * 0.42, y, z - 0.03)), V((s * (xin + 0.04), y + 0.03, 0.30)), 0.014,
                  0.014, plastic)  # push-rod
        geo.box(bm, V((s * 0.445, y, z)), (0.03, 0.07, 0.09), plastic)
geo.cylinder(bm, V((0, -0.95, 0.20)), V((0, -1.02, 0.20)), 0.03, mi('chrome'), n=12)
geo.cylinder(bm, V((0, -0.99, 0.20)), V((0, -1.021, 0.20)), 0.022, plastic, n=12)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='slick', rim=dict(spokes=7, spoke_w=0.16, skew=0.5,
                                                              depth=0.3), rim_ratio=0.64,
                                     letter_ring='glow'))
ant_base = V((-0.16, -0.50, 0.30))
ant = parts.antenna(ant_base, 0.40)
export.finish(CAR, D, [mono, pods, decals, hard, fin_glow, round_], wheels, ant, ant_base)
