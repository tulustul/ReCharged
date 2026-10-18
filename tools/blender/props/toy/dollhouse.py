"""Display dollhouse the track drives through (hero). Two storeys and a steep gable roof, the ridge
along the drive-through so cars approach a gable end with an arched opening; cream walls, white
trims, red shingle roof, blue shutters; a porch, yellow door and flower boxes on both long
sides (symmetric, so a porch faces the room in mirrored layouts too). The tunnel (9 u wide, 4 u high, wallpapered) runs along Y (the prop's
front/back). 14 x 10 u plan, ~13 u tall: a walk-in display piece of a toy store. Static.

    blender -b -P tools/blender/props/toy/dollhouse.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

HX, HY = 7.0, 5.0        # half plan size (x across the tunnel, y along it)
TX, TZ, TR = 4.5, 4.0, 1.1  # tunnel half width, height, corner radius
EAVES = 7.4              # wall top (z): two storeys
PITCH = 0.78             # roof rise per unit run (~38 deg)
core.reset()
wood('wall', WALL, 0.55, contrast=0.035, seed=3)  # painted boards: faint grain
painted('trim', '#F4F1EA', 0.45)
painted('roof', '#B9442F', 0.5)
painted('shutter', '#3567B8', 0.4)
painted('door', YELLOW, 0.35)
painted('paper', '#F1D9A0', 0.7)      # tunnel wallpaper
core.material('pane', '#9EC3DA', 0.0, 0.08)
painted('box', '#3E8A4B', 0.45)
plastic('flower_r', RED)
plastic('flower_y', YELLOW)
painted('brick', '#A9503A', 0.7)
mi = core.mi

# --- walls: U-section extruded along y, tunnel faces wallpapered -----------------------------
sec = rpoly([(-HX, 0), (-TX, 0), (-TX, TZ), (TX, TZ), (TX, 0), (HX, 0), (HX, EAVES),
                        (-HX, EAVES)], [0, 0, TR, TR, 0, 0, 0, 0], 6)
walls = core.new_bm()
fs = extrude(walls, 'y', [(p.x, p.y) for p in sec], -HY, HY, mi('wall'))
for f in fs:
    c = f.calc_center_median()
    if abs(c.x) < TX + 0.01 and c.z < TZ + 0.01 and abs(f.normal.y) < 0.5:
        f.material_index = mi('paper')
# gable triangles (the attic) above the eaves
gable = [(-HX, EAVES), (HX, EAVES), (0, EAVES + HX * PITCH)]
extrude(walls, 'y', gable, -HY, HY, mi('wall'))
wall_ob = part('walls', box_uv(walls), bevel=0.06, segments=3)

# --- roof: two slabs + shingle rows + ridge --------------------------------------------------
roof = core.new_bm()
over, th = 0.7, 0.35
ridge = EAVES + HX * PITCH
for s in (-1, 1):  # slab section in xz, extruded along y with gable overhang
    p0 = (s * (HX + over), EAVES - over * PITCH)
    p1 = (0.0, ridge)
    n = V((s * PITCH, 1.0)).normalized()
    slab = [p0, p1, (p1[0] + n.x * th, p1[1] + n.y * th), (p0[0] + n.x * th, p0[1] + n.y * th)]
    extrude(roof, 'y', slab if s > 0 else slab[::-1], -HY - 0.5, HY + 0.5, mi('roof'))
    rows = 8
    for r in range(rows):  # overlapping shingle courses, lowest first
        t0 = r / rows
        a = V(p0).lerp(V(p1), t0) + n * th
        b = V(p0).lerp(V(p1), t0 + 1.25 / rows) + n * th
        d = n * 0.14
        course = [(a.x, a.y), (b.x, b.y), (b.x + d.x, b.y + d.y), (a.x + d.x * 1.6, a.y + d.y * 1.6)]
        extrude(roof, 'y', course if s > 0 else course[::-1], -HY - 0.45, HY + 0.45, mi('roof'))
extrude(roof, 'y', geo.rrect(0.6, 0.5, 0.2, 3, 0.0, ridge + th + 0.15), -HY - 0.55, HY + 0.55,
              mi('roof'))
roof_ob = part('roof', roof, bevel=0.04, segments=2)

# --- chimney ---------------------------------------------------------------------------------
chim = core.new_bm()
geo.box(chim, V((-3.2, -1.8, ridge - 0.5)), (1.4, 1.4, 5.4), mi('brick'))
geo.box(chim, V((-3.2, -1.8, ridge + 2.3)), (1.75, 1.75, 0.35), mi('trim'))
chim_ob = part('chimney', chim, bevel=0.05, segments=2)

# --- trims: corner boards, floor band, eaves fascia, tunnel arch frames ----------------------
trim = core.new_bm()
for sx in (-1, 1):
    for sy in (-1, 1):
        geo.box(trim, V((sx * (HX + 0.02), sy * (HY + 0.02), EAVES / 2)), (0.5, 0.5, EAVES),
                mi('trim'))
for sy in (-1, 1):
    geo.box(trim, V((0, sy * (HY + 0.08), TZ + 0.45)), (2 * HX, 0.18, 0.3), mi('trim'))
    arch = rpoly([(-TX - 0.5, 0), (-TX - 0.5, TZ + 0.5), (TX + 0.5, TZ + 0.5),
                             (TX + 0.5, 0), (TX, 0), (TX, TZ), (-TX, TZ), (-TX, 0)],
                            [0, TR + 0.5, TR + 0.5, 0, 0, TR, TR, 0], 6)
    y = sy * HY
    extrude(trim, 'y', [(p.x, p.y) for p in arch], y - 0.05 * sy, y + 0.16 * sy, mi('trim'))
for sx in (-1, 1):
    geo.box(trim, V((sx * (HX + 0.08), 0, TZ + 0.45)), (0.18, 2 * HY, 0.3), mi('trim'))
trim_ob = part('trims', trim, bevel=0.03, segments=2)


# --- windows (frame, pane, shutters, sill) on any axis-aligned face ---------------------------
win = core.new_bm()


def window(c, n, w, h, shutters=True, round_top=False):
    c, n = V(c), V(n)
    u = V((n.y, -n.x, 0)) if abs(n.z) < 0.5 else V((1, 0, 0))  # along the face
    up = V((0, 0, 1))

    def box_on(cx, cz, sw, sh, depth, out, mat):
        size = [abs(u.x) * sw + abs(n.x) * depth, abs(u.y) * sw + abs(n.y) * depth, sh]
        geo.box(win, c + u * cx + up * cz + n * out, size, mi(mat))

    box_on(0, 0, w, h, 0.1, 0.02, 'pane')
    for sgn in (-1, 1):  # frame bars
        box_on(sgn * (w / 2 + 0.09), 0, 0.2, h + 0.36, 0.22, 0.08, 'trim')
        box_on(0, sgn * (h / 2 + 0.09), w + 0.36, 0.2, 0.22, 0.08, 'trim')
    box_on(0, 0, 0.12, h, 0.12, 0.08, 'trim')  # mullion
    box_on(0, -(h / 2 + 0.28), w + 0.7, 0.16, 0.42, 0.18, 'trim')  # sill
    if shutters:
        for sgn in (-1, 1):
            box_on(sgn * (w / 2 + 0.2 + w * 0.27), 0, w * 0.5, h + 0.1, 0.12, 0.08, 'shutter')


for sy in (-1, 1):  # gable ends: two upstairs windows and an attic window
    for x in (-3.4, 3.4):
        window((x, sy * HY, 5.9), (0, sy, 0), 1.6, 1.8)
    window((0, sy * HY, 9.0), (0, sy, 0), 1.3, 1.3, shutters=False)
for y0 in (-3.0, 3.0):  # long sides: upstairs, and downstairs flanking the doors
    for sx in (-1, 1):
        window((sx * HX, y0, 5.9), (sx, 0, 0), 1.6, 1.8)
        window((sx * HX, y0, 2.2), (sx, 0, 0), 1.6, 1.8)
win_ob = part('windows', win, bevel=0.02, segments=1)

# --- porch, door, flower boxes on both long sides (prop meshes are never mirrored: a symmetric
# house keeps a porch toward the room in mirrored layouts too) ------------------------------
porch = core.new_bm()
rnd = core.new_bm()  # round parts: shaped by their profile, no bevel
for sx in (-1, 1):
    def X(a):
        return sx * (HX + a)

    geo.box(porch, V((X(0.06), 0, 1.45)), (0.16, 1.9, 2.9), mi('door'))
    for sgn in (-1, 1):
        geo.box(porch, V((X(0.14), sgn * 1.07, 1.55)), (0.24, 0.22, 3.1), mi('trim'))
    geo.box(porch, V((X(0.14), 0, 3.08)), (0.24, 2.36, 0.22), mi('trim'))
    geo.box(porch, V((X(0.18), 0, 1.0)), (0.08, 1.3, 0.12), mi('trim'))
    geo.cylinder(rnd, V((X(0.2), 0.62, 1.45)), V((X(0.34), 0.62, 1.45)), 0.11, mi('trim'), n=12)
    geo.box(porch, V((X(1.0), 0, 0.12)), (2.0, 3.6, 0.24), mi('trim'))   # deck
    geo.box(porch, V((X(2.15), 0, 0.06)), (0.5, 2.6, 0.12), mi('trim'))  # step
    for sgn in (-1, 1):  # posts and roof
        geo.cylinder(rnd, V((X(1.75), sgn * 1.55, 0.24)), V((X(1.75), sgn * 1.55, 3.45)), 0.13,
                     mi('trim'), n=12)
    extrude(porch, 'y', [(X(0), 3.9), (X(0), 3.6), (X(2.3), 3.25), (X(2.3), 3.45)], -1.95, 1.95,
            mi('roof'))
    for y0 in (-3.0, 3.0):  # flower boxes under the downstairs windows
        geo.box(porch, V((X(0.35), y0, 0.95)), (0.5, 2.0, 0.42), mi('box'))
        for k in range(5):
            geo.sphere(rnd, V((X(0.38), y0 - 0.8 + k * 0.4, 1.25)), 0.2,
                       mi('flower_r' if k % 2 else 'flower_y'), n=8, rings=5)
porch_ob = part('porch', porch, bevel=0.025, segments=2)
rnd_ob = smooth('round', rnd)

# --- tunnel interior: dado rail, skirting, picture frames ------------------------------------
inner = core.new_bm()
for sx in (-1, 1):
    geo.box(inner, V((sx * (TX - 0.05), 0, 1.55)), (0.12, 2 * HY - 0.4, 0.14), mi('trim'))
    geo.box(inner, V((sx * (TX - 0.05), 0, 0.25)), (0.12, 2 * HY - 0.4, 0.4), mi('trim'))
    for y0 in (-2.5, 2.5):
        geo.box(inner, V((sx * (TX - 0.06), y0, 2.6)), (0.1, 1.5, 1.1), mi('trim'))
        geo.box(inner, V((sx * (TX - 0.1), y0, 2.6)), (0.06, 1.15, 0.78),
                mi('shutter' if y0 < 0 else 'box'))
inner_ob = part('interior', inner, bevel=0.015, segments=1)

propkit.finish(ENV, 'dollhouse', [wall_ob, roof_ob, chim_ob, trim_ob, win_ob, porch_ob,
                                  rnd_ob, inner_ob], budget=propkit.HERO_TRIS, car=(0.0, 9.0, 180))
