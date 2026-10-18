"""Drive-through spray booth (hero), 36 x 18 x 13 u (8 x 4 x 3 m inside): white insulated
panel walls with seams, angled light boxes along both roof edges, a filter plenum on the roof
with an exhaust duct rising to the workshop ceiling, steel door frames with bi-fold doors folded
open at both ends, exhaust grates along the floor, two paint stands with primered panels
(a bonnet with masking paper, a bumper) by the left wall. Long axis = Y (Bevy z).

    blender -b -P tools/blender/props/workshop/paint_booth.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

BO = C.BOOTH
HL, HW, H, WT = BO['length'] / 2, BO['width'] / 2, BO['height'], BO['wall']
DW, DH = BO['door_w'] / 2, BO['door_h']

start('paint_white', 'filter_white', 'steel', 'steel_dark', 'light_panel', 'glass_dark',
      'primer', 'masking', 'plastic_black', 'paint_grey', 'plastic_red', 'safety_yellow')
wh = mi('paint_white')

# --- shell: side walls, end walls around the door openings, roof, plenum ----------------------
bm = core.new_bm()
for sx in (-1, 1):
    geo.box(bm, V((sx * (HW - WT / 2), 0, H / 2)), (WT, 2 * HL, H), wh)
    for sy in (-1, 1):
        w = HW - DW
        geo.box(bm, V((sx * (DW + w / 2), sy * (HL - WT / 2), H / 2)), (w, WT, H), wh)
for sy in (-1, 1):  # lintels over the doors
    geo.box(bm, V((0, sy * (HL - WT / 2), DH + (H - DH) / 2)), (2 * DW, WT, H - DH), wh)
geo.box(bm, V((0, 0, H + 0.25)), (2 * HW, 2 * HL, 0.5), wh)                   # roof deck
geo.box(bm, V((0, 0, H + 1.6)), (2 * HW - 2.0, 2 * HL - 3.0, 2.2), mi('paint_grey'))  # plenum
shell = core.obj_from_bm('shell', bm, bevel=0.06, segments=2, angle=40)

# --- steel: door frames, panel seams (inside and out), corner posts, base channel ---------------
bm = core.new_bm()
st = mi('steel')
for sy in (-1, 1):
    y = sy * (HL + 0.05)
    for sx in (-1, 1):
        geo.box(bm, V((sx * (DW + 0.15), y, DH / 2)), (0.3, WT + 0.2, DH), st)
    geo.box(bm, V((0, y, DH + 0.15)), (2 * DW + 0.6, WT + 0.2, 0.3), st)
for sx in (-1, 1):
    for k in range(1, 8):  # vertical seams every 4.5 u on both faces of the side walls
        y = -HL + k * 4.5
        for face in (-1, 1):
            x = sx * (HW - WT / 2) + face * (WT / 2 + 0.02)
            geo.box(bm, V((x, y, H / 2)), (0.04, 0.12, H - 0.2), st)
    for sy in (-1, 1):  # corner posts
        geo.box(bm, V((sx * HW, sy * HL, (H + 0.1) / 2)), (0.4, 0.4, H + 0.1), st)
    geo.box(bm, V((sx * (HW - WT / 2), 0, 0.2)), (WT + 0.12, 2 * HL, 0.4), mi('steel_dark'))
for sx in (-1, 1):  # exhaust floor grates along both walls (visual, flush)
    geo.box(bm, V((sx * (HW - WT - 0.9), 0, 0.012)), (1.6, 2 * HL - 2 * WT - 0.4, 0.024),
            mi('steel_dark'))
steel = core.obj_from_bm('steel', bm, bevel=0.02, segments=1, angle=40)

# --- light boxes: angled glass panels along both roof edges, 4 per side ---------------------------
bm = core.new_bm()
for sx in (-1, 1):
    for k in range(4):
        y0 = -HL + WT + 0.4 + k * ((2 * HL - 2 * WT - 0.8) / 4)
        y1 = y0 + (2 * HL - 2 * WT - 0.8) / 4 - 0.4
        prof = [(sx * (HW - WT), H - 0.05), (sx * (HW - WT - 1.9), H - 0.05),
                (sx * (HW - WT), H - 1.9)]
        geo.extrude_y(bm, prof if sx > 0 else prof[::-1], y0, y1, mi('paint_grey'))
        # the emissive panel on the slanted face, slightly proud
        a, b = V((sx * (HW - WT - 1.75), 0, H - 0.2)), V((sx * (HW - WT - 0.15), 0, H - 1.75))
        nrm = V((-sx, 0, -1)).normalized() * 0.03
        vs = [bm.verts.new(p + V((0, y, 0)) + nrm) for p, y in ((a, y0 + 0.15), (b, y0 + 0.15),
                                                               (b, y1 - 0.15), (a, y1 - 0.15))]
        f = bm.faces.new(vs)
        f.material_index = mi('light_panel')
        f.normal_update()
        if f.normal.dot(nrm) < 0:
            f.normal_flip()
lights = core.obj_from_bm('lights', bm, weighted=False, angle=40)

# --- filter ceiling inside (white with a grid of frames) -----------------------------------------
bm = core.new_bm()
geo.box(bm, V((0, 0, H - 0.08)), (2 * HW - 2 * WT - 3.9, 2 * HL - 2 * WT, 0.1), mi('filter_white'))
for k in range(1, 8):
    geo.box(bm, V((0, -HL + k * 4.5, H - 0.16)), (2 * HW - 2 * WT - 3.9, 0.12, 0.08), st)
for x in (-2.5, 2.5):
    geo.box(bm, V((x, 0, H - 0.16)), (0.12, 2 * HL - 2 * WT, 0.08), st)
ceiling = core.obj_from_bm('ceiling', bm, weighted=False, angle=40)

# --- bi-fold doors folded open against the end walls (two leaves each side, windows) ------------
bm = core.new_bm()
for sy in (-1, 1):
    y = sy * (HL + 0.32)
    for sx in (-1, 1):
        for k in range(2):
            x = sx * (DW + 0.5 + 0.08 + k * 0.2)
            geo.box(bm, V((x + sx * 1.2, y + sy * k * 0.18, DH / 2)), (2.5, 0.14, DH - 0.2), wh)
            geo.box(bm, V((x + sx * 1.2, y + sy * (k * 0.18 + 0.08), DH * 0.62)),
                    (1.5, 0.03, DH * 0.42), mi('glass_dark'))
doors = core.obj_from_bm('doors', bm, bevel=0.03, segments=1, angle=40)

# --- exhaust duct up to the workshop ceiling, control panel by the entry -------------------------
bm = core.new_bm()
geo.cylinder(bm, V((0, -HL + 5.0, H + 2.7)), V((0, -HL + 5.0, 23.6)), 1.15, mi('galvanised')
             if 'galvanised' in core.MATS else st, n=16)
for z in (H + 4.5, H + 7.5):
    geo.cylinder(bm, V((0, -HL + 5.0, z)), V((0, -HL + 5.0, z + 0.25)), 1.22, st, n=16)
cp = V((HW + 0.25, HL - 2.2, 4.2))
geo.box(bm, cp, (0.4, 1.4, 2.0), mi('paint_grey'))
for k, mat in enumerate(('safety_yellow', 'plastic_red', 'plastic_black')):
    geo.cylinder(bm, cp + V((0.2, -0.4 + k * 0.4, 0.45)), cp + V((0.32, -0.4 + k * 0.4, 0.45)),
                 0.12, mi(mat), n=10)
duct = core.obj_from_bm('duct', bm, weighted=False, angle=40)

# --- paint stands with primered panels (left wall): a bonnet tilted up, a front bumper -------
bm = core.new_bm()
dk = mi('steel_dark')
for i, bz in enumerate(C.BOOTH_STANDS_Z):
    y = -bz  # Bevy z -> Blender -y
    x = C.BOOTH_STAND_X
    for dy in (-1.3, 1.3):  # two uprights on foot bars
        geo.box(bm, V((x, y + dy, 1.4)), (0.16, 0.16, 2.8), dk)
        geo.box(bm, V((x, y + dy, 0.06)), (1.7, 0.2, 0.12), dk)
    geo.box(bm, V((x, y, 2.7)), (0.16, 2.9, 0.14), dk)
    if i == 0:  # bonnet (1.4 x 1.0 m) leaning back 20 deg, front edge masked with paper
        n = V((math.cos(math.radians(20)), 0, math.sin(math.radians(20))))
        up = V((-n.z, 0, n.x))  # along the panel, upward
        c = V((x + 0.35, y, 2.95)) + up * 1.6
        for a, b, mat in ((-2.05, 2.05, 'primer'), (-2.05, -1.25, 'masking')):
            p0, p1 = c + up * a, c + up * b
            geo.prism(bm, [(-3.1, -0.05), (3.1, -0.05), (3.1, 0.05), (-3.1, 0.05)], p0, p1,
                      mi(mat), up=n)
    else:  # front bumper: a long rounded section with a grille opening
        geo.slab(bm, [(y - 3.6, 0.4, 2.75, 3.85, 0.25), (y - 2.8, 0.55, 2.75, 3.9, 0.3),
                      (y + 2.8, 0.55, 2.75, 3.9, 0.3), (y + 3.6, 0.4, 2.75, 3.85, 0.25)],
                 mi('primer'), cx=x + 0.3)
        geo.box(bm, V((x + 0.86, y, 3.2)), (0.04, 3.2, 0.45), mi('plastic_black'))
stands = core.obj_from_bm('stands', bm, bevel=0.02, segments=1, angle=40)

finish('paint_booth', [shell, steel, lights, ceiling, doors, duct, stands], budget=HERO,
       car=(2.0, HL + 4.0, 180.0))
extra_shots('paint_booth', [('rc_entry', (2.5, HL + 16.0, 1.0), (0.0, 0.0, 4.0), 26),
                            ('rc_inside', (1.5, HL - 3.0, 1.0), (-2.0, -HL, 3.0), 22)])
