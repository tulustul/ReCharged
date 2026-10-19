"""Torque — Pro wasteland interceptor: a rusty 70s coke-bottle fastback with a steel blower and
twin scoops through the hood, front dozer blade with teeth, bull bar, mesh-caged windows,
riveted scrap plates, tall J exhaust stacks on the rear quarters, an open boot with two fuel
tanks, a spiked ducktail spoiler, side pipes and a roof harpoon gun. Rusty steel dish wheels
with spiked hubs on chunky block tyres. No number.

    blender -b -P tools/blender/cars/torque.py -- [--preview DIR]
"""
import math
import os
import random
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

import bpy  # noqa: E402
from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'torque'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#3F2419', accent='#2E2B28', rim='#3E2C22', metal='#8A8580',
                   paint_rough=0.7)
core.material('rust', '#552B15', 0.0, 0.95)
core.material('scrap', '#696A66', 0.55, 0.5)
core.material('steel', '#45423F', 0.7, 0.55)
for name, rough in (('paint', 0.7), ('paint_accent', 0.75), ('rim', 0.8)):  # wasteland: matte
    b = bpy.data.materials[name].node_tree.nodes['Principled BSDF']
    b.inputs['Coat Weight'].default_value = 0.0
    b.inputs['Roughness'].default_value = rough
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']
NOSE = 0.97  # body nose; the dozer blade fills the rest of the length
RUST, SCRAP, STEEL, ACC = mi('rust'), mi('scrap'), mi('steel'), mi('paint_accent')
rnd = random.Random(88)


def spike(bm, base, tip, r, mat=STEEL):
    geo.cylinder(bm, V(base), V(tip), r, mat, n=5, r1=0.0015)


# --- shell: long hood, cabin hump, fastback sweeping into a boot deck between haunches - ---------
prof = shell.Profile([
    (-1.06, dict(hw=0.53, z0=0.17, zb=0.43, hg=0.50, zg=0.45, hr=0.42, zt=0.46, rs=0.03)),
    (-0.95, dict(hw=0.56, z0=0.14, zb=0.47, hg=0.53, zg=0.485, hr=0.42, zt=0.485)),
    (yR, dict(hw=HW, zb=0.50, hg=0.52, zg=0.50, hr=0.40, zt=0.48, bulge=0.008)),
    (-0.58, dict(hw=0.565, zb=0.48, hg=0.52, zg=0.49, hr=0.40, zt=0.50)),
    (-0.40, dict(hw=0.565, zb=0.47, hg=0.51, zg=0.48, hr=0.38, zt=0.585)),
    (-0.18, dict(hw=0.56, zb=0.455, hg=0.50, zg=0.47, hr=0.36, zt=0.68, crown=0.012)),
    (0.12, dict(hw=0.56, zb=0.455, hg=0.50, zg=0.47, hr=0.365, zt=0.68, crown=0.012)),
    (0.40, dict(hw=0.56, zb=0.46, hg=0.51, zg=0.465, hr=0.42, zt=0.475, crown=0.01)),
    (yF, dict(hw=0.57, zb=0.47, hg=0.52, zg=0.47, hr=0.42, zt=0.465, bulge=0.006)),
    (NOSE, dict(hw=0.54, z0=0.16, zb=0.41, hg=0.50, zg=0.415, hr=0.42, zt=0.42, rs=0.03)),
    (0.0, dict(z0=0.14)),
])
bm = core.new_bm()
shell.shell(bm, prof, -1.06, NOSE, arches=shell.arches(D), ends=(0.06, 0.06), end_scale=0.7,
            step=0.08, seg=2, roof=(-0.18, 0.12, 'paint_accent'))
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- decals: glass + mesh cages, rust, scrap plates, grille, lamps, boot ----------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.15, 0.38), side=(-0.44, 0.36), rear=(-0.52, -0.21))
TOP = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))


def side(s):
    return V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1))


def zwin(y):
    p = prof.at(y)
    return p['zg'] + 0.008, p['zt'] - p['rr'] * 0.6 - 0.006


