"""Kids' activity play table (Golden Battery perch): a 6 x 9 u beech table, top 2.5 u high and
drivable, printed with a little town map (green board, grey road loop with dashes, blue pond,
red-roofed houses, trees) - a toy town on the Toy Town floor. Cars fly onto it from the toy-track
ramp and can slip under it (1.8 u clearance). Static, Wood surface.

    blender -b -P tools/blender/props/toy/play_table.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

W, L, TOP = 6.0, 9.0, 2.5   # x, y, top height
core.reset()
beech()
painted('board', '#5E9E4E', 0.5)
painted('road', '#6F7178', 0.55)
painted('dash', '#EDE6DA', 0.5)
painted('pond', '#4A8FC9', 0.2)
painted('house', '#F1E6CF', 0.5)
painted('roof', '#C9402E', 0.45)
painted('tree', '#3E8A4B', 0.5)
mi = core.mi

frame = core.new_bm()
geo.box(frame, V((0, 0, TOP - 0.15)), (W, L, 0.3), mi('beech'))       # top slab
for sx in (-1, 1):                                                   # apron
    geo.box(frame, V((sx * (W / 2 - 0.35), 0, TOP - 0.5)), (0.12, L - 0.7, 0.4), mi('beech'))
for sy in (-1, 1):
    geo.box(frame, V((0, sy * (L / 2 - 0.35), TOP - 0.5)), (W - 0.7, 0.12, 0.4), mi('beech'))
for sx in (-1, 1):                                                   # legs
    for sy in (-1, 1):
        geo.box(frame, V((sx * (W / 2 - 0.55), sy * (L / 2 - 0.55), (TOP - 0.3) / 2)),
                (0.5, 0.5, TOP - 0.3), mi('beech'))
frame_ob = part('frame', box_uv(frame), bevel=0.04, segments=2)

# printed board: thin layers just above the top
z = TOP + 0.004
top = core.new_bm()
extrude(top, 'z', rpoly([(-W / 2 + 0.25, -L / 2 + 0.25), (W / 2 - 0.25, -L / 2 + 0.25),
                         (W / 2 - 0.25, L / 2 - 0.25), (-W / 2 + 0.25, L / 2 - 0.25)],
                        [0.3] * 4, 4), z - 0.004, z, mi('board'))
loop_out = geo.rrect(4.4, 7.4, 1.6, 6)
loop_in = geo.rrect(3.4, 6.4, 1.1, 6)
ring_o, ring_i = [V((p.x, p.y, z + 0.006)) for p in loop_out], [V((p.x, p.y, z + 0.006))
                                                              for p in loop_in]
vo = [top.verts.new(p) for p in ring_o]
vi = [top.verts.new(p) for p in ring_i]
for j in range(len(vo)):
    f = top.faces.new((vo[j], vo[(j + 1) % len(vo)], vi[(j + 1) % len(vi)], vi[j]))
    f.material_index = mi('road')
dash_path = geo.rrect(3.9, 6.9, 1.35, 6)
for j in range(0, len(dash_path), 2):  # centre-line dashes
    p, q = dash_path[j], dash_path[(j + 1) % len(dash_path)]
    c, d = (p + q) / 2, (q - p)
    if d.length < 1e-3:
        continue
    n = V((-d.y, d.x)).normalized() * 0.04
    t = d.normalized() * min(d.length * 0.35, 0.25)
    quad = [c - t - n, c + t - n, c + t + n, c - t + n]
    vs = [top.verts.new(V((v.x, v.y, z + 0.01))) for v in quad]
    top.faces.new(vs).material_index = mi('dash')
extrude(top, 'z', [V((0.9 + 0.7 * math.cos(a), 0.5 + 0.5 * math.sin(a)))
                   for a in (2 * math.pi * k / 20 for k in range(20))], z, z + 0.008, mi('pond'))
top_ob = part('board', top, bevel=0.0)

deco = core.new_bm()  # houses and trees on the corners only: the middle is the landing strip
for x, y in ((-2.55, -3.95), (2.55, 3.95)):
    geo.box(deco, V((x, y, TOP + 0.2)), (0.5, 0.55, 0.4), mi('house'))
    extrude(deco, 'y', [(x - 0.31, TOP + 0.4), (x + 0.31, TOP + 0.4), (x, TOP + 0.7)],
            y - 0.3, y + 0.3, mi('roof'))
for x, y in ((2.6, -3.95), (-2.6, 3.95), (2.65, -3.2), (-2.65, 3.2)):
    geo.cylinder(deco, V((x, y, TOP)), V((x, y, TOP + 0.25)), 0.06, mi('beech'), n=8)
    geo.cylinder(deco, V((x, y, TOP + 0.2)), V((x, y, TOP + 0.85)), 0.26, mi('tree'), n=10, r1=0.02)
deco_ob = part('deco', deco, bevel=0.0)
propkit.finish(ENV, 'play_table', [frame_ob, top_ob, deco_ob], car=(-1.5, 7.5, 160))
