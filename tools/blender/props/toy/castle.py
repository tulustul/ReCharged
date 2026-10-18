"""Play castle (hero) that stands inside the toy-track helix: a painted wooden fort with
stone-block walls, four round corner towers under red and blue cones with pennants, a gatehouse
with a lowered drawbridge (front, +Y) and a tall keep flying a big flag, the landmark of the
room. Fits inside r = 9.3 u (the helix's inner pillars stand at 10.35). Static.

    blender -b -P tools/blender/props/toy/castle.py -- [--preview DIR]
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

H = 5.0          # half size of the curtain wall square (outer face)
WT, WH = 0.8, 4.6   # wall thickness, height
TR, TH = 1.7, 7.0   # corner tower radius, height
core.reset()
textured('stone', '#BEB6A6', 0.7, blocks_image('stone', '#BEB6A6', rows=6, cols=3, mortar=0.035,
                                                jitter=0.05, seed=4))
painted('roof_red', '#C9402E', 0.4)
painted('roof_blue', '#2F64C9', 0.4)
painted('slit', '#2A2C33', 0.6)
wood('plank', '#B98A55', 0.55, seed=5)
plastic('flag_y', YELLOW, 0.5)
plastic('flag_r', RED, 0.5)
core.material('iron', '#3A3B40', 1.0, 0.45)
painted('gold', '#D8A93A', 0.35)
mi = core.mi

stone = core.new_bm()
merlon = core.new_bm()


def crenels(cx, cy, z, length, along_x, n, depth=WT):
    for k in range(n):
        t = (k + 0.5) / n - 0.5
        c = V((cx + t * length, cy, z + 0.35)) if along_x else V((cx, cy + t * length, z + 0.35))
        size = (length / n * 0.55, depth, 0.7) if along_x else (depth, length / n * 0.55, 0.7)
        geo.box(merlon, c, size, mi('stone'))


# curtain walls between the towers (the front wall carries the gatehouse)
span = 2 * H - 2 * TR + 0.6
for s in (-1, 1):
    geo.box(stone, V((0, s * (H - WT / 2), WH / 2)), (span, WT, WH), mi('stone'))
    geo.box(stone, V((s * (H - WT / 2), 0, WH / 2)), (WT, span, WH), mi('stone'))
    crenels(0, s * (H - WT / 2), WH, span - 0.4, True, 5)
    crenels(s * (H - WT / 2), 0, WH, span - 0.4, False, 5)
stone_ob = part('walls', box_uv(stone, 0.25), bevel=0.04, segments=2)

# gatehouse on the front (+Y) wall with an arched gate
gate = core.new_bm()
gw, gh, gd = 4.2, 6.2, 1.6
arch = rpoly([(-gw / 2, 0), (-1.1, 0), (-1.1, 2.4), (1.1, 2.4), (1.1, 0), (gw / 2, 0),
              (gw / 2, gh), (-gw / 2, gh)], [0, 0, 1.05, 1.05, 0, 0, 0, 0], 6)
extrude(gate, 'y', arch, H - gd + 0.2, H + 0.2, mi('stone'))
for s in (-1, 1):
    geo.box(merlon, V((s * 1.55, H - 0.6, gh + 0.35)), (0.75, 1.6, 0.7), mi('stone'))
geo.box(merlon, V((0, H - 0.6, gh + 0.35)), (0.75, 1.6, 0.7), mi('stone'))
gate_ob = part('gatehouse', box_uv(gate, 0.25), bevel=0.04, segments=2)
door = core.new_bm()  # dark gate passage behind the open drawbridge
extrude(door, 'y', rpoly([(-1.1, 0), (1.1, 0), (1.1, 2.4), (-1.1, 2.4)], [0, 0, 1.05, 1.05], 6),
        H - gd + 0.22, H - gd + 0.32, mi('slit'))
door_ob = part('door', door, bevel=0.0)

# drawbridge (lowered, planks + iron straps) and its chains
bridge = core.new_bm()
for k in range(7):
    geo.box(bridge, V((-0.96 + k * 0.32, H + 0.2 + 1.5, 0.09)), (0.3, 3.0, 0.18), mi('plank'))
for x in (-0.65, 0.65):
    geo.box(bridge, V((x, H + 0.2 + 1.5, 0.19)), (0.16, 2.9, 0.04), mi('iron'))
bridge_ob = part('drawbridge', box_uv(bridge), bevel=0.02, segments=1)
chains = core.new_bm()
for x in (-1.0, 1.0):
    geo.tube(chains, [V((x, H + 0.2 + 2.8, 0.2)), V((x * 1.4, H + 0.25, 3.3))], 0.06, mi('iron'),
             n=6)

# corner towers: stone drums, corbel ring, crenels, cones, pennants, arrow slits
towers = core.new_bm()
roofs = core.new_bm()
deco = core.new_bm()
for k, (sx, sy) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
    c = V((sx * H, sy * H, 0))
    fs = geo.cylinder(towers, c, c + V((0, 0, TH)), TR, mi('stone'), n=24)
    cyl_uv(towers, fs, c)
    fs = geo.cylinder(towers, c + V((0, 0, TH - 0.5)), c + V((0, 0, TH)), TR + 0.25, mi('stone'),
                      n=24)
    cyl_uv(towers, fs, c)
    roof = 'roof_red' if k % 2 == 0 else 'roof_blue'
    geo.cylinder(roofs, c + V((0, 0, TH + 0.05)), c + V((0, 0, TH + 4.2)), TR + 0.45, mi(roof),
                 n=24, r1=0.05)
    geo.cylinder(deco, c + V((0, 0, TH + 4.0)), c + V((0, 0, TH + 5.6)), 0.06, mi('iron'), n=6)
    geo.sphere(deco, c + V((0, 0, TH + 4.15)), 0.18, mi('gold'), n=10, rings=6)
    a_out = math.atan2(sy, sx)
    top = c + V((0, 0, TH + 5.55))  # stiff plastic pennant, all pointing +X
    pen = [top, top + V((0, 0, -0.62)), top + V((1.25, 0, -0.31))]
    geo.loft(deco, [[p - V((0, 0.03, 0)) for p in pen], [p + V((0, 0.03, 0)) for p in pen]],
             mi('flag_y' if k % 2 == 0 else 'flag_r'))
    for j in range(3):  # arrow slits facing outward
        a = a_out + (j - 1) * 0.7
        p = c + V((math.cos(a) * (TR - 0.02), math.sin(a) * (TR - 0.02), 3.6))
        geo.box(deco, p, (0.28 * abs(math.sin(a)) + 0.06, 0.28 * abs(math.cos(a)) + 0.06, 1.1),
                mi('slit'))
tower_ob = smooth('towers', towers, angle=40)
roof_ob = smooth('cones', roofs, angle=40)

# keep: tall square tower, crenels, windows, flagpole with a big flag
keep = core.new_bm()
geo.box(keep, V((0, -0.8, 5.0)), (4.0, 4.0, 10.0), mi('stone'))
geo.box(keep, V((0, -0.8, 9.8)), (4.5, 4.5, 0.4), mi('stone'))
for s in (-1, 1):
    crenels(0, -0.8 + s * 2.0, 10.0, 4.3, True, 4, depth=0.5)
    crenels(s * 2.0, -0.8, 10.0, 4.3, False, 4, depth=0.5)
keep_ob = part('keep', box_uv(keep, 0.25), bevel=0.05, segments=2)
for z in (5.2, 7.8):  # arched windows on every face
    for n in ((0, 1), (1, 0), (0, -1), (-1, 0)):
        p = V((n[0] * 2.01, -0.8 + n[1] * 2.01, z))
        geo.box(deco, p, (0.7 if n[1] else 0.08, 0.08 if n[1] else 0.7, 1.2), mi('slit'))
geo.cylinder(deco, V((0, -0.8, 10.0)), V((0, -0.8, 13.6)), 0.09, mi('iron'), n=8)  # room: 16 u
geo.sphere(deco, V((0, -0.8, 13.65)), 0.22, mi('gold'), n=10, rings=6)
flag = [(0.1, 13.4), (3.2, 12.7), (2.4, 12.1), (3.2, 11.5), (0.1, 11.5)]
extrude(deco, 'y', flag, -0.84, -0.76, mi('flag_r'))
geo.box(deco, V((1.3, -0.8, 12.45)), (0.9, 0.1, 0.9), mi('flag_y'))  # emblem on the flag
deco_ob = part('details', deco, bevel=0.0)
merlon_ob = part('merlons', box_uv(merlon, 0.25), bevel=0.04, segments=1)
chain_ob = smooth('chains', chains)

propkit.finish(ENV, 'castle', [stone_ob, gate_ob, door_ob, bridge_ob, chain_ob, tower_ob, roof_ob,
                               keep_ob, deco_ob, merlon_ob], budget=propkit.HERO_TRIS,
               car=(3.0, 10.5, 200))