for s in (-1, 1):  # mesh cage over the door windows, B-pillar, open rear quarter window
    geo.strip(bm, surf, [(s * -0.05, 0.45), (s * -0.05, 0.64)], 0.035, *side(s), mi('paint'),
              offset=0.0055, step=0.03, across=1)
    for k in range(12):
        y = -0.02 + k * 0.034
        z0, z1 = zwin(y)
        if z1 - z0 > 0.02:
            geo.strip(bm, surf, [(s * y, z0), (s * y, z1)], 0.006, *side(s), STEEL,
                      offset=0.007, step=0.03, across=1)
    for z in (0.52, 0.565, 0.61):  # level bars, clipped to the window opening
        ys = [y / 100 for y in range(-2, 40, 2) if zwin(y / 100)[0] < z < zwin(y / 100)[1]]
        if len(ys) > 1:
            geo.strip(bm, surf, [(s * ys[0], z), (s * ys[-1], z)], 0.006, *side(s), STEEL,
                      offset=0.007, step=0.05, across=1)
for k in range(9):  # rear window cage
    y = -0.505 + k * 0.035
    hr = prof.at(y)['hr'] - 0.03
    geo.strip(bm, surf, [(-hr, y), (hr, y)], 0.006, *TOP, STEEL, offset=0.007, step=0.06,
              across=1)
for x in (-0.24, -0.12, 0.0, 0.12, 0.24):
    geo.strip(bm, surf, [(x, -0.51), (x, -0.22)], 0.006, *TOP, STEEL, offset=0.007, step=0.05,
              across=1)
for k in range(7):  # half-caged windshield (driver side)
    y = 0.17 + k * 0.032
    geo.strip(bm, surf, [(-prof.at(y)['hr'] + 0.03, y), (-0.02, y)], 0.006, *TOP, STEEL,
              offset=0.007, step=0.06, across=1)


def blob(r, n=10, aspect=0.6):
    return [V((math.cos(2 * math.pi * i / n), math.sin(2 * math.pi * i / n) * aspect))
            * r * rnd.uniform(0.7, 1.2) for i in range(n)]


def at(poly, x, y, ang=0.0):
    c, s_ = math.cos(ang), math.sin(ang)
    return [V((x + p.x * c - p.y * s_, y + p.x * s_ + p.y * c)) for p in poly]


# rust blooms + dark grime (authored materials: the wasteland look survives any player paint)
for x, y, r, m in ((0.27, 0.88, 0.10, RUST), (-0.30, 0.48, 0.09, RUST), (0.12, -0.02, 0.07, RUST),
                   (-0.24, -0.94, 0.08, RUST), (0.36, -0.68, 0.08, RUST),
                   (-0.08, 0.86, 0.12, ACC), (0.25, 0.45, 0.10, ACC), (0.30, -0.98, 0.08, ACC)):
    geo.decal(bm, surf, at(blob(r), x, y, rnd.uniform(0, 3)), *TOP, m, offset=0.004, h=0.04)
for s in (-1, 1):
    sill = [(y, 0.16) for y in (-0.36, 0.38)]  # wavy rust band along the sill
    sill += [(0.38 - k * 0.0592, rnd.uniform(0.20, 0.25)) for k in range(13)]
    geo.decal(bm, surf, geo.ccw([V((s * y, z)) for y, z in sill]), *side(s), RUST, offset=0.004, h=0.04)
    for y, z, r, m in ((0.86, 0.30, 0.07, RUST), (-0.97, 0.30, 0.07, RUST),
                       (-0.48, 0.42, 0.09, RUST), (0.40, 0.42, 0.06, RUST),
                       (-0.62, 0.48, 0.08, ACC), (0.78, 0.40, 0.08, ACC)):
        geo.decal(bm, surf, at(blob(r), s * y, z, rnd.uniform(0, 3)), *side(s), m,
                  offset=0.004, h=0.04)
    # riveted scrap plates: door, front fender, rear quarter
    for y, z, w, h, ang in ((0.10, 0.31, 0.34, 0.20, 0.03), (0.52, 0.30, 0.15, 0.17, -0.05),
                            (-0.27, 0.34, 0.13, 0.14, 0.06)):
        poly = at(geo.rrect(w, h, 0.01, 1), s * y, z, s * ang)
        geo.solid_decal(bm, surf, poly, *side(s), SCRAP, height=0.006, side_mat=STEEL)
        for cx, cz in ((-1, -1), (-1, 1), (1, -1), (1, 1), (0, -1), (0, 1)):
            p = at([V((cx * (w / 2 - 0.016), cz * (h / 2 - 0.016)))], s * y, z, s * ang)[0]
            geo.decal(bm, surf, geo.circle2(0.008, 5, p.x, p.y), *side(s), STEEL,
                      offset=0.0095)
    # gills behind the front wheels
    for k in range(3):
        y = 0.33 - k * 0.045
        geo.decal(bm, surf, at(geo.rrect(0.018, 0.12, 0.006, 1), s * y, 0.30, s * 0.35),
                  *side(s), mi('plastic'), offset=0.004)
