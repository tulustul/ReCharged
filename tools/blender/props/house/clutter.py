"""Small family-house clutter (kit), built together, real size x 4.44:
book_small / book_medium / book_large (paperback, hardcover, art book), magazines (fanned pile),
mug, remote, cushion (floor cushion), slipper, shoe (kid's sneaker), laundry_basket (with
clothes), bowl (dog bowl), cable (extension cable with plug), toy_truck (wooden pull-along
truck), pendant (ceiling lamp) and rug_fringe (decor strip for the rugs' short edges). Most are knockable in the catalog.

    blender -b -P tools/blender/props/house/clutter.py -- [--preview DIR] [name ...]
"""
import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from housekit import *  # noqa: E402,F403

ARGS = [a for a in (sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])]
if '--preview' in ARGS:
    i = ARGS.index('--preview')
    ARGS = ARGS[:i] + ARGS[i + 2:]
BUILDERS = {}


def builder(fn):
    BUILDERS[fn.__name__] = fn
    return fn


def mats(*names):
    core.reset()
    materials(*names)


@builder
def book_small():
    mats('pages', 'book_red')
    bm = core.new_bm()
    book(bm, frame_m((0, 0, 0.055)), (0.5, 0.8, 0.11), 'book_red', board=0.012)
    return [core.obj_from_bm('book', bm, bevel=0.006, segments=1)], (1.4, 0.4, 200)


@builder
def book_medium():
    mats('pages', 'book_blue')
    bm = core.new_bm()
    book(bm, frame_m((0, 0, 0.08)), (0.71, 1.07, 0.16), 'book_blue')
    return [core.obj_from_bm('book', bm, bevel=0.008, segments=1)], (1.6, 0.4, 200)


@builder
def book_large():
    mats('pages', 'book_mustard', 'book_teal')
    bm = core.new_bm()
    book(bm, frame_m((0, 0, 0.065)), (1.07, 1.38, 0.13), 'book_mustard')
    obox(bm, frame_m((0.0, 0.1, 0.131)), (0.7, 0.5, 0.004), mi('book_teal'))  # cover print
    return [core.obj_from_bm('book', bm, bevel=0.008, segments=1)], (1.8, 0.4, 200)


@builder
def magazines():
    mats('book_teal', 'book_red', 'book_cream', 'pages')
    bm = core.new_bm()
    for k, (col, yaw, dx) in enumerate([('book_cream', 0, 0), ('book_red', 14, 0.12),
                                        ('book_teal', -9, -0.08)]):
        M = frame_m((dx, 0.06 * k, 0.022 + 0.042 * k), yaw=yaw)
        obox(bm, M, (0.93, 1.24, 0.04), mi(col))
        obox(bm, M, (0.9, 1.22, 0.034), mi('pages'), center=(0.02, 0, 0))
    return [core.obj_from_bm('mags', bm, bevel=0.006, segments=1)], (1.6, 0.4, 200)


@builder
def mug():
    mats('fabric_mustard', 'ceramic', 'soil')
    bm = core.new_bm()
    mug_glaze = mi('fabric_mustard')
    geo.cylinder(bm, V((0, 0, 0)), V((0, 0, 0.42)), 0.19, mug_glaze, n=20)
    geo.cylinder(bm, V((0, 0, 0.36)), V((0, 0, 0.421)), 0.165, mi('soil'), n=20)  # coffee
    geo.tube(bm, [V((0.17, 0, 0.33)), V((0.31, 0, 0.3)), V((0.31, 0, 0.13)), V((0.17, 0, 0.1))],
             0.03, mug_glaze, n=8, corner=0.07, cseg=3)
    return [core.obj_from_bm('mug', bm, weighted=False, angle=50)], (1.0, 0.3, 200)


