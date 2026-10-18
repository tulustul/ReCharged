"""Plywood ramp: two 2.44 m sheets end to end (a full and a cut sheet across, 8 u wide) on three
timber stringers and posts, rising 4.5 u over 21.7 u (11.7 deg) toward its front (+Y, Bevy -Z),
where it rests against the workbench kicker lip; a galvanised plate at the foot. The collider is
the catalog's Wedge, so the open framework is never driven into.

    blender -b -P tools/blender/props/workshop/ramp.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wkit import *  # noqa: E402,F403

B = C.BENCH
RUN, RISE, HW = B['ramp_run'], B['top'] + B['kicker_rise'], B['ramp_width'] / 2
T = 0.1  # deck thickness (two glued 18 mm sheets, chunky for readability)
SLOPE = RISE / RUN
ANG = math.atan(SLOPE)

start('plywood', 'plywood_edge', 'timber', 'steel_dark', 'steel')


def z_top(y):
    return (y + RUN / 2) * SLOPE


def slab(bm, x0, x1, y0, y1, t, mat):
    """Box lying on the slope: top face on the ramp surface between y0..y1 (plan), thickness t
    measured perpendicular to the slope."""
    n = V((0, -math.sin(ANG), math.cos(ANG)))  # slope normal
    top = [V((x, y, z_top(y))) for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]
    bot = [p - n * t for p in top]
    vs = [bm.verts.new(p) for p in bot + top]
    idx = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
    fs = [bm.faces.new([vs[i] for i in q]) for q in idx]
    for f in fs:
        f.material_index = mat
    bmesh.ops.recalc_face_normals(bm, faces=fs)
    return fs


# --- deck: 2 x (full sheet 5.4 u + cut sheet 2.6 u) with 0.03 u seams -----------------------------
bm = core.new_bm()
half = RUN / 2
for y0, y1 in ((-half, -0.015), (0.015, half)):
    for x0, x1 in ((-HW, 1.385), (1.415, HW)):
        fs = slab(bm, x0, x1, y0, y1, T, mi('plywood'))
        top = [f for f in fs if f.normal.z > 0.9]
        planar_uv(bm, top, (1, 0, 0), (0, math.cos(ANG), math.sin(ANG)), 0.25)
        edges = [f for f in fs if f.normal.z <= 0.9]
        for f in edges:
            f.material_index = mi('plywood_edge')
        box_uv(bm, edges, 1.8)
for v in bm.verts:  # the sheet foot rests on the floor: chamfer it at z = 0
    v.co.z = max(v.co.z, 0.0)
deck = core.obj_from_bm('deck', bm, bevel=0.012, segments=1, angle=30)

# --- framework: stringers under the deck, posts, a cross bearer at the seam ------------------------
bm = core.new_bm()
tm = mi('timber')
n = V((0, -math.sin(ANG), math.cos(ANG)))
for x in (-HW + 0.35, 0.0, HW - 0.35):
    a = V((x, -half + 3.6, z_top(-half + 3.6))) - n * (T + 0.3)
    b = V((x, half - 0.1, z_top(half - 0.1))) - n * (T + 0.3)
    f = geo.prism(bm, geo.rrect(0.17, 0.6, 0.0, 1)[::2], a, b, tm, up=n)
    box_uv(bm, f, 0.5)
    for y in (-3.5, 3.0, 9.0):  # posts down to the floor
        zt = z_top(y) - T - 0.6
        if zt > 0.4:
            f = geo.box(bm, V((x, y, zt / 2)), (0.17, 0.4, zt), tm)
            box_uv(bm, f, 0.5)
zc = z_top(0.0) - T - 0.6
f = geo.box(bm, V((0, 0.0, zc - 0.2)), (2 * HW - 0.4, 0.4, 0.4), tm)  # cross bearer
box_uv(bm, f, 0.5)
frame = core.obj_from_bm('frame', bm, bevel=0.02, segments=1, angle=40)

# --- galvanised foot plate bridging the deck edge to the floor -------------------------------------
bm = core.new_bm()
p0, p1 = V((0, -half - 0.55, 0.031)), V((0, -half + 0.8, z_top(-half + 0.8) + 0.004))
geo.prism(bm, [(-HW, 0.0), (HW, 0.0), (HW, -0.03), (-HW, -0.03)], p0, p1, mi('steel_dark'),
          up=V((0, 0, 1)))
plate = core.obj_from_bm('plate', bm, bevel=0.01, segments=1, angle=30)

finish('ramp', [deck, frame, plate], budget=KIT, car=(0.0, -half - 3.0, 0.0))
extra_shots('ramp', [('rc_foot', (1.5, -half - 14.0, 1.0), (0.0, 0.0, 2.0), 30),
                     ('side', (-14.0, 0.0, 2.0), (0.0, 0.0, 1.8), 40)])
