"""The raised front yard's pieces: the kids' plank ramp (two plywood sheets on 2x4 rails and
cinder-block stacks), the plywood kicker on bricks, and the stacked-stone retaining wall.

    blender -b -P tools/blender/props/street/yard_ramps.py -- [--preview DIR] [--only NAME]

Ramps rise toward +Y (Bevy -Z, the catalog's Wedge front); origins at the ground contact.
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

import catalog as C  # noqa: E402

PLY = 0.09  # plywood thickness (20 mm)


def sheet(bm, x0, x1, y0, z0, y1, z1, top, edge):
    """Plywood sheet whose top surface runs from (y0, z0) to (y1, z1), across x0..x1."""
    ln = math.hypot(y1 - y0, z1 - z0)
    ny, nz = -(z1 - z0) / ln * PLY, (y1 - y0) / ln * PLY  # down the sheet normal
    quad = [(y0, z0), (y1, z1), (y1 - ny, z1 - nz), (y0 - ny, z0 - nz)]
    f = geo.extrude_x(bm, quad, x0, x1, edge)
    for face in f:  # the up-facing big face gets the veneer
        if face.normal.z > 0.5 and face.calc_area() > 1.0:
            face.material_index = top
    return f


def cinder(bm, c, along_y=True):
    """Cinder block 1.8 x 0.9 x 0.9 u with two cores (open top/bottom)."""
    sx, sy = (0.9, 1.8) if along_y else (1.8, 0.9)
    m = mi('cinder')
    w = 0.16
    x, y, z = c
    if along_y:
        for dy in (-0.9 + w / 2, 0.0, 0.9 - w / 2):
            geo.box(bm, V((x, y + dy, z + 0.45)), (0.9, w, 0.9), m)
        for dx in (-0.45 + w / 2, 0.45 - w / 2):
            geo.box(bm, V((x + dx, y, z + 0.45)), (w, 1.8, 0.9), m)
    else:
        for dx in (-0.9 + w / 2, 0.0, 0.9 - w / 2):
            geo.box(bm, V((x + dx, y, z + 0.45)), (w, 0.9, 0.9), m)
        for dy in (-0.45 + w / 2, 0.45 - w / 2):
            geo.box(bm, V((x, y + dy, z + 0.45)), (1.8, w, 0.9), m)


def plank_ramp():
    start('plywood', 'plywood_edge', 'cinder', 'brick')
    p = C.PLANK
    hw, rise, run = p['width'] / 2, p['rise'], p['run']
    y0, y1 = -run / 2, run / 2
    bm = core.new_bm()
    gap = 0.03
    for xa, xb in ((-hw, -gap), (gap, hw)):
        sheet(bm, xa, xb, y0, 0.0, y1, rise, mi('plywood'), mi('plywood_edge'))
    rails = core.new_bm()
    for x in (-hw + 0.4, -0.25, 0.25, hw - 0.4):  # 2x4 rails under the sheets
        prof = [(-0.08, -0.36), (0.08, -0.36), (0.08, 0.0), (-0.08, 0.0)]
        a = V((x, y0 + 1.8, rise * 1.8 / run - PLY))
        b = V((x, y1 - 0.3, rise * (run - 0.3) / run - PLY))
        geo.prism(rails, prof, a, b, mi('plywood_edge'), up=V((0, -rise, run)).normalized())
    blocks = core.new_bm()
    for frac, layers in ((0.25, 1), (0.6, 2), (0.94, 3)):  # cinder stacks under the rails
        y = y0 + run * frac
        h = rise * frac - PLY - 0.36
        n = min(layers, int(h / 0.9))
        for x in (-hw + 0.9, 0.0, hw - 0.9):
            for k in range(n):
                cinder(blocks, (x, y, 0.9 * k), along_y=(k % 2 == 0))
            if h - 0.9 * n > 0.05:  # brick shim up to the rail
                geo.box(blocks, V((x, y, 0.9 * n + (h - 0.9 * n) / 2)),
                        (1.0, 0.5, h - 0.9 * n), mi('brick'))
    parts = [core.obj_from_bm('sheets', bm, bevel=0.02, segments=1, angle=30),
             core.obj_from_bm('rails', rails, bevel=0.015, segments=1, angle=30),
             core.obj_from_bm('blocks', blocks, bevel=0.03, segments=1, angle=30)]
    propkit.finish(ENV, 'plank_ramp', parts, below=0.1, car=(hw + 2.5, -4.0, 0))


def kicker():
    start('plywood', 'plywood_edge', 'brick')
    k = C.KICKER
    hw, rise, run = k['width'] / 2, k['rise'], k['run']
    y0, y1 = -run / 2, run / 2
    bm = core.new_bm()
    sheet(bm, -hw, hw, y0, 0.0, y1, rise, mi('plywood'), mi('plywood_edge'))
    for x in (-hw + 0.05, -hw / 3, hw / 3, hw - 0.05):  # triangular plywood ribs
        tri = [(y0 + 0.3, 0.0), (y1, 0.0), (y1, rise - PLY - 0.02)]
        geo.extrude_x(bm, tri, x - 0.045, x + 0.045, mi('plywood_edge'))
    for x in (-hw + 1.2, 0.0, hw - 1.2):  # bricks along the high end
        geo.box(bm, V((x, y1 - 0.45, 0.2)), (1.0, 0.5, 0.4), mi('brick'))
    propkit.finish(ENV, 'kicker', [core.obj_from_bm('kicker', bm, bevel=0.015, segments=1,
                                                    angle=30)], below=0.1, car=(hw + 2.5, -3.0, 0))


def retaining_wall():
    start('stone_wall', 'kerb')
    wl = C.WALL
    L, H, T = wl['length'], wl['height'], wl['thick']
    rnd = random.Random(7)
    bm = core.new_bm()
    courses = 5
    z = 0.0
    ch = (H - 0.2) / courses
    for c in range(courses):
        x = -L / 2 + (0.0 if c % 2 == 0 else -0.45)
        while x < L / 2 - 0.05:
            w = rnd.uniform(0.9, 1.6)
            xa, xb = max(x, -L / 2), min(x + w, L / 2)
            if xb - xa > 0.2:
                inset = rnd.uniform(0.0, 0.05)
                h = ch - rnd.uniform(0.03, 0.08)
                geo.box(bm, V(((xa + xb) / 2, -inset / 2, z + h / 2 + 0.02)),
                        (xb - xa - 0.06, T - inset, h), mi('stone_wall'))
            x += w
        z += ch
    cap = core.new_bm()  # flat cap stones, slightly overhanging
    x = -L / 2
    while x < L / 2 - 0.05:
        w = min(rnd.uniform(1.4, 2.2), L / 2 - x)
        geo.box(cap, V((x + w / 2, 0.0, H - 0.1)), (w - 0.05, T + 0.12, 0.2), mi('kerb'))
        x += w
    parts = [core.obj_from_bm('stones', bm, bevel=0.05, segments=1, angle=30),
             core.obj_from_bm('cap', cap, bevel=0.03, segments=1, angle=30)]
    propkit.finish(ENV, 'retaining_wall', parts, car=(0, -3.5, 90))


JOBS = {'plank_ramp': plank_ramp, 'kicker': kicker, 'retaining_wall': retaining_wall}
only = propkit._argv()[propkit._argv().index('--only') + 1] if '--only' in propkit._argv() \
    else None
for name, job in JOBS.items():
    if only in (None, name):
        job()