@builder
def remote():
    mats('plastic_black', 'book_red', 'plastic_white', 'book_blue')
    bm = core.new_bm()
    rbox(bm, V((0, 0, 0.05)), (0.22, 0.8, 0.1), 0.04, mi('plastic_black'), seg=2, axis='y')
    geo.cylinder(bm, V((0, 0.28, 0.1)), V((0, 0.28, 0.115)), 0.035, mi('book_red'), n=8)
    for i in range(3):
        for j in range(3):
            geo.box(bm, V((-0.06 + 0.06 * i, 0.05 - 0.07 * j, 0.105)), (0.035, 0.035, 0.012),
                    mi('plastic_white'))
    geo.box(bm, V((0, -0.22, 0.105)), (0.12, 0.1, 0.012), mi('book_blue'))
    return [core.obj_from_bm('remote', bm, weighted=False)], (0.9, 0.2, 200)


@builder
def cushion():
    mats('fabric_rust', 'fabric_cream')
    bm = core.new_bm()
    rbox(bm, V((0, 0, 0.3)), (2.0, 2.0, 0.42), 0.18, mi('fabric_rust'), seg=4, puff=0.35)
    for s in (-1, 1):  # piping-ish tassel corners
        geo.sphere(bm, V((s * 0.98, s * 0.98, 0.3)), 0.07, mi('fabric_cream'), n=6, rings=4)
    return [core.obj_from_bm('cushion', box_uv(bm, 'x'), weighted=False, angle=60)], (2.2, 0.4, 200)


@builder
def slipper():
    mats('fabric_grey', 'rubber', 'knit_cream')
    bm = core.new_bm()
    rbox(bm, V((0, 0, 0.04)), (0.46, 1.22, 0.08), 0.04, mi('rubber'), seg=2, axis='y')  # sole
    rbox(bm, V((0, 0.2, 0.17)), (0.42, 0.72, 0.24), 0.16, mi('fabric_grey'), seg=4, axis='y')
    rbox(bm, V((0, -0.3, 0.12)), (0.43, 0.6, 0.12), 0.05, mi('knit_cream'), seg=2, axis='y')
    return [core.obj_from_bm('slipper', box_uv(bm, 'y'), weighted=False, angle=60)], (1.2, 0.3, 200)


@builder
def shoe():
    mats('book_red', 'plastic_white', 'rubber', 'book_cream')
    bm = core.new_bm()
    rbox(bm, V((0, 0, 0.06)), (0.5, 1.33, 0.12), 0.05, mi('plastic_white'), seg=2, axis='y')
    rbox(bm, V((0, -0.12, 0.27)), (0.46, 1.05, 0.32), 0.15, mi('book_red'), seg=3, axis='y')
    rbox(bm, V((0, 0.5, 0.2)), (0.47, 0.34, 0.2), 0.1, mi('plastic_white'), seg=2, axis='y')  # toe
    rbox(bm, V((0, -0.45, 0.45)), (0.44, 0.34, 0.2), 0.08, mi('book_red'), seg=2, axis='y')  # heel
    for k in range(4):  # laces
        geo.box(bm, V((0, 0.2 - 0.12 * k, 0.43 - 0.02 * k)), (0.3, 0.04, 0.03), mi('book_cream'))
    geo.box(bm, V((0, 0, 0.02)), (0.48, 1.25, 0.04), mi('rubber'))
    return [core.obj_from_bm('shoe', bm, weighted=False, angle=60)], (1.3, 0.3, 200)


