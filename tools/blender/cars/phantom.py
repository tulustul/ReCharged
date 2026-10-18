"""Phantom — Elite low sports coupe: metallic violet GT body with wide haunches, full-width
wing on swan-neck stands, splitter + canards, diffuser, slim LED lamps, white roof and side
blades, gold 6-spoke rims. Number 9.

    blender -b -P tools/blender/cars/phantom.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell  # noqa: E402

CAR = 'phantom'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#3F1F80', accent='#1E1E22', paint_metallic=0.45, accent_metallic=0.0,
                   rim='#D4A63C', rim_metallic=1.0, metal='#D4A63C')
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zF, zR = D['radius']
CLR = D['clearance']
HL, HW, HH = D['length'] / 2, D['width'] / 2, D['height']

# --- shell -------------------------------------------------------------------------------------
prof = shell.Profile([
    (-HL, dict(hw=0.53, z0=0.13, zb=0.33, hg=0.46, zg=0.34, hr=0.40, zt=0.36, rs=0.03)),
    (-0.92, dict(hw=0.55, z0=0.10, zb=0.40, hg=0.49, zg=0.405, hr=0.38, zt=0.405)),
    (yR, dict(hw=HW, zb=0.492, hg=0.47, zg=0.484, hr=0.28, zt=0.42, bulge=0.006)),
    (-0.42, dict(zt=0.425)),
    (-0.22, dict(hw=0.54, zb=0.40, hg=0.45, zg=0.405, hr=0.32, zt=0.448, crown=0.01)),
    (0.02, dict(hw=0.535, zb=0.385, hg=0.445, zg=0.39, hr=0.33, zt=0.455, bulge=0.01)),
    (0.12, dict(zt=0.453)),
    (0.34, dict(hw=0.54, zb=0.40, hg=0.46, zg=0.405, hr=0.34, zt=0.395)),
    (yF, dict(hw=HW - 0.005, zb=0.478, hg=0.47, zg=0.47, hr=0.28, zt=0.385, bulge=0.005)),
    (0.90, dict(hw=0.55, z0=0.09, zb=0.36, hg=0.47, zg=0.355, hr=0.36, zt=0.345)),
    (HL, dict(hw=0.50, z0=0.11, zb=0.27, hg=0.44, zg=0.275, hr=0.36, zt=0.285, rs=0.03)),
    (0.0, dict(z0=CLR + 0.01)),
])
bm = core.new_bm()
shell.shell(bm, prof, -HL, HL, arches=shell.arches(D, clear=0.014), ends=(0.06, 0.07),
            end_scale=0.62, step=0.055, roof=(-0.20, 0.10, 'paint_accent'))
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- glass, lamps, decals ----------------------------------------------------------------------
bm = core.new_bm()
shell.glass(bm, surf, prof, windshield=(0.10, 0.32), side=(-0.36, 0.30), rear=(-0.41, -0.20))
front = (V((-1, 0, 0)), V((0, 0, 1)))
rear = (V((1, 0, 0)), V((0, 0, 1)))
lens = [V(p) for p in ((-0.09, -0.016), (0.09, -0.024), (0.085, 0.022), (-0.08, 0.018))]
parts.lamps(bm, surf, lens, 0.35, 0.272, True, 'light_head', bezel='plastic', bezel_w=0.008)
geo.decal(bm, surf, geo.rrect(0.36, 0.06, 0.025, 3), V((0, 3, 0.205)), *front, mi('plastic'))
geo.decal(bm, surf, geo.rrect(0.96, 0.05, 0.02, 3), V((0, -3, 0.315)), *rear, mi('plastic'))
parts.lamps(bm, surf, geo.rrect(0.20, 0.03, 0.012, 3), 0.33, 0.315, False, 'light_tail')
parts.lamps(bm, surf, geo.rrect(0.12, 0.03, 0.012, 3), 0.11, 0.315, False, 'light_brake')
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
for x in (-0.17, 0.17):  # hood vents
    geo.decal(bm, surf, geo.rrect(0.10, 0.16, 0.025, 3, x, 0.58), *top, mi('plastic'))
acc = mi('paint_accent')
for s in (-1, 1):
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
    blade = [(s * 0.36, 0.14), (s * 0.34, 0.25), (s * -0.36, 0.27), (s * -0.34, 0.16)]
    geo.decal(bm, surf, blade, *side, acc)  # side blade
    parts.number(bm, surf, '9', V((s * 2.0, -0.02, 0.335)), side[1], side[2], 0.085,
                 roundel=0.065)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: wing + stands, splitter, canards, skirts, diffuser, mirrors ---------------------
bm = core.new_bm()
parts.wing(bm, V((0, -0.875, 0.458)), 1.07, 0.24, thick=0.11, angle=-6, mat='paint_accent',
           plate=(0.15, 0.27), plate_mat='paint_accent')
parts.wing(bm, V((0, -0.965, 0.482)), 1.07, 0.09, thick=0.10, angle=-24, mat='paint_accent')
geo.slab(bm, [(0.86, 0.53, CLR, CLR + 0.022, 0.01), (1.012, 0.50, CLR, CLR + 0.022, 0.01)],
         mi('plastic'))  # splitter
for s in (-1, 1):
    geo.tube(bm, [(s * 0.22, -0.77, 0.41), (s * 0.22, -0.79, 0.445), (s * 0.22, -0.845, 0.462),
                  (s * 0.22, -0.875, 0.45)], 0.014, mi('plastic'), n=6, corner=0.03)  # swan neck
    geo.extrude_x(bm, [(0.96, 0.17), (0.86, 0.20), (0.86, 0.215), (0.97, 0.19)],
                  s * 0.48, s * 0.545, mi('plastic'))  # canards
    geo.box(bm, V((s * 0.535, 0.0, CLR + 0.025)), (0.035, 0.74, 0.05), mi('plastic'))  # skirts
    geo.box(bm, V((s * 0.52, 0.29, 0.425)), (0.04, 0.07, 0.03), mi('paint'))  # mirror
    parts.bar(bm, V((s * 0.47, 0.29, 0.41)), V((s * 0.505, 0.29, 0.42)), 0.016, 0.01,
              mi('plastic'))
geo.slab(bm, [(-1.012, 0.42, 0.095, 0.15, 0.01), (-0.86, 0.44, 0.095, 0.14, 0.01)],
         mi('plastic'))  # diffuser
for x in (-0.28, -0.14, 0.14, 0.28):
    geo.box(bm, V((x, -0.95, 0.12)), (0.012, 0.15, 0.05), mi('plastic'))
hard = core.obj_from_bm('hard', bm, bevel=0.004, segments=1, angle=40)

# --- round parts: exhausts, suspension -------------------------------------------------------
bm = core.new_bm()
for x in (-0.05, 0.05):
    geo.cylinder(bm, V((x, -0.95, 0.125)), V((x, -1.01, 0.125)), 0.026, mi('chrome'), n=12)
    geo.cylinder(bm, V((x, -0.98, 0.125)), V((x, -1.011, 0.125)), 0.019, mi('plastic'), n=12)
for s in (-1, 1):
    for y, z in ((yF, zF), (yR, zR)):
        parts.a_arm(bm, (s * 0.15, y - 0.07, 0.12), (s * 0.15, y + 0.07, 0.12),
                    (s * 0.37, y, z - 0.03), w=0.028, t=0.018)
        geo.box(bm, V((s * 0.375, y, z)), (0.03, 0.07, 0.07), mi('plastic'))
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheels = export.wheels(D, front=dict(tread='street', rim=dict(spokes=6, spoke_w=0.24, depth=0.3),
                                     rim_ratio=0.66))
ant_base = V((-0.30, -0.70, 0.415))
ant = parts.antenna(ant_base, 0.40)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
