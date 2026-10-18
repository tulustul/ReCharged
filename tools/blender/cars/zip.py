"""Zip — Rookie mini touring car: low turquoise sedan shell with bulging box flares, white twin
stripes, trunk spoiler, splitter and diffuser, multi-spoke wheels on street tyres. Number 3.

    blender -b -P tools/blender/cars/zip.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'zip'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#12B5CF', accent='#F4F4F2', rim='#D8DCE0', rim_metallic=1.0,
                   metal='#3A6FE0')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zW = D['radius'][0]
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']
ARCH = 0.222

# --- shell ---------------------------------------------------------------------------------
prof = shell.Profile([
    (-HL, dict(hw=0.44, z0=0.14, zb=0.33, hg=0.38, zg=0.335, hr=0.30, zt=0.35, rs=0.03)),
    (-0.86, dict(hw=0.48, z0=0.10, zb=0.39, hg=0.42, zg=0.40, hr=0.31, zt=0.405)),
    (yR, dict(hw=HW, zb=0.455, hg=0.43, zg=0.435, hr=0.27, zt=0.415, bulge=0.004)),
    (-0.46, dict(zt=0.42)),
    (-0.30, dict(hw=0.485, zb=0.37, hg=0.41, zg=0.375, hr=0.30, zt=0.488, crown=0.012)),
    (0.0, dict(hw=0.475, zb=0.36, hg=0.405, zg=0.365, hr=0.31, zt=HH - 0.012, bulge=0.012)),
    (0.12, dict(zt=HH - 0.012)),
    (0.38, dict(hw=0.485, zb=0.38, hg=0.42, zg=0.385, hr=0.30, zt=0.405)),
    (yF, dict(hw=HW, zb=0.455, hg=0.43, zg=0.435, hr=0.27, zt=0.415, bulge=0.004)),
    (0.84, dict(hw=0.47, z0=0.09, zb=0.38, hg=0.40, zg=0.38, hr=0.31, zt=0.37)),
    (HL, dict(hw=0.42, z0=0.13, zb=0.28, hg=0.37, zg=0.285, hr=0.30, zt=0.295, rs=0.03)),
])
bm = core.new_bm()
shell.shell(bm, prof, -HL, HL, arches=shell.arches(D), ends=(0.07, 0.08),
            end_scale=0.62, step=0.055)
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- glass, lamps, decals ----------------------------------------------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.14, 0.36), side=(-0.42, 0.27), rear=(-0.44, -0.31))
parts.lamps(bm, surf, geo.rrect(0.15, 0.052, 0.022, 3), 0.30, 0.255, True, 'light_head',
            bezel='chrome')
geo.decal(bm, surf, geo.rrect(0.80, 0.085, 0.03, 3), V((0, -3, 0.325)), V((1, 0, 0)),
          V((0, 0, 1)), mi('plastic'))  # black rear panel
parts.lamps(bm, surf, geo.rrect(0.11, 0.06, 0.02, 3), 0.33, 0.325, False, 'light_tail')
parts.lamps(bm, surf, geo.rrect(0.07, 0.06, 0.02, 3), 0.215, 0.325, False, 'light_brake')
geo.decal(bm, surf, geo.rrect(0.30, 0.05, 0.02, 3), V((0, 3, 0.255)), V((-1, 0, 0)),
          V((0, 0, 1)), mi('plastic'))  # grille
geo.decal(bm, surf, geo.rrect(0.56, 0.055, 0.025, 3), V((0, 3, 0.17)), V((-1, 0, 0)),
          V((0, 0, 1)), mi('plastic'))  # lower intake
acc = mi('paint_accent')
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
for x in (-0.065, 0.065):  # twin stripes: hood, roof, trunk (gaps over the glass)
    for y0, y1 in ((0.93, 0.39), (0.125, -0.295), (-0.455, -0.935)):
        geo.strip(bm, surf, [(x, y0), (x, y1)], 0.07, *top, acc, step=0.04, back=0.3)
for s in (-1, 1):
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
    geo.strip(bm, surf, [(s * -0.36, 0.16), (s * 0.36, 0.16)], 0.03, *side, acc, back=0.2)
    parts.number(bm, surf, '3', V((s * 2.0, -0.03, 0.275)), side[1], side[2], 0.10,
                 plate=(0.15, 0.13))
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: splitter, skirts, diffuser, spoiler, mirrors ---------------------------------
bm = core.new_bm()
geo.slab(bm, [(0.78, 0.43, CLR, CLR + 0.022, 0.008), (0.945, 0.40, CLR, CLR + 0.022, 0.008)],
         mi('plastic'))
geo.slab(bm, [(-0.945, 0.38, 0.095, 0.135, 0.01), (-0.80, 0.40, 0.095, 0.135, 0.01)],
         mi('plastic'))
for x in (-0.24, -0.08, 0.08, 0.24):  # diffuser fins
    geo.box(bm, V((x, -0.90, 0.115)), (0.012, 0.14, 0.05), mi('plastic'))
for s in (-1, 1):
    geo.box(bm, V((s * 0.475, 0.0, CLR + 0.022)), (0.05, 0.70, 0.045), mi('plastic'))
    geo.box(bm, V((s * 0.468, 0.30, 0.40)), (0.05, 0.034, 0.03), mi('paint'))  # mirror
    parts.bar(bm, V((s * 0.42, 0.30, 0.38)), V((s * 0.45, 0.30, 0.395)), 0.016, 0.01,
              mi('plastic'))  # stalk
    parts.bar(bm, V((s * 0.22, -0.83, 0.40)), V((s * 0.22, -0.875, 0.475)), 0.04, 0.014,
              mi('plastic'), up=V((1, 0, 0)))  # wing stands
parts.wing(bm, V((0, -0.88, 0.478)), 0.86, 0.12, thick=0.11, angle=-5, mat='paint_accent',
           plate=(0.055, 0.14), plate_mat='paint_accent')
hard = core.obj_from_bm('hard', bm, bevel=0.004, angle=40)

# --- round parts: exhaust, suspension in the arches --------------------------------------------
bm = core.new_bm()
geo.cylinder(bm, V((0.24, -0.88, 0.115)), V((0.24, -0.948, 0.115)), 0.022, mi('chrome'), n=12)
geo.cylinder(bm, V((0.24, -0.92, 0.115)), V((0.24, -0.949, 0.115)), 0.016, mi('plastic'), n=12)
for s in (-1, 1):
    for y in (yF, yR):
        parts.a_arm(bm, (s * 0.14, y - 0.06, 0.13), (s * 0.14, y + 0.06, 0.13),
                    (s * 0.37, y, 0.17), w=0.026, t=0.018)
        geo.box(bm, V((s * 0.375, y, zW)), (0.03, 0.06, 0.07), mi('plastic'))
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='street', rim=dict(spokes=8, spoke_w=0.14,
                                                               depth=0.3), rim_ratio=0.66))
ant_base = V((-0.26, -0.62, 0.43))
ant = parts.antenna(ant_base, 0.42)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
