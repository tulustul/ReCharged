"""Volt GT — Elite hypercar: pearl-white mid-engine wedge with a teardrop canopy and black roof,
haunches with louvres over the rear tyres, deep side intakes, louvred engine cover and glowing
cyan accent strips (front DRL band, side rockers, rear band, engine-cover lines; material
`glow`, separate from the light_* materials). Black aero rims. Number 1.

    blender -b -P tools/blender/cars/volt_gt.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'volt_gt'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#E9EDF0', accent='#17181B', paint_metallic=0.15, accent_metallic=0.0,
                   rim='#5A6068', rim_metallic=1.0, metal='#1FC7F0', glow='#22DDFF')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']

# --- shell -------------------------------------------------------------------------------------
prof = shell.Profile([
    (-HL, dict(hw=0.54, z0=0.11, zb=0.27, hg=0.46, zg=0.275, hr=0.40, zt=0.29, rs=0.03)),
    (-0.95, dict(hw=0.565, z0=0.08, zb=0.36, hg=0.50, zg=0.365, hr=0.40, zt=0.37)),
    (yR, dict(hw=HW, zb=0.458, hg=0.40, zg=0.455, hr=0.26, zt=0.372, bulge=0.008, rs=0.02)),
    (-0.45, dict(zt=0.386)),
    (-0.20, dict(hw=0.555, zb=0.33, hg=0.44, zg=0.335, hr=0.27, zt=0.438, crown=0.008, rs=0.04)),
    (0.02, dict(hw=0.55, zb=0.30, hg=0.42, zg=0.305, hr=0.28, zt=HH - 0.01, bulge=0.012)),
    (0.25, dict(zt=0.43)),
    (0.45, dict(hw=0.555, zb=0.32, hg=0.44, zg=0.32, hr=0.27, zt=0.335)),
    (yF, dict(hw=HW - 0.005, zb=0.458, hg=0.40, zg=0.455, hr=0.24, zt=0.30, bulge=0.005,
              rs=0.02)),
    (0.90, dict(hw=0.55, z0=0.075, zb=0.32, hg=0.47, zg=0.30, hr=0.30, zt=0.255)),
    (HL, dict(hw=0.48, z0=0.09, zb=0.20, hg=0.42, zg=0.205, hr=0.30, zt=0.212, rs=0.03)),
    (0.0, dict(z0=CLR + 0.006)),
])
bm = core.new_bm()
shell.shell(bm, prof, -HL, HL, arches=shell.arches(D, clear=0.012), ends=(0.06, 0.08),
            end_scale=0.62, step=0.055, roof=(-0.17, 0.20, 'paint_accent'), cap=HH - 0.004)
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- glass, lamps, glow strips, decals ---------------------------------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.21, 0.43), side=(-0.36, 0.40), rear=(-0.42, -0.19),
            m=0.014)
front = (V((-1, 0, 0)), V((0, 0, 1)))
rear = (V((1, 0, 0)), V((0, 0, 1)))
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
glow, plastic, acc = mi('glow'), mi('plastic'), mi('paint_accent')
lens = [V(p) for p in ((-0.10, -0.012), (0.10, -0.02), (0.095, 0.014), (-0.09, 0.012))]
parts.lamps(bm, surf, lens, 0.36, 0.25, True, 'light_head', bezel='plastic', bezel_w=0.006)
geo.decal(bm, surf, geo.rrect(0.44, 0.07, 0.03, 3), V((0, 3, 0.155)), *front, plastic)
geo.strip(bm, surf, [(-0.46, 0.205), (0.46, 0.205)], 0.022, V((0, 3, 0)), *front, glow,
          across=1)  # front DRL band
geo.decal(bm, surf, geo.rrect(1.0, 0.07, 0.03, 3), V((0, -3, 0.255)), *rear, plastic)
parts.lamps(bm, surf, geo.rrect(0.30, 0.03, 0.012, 3), 0.30, 0.255, False, 'light_tail')
parts.lamps(bm, surf, geo.rrect(0.12, 0.03, 0.012, 3), 0.06, 0.255, False, 'light_brake')
geo.strip(bm, surf, [(-0.50, 0.305), (0.50, 0.305)], 0.024, V((0, -3, 0)), *rear, glow,
          across=1)  # rear glow band
for x in (-0.20, 0.20):  # glowing lines along the engine cover
    geo.strip(bm, surf, [(x, -0.46), (x * 1.15, -0.98)], 0.022, *top, glow, across=1)
for i in range(5):  # engine-cover louvres
    y = -0.55 - i * 0.075
    geo.decal(bm, surf, geo.rrect(0.26, 0.03, 0.012, 3, 0.0, y), *top, plastic)
for s in (-1, 1):
    for i in range(3):  # louvres over the rear tyres
        y = yR + 0.09 - i * 0.07
        geo.decal(bm, surf, geo.rrect(0.11, 0.035, 0.014, 3, s * 0.51, y), *top, plastic)
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
    intake = [(s * -0.24, 0.13), (s * -0.36, 0.13), (s * -0.36, 0.29), (s * -0.12, 0.29)]
    geo.decal(bm, surf, intake, *side, plastic)  # deep side intake
    geo.strip(bm, surf, [(s * -0.38, 0.105), (s * 0.40, 0.105)], 0.022, *side, glow, across=1)
    parts.number(bm, surf, '1', V((s * 2.0, 0.12, 0.215)), side[1], side[2], 0.075,
                 roundel=0.058, back_mat='paint_accent', digit_mat='decal')
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: splitter, ducktail, diffuser, mirrors, skirts --------------------------------
bm = core.new_bm()
geo.slab(bm, [(0.88, 0.50, CLR, CLR + 0.02, 0.01), (1.045, 0.44, CLR, CLR + 0.02, 0.01)],
         plastic)
geo.extrude_x(bm, [(-0.96, 0.365), (-1.045, 0.29), (-1.05, 0.31), (-0.99, 0.385)], -0.50, 0.50,
              acc)  # ducktail
geo.slab(bm, [(-1.045, 0.44, 0.08, 0.14, 0.01), (-0.88, 0.46, 0.08, 0.13, 0.01)], plastic)
for x in (-0.30, -0.15, 0.0, 0.15, 0.30):
    geo.box(bm, V((x, -0.97, 0.105)), (0.012, 0.14, 0.05), plastic)
for s in (-1, 1):
    geo.box(bm, V((s * 0.555, 0.0, CLR + 0.02)), (0.03, 0.70, 0.04), plastic)
    geo.box(bm, V((s * 0.52, 0.36, 0.355)), (0.045, 0.065, 0.026), acc)  # mirror
    parts.bar(bm, V((s * 0.46, 0.33, 0.33)), V((s * 0.50, 0.36, 0.35)), 0.014, 0.01, plastic)
hard = core.obj_from_bm('hard', bm, bevel=0.004, segments=1, angle=40)

# --- round parts: exhaust, suspension -----------------------------------------------------------
bm = core.new_bm()
geo.cylinder(bm, V((0, -0.95, 0.15)), V((0, -1.04, 0.15)), 0.035, mi('chrome'), n=14)
geo.cylinder(bm, V((0, -1.0, 0.15)), V((0, -1.041, 0.15)), 0.027, plastic, n=14)
for s in (-1, 1):
    for y, z in ((yF, zF), (yR, zR)):
        parts.a_arm(bm, (s * 0.15, y - 0.07, 0.11), (s * 0.15, y + 0.07, 0.11),
                    (s * 0.38, y, z - 0.03), w=0.028, t=0.018)
        geo.box(bm, V((s * 0.385, y, z)), (0.03, 0.07, 0.07), plastic)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='street', rim=dict(spokes=7, spoke_w=0.16, skew=0.45,
                                                               depth=0.3), rim_ratio=0.66))
ant_base = V((-0.30, -0.80, 0.37))
ant = parts.antenna(ant_base, 0.40)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
