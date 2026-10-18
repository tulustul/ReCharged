"""Suburban houses (backdrop heroes), real size x 4.44, front (+Y) toward the street, origin at
the ground-contact centre of the footprint (placed on the lawn). Lap siding is sawtooth
geometry; windows, doors and trim sit proud of it; shingle courses on the roofs.

  house_a  two-storey colonial with porch and attached two-car garage (garage on +X)
  house_b  single-storey ranch with brick base, picture window and garage (garage on +X)
  *_m      the same house mirrored (garage on -X) in another colour

    blender -b -P tools/blender/props/street/house.py -- [--preview DIR] [--only NAME]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

BOARD = 0.9    # lap siding exposure (20 cm)
COURSE = 0.6   # shingle course


def siding(bm, x0, x1, y, z0, z1, mat, facing=1):
    """Lap siding wall in the XZ plane at depth y facing +Y (facing=1) or -Y: each board
    leans out 0.06 at its bottom edge, like overlapping clapboards."""
    n = max(1, round((z1 - z0) / BOARD))
    h = (z1 - z0) / n
    fs = []
    for i in range(n):
        za, zb = z0 + i * h, z0 + (i + 1) * h
        lo, hi = y + facing * 0.06, y
        quad = [(lo, za), (hi, zb), (hi - facing * 0.02, zb), (lo - facing * 0.02, za)]
        fs += geo.extrude_x(bm, [(p[0], p[1]) for p in quad], x0, x1, mat)
    return fs


def siding_y(bm, y0, y1, x, z0, z1, mat, facing=1):
    """Lap siding wall in the YZ plane at x facing +X (facing=1) or -X."""
    n = max(1, round((z1 - z0) / BOARD))
    h = (z1 - z0) / n
    for i in range(n):
        za, zb = z0 + i * h, z0 + (i + 1) * h
        lo, hi = x + facing * 0.06, x
        quad = [(lo, za), (hi, zb), (hi - facing * 0.02, zb), (lo - facing * 0.02, za)]
        rings = [[V((p[0], y0, p[1])) for p in quad], [V((p[0], y1, p[1])) for p in quad]]
        geo._finish(bm, geo._bridge(bm, rings, mat))


def gable_roof(bm, x0, x1, y0, y1, z_eave, pitch, over=1.2, mat='roof', rake=0.8):
    """Gable roof with the ridge along X; shingle courses as shallow steps. Returns ridge z."""
    yc = (y0 + y1) / 2
    half = (y1 - y0) / 2 + over
    rise = half * math.tan(math.radians(pitch))
    m = mi(mat)
    for s in (-1, 1):
        n = max(2, round(half / math.cos(math.radians(pitch)) / COURSE))
        for i in range(n):
            ta, tb = i / n, (i + 1) / n
            ya, yb = yc + s * half * (1 - ta), yc + s * half * (1 - tb)
            za, zb = z_eave - over * math.tan(math.radians(pitch)) + rise * ta, \
                z_eave - over * math.tan(math.radians(pitch)) + rise * tb
            quad = [(ya, za - 0.12), (yb, zb), (yb, zb + 0.22), (ya + s * 0.02, za + 0.1)]
            geo.extrude_x(bm, quad, x0 - rake, x1 + rake, m)
    zr = z_eave - over * math.tan(math.radians(pitch)) + rise
    geo.box(bm, V(((x0 + x1) / 2, yc, zr + 0.18)), (x1 - x0 + 2 * rake + 0.2, 0.7, 0.36), m)
    return zr


def gable_end(bm, x, y0, y1, z_eave, pitch, mat):
    """Triangular gable wall at x between the eaves and the ridge."""
    yc = (y0 + y1) / 2
    rise = (y1 - y0) / 2 * math.tan(math.radians(pitch))
    tri = [(y0, z_eave), (y1, z_eave), (yc, z_eave + rise)]
    geo.extrude_x(bm, [(p[0], p[1]) for p in tri], x - 0.2, x + 0.2, mat)


def window(bm, x, y, z, w, h, facing=1, shutters=True, panes=(2, 3)):
    """Trimmed window on a +Y (facing=1) wall at depth y, centre (x, z)."""
    t = 0.32
    f = facing
    trim, glass = mi('trim_white'), mi('glass')
    geo.box(bm, V((x, y + f * 0.1, z)), (w, 0.08, h), glass)
    for dx in (-(w + t) / 2, (w + t) / 2):
        geo.box(bm, V((x + dx, y + f * 0.16, z)), (t, 0.2, h + 2 * t), trim)
    geo.box(bm, V((x, y + f * 0.16, z + (h + t) / 2)), (w + 2 * t, 0.2, t), trim)
    geo.box(bm, V((x, y + f * 0.24, z - (h + t) / 2)), (w + 2.6 * t, 0.36, t * 0.8), trim)  # sill
    for i in range(1, panes[0]):  # muntins
        geo.box(bm, V((x - w / 2 + w * i / panes[0], y + f * 0.15, z)), (0.1, 0.06, h), trim)
    for j in range(1, panes[1]):
        geo.box(bm, V((x, y + f * 0.15, z - h / 2 + h * j / panes[1])), (w, 0.06, 0.1), trim)
    if shutters:
        for s in (-1, 1):
            geo.box(bm, V((x + s * (w / 2 + t + 0.75), y + f * 0.12, z)), (1.3, 0.12, h + 0.3),
                    mi('shutter'))


def window_y(bm, x, y, z, w, h, facing=1):
    """Trimmed window on a +X (facing=1) wall (side walls), no shutters."""
    t = 0.32
    f = facing
    trim, glass = mi('trim_white'), mi('glass')
    geo.box(bm, V((x + f * 0.1, y, z)), (0.08, w, h), glass)
    for dy in (-(w + t) / 2, (w + t) / 2):
        geo.box(bm, V((x + f * 0.16, y + dy, z)), (0.2, t, h + 2 * t), trim)
    for dz in (-(h + t) / 2, (h + t) / 2):
        geo.box(bm, V((x + f * 0.16, y, z + dz)), (0.2, w + 2 * t, t), trim)


def garage_door(bm, x, y, w, h, panels=4):
    geo.box(bm, V((x, y + 0.1, h / 2 + 0.05)), (w, 0.14, h), mi('garage'))
    for i in range(1, panels):  # panel grooves
        geo.box(bm, V((x, y + 0.18, h * i / panels)), (w - 0.2, 0.04, 0.12), mi('trim_white'))
    for j in range(1, 6):  # raised panel ribs
        geo.box(bm, V((x - w / 2 + w * j / 6, y + 0.18, h / 2)), (0.1, 0.04, h - 0.4),
                mi('trim_white'))
    t = 0.45
    for dx in (-(w + t) / 2, (w + t) / 2):
        geo.box(bm, V((x + dx, y + 0.14, (h + t) / 2)), (t, 0.24, h + t), mi('trim_white'))
    geo.box(bm, V((x, y + 0.14, h + t / 2 + 0.05)), (w + 2 * t, 0.24, t), mi('trim_white'))


def door(bm, x, y, z, w=4.0, h=9.0):
    geo.box(bm, V((x, y + 0.08, z + h / 2)), (w, 0.14, h), mi('door'))
    for dz in (h * 0.3, h * 0.72):  # raised panels
        for dx in (-w / 4, w / 4):
            geo.box(bm, V((x + dx, y + 0.17, z + dz)), (w * 0.34, 0.06, h * 0.32), mi('door'))
    t = 0.4
    for dx in (-(w + t) / 2, (w + t) / 2):
        geo.box(bm, V((x + dx, y + 0.14, z + (h + t) / 2)), (t, 0.24, h + t), mi('trim_white'))
    geo.box(bm, V((x, y + 0.14, z + h + t / 2)), (w + 2 * t, 0.24, t), mi('trim_white'))
    geo.box(bm, V((x + w * 0.32, y + 0.26, z + h * 0.48)), (0.22, 0.2, 0.22), mi('brass'))


def bushes(bm, x0, x1, y, n, seed, r=1.6):
    import random
    rnd = random.Random(seed)
    for i in range(n):
        x = x0 + (x1 - x0) * (i + 0.5) / n + rnd.uniform(-0.5, 0.5)
        rr = r * rnd.uniform(0.8, 1.2)
        geo.sphere(bm, V((x, y + rnd.uniform(-0.3, 0.3), rr * 0.7)), rr, mi('hedge'), n=8,
                   rings=5, scale=(1.15, 0.9, 0.85))


def materials(siding, trim, door, shutter, roof, extra=()):
    core.material('siding', siding, 0.0, 0.6)
    core.material('trim_white', trim, 0.0, 0.45)
    core.material('door', door, 0.0, 0.3)
    core.material('shutter', shutter, 0.0, 0.5)
    core.material('roof', roof, 0.0, 0.92)
    mats('glass', 'concrete', 'brick', 'hedge', 'lamp_warm', *extra)
    core.material('garage', '#E8E6DF', 0.0, 0.5)
    core.material('brass', '#C29A4B', 1.0, 0.35)
    core.material('gutter_metal', '#D9D9D4', 0.6, 0.4)


def colonial(name, mirror, palette):
    core.reset()
    street_props.use_outdoor(ground='#5C8F3C')
    materials(*palette)
    bm = core.new_bm()
    wall = mi('siding')
    # main block x -24..6, y -14..14, two storeys to 23 on a 1.2 foundation
    X0, X1, Y0, Y1, F, ZE = -24.0, 6.0, -14.0, 14.0, 1.2, 23.0
    geo.box(bm, V(((X0 + X1) / 2, 0, F / 2)), (X1 - X0 + 0.4, Y1 - Y0 + 0.4, F), mi('concrete'))
    geo.box(bm, V(((X0 + X1) / 2, 0, (F + ZE) / 2)), (X1 - X0 - 0.2, Y1 - Y0 - 0.2, ZE - F),
            wall)  # core (behind the siding)
    siding(bm, X0, X1, Y1, F, ZE, wall, 1)
    siding(bm, X0, X1, Y0, F, ZE, wall, -1)
    siding_y(bm, Y0, Y1, X0, F, ZE, wall, -1)
    siding_y(bm, Y0, Y1, X1, F, ZE, wall, 1)
    for x in (X0, X1):
        gable_end(bm, x, Y0, Y1, ZE, 40, wall)
    zr = gable_roof(bm, X0, X1, Y0, Y1, ZE, 40)
    geo.box(bm, V((X0 + 4, -3.0, (ZE + zr) / 2 + 3)), (3.2, 3.2, zr - ZE + 8), mi('brick'))
    # garage x 6..30, y -12..12, one storey to 12
    G0, G1, GY0, GY1, GE = 6.0, 30.0, -12.0, 12.0, 12.5
    geo.box(bm, V(((G0 + G1) / 2, 0, F / 2 - 0.3)), (G1 - G0 + 0.4, GY1 - GY0 + 0.4, F - 0.6),
            mi('concrete'))
    geo.box(bm, V(((G0 + G1) / 2, 0, GE / 2)), (G1 - G0 - 0.2, GY1 - GY0 - 0.2, GE), wall)
    siding(bm, G0, G1, GY1, 0.6, GE, wall, 1)
    siding(bm, G0, G1, GY0, 0.6, GE, wall, -1)
    siding_y(bm, GY0, GY1, G1, 0.6, GE, wall, 1)
    gable_end(bm, G1, GY0, GY1, GE, 32, wall)
    gable_roof(bm, G0 - 0.5, G1, GY0, GY1, GE, 32)
    garage_door(bm, (G0 + G1) / 2, GY1, 19.0, 8.6)
    # porch: shed roof on two columns, steps, door, light
    PX0, PX1, PY = -19.0, -3.0, Y1 + 5.0
    geo.box(bm, V(((PX0 + PX1) / 2, (Y1 + PY) / 2, F - 0.1)), (PX1 - PX0, PY - Y1, 0.4),
            mi('concrete'))
    for i, h in enumerate((0.8, 0.4)):
        geo.box(bm, V(((PX0 + PX1) / 2, PY + 0.9 + i * 0.9, h / 2)), (6.0, 0.9 + 0.0 * i, h),
                mi('concrete'))
    for x in (PX0 + 0.6, PX1 - 0.6):
        geo.box(bm, V((x, PY - 0.6, (F + 11.6) / 2)), (0.8, 0.8, 11.6 - F), mi('trim_white'))
    geo.extrude_x(bm, [(Y1, 12.4), (PY + 0.8, 11.4), (PY + 0.8, 11.0), (Y1, 11.8)], PX0 - 0.6,
                  PX1 + 0.6, mi('roof'))
    door(bm, -11.0, Y1, F)
    geo.sphere(bm, V((-8.0, Y1 + 0.5, F + 7.5)), 0.4, mi('lamp_warm'), n=10, rings=6)
    # windows
    for x in (-21.0, -1.0):
        window(bm, x, Y1, 6.2, 3.4, 5.4)
    for x in (-19.0, -11.0, -3.0):
        window(bm, x, Y1, 17.0, 3.4, 5.0)
    for y in (-6.0, 6.0):
        window_y(bm, X0, y, 6.2, 3.0, 5.0, -1)
        window_y(bm, X0, y, 17.0, 3.0, 5.0, -1)
    window(bm, -9.0, Y0, 17.0, 3.4, 5.0, -1, shutters=False)
    # gutters and downspouts
    for y in (Y1 + 1.0, Y0 - 1.0):
        geo.box(bm, V(((X0 + X1) / 2, y, ZE - 0.9), ), (X1 - X0 + 1.6, 0.35, 0.35),
                mi('gutter_metal'))
    for x in (X0 - 0.5, X1 - 0.6):
        geo.box(bm, V((x, Y1 + 0.9, (ZE - 1.0) / 2)), (0.3, 0.3, ZE - 1.0), mi('gutter_metal'))
    bushes(bm, X0 + 1, PX0 - 1, Y1 + 2.0, 2, 3)
    bushes(bm, PX1 + 1, X1 - 0.5, Y1 + 2.0, 2, 4)
    return finish_house(name, bm, mirror)


def ranch(name, mirror, palette):
    core.reset()
    street_props.use_outdoor(ground='#5C8F3C')
    materials(*palette)
    bm = core.new_bm()
    wall = mi('siding')
    X0, X1, Y0, Y1, F, ZE = -30.0, 8.0, -15.0, 15.0, 0.9, 12.5
    geo.box(bm, V(((X0 + X1) / 2, 0, F / 2)), (X1 - X0 + 0.4, Y1 - Y0 + 0.4, F), mi('concrete'))
    geo.box(bm, V(((X0 + X1) / 2, 0, (F + ZE) / 2)), (X1 - X0 - 0.2, Y1 - Y0 - 0.2, ZE - F),
            wall)
    geo.box(bm, V(((X0 + X1) / 2, Y1 + 0.15, F + 2.0)), (X1 - X0, 0.5, 4.0), mi('brick'))  # wainscot
    siding(bm, X0, X1, Y1, F + 4.0, ZE, wall, 1)
    siding(bm, X0, X1, Y0, F, ZE, wall, -1)
    siding_y(bm, Y0, Y1, X0, F, ZE, wall, -1)
    zr = gable_roof(bm, X0 - 0.2, X1 + 0.2, Y0, Y1, ZE, 26, over=1.6)
    for x in (X0, X1):
        gable_end(bm, x, Y0, Y1, ZE, 26, wall)
    geo.box(bm, V((X0 + 9, 4.0, (ZE + zr) / 2 + 2)), (3.0, 3.0, zr - ZE + 6), mi('brick'))
    # garage wing x 8..32 set back 3, door to the street
    G0, G1, GY0, GY1, GE = 8.0, 32.0, -15.0, 12.0, 11.0
    geo.box(bm, V(((G0 + G1) / 2, (GY0 + GY1) / 2, GE / 2)), (G1 - G0, GY1 - GY0 - 0.2, GE),
            wall)
    siding(bm, G0, G1, GY1, 0.6, GE, wall, 1)
    siding_y(bm, GY0, GY1, G1, 0.6, GE, wall, 1)
    gable_end(bm, G1, GY0, GY1, GE, 26, wall)
    gable_roof(bm, G0 - 1.0, G1, GY0, GY1, GE, 26, over=1.4)
    garage_door(bm, (G0 + G1) / 2, GY1, 19.0, 8.2)
    # picture window, two windows, door with a small stoop, light
    window(bm, -18.0, Y1 + 0.3, 7.2, 10.0, 5.2, panes=(4, 2))
    for x in (-27.0, 3.0):
        window(bm, x, Y1 + 0.3, 7.6, 3.2, 4.2)
    door(bm, -6.0, Y1 + 0.3, F)
    geo.box(bm, V((-6.0, Y1 + 2.0, F / 2)), (6.5, 3.0, F), mi('concrete'))
    geo.sphere(bm, V((-3.4, Y1 + 0.7, F + 7.2)), 0.4, mi('lamp_warm'), n=10, rings=6)
    window_y(bm, X0, 0.0, 7.6, 3.2, 4.2, -1)
    for y in (Y1 + 1.4,):
        geo.box(bm, V(((X0 + X1) / 2, y, ZE - 0.7)), (X1 - X0 + 2.0, 0.35, 0.35),
                mi('gutter_metal'))
    bushes(bm, X0 + 1, -24.0, Y1 + 2.2, 2, 5)
    bushes(bm, -13.0, -9.0, Y1 + 2.2, 1, 6)
    return finish_house(name, bm, mirror)


def finish_house(name, bm, mirror):
    if mirror:
        import bmesh
        bmesh.ops.scale(bm, vec=(-1, 1, 1), verts=bm.verts)
        bmesh.ops.reverse_faces(bm, faces=bm.faces)
    ob = core.obj_from_bm(name, bm, bevel=0.0, angle=30)
    return street_props.finish(ENV, name, [ob], budget=HERO, below=0.3, car=(0.0, 24.0, 160),
                               cams=[('hero', (60, 95, 32), (0, 0, 10), 35),
                                     ('front', (8, 70, 6), (0, 0, 12), 35),
                                     ('rc', (-6, 40, 0.7), (0, 0, 9), 22)])


# palettes: siding, trim, door, shutter, roof
HOUSES = {
    'house_a': (colonial, False, ('#7F96A8', '#EEEDE6', '#7A2A2A', '#2B3138', '#3B3A3A')),
    'house_a_m': (colonial, True, ('#D9CBA6', '#F1EFE8', '#2D4560', '#3E4A36', '#5A4A3C')),
    'house_b': (ranch, False, ('#BFB39A', '#F0EEE7', '#2F4F3A', '#2B3138', '#4A4646')),
    'house_b_m': (ranch, True, ('#8EA286', '#F1EFE8', '#8C3B2C', '#2B3138', '#3B3A3A')),
}
only = propkit._argv()[propkit._argv().index('--only') + 1] if '--only' in propkit._argv() \
    else None
for name, (fn, mirror, pal) in HOUSES.items():
    if only in (None, name):
        fn(name, mirror, pal)