# scrap patch on the hood
geo.solid_decal(bm, surf, at(geo.rrect(0.16, 0.14, 0.01, 1), -0.30, 0.82, 0.08), *TOP, SCRAP,
                height=0.006, side_mat=STEEL)
# open boot: dark floor between the haunches
geo.decal(bm, surf, geo.rrect(0.56, 0.40, 0.04, 2, 0.0, -0.75), *TOP, mi('plastic'), offset=0.004)
# nose: mesh grille + quad round headlights
FRONT = (V((-1, 0, 0)), V((0, 0, 1)))
geo.decal(bm, surf, geo.rrect(0.86, 0.10, 0.02, 2), V((0, 3, 0.355)), *FRONT, mi('plastic'))
for k in range(9):
    x = -0.14 + k * 0.035
    geo.strip(bm, surf, [(x, 0.31), (x, 0.40)], 0.006, V((0, 3, 0)), *FRONT, STEEL,
              offset=0.006, step=0.05, across=1)
for x in (0.24, 0.35):
    parts.lamps(bm, surf, geo.circle2(0.034, 14), x, 0.355, True, 'light_head', bezel='chrome',
                bezel_w=0.007)
# tail: mesh panel + lamps
REAR = (V((1, 0, 0)), V((0, 0, 1)))
geo.decal(bm, surf, geo.rrect(0.90, 0.075, 0.02, 2), V((0, -3, 0.39)), *REAR, mi('plastic'))
geo.decal(bm, surf, geo.rrect(0.70, 0.07, 0.02, 2), V((0, -3, 0.285)), *REAR, mi('plastic'))
geo.solid_decal(bm, surf, at(geo.rrect(0.16, 0.06, 0.008, 1), 0.0, 0.0, 0.06),
                V((0, -3, 0.30)), *REAR, SCRAP, height=0.005, side_mat=STEEL)
for k in range(7):
    x = -0.12 + k * 0.04
    geo.strip(bm, surf, [(x, 0.36), (x, 0.42)], 0.006, V((0, -3, 0)), *REAR, STEEL,
              offset=0.006, step=0.05, across=1)
parts.lamps(bm, surf, geo.rrect(0.14, 0.045, 0.012, 2), 0.33, 0.39, False, 'light_tail')
parts.lamps(bm, surf, geo.rrect(0.07, 0.045, 0.012, 2), 0.21, 0.39, False, 'light_brake')
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: blade, bull bar, blower, spoiler, gun receiver -------------------------------
bm = core.new_bm()
# dozer blade: concave plate leaning back, wing tips swept back, ribs + push arms behind
blade = [(HL - 0.002, CLR), (HL - 0.03, CLR + 0.08), (HL - 0.05, CLR + 0.17),
         (HL - 0.032, CLR + 0.175), (HL - 0.016, CLR + 0.08), (HL - 0.022, CLR)]
geo.extrude_x(bm, blade[::-1], -0.40, 0.40, RUST)
for s in (-1, 1):  # swept wing tips
    pts = [V((s * 0.395, 0, 0)), V((s * 0.50, -0.09, 0))]
    a, b = pts
    rings = []
    for p in (a, b):
        rings.append([V((p.x, y + p.y, z)) for y, z in blade])
    if s < 0:
        rings = rings[::-1]
    geo.loft(bm, rings, RUST)
for x in (-0.30, -0.10, 0.10, 0.30):  # ribs behind the blade
    parts.bar(bm, V((x, HL - 0.045, CLR + 0.01)), V((x, HL - 0.065, CLR + 0.16)), 0.03, 0.025,
              STEEL, up=V((0, 1, 0)))
for z in (CLR + 0.035, CLR + 0.12):  # riveted straps across the blade face
    zz = (z - CLR) / 0.175
    y = HL - 0.002 - 0.048 * zz
    parts.bar(bm, V((-0.38, y + 0.004, z)), V((0.38, y + 0.004, z)), 0.022, 0.012, SCRAP,
              up=V((0, 1, 0.5)))
for s in (-1, 1):
    parts.bar(bm, V((s * 0.30, HL - 0.06, CLR + 0.10)), V((s * 0.30, NOSE - 0.04, 0.22)), 0.04,
              0.03, STEEL)  # push arms into the chassis
    # bull bar uprights through the grille line
    parts.bar(bm, V((s * 0.17, NOSE + 0.035, CLR + 0.10)), V((s * 0.17, NOSE + 0.02, 0.43)),
              0.035, 0.03, STEEL, up=V((0, 1, 0)))
