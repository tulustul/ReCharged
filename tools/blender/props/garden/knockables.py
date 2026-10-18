"""Knockable kit, one glb each (catalog: garden_watering_can / _gnome / _bucket / _football /
_tennis_ball): a galvanised watering can (spout toward the front, +Y here), a ceramic garden
gnome, a plastic bucket, a football (black pentagons) and a tennis ball.

    blender -b -P tools/blender/props/garden/knockables.py -- [--preview DIR] [--only NAME]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gardenkit import *  # noqa: E402,F403

argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
only = argv[argv.index('--only') + 1] if '--only' in argv else None


def watering_can():
    start('steel', 'steel_dark', 'plastic_black')
    S = mi('steel')
    bm = core.new_bm()
    rings = []
    for z, sx, sy in ((0.0, 0.36, 0.5), (0.04, 0.4, 0.56), (1.0, 0.36, 0.52), (1.12, 0.3, 0.44),
                      (1.16, 0.0, 0.0)):
        rings.append([V((sx * math.cos(a), -0.15 + sy * math.sin(a), z))
                      for a in (2 * math.pi * k / 20 for k in range(20))])
    geo.loft(bm, rings[:-1], S)
    geo.cylinder(bm, V((0, 0.0, 1.0)), V((0, 0.0, 1.2)), 0.2, S, n=14)          # filler neck
    geo.tube(bm, [V((0, -0.6, 0.6)), V((0, -0.65, 1.35)), V((0, 0.1, 1.42)), V((0, 0.22, 1.05))],
             0.06, S, n=8, corner=0.2)                                      # handle
    geo.cylinder(bm, V((0, 0.3, 0.2)), V((0, 1.05, 1.0)), 0.09, S, n=10, r1=0.05)  # spout
    geo.cylinder(bm, V((0, 1.02, 0.97)), V((0, 1.12, 1.08)), 0.14, mi('steel_dark'), n=12)  # rose
    return [smooth('can', bm, angle=40)], dict(car=(1.6, -0.6, 20.0))


def gnome():
    start('ceramic_red', 'ceramic_blue', 'ceramic_skin', 'ceramic_white', 'leather_brown',
          'enamel_black')
    bm = core.new_bm()
    revolve(bm, [(0, 0), (0.42, 0), (0.42, 0.12), (0, 0.12)], mi('enamel_black'), n=16)   # base
    revolve(bm, [(0, 0.1), (0.38, 0.12), (0.4, 0.55), (0.36, 0.78), (0.0, 0.8)],
            mi('ceramic_blue'), n=16)                                                  # coat
    revolve(bm, [(0, 0.75), (0.33, 0.78), (0.3, 0.95), (0, 1.0)], mi('ceramic_white'), n=14)
    geo.sphere(bm, V((0, 0.05, 1.08)), 0.24, mi('ceramic_skin'), n=14, rings=8)          # face
    geo.sphere(bm, V((0, 0.27, 1.07)), 0.07, mi('ceramic_red'), n=8, rings=4)             # nose
    revolve(bm, [(0, 0.82), (0.26, 0.86), (0.22, 1.05), (0, 1.02)], mi('ceramic_white'), n=12,
            centre=(0, 0.12, 0))                                                       # beard
    revolve(bm, [(0, 1.12), (0.28, 1.14), (0.16, 1.4), (0.04, 1.62), (0, 1.64)],
            mi('ceramic_red'), n=14, centre=(0, -0.04, 0))                             # hat
    for sx in (-1, 1):
        geo.sphere(bm, V((sx * 0.3, 0.16, 0.6)), 0.1, mi('ceramic_skin'), n=8, rings=4)  # hands
        geo.sphere(bm, V((sx * 0.17, 0.22, 0.15)), 0.12, mi('leather_brown'), n=8, rings=4,
                   scale=(1, 1.5, 0.8))                                                 # boots
    revolve(bm, [(0, 0.47), (0.41, 0.47), (0.41, 0.54), (0, 0.54)], mi('leather_brown'), n=16)  # belt
    return [smooth('gnome', bm, angle=45)], dict(car=(1.4, -0.4, 20.0))


def bucket():
    start('plastic_blue', 'steel')
    bm = core.new_bm()
    revolve(bm, [(0, 0), (0.5, 0), (0.52, 0.04), (0.64, 1.2), (0.68, 1.24), (0.66, 1.3),
                 (0.6, 1.3), (0.58, 1.22), (0.47, 0.1), (0, 0.1)], mi('plastic_blue'), n=24)
    geo.tube(bm, [V((-0.66, 0, 1.1)), V((-0.5, 0, 1.55)), V((0.5, 0, 1.55)), V((0.66, 0, 1.1))], 0.03,
             mi('steel'), n=5, corner=0.35)
    return [smooth('bucket', bm, angle=40)], dict(car=(1.5, -0.5, 20.0))


def football():
    start('football_white', 'football_black')
    bm = core.new_bm()
    r = 0.49
    ico = bmesh.ops.create_icosphere(bm, subdivisions=3, radius=r,
                                     matrix=Matrix.Translation(V((0, 0, r))))
    phi = (1 + 5 ** 0.5) / 2
    poles = [V(p).normalized() for p in
             [(0, 1, phi), (0, -1, phi), (0, 1, -phi), (0, -1, -phi), (1, phi, 0), (-1, phi, 0),
              (1, -phi, 0), (-1, -phi, 0), (phi, 0, 1), (-phi, 0, 1), (phi, 0, -1), (-phi, 0, -1)]]
    for f in {f for v in ico['verts'] for f in v.link_faces}:
        d = (f.calc_center_median() - V((0, 0, r))).normalized()
        f.material_index = mi('football_black') if max(d.dot(p) for p in poles) > 0.94 else \
            mi('football_white')
    return [smooth('ball', bm, angle=80)], dict(car=(1.3, -0.4, 20.0))


def tennis_ball():
    start('tennis_felt')
    bm = core.new_bm()
    bmesh.ops.create_icosphere(bm, subdivisions=2, radius=0.15,
                               matrix=Matrix.Translation(V((0, 0, 0.15))))
    for f in bm.faces:
        f.material_index = 0
    return [smooth('ball', bm, angle=80)], dict(car=(0.9, -0.3, 20.0))


for name, fn in (('watering_can', watering_can), ('gnome', gnome), ('bucket', bucket),
                 ('football', football), ('tennis_ball', tennis_ball)):
    if only and name != only:
        continue
    parts, kw = fn()
    finish(name, parts, **kw)
