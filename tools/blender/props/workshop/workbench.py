"""Workbench island (hero): two steel-frame benches back to back under one butcher-block top,
5.4 x 1.8 m, 0.9 m high (24 x 8 u, top at 4.0 u); a plywood kicker screwed on each end rising to
the edge (the M lane's jump lips); a red drawer unit under one half, a shelf with clutter under
the other. The racing line runs along the top (long axis = Y, Bevy z).

    blender -b -P tools/blender/props/workshop/workbench.py -- [--preview DIR]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

B = C.BENCH
HL, HW, TOP, TT = B['length'] / 2, B['width'] / 2, B['top'], B['top_thick']
KR, KZ = B['kicker_rise'], B['kicker_run']

start('butcher_block', 'plywood', 'plywood_edge', 'paint_grey', 'steel_dark', 'chest_red',
      'aluminium', 'cardboard', 'plastic_red', 'amber_plastic', 'tyre', 'rubber',
      'plastic_black', 'galvanised')
rng = random.Random(7)

# --- top and kickers (the drivable surface: matches the catalog colliders) -------------------
bm = core.new_bm()
f = geo.box(bm, V((0, 0, TOP - TT / 2)), (2 * HW, 2 * HL, TT), mi('butcher_block'))
planar_uv(bm, [x for x in f if abs(x.normal.z) > 0.5], (1, 0, 0), (0, 1, 0), 0.25)
box_uv(bm, [x for x in f if abs(x.normal.z) <= 0.5], 0.25)
top = core.obj_from_bm('top', bm, bevel=0.03, segments=2, angle=40)

bm = core.new_bm()
for sy in (-1, 1):  # kicker wedge: from the top up to KR at the bench end
    y0, y1 = sy * (HL - KZ), sy * HL
    prof = [(y0, TOP), (y1, TOP), (y1, TOP + KR)]
    f = geo.extrude_x(bm, prof if sy > 0 else prof[::-1], -HW, HW, mi('plywood'))
    planar_uv(bm, [x for x in f if x.normal.z > 0.3], (1, 0, 0), (0, 1, 0), 0.25)
    edge = [x for x in f if x.normal.z <= 0.3]
    for x in edge:
        x.material_index = mi('plywood_edge')
    box_uv(bm, edge, 1.6)
kick = core.obj_from_bm('kickers', bm, bevel=0.015, segments=1, angle=30)

# --- steel frame: legs, apron and lower rails, lower shelf -------------------------------------
bm = core.new_bm()
fr = mi('paint_grey')
LX, LY = HW - 0.35, HL - 0.35
for sx in (-1, 1):
    for y in (-LY, 0.0, LY):
        geo.box(bm, V((sx * LX, y, (TOP - TT) / 2)), (0.4, 0.4, TOP - TT), fr)
        geo.box(bm, V((sx * LX, y, 0.04)), (0.62, 0.62, 0.08), mi('steel_dark'))  # foot plate
    geo.box(bm, V((sx * LX, 0, TOP - TT - 0.15)), (0.3, 2 * LY, 0.3), fr)        # apron rails
    geo.box(bm, V((sx * LX, 0, 0.55)), (0.25, 2 * LY, 0.25), fr)                 # lower rails
for y in (-LY, 0.0, LY):
    geo.box(bm, V((0, y, TOP - TT - 0.15)), (2 * LX, 0.3, 0.3), fr)
    geo.box(bm, V((0, y, 0.55)), (2 * LX, 0.25, 0.25), fr)
geo.box(bm, V((0, -LY / 2, 0.70)), (2 * LX - 0.1, LY - 0.3, 0.06), mi('galvanised'))  # shelf
# drawer unit under the +Y half: 3 drawers per long side, aluminium pulls
geo.box(bm, V((0, LY / 2, 2.55)), (2 * LX - 0.3, LY - 0.5, 1.9), mi('chest_red'))
frame = core.obj_from_bm('frame', bm, bevel=0.03, segments=1, angle=40)

bm = core.new_bm()
for sx in (-1, 1):
    for k in range(3):
        z = 1.95 + k * 0.62
        geo.box(bm, V((sx * (LX - 0.12), LY / 2, z)), (0.05, LY - 0.8, 0.54), mi('chest_red'))
        geo.box(bm, V((sx * (LX - 0.07), LY / 2, z + 0.12)), (0.06, LY * 0.45, 0.09),
                mi('aluminium'))
drawers = core.obj_from_bm('drawers', bm, bevel=0.012, segments=1, angle=40)

# --- clutter on the lower shelf (under the -Y half) ----------------------------------------------
bm = core.new_bm()
y = -LY + 0.6
for _ in range(4):  # cardboard boxes of parts
    w, d, h = rng.uniform(1.4, 2.4), rng.uniform(1.2, 1.9), rng.uniform(0.9, 1.6)
    if y + d > -0.8:
        break
    x = rng.uniform(-LX + w / 2 + 0.3, LX - w / 2 - 0.3)
    geo.box(bm, V((x, y + d / 2, 0.73 + h / 2)), (w, d, h), mi('cardboard'))
    geo.box(bm, V((x, y + d / 2, 0.73 + h + 0.006)), (w * 0.12, d + 0.01, 0.012),
            mi('plastic_black'))  # tape strip
    y += d + rng.uniform(0.3, 0.8)
geo.box(bm, V((-1.6, -4.8, 0.73 + 0.5)), (2.1, 1.0, 1.0), mi('plastic_red'))  # toolbox
geo.box(bm, V((-1.6, -4.8, 0.73 + 1.08)), (1.3, 0.16, 0.12), mi('plastic_black'))
for j in range(2):  # oil jugs
    geo.box(bm, V((2.3 + j * 0.95, -6.6, 0.73 + 0.62)), (0.85, 0.5, 1.24), mi('amber_plastic'))
    geo.cylinder(bm, V((2.3 + j * 0.95 + 0.25, -6.6, 0.73 + 1.24)),
                 V((2.3 + j * 0.95 + 0.25, -6.6, 0.73 + 1.42)), 0.12, mi('plastic_black'), n=8)
clutter = core.obj_from_bm('clutter', bm, bevel=0.02, segments=1, angle=40)

bm = core.new_bm()
tyre(bm, V((1.4, -2.6, 0.73 + C.TYRE['w'] / 2)), spin=40)  # a spare tyre on the shelf
spare = core.obj_from_bm('spare', bm, weighted=False, angle=60)

finish('workbench', [top, kick, frame, drawers, clutter, spare], budget=HERO,
       car=(0.0, -HL + 2.0, 0.0), before_preview=lambda ob: propkit.ref_car(0.0, 2.0, 0.0))
extra_shots('workbench', [('rc_end', (2.0, -34.0, 1.0), (0.0, 0.0, 3.0), 30),
                          ('chase_top', (0.0, -16.0, TOP + 1.7), (0.0, 0.0, TOP + 0.3), 50)])