parts.bar(bm, V((-0.36, NOSE + 0.03, 0.27)), V((0.36, NOSE + 0.03, 0.27)), 0.04, 0.035, STEEL,
          up=V((0, 1, 0)))
# blower: manifold, ribbed case, front pulley housing
geo.slab(bm, [(0.54, 0.15, 0.44, 0.50, 0.02), (0.82, 0.15, 0.43, 0.49, 0.02)], STEEL)
geo.slab(bm, [(0.57, 0.12, 0.49, 0.555, 0.025), (0.79, 0.12, 0.49, 0.555, 0.025)], mi('metal'))
for k in range(5):
    y = 0.59 + k * 0.045
    geo.slab(bm, [(y - 0.008, 0.13, 0.485, 0.56, 0.02), (y + 0.008, 0.13, 0.485, 0.56, 0.02)],
             mi('chrome'), seg=1)
geo.box(bm, V((0, 0.80, 0.50)), (0.18, 0.03, 0.08), STEEL)
# ducktail spoiler kicking up off the tail
geo.extrude_x(bm, [(-0.92, 0.465), (-0.92, 0.48), (-1.05, 0.548), (-1.072, 0.546),
                   (-1.066, 0.505)], -0.47, 0.47, mi('paint'))
# rear bumper bar, tow hooks, plate
geo.slab(bm, [(-1.068, 0.47, 0.19, 0.245, 0.02), (-1.03, 0.48, 0.19, 0.245, 0.02)], STEEL, seg=1)
for s in (-1, 1):
    geo.box(bm, V((s * 0.22, -1.055, 0.17)), (0.03, 0.05, 0.05), STEEL)
# harpoon gun: pedestal, receiver, ammo box
geo.box(bm, V((0, 0.03, 0.69)), (0.10, 0.10, 0.04), STEEL)
geo.box(bm, V((0, 0.03, 0.725)), (0.03, 0.04, 0.04), STEEL)
geo.box(bm, V((0, 0.0, 0.745)), (0.075, 0.20, 0.045), SCRAP)
geo.box(bm, V((0.06, -0.02, 0.738)), (0.045, 0.08, 0.04), STEEL)
hard = core.obj_from_bm('hard', bm, bevel=0.004, segments=1, angle=40)

# --- round parts: scoops, pulley, stacks, tanks, pipes, harpoon, spikes, suspension ------------
bm = core.new_bm()
for x in (-0.055, 0.055):  # twin forward scoops on the blower
    geo.cylinder(bm, V((x, 0.62, 0.574)), V((x, 0.84, 0.574)), 0.025, mi('chrome'), n=12,
                 r1=0.026)
    geo.cylinder(bm, V((x, 0.835, 0.574)), V((x, 0.842, 0.574)), 0.02, mi('plastic'), n=12)
geo.cylinder(bm, V((0, 0.815, 0.515)), V((0, 0.835, 0.515)), 0.045, mi('plastic'), n=14)
geo.cylinder(bm, V((0, 0.834, 0.515)), V((0, 0.842, 0.515)), 0.02, mi('chrome'), n=10)
for s in (-1, 1):
    # J exhaust stacks up the rear quarters: heat shield, pipe bending back at the top
    x, y = s * 0.550, -0.31
    top = V((x + s * 0.004, y - 0.10, 0.735))
    geo.tube(bm, [(x, y + 0.06, 0.20), (x, y, 0.28), (x, y, 0.70), top], 0.029, mi('metal'),
             n=10, corner=0.04)
    geo.cylinder(bm, V((x, y, 0.36)), V((x, y, 0.64)), 0.031, STEEL, n=10)
    for z in (0.365, 0.635):
        geo.cylinder(bm, V((x, y, z - 0.008)), V((x, y, z + 0.008)), 0.033, mi('metal'), n=10)
    d = (top - V((x, y, 0.70))).normalized()
    geo.cylinder(bm, top - d * 0.004, top + d * 0.002, 0.022, mi('plastic'), n=10)
    # side pipes dumping ahead of the rear wheels
    for k in range(3):
        yy = -0.20 - k * 0.05
        geo.cylinder(bm, V((s * 0.54, yy, 0.27)), V((s * 0.562, yy - 0.05, 0.15)), 0.016,
                     mi('metal'), n=8)
    # suspension in the arches
    for yy, z in ((yF, zF), (yR, zR)):
        parts.a_arm(bm, (s * 0.16, yy - 0.07, 0.15), (s * 0.16, yy + 0.07, 0.15),
                    (s * 0.40, yy, z - 0.03), w=0.03, t=0.02)
        geo.box(bm, V((s * 0.405, yy, z)), (0.03, 0.07, 0.08), mi('plastic'))
    # spoiler and roof-edge spikes
    spike(bm, (s * 0.45, -1.0, 0.53), (s * 0.47, -1.066, 0.598), 0.022)
    spike(bm, (s * 0.45, -0.93, 0.50), (s * 0.47, -0.97, 0.599), 0.018)
    spike(bm, (s * 0.30, -0.22, 0.655), (s * 0.36, -0.40, 0.70), 0.018)
