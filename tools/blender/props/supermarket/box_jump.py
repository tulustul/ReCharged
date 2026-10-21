"""Cardboard-box jump (hero 'cardboard-box ramp', art bible §3.5): a home-made RC jump across a
whole aisle, 9.8 x 40 u, takeoff toward +Y (Bevy -Z). A flattened-box ramp (2.2 u over 12 u) up
onto a table of taped shipping cartons, a folded-box kicker to the 2.7 u lip, and a long
double-sheet down-ramp (18 u). Matches the four collider primitives in catalog.py exactly.

    blender -b -P tools/blender/props/supermarket/box_jump.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

W = C.JUMP_W
J = C.JUMP
# prop z (Bevy, back +) -> Blender y = -z
Y0 = -C.JUMP_LEN / 2                 # back end (ramp foot)
Y1 = Y0 + J['ramp']                  # ramp top
Y2 = Y1 + J['table']                 # kicker foot
Y3 = Y2 + J['kicker']                # lip
Y4 = Y3 + J['down']                  # front end
T = 0.07                             # sheet thickness


def wedge_panels(bm, ya, yb, ha, hb, mat_side, mat_top, inset=0.0):
    """A cardboard wedge from (ya, ha) to (yb, hb): two side panels and a top sheet."""
    prof = [(ya, 0.0), (yb, 0.0), (yb, hb), (ya, ha)]
    for s in (-1, 1):
        x = s * (W / 2 - 0.05 - inset)
        geo.extrude_x(bm, prof, x - 0.05, x + 0.05, mat_side)
    # top sheet: a thin slab following the slope, square-edged where it meets the floor (a
    # sharp toe is so acute that the part bevel turns it inside out)
    rows = []
    for x in (-W / 2 + inset, W / 2 - inset):
        rows.append([V((x, ya, max(ha - T, 0.0))), V((x, yb, max(hb - T, 0.0))),
                     V((x, yb, max(hb, T))), V((x, ya, max(ha, T)))])
    return geo._finish(bm, geo._bridge(bm, rows, mat_top))


def sheets(bm, ya, yb, ha, hb, n, rnd):
    """Flattened, printed boxes laid over a slope in n overlapping rows of two, each a hair
    above the last so they never z-fight; tape across every seam."""
    dy = (yb - ya) / n
    k = 0
    for i in range(n):
        for side in (-1, 1):
            k += 1
            y0, y1 = ya + dy * i - 0.15, ya + dy * (i + 1) + 0.15
            y0, y1 = max(min(y0, y1), min(ya, yb)), min(max(y0, y1), max(ya, yb))
            x0, x1 = (-W / 2 + 0.1, -0.05) if side < 0 else (0.05, W / 2 - 0.1)
            lift = 0.012 * (k % 3 + 1)

            def h(y):
                return ha + (hb - ha) * (y - ya) / (yb - ya) + lift
            vs = [bm.verts.new(V(p)) for p in ((x0, y0, h(y0)), (x1, y0, h(y0)),
                                                (x1, y1, h(y1)), (x0, y1, h(y1)))]
            f = bm.faces.new(vs)
            f.material_index = mi('packaging')
            f.normal_update()
            if f.normal.z < 0:
                f.normal_flip()
            map_face(bm, f, cell('carton_%d' % (1 + rnd.randrange(3))), up=V((0, 1, 0)))
        yt = ya + dy * (i + 1)
        if i < n - 1:
            hh = ha + (hb - ha) * (yt - ya) / (yb - ya) + 0.05
            geo.box(bm, V((0, yt, hh)), (W - 0.4, 0.35, 0.02), mi('tape'))


def build():
    start('cardboard', 'cardboard_dark', 'tape', 'packaging', 'signs')
    bm = core.new_bm()
    rnd = rng(5)
    # short ramp (the flattened box lies on two side boards)
    wedge_panels(bm, Y0, Y1, 0.0, J['top'], mi('cardboard_dark'), mi('cardboard'))
    sheets(bm, Y0 + 0.4, Y1, 0.0, J['top'], 2, rnd)
    # table: 2 layers of 4 x 3 shipping cartons (sides printed, tape across the top)
    nx, ny = 4, 3
    bw, bd, bh = W / nx, (Y3 - Y1) / ny, J['top'] / 2
    for layer in range(2):
        for i in range(nx):
            for j in range(ny):
                c = V((-W / 2 + bw * (i + 0.5), Y1 + bd * (j + 0.5), bh * layer))
                product_box(bm, c, bw - 0.03, bd - 0.03, bh - 0.01,
                                 'carton_%d' % (1 + (i + j + layer) % 3), front=1,
                                 mat='packaging')
                if layer == 1:
                    geo.box(bm, V((c.x, c.y, c.z + bh - 0.005)), (0.35, bd - 0.05, 0.02),
                            mi('tape'))
    # kicker: a folded box over the front cartons
    wedge_panels(bm, Y2, Y3, J['top'], J['lip'], mi('cardboard_dark'), mi('cardboard'),
                 inset=0.05)
    # long down-ramp: two overlapping sheets on side boards
    wedge_panels(bm, Y4, Y3, 0.0, J['lip'], mi('cardboard_dark'), mi('cardboard'))
    sheets(bm, Y3, Y4 - 0.4, J['lip'], 0.0, 3, rnd)
    return propkit.finish(ENV, 'box_jump', [part('jump', bm, bevel=0.02)], budget=HERO,
                          car=(W / 2 + 2.5, Y0 + 2, 0), shots=None)


build()