@builder
def laundry_basket():
    mats('fabric_grey', 'fabric_blue', 'book_cream', 'fabric_mustard', 'fabric_rust', 'knit_cream')
    bm = core.new_bm()
    W, D, H, t = 2.44, 1.69, 1.33, 0.08
    for s in (-1, 1):
        rbox(bm, V((0, s * (D / 2 - t / 2), H / 2)), (W, t, H), 0.035, mi('fabric_grey'), seg=2)
        rbox(bm, V((s * (W / 2 - t / 2), 0, H / 2)), (t, D - 2 * t, H), 0.035, mi('fabric_grey'),
             seg=2)
    geo.box(bm, V((0, 0, 0.03)), (W - 0.1, D - 0.1, 0.06), mi('fabric_grey'))
    for k in range(2):  # rope handles
        geo.tube(bm, [V((-0.4, (2 * k - 1) * (D / 2 + 0.02), H - 0.25)),
                      V((0, (2 * k - 1) * (D / 2 + 0.1), H - 0.05)),
                      V((0.4, (2 * k - 1) * (D / 2 + 0.02), H - 0.25))], 0.035, mi('knit_cream'),
                 n=6, corner=0.15, cseg=3)
    basket = core.obj_from_bm('basket', box_uv(bm, 'x'), weighted=False, angle=50)
    bm = core.new_bm()
    rnd = random.Random(3)
    z = 0.1
    for col in ('fabric_blue', 'book_cream', 'fabric_mustard', 'fabric_rust', 'fabric_blue'):
        th = rnd.uniform(0.18, 0.28)
        rbox(bm, V((rnd.uniform(-0.2, 0.2), rnd.uniform(-0.1, 0.1), z + th / 2)),
             (W - 0.35 - rnd.uniform(0, 0.5), D - 0.35 - rnd.uniform(0, 0.3), th), 0.08, mi(col),
             seg=2, puff=0.15, rot=(rnd.uniform(-10, 10), 0, rnd.uniform(-3, 3)))
        z += th * 0.85
    rbox(bm, V((0.6, 0.2, H + 0.05)), (0.9, 0.5, 0.25), 0.1, mi('fabric_rust'), seg=2, puff=0.3,
         rot=(25, 10, 18))  # a sleeve hanging over the rim
    clothes = core.obj_from_bm('clothes', box_uv(bm, 'x'), weighted=False, angle=60)
    return [basket, clothes], (2.8, 0.6, 200)


@builder
def bowl():
    mats('steel', 'soil')
    bm = core.new_bm()
    n = 24
    prof = [(0.28, 0.0), (0.45, 0.02), (0.45, 0.06), (0.36, 0.28), (0.34, 0.3), (0.3, 0.27)]
    rings = [[V((r * math.cos(2 * math.pi * k / n), r * math.sin(2 * math.pi * k / n), z))
              for k in range(n)] for r, z in prof]
    geo._bridge(bm, rings, mi('steel'), cap0=True, cap1=True)
    geo.cylinder(bm, V((0, 0, 0.06)), V((0, 0, 0.16)), 0.28, mi('soil'), n=n)  # kibble
    return [core.obj_from_bm('bowl', bm, weighted=False, angle=40)], (1.0, 0.3, 200)


@builder
def cable():
    mats('plastic_white', 'plastic_black')
    bm = core.new_bm()
    path = [V((x, 0.35 * math.sin(x * 0.9) + 0.12 * math.sin(x * 2.3), 0.035))
            for x in [-3.35 + 6.7 * i / 40 for i in range(41)]]
    sweep(bm, path, 0.07, 0.06, mi('plastic_white'), r=0.03, side=(0, 0, 1))
    end = path[-1]
    geo.box(bm, V((end.x + 0.15, end.y, 0.08)), (0.3, 0.22, 0.16), mi('plastic_white'))  # plug
    for dy in (-0.05, 0.05):
        geo.box(bm, V((end.x + 0.36, end.y + dy, 0.08)), (0.14, 0.025, 0.025), mi('plastic_black'))
    return [core.obj_from_bm('cable', bm, weighted=False, angle=40)], (0.0, 1.6, 200)


