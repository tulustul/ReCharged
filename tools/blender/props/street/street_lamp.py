"""Street lamp (hero): a 7 m tapered steel pole on a concrete footing with a fluted base cover,
an upswept arm and a cobra-head luminaire over the road (+Y) with a sodium lens (emissive,
catalog `lights` at the lens).

    blender -b -P tools/blender/props/street/street_lamp.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from streetkit import *  # noqa: E402,F403

TOP = 28.0          # pole top
REACH = 7.4         # arm reach toward the road (+Y)
LENS = V((0.0, 6.6, 28.75))


def build():
    start('concrete', 'pole_grey', 'lamp_sodium')
    bm = core.new_bm()
    pg = mi('pole_grey')
    geo.cylinder(bm, V((0, 0, -0.2)), V((0, 0, 0.35)), 1.05, mi('concrete'), n=16)  # footing
    geo.cylinder(bm, V((0, 0, 0.35)), V((0, 0, 1.9)), 0.78, pg, n=12, r1=0.62)        # base cover
    geo.cylinder(bm, V((0, 0, 1.9)), V((0, 0, 2.05)), 0.66, pg, n=12)
    geo.cylinder(bm, V((0, 0, 2.0)), V((0, 0, TOP)), 0.5, pg, n=12, r1=0.3)          # tapered pole
    geo.cylinder(bm, V((0, 0, TOP)), V((0, 0, TOP + 0.25)), 0.34, pg, n=12, r1=0.2)  # cap
    # upswept arm: from the pole just below the top, curving out over the road
    geo.tube(bm, [(0, 0.1, TOP - 1.6), (0, 1.2, TOP - 0.2), (0, 3.4, TOP + 0.65),
                  (0, REACH - 1.8, TOP + 0.9)], 0.2, pg, n=8, corner=1.2)
    geo.tube(bm, [(0, 0.3, TOP - 3.2), (0, 1.6, TOP - 1.0), (0, 3.2, TOP + 0.3)], 0.09, pg,
             n=6, corner=0.6)  # brace
    arm = core.obj_from_bm('pole', bm, weighted=False, angle=40)
    # cobra head: lofted rounded housing, lens underneath
    h = core.new_bm()
    keys = [(REACH - 2.4, 0.35, TOP + 0.55, TOP + 1.15, 0.2),
            (REACH - 1.6, 0.75, TOP + 0.35, TOP + 1.25, 0.35),
            (REACH - 0.2, 0.95, TOP + 0.3, TOP + 1.2, 0.4),
            (REACH + 0.9, 0.85, TOP + 0.35, TOP + 1.05, 0.35),
            (REACH + 1.5, 0.5, TOP + 0.45, TOP + 0.85, 0.2)]
    geo.slab(h, keys, pg)
    geo.slab(h, [(REACH - 1.3, 0.62, TOP + 0.18, TOP + 0.4, 0.2),
                 (REACH + 0.9, 0.55, TOP + 0.18, TOP + 0.4, 0.2)], mi('lamp_sodium'))  # lens bowl
    head = core.obj_from_bm('head', h, weighted=False, angle=50)
    street_props.finish(ENV, 'street_lamp', [arm, head], budget=HERO, below=0.25,
                        car=(2.0, -1.5, 30),
                        cams=[('hero', (34, -30, 22), (0, 2, 15), 40),
                              ('head', (9, 16, 27), (0, 6, 28.5), 45),
                              ('rc', (3.0, 14.0, 0.7), (0, 3, 12), 22)])


build()
