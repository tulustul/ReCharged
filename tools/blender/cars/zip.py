"""Zip — Rookie mini truggy: low lime wedge shell with a pointed nose, cab-forward canopy with
black side windows, black lower sides with grey/white swooshes, side scoops and a flat rear
deck; exposed chassis with black A-arms, red anodised shocks and hubs, a tubular front bumper
and a big pitched rear wing high on stands. Pin-spike tyres on black dish rims, red nuts.

    blender -b -P tools/blender/cars/zip.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

from mathutils import Vector as V  # noqa: E402

from lib import core, export, geo, parts, shell, wheel  # noqa: E402

CAR = 'zip'
D = export.dims(CAR)
core.reset()
core.car_materials(paint='#8BD12A', accent='#1D1D20', rim='#1E1F22', metal='#D41F26')
core.material('trim_grey', '#9AA0A8', 0.0, 0.4, coat=1.0, coat_rough=0.05)
mi = core.mi
yF, yR = D['wheelbase'] / 2, -D['wheelbase'] / 2
zW = D['radius'][0]
CLR = D['clearance']
Y_TAIL, Y_NOSE = -0.56, 0.80

# --- shell: narrow wedge between the wheels -------------------------------------------------
prof = shell.Profile([
    (Y_TAIL, dict(hb=0.27, hw=0.26, z0=0.27, zb=0.51, hg=0.22, zg=0.525, hr=0.15, zt=0.535,
                  rs=0.025)),
    (-0.46, dict(hb=0.31, hw=0.29, z0=0.23, zb=0.49, hg=0.25, zg=0.51, hr=0.16, zt=0.55)),
    (-0.34, dict(zt=0.61)),
    (-0.22, dict(hb=0.335, hw=0.30, z0=0.21, zb=0.47, hg=0.25, zg=0.495, hr=0.165, zt=0.66)),
    (0.00, dict(hb=0.335, hw=0.30, zb=0.46, hg=0.245, zg=0.485, hr=0.16, zt=0.668)),
    (0.12, dict(hb=0.32, hw=0.29, zt=0.645)),
    (0.32, dict(hb=0.26, hw=0.25, z0=0.22, zb=0.50, hg=0.21, zg=0.525, hr=0.14, zt=0.55,
                crown=-0.02)),
    (0.46, dict(hb=0.22, hw=0.22, z0=0.25, zb=0.51, hg=0.19, zg=0.53, hr=0.14, zt=0.54,
                crown=-0.04)),
    (0.64, dict(hb=0.18, hw=0.175, z0=0.30, zb=0.475, hg=0.15, zg=0.49, hr=0.10, zt=0.495,
                crown=-0.03)),
    (Y_NOSE, dict(hb=0.12, hw=0.12, z0=0.35, zb=0.43, hg=0.085, zg=0.44, hr=0.05, zt=0.445,
                  crown=0.0)),
    (0.0, dict(bulge=0.012, zm=0.35, rs=0.02, rg=0.025)),
])
bm = core.new_bm()
faces = shell.shell(bm, prof, Y_TAIL, Y_NOSE, ends=(0.035, 0.09), end_scale=0.55, step=0.045,
                    roof=(-0.30, 0.10, 'paint_accent'))
L = 2 * len(shell.section(prof.at(0.0), 0.21, 0.21, 1e3, 3)) - 2  # verts per ring
for i, f in enumerate(faces):  # black lower lip: section columns up to the first side face
    if len(f.verts) == 4 and f.material_index == mi('paint') and min(i % L, L - 1 - i % L) <= 12:
        f.material_index = mi('paint_accent')
body_shell = core.obj_from_bm('shell', bm, weighted=False, angle=48)
surf = geo.Surface([body_shell])

# --- glass, lamps, livery ----------------------------------------------------------------------
bm = core.new_bm()
acc, white, grey, pl = mi('paint_accent'), mi('decal'), mi('trim_grey'), mi('plastic')
shell.glass(bm, surf, prof, windshield=(0.13, 0.31), side=(-0.32, 0.29))
geo.decal(bm, surf, geo.rrect(0.40, 0.10, 0.03, 3), V((0, -3, 0.47)), V((1, 0, 0)), V((0, 0, 1)),
          acc)  # black tail panel
geo.decal(bm, surf, geo.ccw([V(p) for p in ((-0.115, 0.425), (0.115, 0.425), (0.10, 0.385),
                                             (0.03, 0.37), (-0.03, 0.37), (-0.10, 0.385))]),
          V((0, 3, 0)), V((-1, 0, 0)), V((0, 0, 1)), mi('plastic'))  # dark headlight band
parts.lamps(bm, surf, [(-0.025, -0.008), (0.03, -0.004), (0.032, 0.012), (-0.022, 0.01)],
            0.068, 0.405, True, 'light_head', height=0.006)
parts.lamps(bm, surf, geo.rrect(0.07, 0.03, 0.01, 2), 0.17, 0.497, False, 'light_tail')
parts.lamps(bm, surf, geo.rrect(0.05, 0.03, 0.01, 2), 0.07, 0.497, False, 'light_brake')
for s in (-1, 1):  # side planes: 2D x = s * world y, normal points outward
    side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))

    def sp(pts):
        return geo.ccw([V((s * y, z)) for y, z in pts])

    # black lower side rising toward the tail, grey + white swooshes along its edge
    low = [(y, prof.at(y)['z0'] + 0.03) for y in (-0.53, -0.3, 0.0, 0.25, 0.40, 0.50)]
    geo.decal(bm, surf, sp([(0.50, 0.30), (0.40, 0.31), (0.10, 0.35), (-0.20, 0.45),
                            (-0.36, 0.50), (-0.45, 0.54), (-0.53, 0.53)] + low), *side, acc, offset=0.006, back=0.3,
              h=0.024)
    geo.strip(bm, surf, [(s * 0.46, 0.335), (s * 0.10, 0.375), (s * -0.18, 0.47),
                         (s * -0.30, 0.50)], 0.026, *side, white, offset=0.008, back=0.3)
    geo.strip(bm, surf, [(s * 0.32, 0.30), (s * 0.02, 0.33), (s * -0.26, 0.43)], 0.02,
              *side, grey, offset=0.008, back=0.3)
    geo.strip(bm, surf, [(s * -0.17, 0.485), (s * -0.085, 0.635)], 0.03, *side, mi('paint'),
              offset=0.0055, back=0.3)  # slanted B-pillar splits the side glass
    # side scoop: raised grey vent with dark slats in the black lower side
    geo.solid_decal(bm, surf, sp([(0.10, 0.265), (-0.15, 0.28), (-0.19, 0.37), (0.05, 0.32)]),
                    *side, grey, height=0.009, side_mat=pl)
    for k in range(3):
        y0 = 0.04 - k * 0.07
        geo.strip(bm, surf, [(s * y0, 0.285), (s * (y0 - 0.04), 0.345)], 0.014, *side, pl,
                  offset=0.0115, back=0.3)
top = (V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)))
geo.decal(bm, surf, geo.ccw([V(p) for p in ((0.0, -0.08), (0.12, -0.30), (0.18, -0.555),
                                             (-0.18, -0.555), (-0.12, -0.30))]),
          *top, acc, back=0.3)  # black deck spine, arrow forward
for x in (-1, 1):  # grey fender-ridge lines, white chevrons on the hood and deck
    geo.strip(bm, surf, [(x * 0.20, 0.30), (x * 0.155, 0.50), (x * 0.10, 0.70)], 0.016, *top,
              grey, back=0.3)
    geo.strip(bm, surf, [(x * 0.10, 0.42), (0.0, 0.62)], 0.022, *top, white, back=0.3)
    geo.strip(bm, surf, [(x * 0.12, -0.36), (0.0, -0.20)], 0.018, *top, white, offset=0.005,
              back=0.3)
decals = core.obj_from_bm('decals', bm, weighted=False, angle=60)

# --- hard parts: chassis, skid, towers, wing ----------------------------------------------------
bm = core.new_bm()
pl = mi('plastic')
geo.slab(bm, [(-0.68, 0.11, CLR + 0.03, CLR + 0.07, 0.015), (-0.55, 0.16, CLR, CLR + 0.04, 0.015),
              (0.48, 0.16, CLR, CLR + 0.04, 0.015), (0.66, 0.12, CLR + 0.03, CLR + 0.07, 0.015),
              (0.80, 0.09, CLR + 0.09, CLR + 0.12, 0.015)], pl)
geo.slab(bm, [(0.78, 0.15, 0.25, 0.30, 0.015), (0.84, 0.13, 0.28, 0.34, 0.015)], pl)  # skid
geo.slab(bm, [(-0.66, 0.10, CLR + 0.03, 0.36, 0.03), (-0.48, 0.10, CLR + 0.03, 0.34, 0.03)],
         pl)  # rear gearbox
red = mi('metal')
parts.bar(bm, V((-0.25, -0.64, 0.43)), V((0.25, -0.64, 0.43)), 0.045, 0.022, red,
          up=V((0, 1, 0)))  # rear shock tower (red anodised): crossbar, legs, centre post
for s in (-1, 1):
    parts.bar(bm, V((s * 0.07, -0.64, CLR + 0.08)), V((s * 0.23, -0.64, 0.435)), 0.04, 0.022, red,
              up=V((0, 1, 0)))
parts.bar(bm, V((0, -0.64, CLR + 0.10)), V((0, -0.64, 0.43)), 0.05, 0.022, red, up=V((0, 1, 0)))
for s in (-1, 1):
    parts.bar(bm, V((s * 0.08, -0.645, 0.43)), V((s * 0.10, -0.74, 0.625)), 0.035, 0.014, pl,
              up=V((1, 0, 0)))  # wing stands
    for yy in (yF, yR):  # hub carriers (red anodised)
        geo.box(bm, V((s * 0.372, yy, zW)), (0.035, 0.07, 0.12), mi('metal'))
parts.wing(bm, V((0, -0.755, 0.645)), 0.62, 0.28, thick=0.09, angle=-14, mat='paint_accent')
geo.box(bm, V((0, -0.886, 0.695)), (0.60, 0.012, 0.036), acc)  # gurney lip
for s in (-1, 1):  # big endplates pitched with the wing
    poly = geo.rounded_poly([(-0.625, 0.55), (-0.61, 0.64), (-0.895, 0.718), (-0.905, 0.60)],
                            [0.015, 0.015, 0.015, 0.015], 2)
    x = s * 0.31
    geo.extrude_x(bm, [(p.x, p.y) for p in poly], min(x, x + s * 0.012), max(x, x + s * 0.012),
                  acc)
hard = core.obj_from_bm('hard', bm, bevel=0.005, angle=40)

# --- round parts: bumper tube, suspension, driveshafts -----------------------------------------
bm = core.new_bm()
geo.tube(bm, [(-0.09, 0.70, 0.22), (-0.17, 0.82, 0.24), (-0.17, 0.87, 0.33),
              (0.17, 0.87, 0.33), (0.17, 0.82, 0.24), (0.09, 0.70, 0.22)], 0.022, pl, n=8,
         corner=0.05)
for s in (-1, 1):
    for yy, top_pt, ty in ((yF, (0.16, 0.47), yF - 0.01), (yR, (0.21, 0.43), yR - 0.065)):
        parts.a_arm(bm, (s * 0.12, yy - 0.07, CLR + 0.04), (s * 0.12, yy + 0.07, CLR + 0.04),
                    (s * 0.36, yy, zW - 0.05), w=0.03, t=0.02)
        parts.bar(bm, V((s * 0.13, yy, 0.36)), V((s * 0.36, yy, zW + 0.05)), 0.018, 0.012, pl)
        parts.shock(bm, (s * 0.30, yy + 0.02, zW - 0.04), (s * top_pt[0], ty, top_pt[1]),
                    r=0.022, spring='plastic', body='metal')
        geo.cylinder(bm, V((s * 0.09, yy, zW)), V((s * 0.36, yy, zW)), 0.012, pl, n=8)
round_ = core.obj_from_bm('round', bm, weighted=False, angle=40)

wheel.TREADS['knobby'] = dict(kh=0.065, shoulder=0.32)  # this run only: dense low pins
wheels = export.wheels(D, front=dict(tread='knobby', rim=dict(spokes=0, depth=0.14, hub=0.34,
                                                               nut=0.5), rim_ratio=0.55, rows=26))
ant_base = V((-0.20, -0.50, 0.50))
ant = parts.antenna(ant_base, 0.32)
export.finish(CAR, D, [body_shell, decals, hard, round_], wheels, ant, ant_base)
