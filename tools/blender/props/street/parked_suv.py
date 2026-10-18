"""Parked family crossover (hero): a generic, brand-free mid-size SUV at real size x 4.44
(4.85 x 1.92 x 1.76 m), parked. RC cars drive under it: the underside is >= 1.8 u everywhere
between the wheels (catalog.py SUV). Three paint variants: a slate blue, b burgundy, c silver.

    blender -b -P tools/blender/props/street/parked_suv.py -- [--preview DIR] [--only a|b|c]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

import catalog as C  # noqa: E402
from lib import shell  # noqa: E402

S = C.SUV
HL, HW = S['length'] / 2, S['width'] / 2
YF, YR = S['wheelbase'] / 2, -S['wheelbase'] / 2
R, TW, TR = S['tyre_r'], S['tyre_w'], S['track'] / 2
Z0 = 2.05          # floor of the body shell; underbody parts hang down to FLOOR
FLOOR = S['floor']


def materials(paint):
    core.material('paint', paint, 0.0, 0.34, coat=1.0, coat_rough=0.06)  # car paint, art bible
    mats('plastic_black', 'chrome', 'rubber')
    core.material('glass', '#141A22', 0.0, 0.05, coat=1.0, coat_rough=0.02)
    core.material('rim', '#B9BDC2', 1.0, 0.32)
    core.material('plastic', '#26272A', 0.0, 0.6)       # underbody, well liners
    core.material('metal', '#9A9EA3', 1.0, 0.4)         # centre caps, exhaust
    core.material('lens', '#D9DEE4', 0.0, 0.08, coat=1.0, coat_rough=0.02)
    core.material('tail', '#9A1A18', 0.0, 0.25)
    core.material('amber', '#B8701E', 0.0, 0.2)
    core.material('plate', '#E9E8E1', 0.0, 0.45)


def body():
    prof = shell.Profile([
        (-HL, dict(hw=3.95, z0=2.35, zb=4.55, hg=3.70, zg=4.60, hr=3.15, zt=7.05, rb=0.35,
                   rm=1.6, rs=0.35, rg=0.2, rr=0.55)),
        (-10.25, dict(hw=4.15, zb=4.62, hg=3.86, zg=4.67, hr=3.32, zt=7.50)),
        (-8.9, dict(z0=Z0)),
        (YR, dict(hw=HW, zb=4.62, hg=3.92, zg=4.67, hr=3.40, zt=7.76)),
        (-2.0, dict(zt=7.80)),
        (0.8, dict(hr=3.36, zt=7.74)),
        (4.7, dict(hw=4.20, zb=4.50, hg=3.98, zg=4.55, hr=3.62, zt=5.02)),
        (YF, dict(hw=HW, zb=4.45, hg=4.02, zg=4.50, hr=3.62, zt=4.86)),
        (9.3, dict(hw=4.10, z0=Z0, zb=4.30, hg=3.92, zg=4.35, hr=3.50, zt=4.62)),
        (HL, dict(hw=3.86, z0=2.25, zb=4.05, hg=3.62, zg=4.10, hr=3.32, zt=4.28)),
    ])
    D = dict(wheelbase=S['wheelbase'], radius=(R, R), wheel_width=(TW, TW),
             track=(S['track'], S['track']))
    bm = core.new_bm()
    shell.shell(bm, prof, -HL, HL, arches=shell.arches(D, clear=0.32, gap=0.28), seg=2,
                step=1.25, ends=(0.55, 0.65), end_scale=0.88, min_thick=0.28)
    ob = core.obj_from_bm('shell', bm, weighted=False, angle=40)
    return prof, ob


def details(prof, shell_ob):
    surf = BigSurface([shell_ob])
    bm = core.new_bm()
    # windows: windshield, three side windows per side (B and C pillars stay paint), tailgate
    big_glass(bm, surf, prof, windshield=(1.15, 4.55), m=0.22, r=0.3)
    for y0, y1 in ((-2.05, 4.35), (-6.55, -2.45), (-10.05, -6.95)):
        big_glass(bm, surf, prof, side=(y0, y1), m=0.2, r=0.28)
    back = (V((0, -14, 0)), V((1, 0, 0)), V((0, 0, 1)))
    geo.decal(bm, surf, geo.rrect(5.9, 1.75, 0.35, 3, 0, 6.12), *back, mi('glass'),
              offset=0.01, back=1.0, h=0.5)
    # lamps (parked: lights off, real lenses)
    for s in (-1, 1):
        hl = geo.ccw([V((s * x, z)) for x, z in ((-1.05, -0.32), (1.0, -0.22), (1.05, 0.30),
                                                   (-1.0, 0.30))])
        geo.solid_decal(bm, surf, geo.rounded_poly(hl, 0.12, 2), V((s * 2.75, 14, 3.98)),
                        V((-1, 0, 0)), V((0, 0, 1)), mi('lens'), height=0.04,
                        side_mat=mi('chrome'), back=1.2, h=0.5)
        geo.decal(bm, surf, geo.circle2(0.2, 12, s * -0.45, 0.0), V((s * 2.75, 14, 3.98)),
                  V((-1, 0, 0)), V((0, 0, 1)), mi('plastic_black'), offset=0.06, back=1.2, h=0.5)
        geo.solid_decal(bm, surf, geo.rrect(0.62, 1.55, 0.18, 2), V((s * 3.28, -14, 5.05)),
                        V((1, 0, 0)), V((0, 0, 1)), mi('tail'), height=0.05,
                        side_mat=mi('plastic_black'), back=1.2, h=0.5)
        geo.decal(bm, surf, geo.rrect(0.62, 0.32, 0.1, 2), V((s * 3.28, -14, 4.08)),
                  V((1, 0, 0)), V((0, 0, 1)), mi('amber'), offset=0.012, back=1.2, h=0.5)
        geo.decal(bm, surf, geo.circle2(0.24, 14), V((s * 3.05, 14, 2.62)), V((-1, 0, 0)),
                  V((0, 0, 1)), mi('lens'), offset=0.012, back=1.2, h=0.5)
    front = (V((-1, 0, 0)), V((0, 0, 1)))
    geo.decal(bm, surf, geo.rrect(3.9, 1.0, 0.3, 3, 0, 3.72), V((0, 14, 0)), *front,
              mi('plastic_black'), offset=0.01, back=1.2, h=0.5)
    for z in (3.48, 3.72, 3.96):  # grille bars
        geo.decal(bm, surf, geo.rrect(3.7, 0.07, 0.03, 1, 0, z), V((0, 14, 0)), *front,
                  mi('chrome'), offset=0.02, back=1.2, h=0.5)
    geo.decal(bm, surf, geo.rrect(5.4, 0.55, 0.2, 3, 0, 2.55), V((0, 14, 0)), *front,
              mi('plastic_black'), offset=0.01, back=1.2, h=0.5)
    geo.decal(bm, surf, geo.rrect(1.36, 0.66, 0.06, 2, 0, 3.02), V((0, 14, 0)), *front,
              mi('plate'), offset=0.02, back=1.2, h=0.5)
    geo.decal(bm, surf, geo.rrect(1.36, 0.66, 0.06, 2, 0, 4.25), *back[:1], V((1, 0, 0)),
              V((0, 0, 1)), mi('plate'), offset=0.02, back=1.2, h=0.5)
    # door seams (front door front edge, B-pillar, rear door rear edge) and handles
    for s in (-1, 1):
        side = (V((s * 8, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
        for y in (4.45, -2.25, -6.75):
            geo.strip(bm, surf, [(s * y, 2.35), (s * y, 4.55)], 0.045, *side,
                      mi('plastic_black'), offset=0.006, step=0.25, back=1.0)
        for y in (0.6, -4.4):
            geo.solid_decal(bm, surf, geo.rrect(0.75, 0.16, 0.06, 2, s * y, 4.12), *side,
                            mi('chrome'), height=0.05, back=1.0, h=0.5)
    dec = core.obj_from_bm('details', bm, weighted=False, angle=50)

    # hard trim: cladding, bumpers, mirrors, roof rails, wipers
    bm = core.new_bm()
    pb = mi('plastic_black')
    for s in (-1, 1):
        geo.box(bm, V((s * (HW - 0.05), 0.0, 2.32)), (0.22, 7.6, 0.55), pb)   # rocker
        for yc in (YF, YR):  # wheel-arch flares
            pts = [V((s * (HW + 0.02), yc + 2.12 * math.cos(a), R + 2.12 * math.sin(a)))
                   for a in [math.radians(t) for t in range(0, 181, 20)]]
            geo.tube(bm, pts, 0.16, pb, n=5)
        geo.slab(bm, [(3.9, 0.22, 4.75, 5.3, 0.1), (4.4, 0.18, 4.8, 5.25, 0.08)], mi('paint'),
                 cx=s * (HW + 0.45))  # mirror cap
        geo.box(bm, V((s * (HW + 0.12), 4.15, 4.85)), (0.35, 0.5, 0.3), pb)  # mirror base
    for y, z, d in ((HL - 0.15, 2.45, 0.5), (-HL + 0.18, 2.6, 0.55)):  # bumper lips
        geo.slab(bm, [(y - d / 2, 3.6, z - 0.25, z + 0.22, 0.12),
                      (y + d / 2, 3.6, z - 0.25, z + 0.22, 0.12)], pb)
    for x in (-2.9, 2.9):  # roof rails on four feet
        hit, _ = surf.project(V((x, -4.0, 9.5)), V((0, 0, -1)), 0.0, back=0.0)
        zr = (hit.z if hit is not None else 7.6) + 0.42
        geo.slab(bm, [(-8.6, 0.12, zr - 0.06, zr + 0.08, 0.05), (0.4, 0.12, zr - 0.06,
                                                                  zr + 0.08, 0.05)],
                 mi('rim'), cx=x)
        for y in (-8.3, -5.5, -2.5, 0.1):
            hit, _ = surf.project(V((x, y, 9.5)), V((0, 0, -1)), 0.0, back=0.0)
            zb = hit.z if hit is not None else 7.5
            geo.box(bm, V((x, y, (zb + zr) / 2)), (0.22, 0.45, zr - zb + 0.1), pb)
    for x0 in (-2.4, 0.3):  # wipers parked at the windshield base
        geo.box(bm, V((x0 + 1.1, 4.62, 5.12)), (2.3, 0.08, 0.06), pb)
    trim = core.obj_from_bm('trim', bm, bevel=0.03, segments=1, angle=35)

    # underbody (all >= FLOOR): fuel tank, exhaust + muffler, cross members, control arms
    bm = core.new_bm()
    pl, mt = mi('plastic'), mi('metal')
    geo.slab(bm, [(-5.6, 1.5, FLOOR + 0.06, Z0 + 0.1, 0.15), (-2.4, 1.5, FLOOR + 0.06,
                                                              Z0 + 0.1, 0.15)], pl, cx=-1.4)
    geo.tube(bm, [(0.9, 8.6, FLOOR + 0.2), (0.9, 2.0, FLOOR + 0.2), (1.6, -7.3, FLOOR + 0.2),
                  (2.4, -10.95, FLOOR + 0.28)], 0.17, mt, n=10, corner=0.8)
    geo.slab(bm, [(-9.6, 0.8, FLOOR + 0.02, Z0 + 0.15, 0.3), (-7.6, 0.8, FLOOR + 0.02,
                                                              Z0 + 0.15, 0.3)], mt, cx=1.8)
    for y in (-1.0, 3.0, -8.4):
        geo.box(bm, V((0, y, Z0 - 0.06)), (5.6, 0.3, 0.2), pl)
    geo.slab(bm, [(7.2, 2.1, FLOOR + 0.1, Z0 + 0.1, 0.2), (9.0, 2.1, FLOOR + 0.1,
                                                          Z0 + 0.1, 0.2)], pl)  # oil pan
    for s in (-1, 1):
        for yc in (YF, YR):
            for dy in (-0.8, 0.8):  # control arms
                a, b = V((s * 1.9, yc + dy * 1.6, Z0 - 0.08)), V((s * (TR - TW / 2 - 0.15), yc,
                                                                    R - 0.25))
                geo.prism(bm, [(-0.11, -0.06), (0.11, -0.06), (0.11, 0.06), (-0.11, 0.06)], a, b,
                          pl)
            geo.box(bm, V((s * (TR - TW / 2 - 0.12), yc, R)), (0.25, 0.6, 0.9), mt)  # hub
    under = core.obj_from_bm('under', bm, bevel=0.02, segments=1, angle=40)
    return [dec, trim, under]


def wheels():
    out = []
    for s in (-1, 1):
        for yc in (YF, YR):
            out.append(road_wheel(f'w{s}{yc}', V((s * TR, yc, R)), R, TW, s, n=22))
    return out


def build(variant):
    core.reset()
    street_props.use_outdoor()
    materials(C.SUV_PAINTS[variant])
    prof, sh = body()
    parts_ = [sh] + details(prof, sh) + wheels()
    propkit.finish(ENV, f'parked_suv_{variant}', parts_, budget=HERO, car=(1.5, -3.0, 25),
                   shots=[('hero', V((1.0, 1.3, 0.62)), 50), ('back', V((-0.8, -1.0, 0.4)), 50),
                          ('side', V((1.0, 0.02, 0.12)), 50),
                          ('under', V((0.25, -1.0, 0.05)), 35), ('rc', None, 24)])


def main():
    only = propkit._argv()[propkit._argv().index('--only') + 1] \
        if '--only' in propkit._argv() else None
    for v in C.SUV_PAINTS:
        if only in (None, v):
            build(v)


main()
