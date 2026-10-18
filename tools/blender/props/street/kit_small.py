"""Street & garden kit: clipped hedge, white picket fence (+ corner hero with roses), rural
mailbox, fire hydrant (+ hero cluster with guard bollards), wheelie bins, traffic cone, stop
sign and street-name sign (block lettering, no font files), road-closed barricade, newspaper,
lawn sprinkler, solar garden light and a kid's bike lying on the lawn.

    blender -b -P tools/blender/props/street/kit_small.py -- [--preview DIR] [--only NAME]

Front = +Y (Bevy -Z), origin at the ground contact.
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

import bmesh  # noqa: E402


def cams(w, h, d=None):
    """Hero 3/4, eye level and a closer 3/4 for a prop about w wide and h tall."""
    d = d or max(w, h) * 1.6 + 3
    return [('hero', (d * 0.8, d * 1.0, h * 0.8 + d * 0.35), (0, 0, h * 0.45), 35),
            ('rc', (d * 0.35, d * 1.2, 0.7), (0, 0, h * 0.4), 26),
            ('back', (-d * 0.9, -d * 0.7, h * 0.7 + d * 0.3), (0, 0, h * 0.45), 35)]


def lumpy_box(bm, c, size, mats_, rnd, cuts=5, amp=0.12):
    """Box subdivided and jittered along its normals (hedges)."""
    out = bmesh.ops.create_cube(bm, size=1.0, matrix=Matrix.Translation(V(c)) @
                                Matrix.Diagonal((*size, 1.0)))
    verts = out['verts']
    edges = list({e for v in verts for e in v.link_edges})
    bmesh.ops.subdivide_edges(bm, edges=edges, cuts=cuts, use_grid_fill=True)
    island = [v for v in bm.verts if all(abs(v.co[i] - c[i]) <= size[i] / 2 + 1e-4
                                         for i in range(3))]
    for v in island:
        d = v.co - V(c)
        n = V((d.x / size[0], d.y / size[1], d.z / size[2]))
        if n.length > 1e-6:
            v.co += n.normalized() * rnd.uniform(-amp, amp)
    for f in {f for v in island for f in v.link_faces}:
        f.material_index = rnd.choice(mats_)


def hedge():
    start('hedge', 'hedge_light', 'soil')
    rnd = random.Random(3)
    bm = core.new_bm()
    lumpy_box(bm, (0, 0, 2.5), (8.0, 3.6, 5.0), [mi('hedge'), mi('hedge'), mi('hedge_light')],
              rnd, cuts=6, amp=0.16)
    geo.box(bm, V((0, 0, 0.05)), (8.2, 3.9, 0.12), mi('soil'))
    ob = core.obj_from_bm('hedge', bm, weighted=False, angle=50)
    street_props.finish(ENV, 'hedge', [ob], below=0.2, car=(5.5, 3.0, 160), cams=cams(8, 5))


def pickets(bm, a, b, h=4.2, mat=None):
    """Picket fence leg from a to b (2D points in XY): rails + pointed pickets."""
    m = mat if mat is not None else mi('trim_white')
    a, b = V((a[0], a[1], 0)), V((b[0], b[1], 0))
    d = b - a
    ln = d.length
    t = d.normalized()
    side = V((-t.y, t.x, 0))
    for z in (1.1, 3.3):  # rails (behind the pickets)
        geo.prism(bm, [(-0.09, -0.18), (0.09, -0.18), (0.09, 0.18), (-0.09, 0.18)],
                  a + V((0, 0, z)) - side * 0.12, b + V((0, 0, z)) - side * 0.12, m)
    n = max(2, round(ln / 0.66))
    for i in range(n):
        c = a + t * (ln * (i + 0.5) / n)
        prof = [(-0.2, 0.0), (0.2, 0.0), (0.2, h - 0.3), (0.0, h), (-0.2, h - 0.3)]
        pts0 = [c + t * px + V((0, 0, pz)) for px, pz in prof]
        geo.loft(bm, [pts0, [p + side * 0.12 for p in pts0]], m)


def post(bm, x, y, h, w=0.48, mat=None):
    m = mat if mat is not None else mi('trim_white')
    geo.box(bm, V((x, y, h / 2)), (w, w, h), m)
    geo.box(bm, V((x, y, h + 0.08)), (w + 0.12, w + 0.12, 0.16), m)
    geo.cylinder(bm, V((x, y, h + 0.16)), V((x, y, h + 0.5)), w * 0.42, m, n=8, r1=0.02)


def fence():
    start('trim_white')
    bm = core.new_bm()
    pickets(bm, (-3.8, 0), (3.8, 0))
    for x in (-4.0, 4.0):
        post(bm, x, -0.06, 4.8)
    ob = core.obj_from_bm('fence', bm, bevel=0.02, segments=1, angle=40)
    street_props.finish(ENV, 'fence', [ob], car=(5.0, 2.0, 150), cams=cams(8, 5))


def fence_corner():
    start('trim_white', 'hedge', 'leaf_red', 'paint_red')
    rnd = random.Random(8)
    bm = core.new_bm()
    pickets(bm, (0.4, 0), (8.0, 0))           # leg along +X
    pickets(bm, (0, -0.4), (0, -8.0))         # leg along -Y (Bevy +Z)
    post(bm, 0, 0, 5.4, w=0.7)
    geo.sphere(bm, V((0, 0, 6.15)), 0.32, mi('trim_white'), n=10, rings=6)
    for p in ((8.0, 0), (0, -8.0)):
        post(bm, p[0], p[1], 4.8)
    wood = core.obj_from_bm('fence', bm, bevel=0.02, segments=1, angle=40)
    roses = core.new_bm()  # a climbing rose on the corner post
    for i in range(16):
        a = rnd.uniform(0, 2 * math.pi)
        z = rnd.uniform(0.8, 5.2)
        r = rnd.uniform(0.45, 0.8)
        c = V((r * math.cos(a), r * math.sin(a), z))
        geo.sphere(roses, c, rnd.uniform(0.35, 0.55), mi('hedge'), n=6, rings=4)
        if rnd.random() < 0.6:
            geo.sphere(roses, c + V((0, 0, 0.2)) + (c - V((0, 0, z))).normalized() * 0.3,
                       0.14, mi('paint_red'), n=6, rings=4)
    bush = core.obj_from_bm('roses', roses, weighted=False, angle=60)
    street_props.finish(ENV, 'fence_corner', [wood, bush], budget=HERO, car=(4.0, 3.0, 160),
                        cams=[('hero', (14, 9, 8), (3, -3, 2.2), 35),
                              ('rc', (6.5, 6.5, 0.7), (1.5, -1.5, 2.5), 26),
                              ('back', (-6, -14, 7), (3, -3, 2.2), 35)])


def mailbox():
    start('trim_white', 'paint_black', 'paint_red')
    bm = core.new_bm()
    geo.box(bm, V((0, 0, 2.0)), (0.42, 0.42, 4.0), mi('trim_white'))       # 4x4 post
    geo.box(bm, V((0, 0.2, 3.95)), (0.42, 1.6, 0.3), mi('trim_white'))      # arm
    prof = geo.rounded_poly([(-0.45, 0.0), (0.45, 0.0), (0.45, 0.5), (0.0, 0.95), (-0.45, 0.5)],
                            [0.03, 0.03, 0.45, 0.45, 0.45], 4)
    geo.prism(bm, [(p.x, p.y) for p in prof], V((0, -0.95, 4.1)), V((0, 1.05, 4.1)),
              mi('paint_black'))
    geo.prism(bm, [(p.x * 1.03, p.y * 1.03 - 0.01) for p in prof], V((0, 1.05, 4.1)),
              V((0, 1.12, 4.1)), mi('paint_black'))  # door lip
    geo.box(bm, V((0.47, -0.2, 4.75)), (0.04, 0.12, 0.9), mi('paint_red'))  # flag arm
    geo.box(bm, V((0.47, -0.45, 5.05)), (0.04, 0.55, 0.38), mi('paint_red'))  # flag
    ob = core.obj_from_bm('mailbox', bm, bevel=0.02, segments=1, angle=40)
    street_props.finish(ENV, 'mailbox', [ob], car=(1.8, 1.8, 160), cams=cams(2.5, 5.3, 9))


def hydrant_parts(bm, x=0.0, y=0.0):
    red, white = mi('paint_red'), mi('trim_white')
    c = V((x, y, 0))
    geo.cylinder(bm, c, c + V((0, 0, 0.22)), 0.78, red, n=16, r1=0.7)       # flange
    geo.cylinder(bm, c + V((0, 0, 0.22)), c + V((0, 0, 2.55)), 0.52, red, n=16, r1=0.48)
    geo.cylinder(bm, c + V((0, 0, 2.5)), c + V((0, 0, 2.72)), 0.64, red, n=16)  # collar
    geo.sphere(bm, c + V((0, 0, 2.72)), 0.55, white, n=16, rings=8, scale=(1, 1, 0.85))  # bonnet
    geo.cylinder(bm, c + V((0, 0, 3.12)), c + V((0, 0, 3.45)), 0.18, white, n=5)  # top nut
    for s in (-1, 1):  # side outlets with caps
        geo.cylinder(bm, c + V((s * 0.4, 0, 1.95)), c + V((s * 0.82, 0, 1.95)), 0.2, red, n=12)
        geo.cylinder(bm, c + V((s * 0.82, 0, 1.95)), c + V((s * 0.98, 0, 1.95)), 0.26, white,
                     n=12)
        geo.cylinder(bm, c + V((s * 0.98, 0, 1.95)), c + V((s * 1.08, 0, 1.95)), 0.1, white,
                     n=5)
    geo.cylinder(bm, c + V((0, 0.4, 1.7)), c + V((0, 0.92, 1.7)), 0.3, red, n=14)  # pumper
    geo.cylinder(bm, c + V((0, 0.92, 1.7)), c + V((0, 1.1, 1.7)), 0.36, white, n=14)


def hydrant():
    start('paint_red', 'trim_white')
    bm = core.new_bm()
    hydrant_parts(bm)
    ob = core.obj_from_bm('hydrant', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'hydrant', [ob], car=(2.2, 1.5, 150), cams=cams(2.2, 3.5, 8))


def hydrant_cluster():
    start('paint_red', 'trim_white', 'paint_yellow_gloss', 'paint_black', 'concrete',
          'reflective_white')
    bm = core.new_bm()
    geo.box(bm, V((0, 0.2, 0.03)), (7.0, 3.4, 0.08), mi('concrete'))  # small concrete pad
    hydrant_parts(bm)
    for x in (-2.4, 2.4):  # guard bollards toward the road
        c = V((x, 0.8, 0))
        geo.cylinder(bm, c, c + V((0, 0, 3.7)), 0.46, mi('paint_yellow_gloss'), n=14)
        geo.sphere(bm, c + V((0, 0, 3.7)), 0.46, mi('paint_yellow_gloss'), n=14, rings=6,
                   scale=(1, 1, 0.5))
        for z in (2.6, 3.1):
            geo.cylinder(bm, c + V((0, 0, z)), c + V((0, 0, z + 0.22)), 0.48, mi('paint_black'),
                         n=14)
    geo.cylinder(bm, V((1.3, -0.9, 0)), V((1.3, -0.9, 5.5)), 0.07, mi('paint_black'), n=6)
    geo.box(bm, V((1.3, -0.9, 5.2)), (0.3, 0.06, 0.5), mi('reflective_white'))  # marker flag
    ob = core.obj_from_bm('hydrant_cluster', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'hydrant_cluster', [ob], budget=HERO, car=(4.5, 2.5, 150),
                        cams=cams(7, 4, 12))


def bin_(name, body, lid):
    core.reset()
    street_props.use_outdoor()
    core.material('bin_body', body, 0.0, 0.5)
    core.material('bin_lid', lid, 0.0, 0.45)
    mats('rubber', 'plastic_black')
    bm = core.new_bm()
    geo.slab(bm, [(-1.5, 1.1, 0.35, 4.3, 0.15), (1.45, 1.3, 0.35, 4.3, 0.15)], mi('bin_body'))
    geo.slab(bm, [(-1.62, 1.33, 4.3, 4.55, 0.1), (1.6, 1.36, 4.3, 4.55, 0.1)], mi('bin_lid'))
    geo.box(bm, V((0, -1.7, 4.15)), (2.2, 0.25, 0.3), mi('bin_lid'))        # handle
    geo.box(bm, V((0, -1.45, 0.55)), (2.3, 0.35, 0.35), mi('plastic_black'))  # axle housing
    for s in (-1, 1):
        geo.cylinder(bm, V((s * 1.05, -1.35, 0.45)), V((s * 1.35, -1.35, 0.45)), 0.45,
                     mi('rubber'), n=14)
    geo.box(bm, V((0, 1.1, 0.2)), (1.8, 0.6, 0.4), mi('plastic_black'))     # front foot
    ob = core.obj_from_bm(name, bm, bevel=0.03, segments=1, angle=40)
    street_props.finish(ENV, name, [ob], car=(2.6, 1.5, 150), cams=cams(2.7, 4.6, 9))


def cone():
    start('orange_plastic', 'reflective_white', 'plastic_black')
    bm = core.new_bm()
    geo.slab(bm, [(-0.78, 0.78, 0.0, 0.12, 0.12), (0.78, 0.78, 0.0, 0.12, 0.12)],
             mi('plastic_black'))
    geo.cylinder(bm, V((0, 0, 0.12)), V((0, 0, 3.1)), 0.62, mi('orange_plastic'), n=16, r1=0.13)
    for z0, z1 in ((1.45, 1.85), (2.25, 2.5)):
        r0 = 0.62 + (0.13 - 0.62) * (z0 - 0.12) / 2.98 + 0.012
        r1 = 0.62 + (0.13 - 0.62) * (z1 - 0.12) / 2.98 + 0.012
        geo.cylinder(bm, V((0, 0, z0)), V((0, 0, z1)), r0, mi('reflective_white'), n=16, r1=r1)
    ob = core.obj_from_bm('cone', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'cone', [ob], car=(1.8, 1.5, 150), cams=cams(1.6, 3.1, 7))


def octagon(r):
    return [V((r * math.cos(math.radians(22.5 + 45 * i)), r * math.sin(math.radians(22.5 + 45 * i))))
            for i in range(8)]


def sign_stop():
    start('steel', 'paint_red', 'trim_white')
    bm = core.new_bm()
    geo.box(bm, V((0, -0.12, 4.9)), (0.22, 0.22, 9.8), mi('steel'))  # square post
    zc = 8.4
    oc = octagon(1.72)
    geo.prism(bm, [(p.x, p.y) for p in oc], V((0, -0.02, zc)), V((0, 0.06, zc)), mi('steel'),
              up=V((0, 0, 1)))
    geo.prism(bm, [(p.x, p.y) for p in oc], V((0, 0.06, zc)), V((0, 0.08, zc)), mi('trim_white'),
              up=V((0, 0, 1)))
    geo.prism(bm, [(p.x * 0.9, p.y * 0.9) for p in oc], V((0, 0.08, zc)), V((0, 0.095, zc)),
              mi('paint_red'), up=V((0, 0, 1)))
    raised_text(bm, 'STOP', V((0, 0.095, zc)), V((-1, 0, 0)), V((0, 0, 1)), 0.95,
                mi('trim_white'), depth=0.012, t=0.2)  # u x v = +Y: reads from the front
    ob = core.obj_from_bm('sign_stop', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'sign_stop', [ob], car=(1.5, 2.0, 150),
                        cams=[('hero', (5, 10, 9), (0, 0, 6.5), 40),
                              ('rc', (2.0, 7.0, 0.7), (0, 0, 7.5), 30),
                              ('back', (-5, -8, 8), (0, 0, 6.5), 40)])


def sign_street():
    start('steel', 'paint_green', 'trim_white')
    bm = core.new_bm()
    geo.cylinder(bm, V((0, 0, 0)), V((0, 0, 12.0)), 0.2, mi('steel'), n=10)
    geo.cylinder(bm, V((0, 0, 12.0)), V((0, 0, 12.25)), 0.26, mi('steel'), n=10)
    for z, ang, text in ((11.5, 0.0, 'MAPLE ST'), (10.35, 90.0, 'BIRCH LN')):
        rot = Matrix.Rotation(math.radians(ang), 3, 'Z')
        u, n = rot @ V((1, 0, 0)), rot @ V((0, 1, 0))
        c = V((0, 0, z))
        blade = geo.rrect(5.4, 1.0, 0.12, 2)
        geo.prism(bm, [(p.x, p.y) for p in blade], c - n * 0.05, c + n * 0.05, mi('paint_green'),
                  up=V((0, 0, 1)))
        for s in (-1, 1):
            border = geo.rrect(5.2, 0.82, 0.1, 2)
            o = c + n * s * 0.05
            for a, b in zip(border, border[1:] + border[:1]):
                pa, pb = o + u * a.x + V((0, 0, a.y)), o + u * b.x + V((0, 0, b.y))
                geo.prism(bm, [(-0.03, 0.0), (0.03, 0.0), (0.03, 0.01), (-0.03, 0.01)], pa, pb,
                          mi('trim_white'), up=n * s)
            raised_text(bm, text, o, V((0, 0, 1)).cross(n * s), V((0, 0, 1)), 0.52,
                        mi('trim_white'), depth=0.012, t=0.19)
        geo.box(bm, V((0, 0, z)), (0.3, 0.3, 0.3), mi('steel'))  # bracket
    ob = core.obj_from_bm('sign_street', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'sign_street', [ob], car=(1.5, 2.0, 150),
                        cams=[('hero', (7, 9, 12), (0, 0, 10.5), 40),
                              ('rc', (2.0, 7.0, 0.7), (0, 0, 9), 30),
                              ('back', (-8, -6, 12), (0, 0, 10.5), 40)])


def barricade():
    start('trim_white', 'orange_plastic', 'plastic_black', 'lamp_amber')
    bm = core.new_bm()
    for x in (-4.4, 4.4):  # A-frame legs
        for s in (-1, 1):
            geo.prism(bm, [(-0.14, -0.1), (0.14, -0.1), (0.14, 0.1), (-0.14, 0.1)],
                      V((x, s * 0.75, 0.0)), V((x, s * 0.08, 4.3)), mi('trim_white'))
        geo.box(bm, V((x, 0, 0.15)), (0.5, 1.8, 0.3), mi('plastic_black'))  # sandbag foot
    for z in (1.6, 2.7, 3.8):  # striped rails
        geo.box(bm, V((0, 0, z)), (10.0, 0.14, 0.82), mi('trim_white'))
        for s in (-1, 1):
            for k in range(8):
                x0 = -5.0 + k * 1.25
                quad = [(x0, z - 0.41), (x0 + 0.62, z - 0.41), (x0 + 1.04, z + 0.41),
                        (x0 + 0.42, z + 0.41)]
                pts = [(min(max(px, -5.0), 5.0), pz) for px, pz in quad]
                vs = [bm.verts.new(V((px, s * 0.08, pz))) for px, pz in pts]
                f = bm.faces.new(vs if s > 0 else vs[::-1])
                f.material_index = mi('orange_plastic')
    geo.box(bm, V((-4.4, 0, 4.5)), (0.3, 0.3, 0.35), mi('plastic_black'))
    geo.cylinder(bm, V((-4.4, 0, 4.65)), V((-4.4, 0, 5.1)), 0.32, mi('lamp_amber'), n=12)
    ob = core.obj_from_bm('barricade', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'barricade', [ob], car=(6.5, 2.0, 150), cams=cams(10, 5))


def newspaper():
    start('paper', 'sleeve')
    bm = core.new_bm()
    geo.cylinder(bm, V((-0.65, 0, 0.25)), V((0.65, 0, 0.25)), 0.25, mi('sleeve'), n=12)
    for s in (-1, 1):
        geo.cylinder(bm, V((s * 0.65, 0, 0.25)), V((s * 0.67, 0, 0.25)), 0.22, mi('paper'), n=12)
    ob = core.obj_from_bm('newspaper', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'newspaper', [ob], car=(1.5, 1.2, 150), cams=cams(1.4, 0.5, 4))


def sprinkler():
    start('hedge', 'paint_yellow_gloss', 'plastic_black')
    bm = core.new_bm()
    geo.slab(bm, [(-0.7, 0.32, 0.0, 0.14, 0.06), (0.7, 0.32, 0.0, 0.14, 0.06)],
             mi('hedge'))
    pts = [(0.0, -0.55 + 1.1 * i / 8, 0.18 + 0.32 * math.sin(math.pi * i / 8)) for i in range(9)]
    geo.tube(bm, pts, 0.07, mi('paint_yellow_gloss'), n=6)
    geo.cylinder(bm, V((0, -0.62, 0.14)), V((0, -0.62, 0.32)), 0.12, mi('plastic_black'), n=8)
    ob = core.obj_from_bm('sprinkler', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'sprinkler', [ob], car=(1.4, 1.2, 150), cams=cams(1.4, 0.6, 4))


def garden_light():
    start('paint_black', 'lamp_warm', 'glass')
    bm = core.new_bm()
    geo.cylinder(bm, V((0, 0, -0.3)), V((0, 0, 1.2)), 0.06, mi('paint_black'), n=6)
    geo.cylinder(bm, V((0, 0, 1.2)), V((0, 0, 1.3)), 0.2, mi('paint_black'), n=6)
    geo.cylinder(bm, V((0, 0, 1.3)), V((0, 0, 1.62)), 0.18, mi('lamp_warm'), n=6)
    geo.cylinder(bm, V((0, 0, 1.62)), V((0, 0, 1.78)), 0.27, mi('paint_black'), n=6, r1=0.12)
    geo.cylinder(bm, V((0, 0, 1.78)), V((0, 0, 1.8)), 0.12, mi('glass'), n=6)  # solar cell
    ob = core.obj_from_bm('garden_light', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'garden_light', [ob], below=0.35, car=(1.2, 1.0, 150),
                        cams=cams(0.6, 1.8, 5))


def bike():
    start('paint_teal', 'rubber', 'steel', 'plastic_black', 'trim_white')
    bm = core.new_bm()
    r = 0.95
    for y in (-1.75, 1.75):  # wheels: tyre + rim tori, spokes, hub
        c = V((0, y, r))
        ring = [(c.x, c.y + r * math.cos(a), c.z + r * math.sin(a)) for a in
                [2 * math.pi * i / 20 for i in range(21)]]
        geo.tube(bm, ring, 0.11, mi('rubber'), n=6)
        rim = [(c.x, c.y + 0.82 * math.cos(a), c.z + 0.82 * math.sin(a)) for a in
               [2 * math.pi * i / 20 for i in range(21)]]
        geo.tube(bm, rim, 0.045, mi('steel'), n=4)
        for k in range(8):
            a = 2 * math.pi * k / 8
            geo.cylinder(bm, c, c + V((0, 0.8 * math.cos(a), 0.8 * math.sin(a))), 0.015,
                         mi('steel'), n=3)
        geo.cylinder(bm, c - V((0.12, 0, 0)), c + V((0.12, 0, 0)), 0.08, mi('steel'), n=8)
    f = mi('paint_teal')
    rear, front = V((0, -1.75, r)), V((0, 1.75, r))
    bb = V((0, -0.3, 0.75))       # bottom bracket
    seat = V((0, -0.75, 2.05))
    head = V((0, 1.15, 2.1))
    for a, b in ((rear, bb), (rear, seat), (bb, seat), (bb, head), (seat, head),
                 (front, head + V((0, -0.1, 0.25)))):
        geo.cylinder(bm, a, b, 0.07, f, n=6)
    geo.slab(bm, [(-1.05, 0.22, 2.12, 2.28, 0.08), (-0.45, 0.14, 2.12, 2.26, 0.06)],
             mi('plastic_black'))  # seat
    hb = head + V((0, -0.05, 0.55))
    geo.cylinder(bm, head, hb, 0.06, mi('steel'), n=6)
    geo.cylinder(bm, hb - V((0.85, 0, 0)), hb + V((0.85, 0, 0)), 0.05, mi('steel'), n=6)
    for s in (-1, 1):
        geo.cylinder(bm, hb + V((s * 0.6, 0, 0)), hb + V((s * 0.9, 0, 0)), 0.09, mi('trim_white'),
                     n=6)  # grips
        geo.box(bm, bb + V((s * 0.5, 0.0, 0.0)), (0.35, 0.2, 0.06), mi('plastic_black'))  # pedals
    geo.cylinder(bm, bb - V((0.08, 0, 0)), bb + V((0.08, 0, 0)), 0.32, mi('steel'), n=12)
    # lay it on its left side on the lawn
    bmesh.ops.rotate(bm, verts=bm.verts, cent=V((0, 0, 0)), matrix=Matrix.Rotation(
        math.radians(-82), 3, 'Y'))
    zmin = min(v.co.z for v in bm.verts)
    bmesh.ops.translate(bm, verts=bm.verts, vec=V((0, 0, -zmin)))
    ob = core.obj_from_bm('bike', bm, weighted=False, angle=40)
    street_props.finish(ENV, 'bike', [ob], car=(2.5, 2.5, 150), cams=cams(3, 1.2, 8))


JOBS = {
    'hedge': hedge, 'fence': fence, 'fence_corner': fence_corner, 'mailbox': mailbox,
    'hydrant': hydrant, 'hydrant_cluster': hydrant_cluster,
    'trash_can': lambda: bin_('trash_can', '#3C4A40', '#34403A'),
    'recycling_bin': lambda: bin_('recycling_bin', '#2D5C96', '#3F73B0'),
    'cone': cone, 'sign_stop': sign_stop, 'sign_street': sign_street, 'barricade': barricade,
    'newspaper': newspaper, 'sprinkler': sprinkler, 'garden_light': garden_light, 'bike': bike,
}
only = propkit._argv()[propkit._argv().index('--only') + 1] if '--only' in propkit._argv() \
    else None
for name, job in JOBS.items():
    if only in (None, name):
        job()
