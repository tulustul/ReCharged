"""Shopping trolley (hero, art bible §3.5): a chrome wire basket that tapers to the nose (so
trolleys nest), the bottom tray, four swivel casters, a red handle grip and the fold-down child
seat (2.7 x 4.6 x 4.4 u). Front (nose) +Y, handle at -Y (Bevy +Z).

    blender -b -P tools/blender/props/supermarket/trolley.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from smkit import *  # noqa: E402,F403

L, W, H = C.TROLLEY_L, C.TROLLEY_W, C.TROLLEY_H
YN, YH = L / 2, -L / 2            # nose, handle end
ZB0, ZB1 = 2.25, 4.15             # basket bottom / top rim
WIRE = 0.035


def rod(bm, a, b, r=WIRE, mat='chrome', n=4):
    geo.cylinder(bm, V(a), V(b), r, mi(mat), n=n)


def basket_half_width(y, z):
    """The basket tapers toward the nose and toward the bottom."""
    t = (y - YH) / (YN - YH)
    k = (z - ZB0) / (ZB1 - ZB0)
    return (W / 2 - 0.1) * (1.0 - 0.18 * t) * (0.88 + 0.12 * k)


def build():
    start('chrome', 'red_plastic', 'black_plastic', 'rubber', 'steel_dark')
    wire = core.new_bm()
    yb0, yb1 = YH + 0.35, YN - 0.1
    # rims (top and bottom) and corner posts
    for z in (ZB0, ZB1):
        pts = []
        for y, s in ((yb0, -1), (yb1, -1), (yb1, 1), (yb0, 1)):
            pts.append(V((s * basket_half_width(y, z), y, z)))
        for a, b in zip(pts, pts[1:] + pts[:1]):
            rod(wire, a, b, 0.05, n=6)
    # side panels: vertical wires
    for s in (-1, 1):
        n = 12
        for i in range(n + 1):
            y = yb0 + (yb1 - yb0) * i / n
            rod(wire, (s * basket_half_width(y, ZB0), y, ZB0), (s * basket_half_width(y, ZB1), y,
                                                                 ZB1))
        for z in (ZB0 + 0.6, ZB0 + 1.2):
            rod(wire, (s * basket_half_width(yb0, z), yb0, z), (s * basket_half_width(yb1, z), yb1, z))
    # nose and back panels
    for y in (yb0, yb1):
        n = 8
        for i in range(n + 1):
            t = i / n * 2 - 1
            rod(wire, (t * basket_half_width(y, ZB0), y, ZB0), (t * basket_half_width(y, ZB1), y, ZB1))
    # basket floor grid
    for i in range(7):
        y = yb0 + (yb1 - yb0) * i / 6
        hw = basket_half_width(y, ZB0)
        rod(wire, (-hw, y, ZB0), (hw, y, ZB0))
    rod(wire, (0, yb0, ZB0), (0, yb1, ZB0))
    # chassis: side rails from the casters up to the handle, the bottom tray
    frame = core.new_bm()
    for s in (-1, 1):
        x = s * (W / 2 - 0.25)
        rod(frame, (x * 0.8, YN - 0.4, 0.45), (x, YH + 0.4, 0.45), 0.07, 'chrome', 8)
        rod(frame, (x, YH + 0.4, 0.45), (x * 1.02, YH + 0.1, H - 0.25), 0.07, 'chrome', 8)
        rod(frame, (x * 0.8, YN - 0.4, 0.45), (x * 0.85, YN - 0.3, ZB0), 0.06, 'chrome', 8)
    for i in range(6):
        y = YN - 0.5 - i * (L - 1.2) / 5
        rod(frame, (-(W / 2 - 0.4), y, 0.62), (W / 2 - 0.4, y, 0.62), 0.04, 'chrome', 4)
    # handle with the red grip
    rod(frame, (-(W / 2 - 0.2), YH + 0.1, H - 0.25), (W / 2 - 0.2, YH + 0.1, H - 0.25), 0.07,
        'chrome', 8)
    geo.cylinder(frame, V((-(W / 2 - 0.45), YH + 0.1, H - 0.25)),
                 V((W / 2 - 0.45, YH + 0.1, H - 0.25)), 0.12, mi('red_plastic'), n=10)
    # child seat flap (red plastic) against the back of the basket
    geo.box(frame, V((0, yb0 + 0.25, ZB1 - 0.45)), (W * 0.62, 0.08, 0.8), mi('red_plastic'))
    # casters
    for sx in (-1, 1):
        for sy, scale in ((YN - 0.45, 0.8), (YH + 0.45, 1.0)):
            x = sx * (W / 2 - 0.25) * scale
            geo.box(frame, V((x, sy, 0.38)), (0.16, 0.22, 0.12), mi('steel_dark'))
            geo.cylinder(frame, V((x - 0.07, sy - 0.05, 0.2)), V((x + 0.07, sy - 0.05, 0.2)), 0.2,
                         mi('rubber'), n=10)
    return propkit.finish(ENV, 'trolley', [smooth('wire', wire, angle=50),
                                           smooth('frame', frame, angle=50)],
                          budget=HERO, car=(W / 2 + 2.2, 0, 180))


build()
