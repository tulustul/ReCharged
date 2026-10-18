"""Kerbs: concrete barrier kerb stones with their gutter pan (straight 4 / 2 / 1 u and the arcs
the street plan needs), the rolled (mountable) kerb and the driveway apron.

    blender -b -P tools/blender/props/street/kerbs.py -- [--preview DIR] [--only NAME]

Prop space (Blender): face line along X at y = 0, road toward +Y, origin on the face line at
road level. Heights: road 0, gutter 0.02-0.03, kerb top 0.65 (catalog.py: KERB_H, KERB_D).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

import catalog as C  # noqa: E402
import plan  # noqa: E402

# kerb stone: battered face, rounded nose, flat top; bottom edges hidden in road and lawn
STONE = [(0.0, -0.02), (0.0, 0.52), (-0.025, 0.60), (-0.07, 0.638), (-0.13, KERB_H),
         (-0.55, KERB_H), (-KERB_D, 0.62), (-KERB_D, 0.45), (-0.3, -0.02)]
# gutter pan: falls 1 cm toward the kerb, chamfered onto the road
GUTTER = [(0.0, 0.02), (GUTTER_W - 0.1, 0.03), (GUTTER_W, 0.0), (GUTTER_W, -0.06),
          (0.0, -0.06)]
# rolled kerb: S-curve from its toe 1 u in front of the face line up to the lawn edge
ROLLED = [(1.0, -0.02), (1.0, 0.02), (0.78, 0.05), (0.5, 0.15), (0.2, 0.34), (-0.1, 0.51),
          (-0.36, 0.6), (-KERB_D, 0.62), (-KERB_D, 0.45), (0.2, -0.02)]
GUTTER_R = [(d + 1.0, z) for d, z in GUTTER]


def kerb_run(frames, length, stone_len=4.0, profile=STONE, gutter=GUTTER, sub=4.0, seg=2):
    """Stones + gutter slabs along the face-line frames (sub: chord length on arcs)."""
    bm_k, bm_g = core.new_bm(), core.new_bm()
    for chunk in stones(frames, length, stone_len, gap=0.03, sub=sub):
        sweep(bm_k, chunk, profile, mi('kerb'))
    for chunk in stones(frames, length, max(stone_len * 2, 4.0), gap=0.02, sub=sub):
        sweep(bm_g, chunk, gutter, mi('gutter'))
    k = core.obj_from_bm('kerb', bm_k, bevel=0.02, segments=seg, angle=30)
    g = core.obj_from_bm('gutter', bm_g, bevel=0.01, segments=1, angle=30)
    return [k, g]


def straight(n):
    start('kerb', 'gutter')
    parts = kerb_run(frames_line(n, 1.0), n, stone_len=min(4.0, n))
    propkit.finish(ENV, f'kerb_{n}', parts, below=0.1, car=(n / 2 + 1.5, 2.5, 180))


def rolled():
    start('kerb', 'gutter')
    parts = kerb_run(frames_line(4.0, 1.0), 4.0, profile=ROLLED, gutter=GUTTER_R)
    propkit.finish(ENV, 'kerb_rolled', parts, below=0.1, car=(3.5, 3.0, 180))


def arc(kind, radius, span, face):
    start('kerb', 'gutter')
    frames = frames_arc(radius, span, face, 0.5)
    length = math.radians(span) * radius
    parts = kerb_run(frames, length, stone_len=4.0 if radius >= 12 else 2.0,
                     sub=min(2.0, radius / 7), seg=1)
    propkit.finish(ENV, kind[len('street_'):], parts, below=0.1)


def apron():
    """14 u ramp from 2 u into the road up to the driveway (0.64) at the kerb back line, with
    3 u flares lofted from the ramp section into the kerb section."""
    start('concrete', 'kerb', 'gutter')
    hw, fl = C.APRON_W / 2, C.APRON_FLARE
    d_in, d_out, top = C.APRON_IN, C.APRON_OUT, C.DRIVE_H

    def ramp_z(d):
        return 0.02 + (d_in - d) / (d_in + d_out) * (top - 0.02)

    # matching sections: same d samples; kerb section = gutter + stone top, ramp = linear
    ds = [d_in, GUTTER_W, 1.0, 0.3, 0.0, -0.025, -0.07, -0.13, -d_out]
    kerb_z = [0.0, 0.03, 0.025, 0.022, 0.52, 0.6, 0.638, KERB_H, KERB_H]
    ramp = [(d, ramp_z(d)) for d in ds]
    kerb = list(zip(ds, kerb_z))
    bm = core.new_bm()
    rings = []
    # ramp body across the driveway width
    for x in (-hw, hw):
        rings.append([V((x, d, z)) for d, z in ramp] + [V((x, -d_out, -0.05)),
                                                        V((x, d_in, -0.05))])
    geo._finish(bm, geo._bridge(bm, rings, mi('concrete')))
    for s in (-1, 1):  # flares: ramp section at |x| = hw -> kerb section at |x| = hw + fl
        rings = []
        for x, sec in ((hw, ramp), (hw + fl, kerb)):
            rings.append([V((s * x, d, z)) for d, z in sec] + [V((s * x, -d_out, -0.05)),
                                                               V((s * x, d_in, -0.05))])
        geo._finish(bm, geo._bridge(bm, rings, mi('kerb')))
    ob = core.obj_from_bm('apron', bm, bevel=0.02, segments=2, angle=25)
    propkit.finish(ENV, 'apron', [ob], below=0.1, car=(hw + fl + 2, 4, 160))


def main():
    only = propkit._argv()[propkit._argv().index('--only') + 1] \
        if '--only' in propkit._argv() else None
    jobs = [(f'kerb_{n}', lambda n=n: straight(n)) for n in (4, 2, 1)]
    jobs += [('kerb_rolled', rolled), ('apron', apron)]
    for kind, (r, span, face) in sorted(plan.build().arcs.items()):
        jobs.append((kind[len('street_'):], lambda k=kind, r=r, s=span, f=face: arc(k, r, s, f)))
    for name, job in jobs:
        if only is None or only == name:
            job()


main()
