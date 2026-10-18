"""Garden shed (hero): a 2.4 × 1.8 m timber shed (10.7 × 8 u), apex roof to 2.3 m (10.2 u) with
felt and fascia boards, horizontal overlap cladding stained brown, a ledged-and-braced green door
and a four-pane window on the front (+Y here, Bevy -Z), corner trims, a timber base
(catalog: garden_shed).

    blender -b -P tools/blender/props/garden/shed.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

HX, HY = 5.35, 4.0          # half footprint (walls)
EAVE, RIDGE = 7.6, 10.2     # wall top at the eaves, ridge height (ridge along x)
BASE = 0.35
BOARD = 0.55                 # cladding board height

start('pine_dark', 'pine', 'paint_green', 'paint_white', 'roof_felt', 'glass', 'steel_dark')
W, P, DG, WH = mi('pine_dark'), mi('pine'), mi('paint_green'), mi('paint_white')
bm = core.new_bm()
geo.box(bm, V((0, 0, BASE / 2)), (2 * HX + 0.2, 2 * HY + 0.2, BASE), P)  # base frame
box_uv(bm, scale=0.5)
k = 0
z = BASE
while z < EAVE - 0.01:
    h = min(BOARD, EAVE - z)
    for side, (cx, cy, sx, sy, axis) in {'front': (0, HY, 2 * HX, 0.12, 0),
                                         'back': (0, -HY, 2 * HX, 0.12, 0),
                                         'left': (-HX, 0, 0.12, 2 * HY, 1),
                                         'right': (HX, 0, 0.12, 2 * HY, 1)}.items():
        k += 1
        out = 0.03 if side in ('front', 'back') else 0.0  # overlap step: board bottoms proud
        o = V((0, out if cy > 0 else -out, 0)) if axis == 0 else V((out if cx > 0 else -out, 0, 0))
        board(bm, V((cx, cy, z + h / 2)) + o, (sx, sy, h + 0.04), W, axis=axis, seed=k)
    z += BOARD
# gable triangles (cladding continues up to the ridge on the short sides)
for sx in (-1, 1):
    x = sx * HX
    tri = [(-HY, EAVE), (HY, EAVE), (0, RIDGE - 0.25)]
    geo.extrude_x(bm, tri, x - 0.06, x + 0.06, W)
walls = part('walls', bm, bevel=0.02, segments=1)

roof = core.new_bm()
pitch = math.atan2(RIDGE - EAVE, HY)
slope = math.hypot(HY + 0.7, (RIDGE - EAVE) * (HY + 0.7) / HY)
for sy in (-1, 1):  # two roof slabs with felt, overhanging 0.7 u
    c = V((0, sy * (HY + 0.7) / 2, (EAVE + RIDGE) / 2 - 0.1))
    fs = geo.box(roof, V((0, 0, 0)), (2 * HX + 1.2, slope, 0.22), mi('roof_felt'))
    bmesh.ops.transform(roof, matrix=Matrix.Translation(c) @ Matrix.Rotation(-sy * pitch, 4, 'X'),
                        verts=list({v for f in fs for v in f.verts}))
    box_uv(roof, fs, scale=0.4)
geo.cylinder(roof, V((-HX - 0.62, 0, RIDGE + 0.02)), V((HX + 0.62, 0, RIDGE + 0.02)), 0.16,
             mi('roof_felt'), n=8)                                    # ridge cap
for sx in (-1, 1):  # barge boards along the gable edges and finials
    x = sx * (HX + 0.62)
    for sy in (-1, 1):
        a = V((x, sy * (HY + 0.75), EAVE - 0.45))
        b = V((x, 0, RIDGE + 0.05))
        geo.prism(roof, geo.rrect(0.1, 0.42, 0.0, 1), a, b, P)
    geo.box(roof, V((x, 0, RIDGE + 0.25)), (0.12, 0.25, 0.6), P)
roof_ob = part('roof', roof, bevel=0.02, segments=1)

trim = core.new_bm()
for sx in (-1, 1):  # corner trims
    for sy in (-1, 1):
        geo.box(trim, V((sx * (HX + 0.05), sy * (HY + 0.05), (BASE + EAVE) / 2)),
                (0.3, 0.3, EAVE - BASE), WH)
# door: ledged and braced, dark green, on the front left
DX, DW, DH = -1.6, 3.2, 6.2
geo.box(trim, V((DX, HY + 0.12, BASE + DH / 2)), (DW, 0.1, DH), DG)
for zz in (0.9, DH / 2, DH - 0.7):  # ledges
    geo.box(trim, V((DX, HY + 0.2, BASE + zz)), (DW - 0.3, 0.08, 0.45), DG)
brace = [V((DX - DW / 2 + 0.3, HY + 0.2, BASE + 1.2)), V((DX + DW / 2 - 0.3, HY + 0.2, BASE + DH / 2 - 0.3))]
geo.prism(trim, geo.rrect(0.4, 0.08, 0.0, 1), brace[0], brace[1], DG, up=V((0, 1, 0)))
for z0 in (BASE + 1.4, BASE + DH - 1.4):  # hinges and latch
    geo.box(trim, V((DX - DW / 2 + 0.6, HY + 0.26, z0)), (1.2, 0.04, 0.12), mi('steel_dark'))
geo.box(trim, V((DX + DW / 2 - 0.35, HY + 0.27, BASE + DH / 2)), (0.18, 0.06, 0.5), mi('steel_dark'))
for sx in (-1, 1):  # door frame
    geo.box(trim, V((DX + sx * (DW / 2 + 0.12), HY + 0.12, BASE + DH / 2)), (0.24, 0.16, DH + 0.2), WH)
geo.box(trim, V((DX, HY + 0.12, BASE + DH + 0.12)), (DW + 0.48, 0.16, 0.24), WH)
# four-pane window on the front right
WX, WZ, WW, WHh = 2.6, 4.2, 2.8, 2.4
geo.box(trim, V((WX, HY + 0.1, WZ)), (WW, 0.06, WHh), mi('glass'))
for x0 in (WX - WW / 2, WX, WX + WW / 2):
    geo.box(trim, V((x0, HY + 0.16, WZ)), (0.18, 0.12, WHh + 0.18), WH)
for z0 in (WZ - WHh / 2, WZ, WZ + WHh / 2):
    geo.box(trim, V((WX, HY + 0.16, z0)), (WW + 0.18, 0.12, 0.18), WH)
geo.box(trim, V((WX, HY + 0.32, WZ - WHh / 2 - 0.12)), (WW + 0.5, 0.45, 0.12), WH)  # sill
box_uv(trim, scale=0.5, along=2)
trim_ob = part('trim', trim, bevel=0.02, segments=1)
finish('shed', [walls, roof_ob, trim_ob], hero=True, car=(4.0, HY + 3.0, 200.0))
