"""House back wall (decor backdrop on the north bound): a two-storey brick house front 72 u
(16 m) wide facing the garden (+Y here), French doors onto the patio, a kitchen door onto the
deck and windows below, four bedroom windows above, white frames and sills, gutter, two downpipes, an
outside tap, and the front slope of a tiled roof up to the ridge (decor/backyard_bash.ron; the
track shell's north wall is the collider).

    blender -b -P tools/blender/props/garden/house.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

HW, EAVE, RIDGE = 36.0, 25.0, 38.0
start('house_brick', 'upvc', 'glass', 'stone', 'roof_tile', 'steel_dark', 'brass', 'paint_blue',
      'lamp_window')
B, U, G, S = mi('house_brick'), mi('upvc'), mi('glass'), mi('stone')
wall = core.new_bm()
geo.box(wall, V((0, -0.5, EAVE / 2)), (2 * HW, 1.0, EAVE), B)
box_uv(wall, scale=0.25)
wall_ob = part('wall', wall, bevel=0.0)

trim = core.new_bm()


def window(x, z0, w, h, panes=2, door=False, glass=G):
    """Frame (proud of the wall), recessed glass, mullions, sill (not for doors)."""
    geo.box(trim, V((x, 0.02, z0 + h / 2)), (w, 0.08, h), glass)
    fw = 0.32
    for sx in (-1, 1):
        geo.box(trim, V((x + sx * (w / 2 - fw / 2), 0.12, z0 + h / 2)), (fw, 0.3, h), U)
    for z in (z0 + fw / 2, z0 + h - fw / 2):
        geo.box(trim, V((x, 0.12, z)), (w, 0.3, fw), U)
    for k in range(1, panes):
        geo.box(trim, V((x - w / 2 + w * k / panes, 0.1, z0 + h / 2)), (0.2, 0.24, h - fw), U)
    if door:
        geo.box(trim, V((x, 0.1, z0 + h * 0.42)), (w - fw, 0.24, 0.2), U)
    else:
        geo.box(trim, V((x, 0.35, z0 - 0.12)), (w + 0.7, 0.75, 0.24), S)


# lit rooms glow at night (lamp_* materials are dimmed to 5 % by day by rc_render)
LIT = mi('lamp_window')
window(16.0, 0.3, 9.0, 9.2, panes=2, door=True, glass=LIT)   # French doors onto the patio
window(3.5, 3.8, 7.0, 5.8, panes=3)                          # living room window
window(-10.0, 5.4, 6.5, 5.2, panes=2, glass=LIT)             # kitchen window
# kitchen door at deck level (the deck is 2.0 u high outside it)
window(-22.0, 2.0, 4.0, 9.0, panes=1, door=True, glass=mi('paint_blue'))
geo.box(trim, V((-22.0, 1.4, 11.6)), (6.0, 2.8, 0.25), U)   # door canopy
for x in (-22.0, -8.0, 6.0, 20.0):
    window(x, 14.6, 5.4, 5.4, panes=2, glass=LIT if x == 6.0 else G)  # bedrooms
geo.box(trim, V((0, 0.1, 0.4)), (2 * HW, 0.25, 0.8), S)    # plinth
trim_ob = part('trim', trim, bevel=0.03, segments=1)

roof = core.new_bm()
slope = math.hypot(17.3, RIDGE - EAVE)
pitch = math.atan2(RIDGE - EAVE, 17.3)
fs = geo.box(roof, V((0, 0, 0)), (2 * HW + 2.4, slope, 0.5), mi('roof_tile'))
bmesh.ops.transform(roof, matrix=Matrix.Translation(V((0, 1.3 - 17.3 / 2, (EAVE + RIDGE) / 2 + 0.1)))
                    @ Matrix.Rotation(pitch, 4, 'X'), verts=list({v for f in fs for v in f.verts}))
box_uv(roof, fs, scale=0.3)
for k in range(1, 12):  # tile courses: thin ridges across the slope
    t = k / 12
    p = V((0, 1.3 - 17.3 * t, EAVE + 0.35 + (RIDGE - EAVE) * t))
    geo.box(roof, p, (2 * HW + 2.4, 0.18, 0.12), mi('roof_tile'))
geo.cylinder(roof, V((-HW - 1.2, -16.0, RIDGE + 0.3)), V((HW + 1.2, -16.0, RIDGE + 0.3)), 0.6,
             mi('roof_tile'), n=8)                           # ridge tiles
geo.box(roof, V((0, 0.6, EAVE - 0.3)), (2 * HW + 2.4, 1.3, 0.6), U)   # fascia board
roof_ob = part('roof', roof, bevel=0.03, segments=1)

pipes = core.new_bm()
K = mi('steel_dark')
gutter = [V((-HW - 1.0, 1.55, EAVE - 0.6)), V((HW + 1.0, 1.55, EAVE - 0.6))]
geo.cylinder(pipes, gutter[0], gutter[1], 0.32, K, n=8)
for x in (-30.0, 30.0):
    geo.tube(pipes, [V((x, 1.5, EAVE - 0.8)), V((x, 0.35, EAVE - 2.0)), V((x, 0.35, 0.6)),
                     V((x, 0.8, 0.15))], 0.22, K, n=8, corner=0.4)
    for z in (4.0, 10.0, 16.0, 21.0):
        geo.box(pipes, V((x, 0.18, z)), (0.6, 0.36, 0.2), K)
geo.cylinder(pipes, V((-27.0, 0.0, 3.2)), V((-27.0, 0.45, 3.2)), 0.1, mi('brass'), n=8)  # tap
geo.cylinder(pipes, V((-27.0, 0.45, 3.2)), V((-27.0, 0.45, 2.85)), 0.08, mi('brass'), n=8)
geo.box(pipes, V((-27.0, 0.45, 3.42)), (0.36, 0.08, 0.08), mi('brass'))
pipes_ob = smooth('pipes', pipes, angle=40)
finish('house', [wall_ob, trim_ob, roof_ob, pipes_ob], hero=True, car=(16.0, 6.0, 180.0),
       shots=[('hero', V((0.5, 1.0, 0.35)), 35), ('rc', None, 24)])
