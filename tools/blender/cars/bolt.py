"""Bolt — secret open-wheel formula (modern F1 look): red needle nose dropping to a wide
multi-element front wing, raised tub with a cockpit, red/white helmet and halo, airbox, black
undercut sidepods with inlets and white sweeps, engine cover with shark fin, big rear wing on a
central pylon over the diffuser, exposed red push-rod wishbones, mirrors, fat slicks on grey
multi-spoke rims with red nuts. Number 1.

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
# metal = anodised red: wheel nuts, wishbones, uprights
core.car_materials(paint='#D8262B', accent='#1D1D20', rim='#A4A7AD', rim_metallic=0.8,
                   metal='#D02028')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']
HL, HW = D['length'] / 2, D['width'] / 2
paint, acc, plastic, red = mi('paint'), mi('paint_accent'), mi('plastic'), mi('metal')


def pod_loft(bm, keys, mat, s):
    """Sidepod with an undercut: (y, x_in, x_out, z0, z1, under) rings, mirrored by s."""
    rings = []
    for y, xi, xo, z0, z1, u in sorted(keys):
        zm = z0 + (z1 - z0) * 0.55
        poly = geo.rounded_poly([(xi, z0), (xo - u, z0), (xo, zm), (xo - 0.035, z1 - 0.02), (xi, z1 + 0.02)],
                                [0, 0.03, 0.06, 0.035, 0], 3)
        rings.append([V((s * q.x, y, q.y)) for q in poly])
    return geo.loft(bm, rings, mat)


# --- tub: needle nose -> cockpit -> engine cover ----------------------------------------------
prof = shell.Profile([
    (-0.93, dict(hw=0.075, z0=0.11, zb=0.17, hg=0.06, zg=0.19, hr=0.03, zt=0.21, rs=0.02)),
    (-0.70, dict(hw=0.11, z0=0.09, zb=0.21, hg=0.08, zg=0.25, hr=0.04, zt=0.28)),
    (-0.45, dict(hw=0.145, z0=0.075, zb=0.25, hg=0.10, zg=0.29, hr=0.05, zt=0.35)),
    (-0.24, dict(hw=0.17, z0=0.075, zb=0.27, hg=0.12, zg=0.31, hr=0.055, zt=0.40)),
    (-0.12, dict(hw=0.17, z0=0.075, zb=0.27, hg=0.14, zg=0.31, hr=0.08, zt=0.36)),
    (0.00, dict(hw=0.17, z0=0.075, zb=0.27, hg=0.145, zg=0.31, hr=0.09, zt=0.325)),
    (0.18, dict(hw=0.16, z0=0.08, zb=0.265, hg=0.135, zg=0.30, hr=0.08, zt=0.32)),
    (0.40, dict(hw=0.125, z0=0.11, zb=0.245, hg=0.105, zg=0.275, hr=0.065, zt=0.29)),
    (0.65, dict(hw=0.095, z0=0.12, zb=0.21, hg=0.08, zg=0.235, hr=0.05, zt=0.245)),
    (0.88, dict(hw=0.07, z0=0.105, zb=0.16, hg=0.06, zg=0.18, hr=0.04, zt=0.19)),
    (1.05, dict(hw=0.05, z0=0.095, zb=0.125, hg=0.045, zg=0.14, hr=0.03, zt=0.145, rs=0.02)),
])
bm = core.new_bm()
fs = shell.shell(bm, prof, -0.93, 1.05, ends=(0.04, 0.06), end_scale=0.7, step=0.05)
for f in fs:  # engine cover: black flanks, red spine
    if f.material_index == paint and f.calc_center_median().y < -0.12 and f.normal.z < 0.8:
        f.material_index = acc
tub = core.obj_from_bm('tub', bm, weighted=False, angle=48)

# --- sidepods (black, undercut) + airbox -----------------------------------------------------
bm = core.new_bm()
for s in (-1, 1):
    fs = pod_loft(bm, [(0.25, 0.10, 0.30, 0.13, 0.25, 0.03), (0.21, 0.10, 0.36, 0.10, 0.285, 0.05),
                       (0.05, 0.10, 0.39, 0.09, 0.295, 0.07), (-0.20, 0.10, 0.375, 0.09, 0.28, 0.07),
                       (-0.40, 0.10, 0.30, 0.09, 0.245, 0.05), (-0.56, 0.10, 0.20, 0.09, 0.20, 0.03),
                       (-0.70, 0.10, 0.14, 0.09, 0.16, 0.02)], acc, s)
    for f in fs:  # red inlet surround
        if f.calc_center_median().y > 0.19:
            f.material_index = paint
pods = core.obj_from_bm('pods', bm, weighted=False, angle=50)

bm = core.new_bm()
geo.slab(bm, [(-0.09, 0.05, 0.30, 0.41, 0.03), (-0.14, 0.065, 0.29, 0.42, 0.035),
              (-0.27, 0.058, 0.29, 0.405, 0.035)], paint)
airbox = core.obj_from_bm('airbox', bm, weighted=False, angle=50)

bm = core.new_bm()  # shark fin on the engine cover
geo.extrude_x(bm, geo.rounded_poly([(-0.28, 0.38), (-0.31, 0.415), (-0.80, 0.36), (-0.87, 0.30),
                                    (-0.84, 0.24), (-0.50, 0.32)], [0, 0.03, 0.04, 0.03, 0, 0], 2),
              -0.006, 0.006, acc)
fin = core.obj_from_bm('fin', bm, bevel=0.003, segments=1, angle=40)
surf = geo.Surface([tub, pods, airbox, fin])

# --- decals: cockpit, inlets, sweeps, numbers --------------------------------------------------
bm = core.new_bm()
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
ck = [(0.0, 0.16), (0.075, 0.13), (0.10, 0.03), (0.095, -0.10), (0.0, -0.12)]
ck = geo.rounded_poly([V(p) for p in ck] + [V((-x, y)) for x, y in reversed(ck[1:-1])], 0.03, 3)
geo.decal(bm, surf, geo.ccw(ck), *top, plastic)  # cockpit opening
geo.decal(bm, surf, geo.rrect(0.075, 0.07, 0.025, 3, 0, 0.355), V((0, 2.0, 0)), V((-1, 0, 0)),
          V((0, 0, 1)), plastic)  # airbox intake (projected from the front)
for s in (-1, 1):
    front = (V((0, 0.6, 0)), V((-1, 0, 0)), V((0, 0, 1)))
    geo.decal(bm, surf, geo.rrect(0.12, 0.08, 0.03, 3, -s * 0.23, 0.195), *front, plastic)
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
    geo.strip(bm, surf, [(s * 0.21, 0.15), (s * 0.08, 0.17), (s * -0.12, 0.225), (s * -0.42, 0.20)],
              0.04, *side, paint, back=0.3)  # red sweep rising along the pod
    for k in range(2):  # white chevrons
        y0 = -0.14 - k * 0.075
        geo.strip(bm, surf, [(s * y0, 0.115), (s * (y0 + 0.06), 0.15), (s * y0, 0.185)], 0.022,
                  *side, mi('decal'), back=0.3)
    geo.strip(bm, surf, [(s * 0.26, 0.11), (s * 0.26, -0.36)], 0.018, *top, mi('decal'),
              across=1)  # white pinstripe on pod tops
    parts.number(bm, surf, '1', V((s * 2.0, -0.54, 0.355)), V((0, s, 0)), V((0, 0, 1)), 0.036,
                 roundel=0.027)
parts.number(bm, surf, '1', V((0, 0.44, 2.0)), V((1, 0, 0)), V((0, 1, 0)), 0.06, roundel=0.045)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: wings, fin, floor, diffuser, crash structure, mirrors, uprights --------------
bm = core.new_bm()
fw_y = 1.015
parts.wing(bm, V((0, fw_y, 0.08)), 2 * HW - 0.02, 0.17, thick=0.10, angle=3, mat='paint_accent')
for s in (-1, 1):
    parts.wing(bm, V((s * 0.355, 0.965, 0.118)), 0.42, 0.09, thick=0.10, angle=-20, mat='paint')
    parts.wing(bm, V((s * 0.375, 0.935, 0.152)), 0.38, 0.06, thick=0.10, angle=-34,
               mat='paint_accent')
    x = s * (HW - 0.012)
    plate = geo.rounded_poly([(0.915, 0.06), (1.10, 0.06), (1.10, 0.13), (1.0, 0.185),
                              (0.915, 0.185)], [0.0, 0.015, 0.03, 0.03, 0.01], 2)
    geo.extrude_x(bm, [(p.x, p.y) for p in plate], x, x + s * 0.012, paint)  # endplates
    geo.box(bm, V((x + s * 0.006, 1.101, 0.095)), (0.008, 0.006, 0.03), mi('light_head'))
    parts.bar(bm, V((s * 0.035, 0.98, 0.105)), V((s * 0.04, 1.0, 0.085)), 0.03, 0.01, plastic,
              up=V((1, 0, 0)))  # nose pylons
# rear wing: main plane + flap, tall endplates, central swan-neck pylon
span = 0.75
parts.wing(bm, V((0, -0.975, 0.465)), span, 0.20, thick=0.10, angle=-10, mat='paint_accent')
parts.wing(bm, V((0, -1.05, 0.525)), span, 0.10, thick=0.09, angle=-30, mat='paint')
parts.wing(bm, V((0, -1.04, 0.255)), span, 0.09, thick=0.10, angle=-12, mat='paint_accent')  # beam
for s in (-1, 1):
    x = s * span / 2
    plate = geo.rounded_poly([(-0.88, 0.26), (-0.90, 0.20), (-1.10, 0.20), (-1.10, 0.555),
                              (-0.94, 0.555), (-0.88, 0.49)], [0, 0.02, 0.015, 0.02, 0.04, 0.03], 2)
    geo.extrude_x(bm, [(p.x, p.y) for p in plate], x, x + s * 0.014, acc)
    geo.box(bm, V((x + s * 0.007, -1.101, 0.40)), (0.008, 0.006, 0.12), mi('light_tail'))
geo.extrude_x(bm, geo.rounded_poly([(-0.84, 0.21), (-0.92, 0.21), (-1.01, 0.46), (-0.96, 0.46)],
                                   [0.01, 0.02, 0.01, 0.02], 2), -0.008, 0.008, acc)  # pylon
# floor plank, diffuser with strakes, crash structure + rain light
geo.slab(bm, [(0.42, 0.13, CLR, CLR + 0.014, 0.006), (0.24, 0.38, CLR, CLR + 0.014, 0.006),
              (-0.42, 0.395, CLR, CLR + 0.014, 0.006), (-0.60, 0.30, CLR, CLR + 0.014, 0.006),
              (-0.74, 0.30, CLR, CLR + 0.014, 0.006)], plastic)
geo.extrude_x(bm, [(-0.72, CLR + 0.014), (-0.74, CLR), (-0.97, 0.15), (-0.96, 0.165)], -0.30, 0.30,
              plastic)
for x in (-0.29, -0.15, 0.0, 0.15, 0.29):
    geo.extrude_x(bm, [(-0.74, CLR + 0.01), (-0.96, 0.15), (-0.96, 0.20), (-0.80, 0.15)],
                  x - 0.006, x + 0.006, plastic)
geo.slab(bm, [(-0.88, 0.05, 0.13, 0.22, 0.02), (-1.03, 0.035, 0.145, 0.205, 0.015)], plastic)
geo.box(bm, V((0, -1.035, 0.175)), (0.05, 0.008, 0.03), mi('light_brake'))
for s in (-1, 1):
    geo.slab(bm, [(0.146, 0.036, 0.326, 0.364, 0.012), (0.185, 0.026, 0.334, 0.358, 0.01)], paint,
             cx=s * 0.25)  # mirror housings
    geo.box(bm, V((s * 0.25, 0.145, 0.345)), (0.064, 0.004, 0.03), mi('chrome'))
    parts.bar(bm, V((s * 0.20, 0.17, 0.28)), V((s * 0.235, 0.165, 0.335)), 0.014, 0.01, plastic)
    for y, z, xo in ((yF, zF, 0.43), (yR, zR, 0.395)):  # uprights inside the rims
        geo.box(bm, V((s * (xo - 0.01), y, z)), (0.025, 0.05, 0.10), plastic if y > 0 else red)
hard = core.obj_from_bm('hard', bm, bevel=0.004, segments=1, angle=40)

# --- round parts: helmet, halo, wishbones, push-rods, driveshafts, exhaust -------------------
bm = core.new_bm()
geo.sphere(bm, V((0, 0.0, 0.352)), 0.062, mi('decal'), n=16, rings=10)  # helmet
geo.sphere(bm, V((0, -0.005, 0.362)), 0.064, paint, n=16, rings=10, scale=(1, 1, 0.82))
geo.sphere(bm, V((0, 0.03, 0.352)), 0.05, mi('glass'), n=16, rings=8, scale=(1.05, 0.7, 0.42))
hx = 0.105
geo.tube(bm, [(-0.125, -0.14, 0.32), (-hx, -0.08, 0.395), (-hx, 0.06, 0.40), (0, 0.13, 0.40),
              (hx, 0.06, 0.40), (hx, -0.08, 0.395), (0.125, -0.14, 0.32)], 0.012, acc, n=8,
         corner=0.05)  # halo
geo.tube(bm, [(0, 0.13, 0.40), (0, 0.20, 0.31)], 0.012, acc, n=8)  # halo centre strut
for s in (-1, 1):
    for y, z, xin, xo in ((yF, zF, 0.07, 0.42), (yR, zR, 0.09, 0.385)):
        for zz, zin in ((z - 0.08, 0.15), (z + 0.07, 0.24)):  # lower / upper wishbones
            for dy in (0.11, -0.11):
                parts.bar(bm, V((s * xin, y + dy, zin)), V((s * xo, y, zz)), 0.024, 0.016, red,
                          taper=0.75)
        parts.bar(bm, V((s * xo, y, z - 0.07)), V((s * (xin + 0.03), y - 0.03, 0.27)), 0.016,
                  0.016, plastic)  # push-rod
    geo.cylinder(bm, V((s * 0.08, yR, zR)), V((s * 0.39, yR, zR)), 0.016, plastic, n=8)
geo.cylinder(bm, V((0, -0.90, 0.25)), V((0, -0.98, 0.25)), 0.025, mi('chrome'), n=12)  # exhaust
geo.cylinder(bm, V((0, -0.95, 0.25)), V((0, -0.981, 0.25)), 0.018, plastic, n=12)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='slick', rim=dict(spokes=10, spoke_w=0.13, depth=0.3,
                                                              hub=0.32, nut=0.55), rim_ratio=0.6))
ant_base = V((-0.25, -0.34, 0.25))
ant = parts.antenna(ant_base, 0.36)
export.finish(CAR, D, [tub, pods, airbox, fin, decals, hard, round_], wheels, ant, ant_base)