# fuel tanks lying in the open boot
for x in (-0.14, 0.14):
    geo.cylinder(bm, V((x, -0.87, 0.49)), V((x, -0.61, 0.49)), 0.10, RUST, n=12)
    for y in (-0.87, -0.61):
        geo.sphere(bm, V((x, y, 0.49)), 0.10, RUST, n=12, rings=5, scale=(1, 0.45, 1))
    for y in (-0.82, -0.67):  # straps
        geo.cylinder(bm, V((x, y, 0.49)), V((x, y + 0.02, 0.49)), 0.104, STEEL, n=12)
    geo.cylinder(bm, V((x, -0.74, 0.58)), V((x, -0.74, 0.598)), 0.02, STEEL, n=8)
# harpoon: barrel, shaft, barbed head, scope
geo.cylinder(bm, V((0, 0.09, 0.748)), V((0, 0.30, 0.748)), 0.017, STEEL, n=8)
geo.cylinder(bm, V((0, 0.30, 0.748)), V((0, 0.40, 0.748)), 0.008, mi('metal'), n=6)
geo.cylinder(bm, V((0, 0.40, 0.748)), V((0, 0.47, 0.748)), 0.026, mi('metal'), n=4, r1=0.001)
for s in (-1, 1):
    spike(bm, (0, 0.41, 0.748), (s * 0.04, 0.375, 0.748), 0.008, mi('metal'))
geo.cylinder(bm, V((0, -0.05, 0.765)), V((0, 0.07, 0.765)), 0.009, mi('plastic'), n=8)
# front teeth on the blade + bumper spikes
for k in range(7):
    x = -0.36 + k * 0.12
    spike(bm, (x, HL - 0.042, CLR + 0.165), (x, HL - 0.02, CLR + 0.24), 0.017)
for s in (-1, 1):  # outward spikes on the blade wing tips
    for z in (CLR + 0.05, CLR + 0.13):
        spike(bm, (s * 0.47, HL - 0.07, z), (s * 0.57, HL - 0.05, z + 0.01), 0.018)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

# --- wheels: rusty steel dish rims, chunky block tyres, spiked hubs ---------------------------
style = dict(tread='block', rim=dict(spokes=0, holes=8, hub=0.34, depth=0.22), rim_ratio=0.56, n=24, rows=13)
wheels = export.wheels(D, front=style)
out = []
for w in wheels:
    i = 0 if w.name[6] == 'f' else 1
    s = -1 if w.name[7] == 'l' else 1
    W, c = D['wheel_width'][i], w.location
    bm = core.new_bm()
    x0, x1 = 0.40 * W, 0.52 * W + 0.011  # stay inside the wheel width tolerance
    geo.cylinder(bm, c + V((s * x0, 0, 0)), c + V((s * (x0 + 0.012), 0, 0)), 0.05, STEEL, n=10)
    spike(bm, c + V((s * x0, 0, 0)), c + V((s * x1, 0, 0)), 0.022)
    for k in range(4):
        a = math.pi / 4 + k * math.pi / 2
        o = V((0, math.cos(a), math.sin(a))) * 0.035
        spike(bm, c + o + V((s * x0, 0, 0)), c + o * 1.4 + V((s * (x1 - 0.008), 0, 0)), 0.012)
    sp = core.obj_from_bm(w.name + '_spikes', bm, weighted=False, angle=40)
    out.append(core.bake([w, sp], w.name, location=c.copy()))
wheels = out

ant_base = V((-0.40, -0.95, 0.47))
ant = parts.antenna(ant_base, 0.40)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