@builder
def toy_truck():
    mats('oak', 'book_red', 'book_blue', 'plastic_black', 'book_mustard')
    bm = core.new_bm()
    geo.box(bm, V((0, -0.15, 0.24)), (0.62, 1.05, 0.14), mi('oak'))  # chassis
    geo.box(bm, V((0, 0.36, 0.48)), (0.6, 0.42, 0.42), mi('book_red'))  # cab
    geo.box(bm, V((0, 0.4, 0.62)), (0.62, 0.3, 0.14), mi('book_blue'))  # windows band
    for s in (-1, 1):  # dumper bed
        geo.box(bm, V((s * 0.28, -0.32, 0.45)), (0.06, 0.62, 0.3), mi('book_mustard'))
    geo.box(bm, V((0, -0.62, 0.45)), (0.62, 0.06, 0.3), mi('book_mustard'))
    geo.box(bm, V((0, -0.32, 0.33)), (0.62, 0.62, 0.06), mi('book_mustard'))
    body = core.obj_from_bm('body', box_uv(bm, 'y'), bevel=0.03, segments=2)
    bm = core.new_bm()
    for x in (-0.36, 0.36):
        for y in (0.33, -0.42):
            geo.cylinder(bm, V((x - 0.06 * (1 if x > 0 else -1), y, 0.17)),
                         V((x + 0.06 * (1 if x > 0 else -1), y, 0.17)), 0.17, mi('oak'), n=16)
            geo.cylinder(bm, V((x, y, 0.17)), V((x + 0.08 * (1 if x > 0 else -1), y, 0.17)), 0.06,
                         mi('plastic_black'), n=8)
    geo.tube(bm, [V((0, 0.6, 0.25)), V((0, 1.0, 0.04)), V((0.2, 1.6, 0.03))], 0.012,
             mi('book_red'), n=4)  # pull string (thin, lies on the floor)
    wheels = core.obj_from_bm('wheels', box_uv(bm, 'x'), weighted=False, angle=40)
    return [body, wheels], (1.5, 0.3, 200)


@builder
def pendant():
    """Pendant lamp hanging from the 11.5 u ceiling: black dome shade (35 cm) over a warm bulb,
    shade rim at 7.0 u; origin on the floor below it (catalog light at the bulb)."""
    mats('metal_black', 'brass', 'bulb', 'plastic_black')
    bm = core.new_bm()
    n = 28
    prof = [(0.08, 8.15), (0.12, 8.0), (0.55, 7.65), (0.78, 7.2), (0.8, 7.0), (0.76, 7.02)]
    rings = [[V((r * math.cos(2 * math.pi * k / n), r * math.sin(2 * math.pi * k / n), z))
              for k in range(n)] for r, z in prof]
    geo._bridge(bm, rings, mi('metal_black'), cap0=True, cap1=False)
    geo.cylinder(bm, V((0, 0, 8.1)), V((0, 0, 8.45)), 0.1, mi('brass'), n=12)
    geo.cylinder(bm, V((0, 0, 8.45)), V((0, 0, 11.5)), 0.02, mi('plastic_black'), n=6)
    geo.cylinder(bm, V((0, 0, 11.4)), V((0, 0, 11.5)), 0.3, mi('metal_black'), n=16)
    geo.sphere(bm, V((0, 0, 7.35)), 0.2, mi('bulb'), n=10, rings=6)
    return [core.obj_from_bm('pendant', bm, weighted=False, angle=40)], (1.8, 0.4, 200)


@builder
def rug_fringe():
    mats('knit_cream')
    bm = core.new_bm()
    geo.box(bm, V((0, 0, 0.03)), (14.0, 0.12, 0.06), mi('knit_cream'))  # binding
    for k in range(70):
        x = -6.9 + 13.8 * k / 69
        geo.box(bm, V((x, 0.3, 0.012)), (0.07, 0.42, 0.024), mi('knit_cream'))
    return [core.obj_from_bm('fringe', bm, weighted=False)], (0.0, 1.6, 200)


for name, fn in BUILDERS.items():
    if ARGS and name not in ARGS:
        continue
    parts, car = fn()
    done(name, parts, car=car, below=0.0)
