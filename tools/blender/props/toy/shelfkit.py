"""Builder of the toy-store gondola shelf variants (shelf_a.py, shelf_b.py): a white steel gondola
(1.2 m wide, 1.8 m tall, real x 4.44) with a pegboard back, a kick plate, three shelves with
price-tag strips and a header sign with a pictogram, stocked with seeded merchandise: windowed
toy boxes, plush bears, balls and block stacks. Front = +Y. Static backdrop along the walls.
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

W, D, TOPZ = 5.4, 2.2, 7.9          # width, depth, top cap height
LEVELS = [0.7, 2.55, 4.4, 6.25]     # deck + shelves (top surfaces)
BOX_COLS = [RED, BLUE, YELLOW, GREEN, PURPLE, PINK, ORANGE, TEAL, '#F2F0EA']


def build(name, seed, header_color, pictogram):
    rnd = random.Random(seed)
    core.reset()
    painted('steel', '#E9EAE8', 0.4)
    textured('peg', '#DCD9D2', 0.6, dots_image('peg', '#DCD9D2', per_tile=8))
    painted('header', header_color, 0.4)
    painted('sign', '#F4F1EA', 0.4)
    painted('tag', '#F7D23A', 0.4)
    painted('tagw', '#F7F5EE', 0.4)
    for k, c in enumerate(BOX_COLS):
        painted(f'box{k}', c, 0.45)
    core.material('window', '#A9C4D6', 0.0, 0.06)
    fabric('plush', '#B07A4A')
    fabric('plush2', '#E9DCC6')
    fabric('plush3', '#E58FB0')
    plastic('dark', '#1C1C1E', 0.3)
    mi = core.mi

    # --- gondola --------------------------------------------------------------------------
    g = core.new_bm()
    for sx in (-1, 1):  # uprights
        geo.box(g, V((sx * (W / 2 - 0.12), -D / 2 + 0.2, TOPZ / 2)), (0.24, 0.4, TOPZ), mi('steel'))
        geo.box(g, V((sx * (W / 2 - 0.13), 0.02, 0.05)), (0.3, D - 0.04, 0.1), mi('steel'))  # feet
    geo.box(g, V((0, -D / 2 + 0.45, 0.35)), (W - 0.1, 0.1, 0.7), mi('steel'))  # kick plate back
    geo.box(g, V((0, D / 2 - 0.05, 0.33)), (W, 0.1, 0.66), mi('steel'))       # kick plate front
    for z in LEVELS:
        geo.box(g, V((0, 0.12, z - 0.06)), (W - 0.1, D - 0.3, 0.12), mi('steel'))
        if z > LEVELS[0]:  # shelf lip with the price strip
            geo.box(g, V((0, D / 2 - 0.15, z - 0.14)), (W - 0.1, 0.08, 0.28), mi('steel'))
    geo.box(g, V((0, 0, TOPZ + 0.05)), (W, D * 0.5, 0.1), mi('steel'))  # top cap
    gondola = part('gondola', g, bevel=0.03, segments=2)
    back = core.new_bm()
    geo.box(back, V((0, -D / 2 + 0.3, (TOPZ + 0.7) / 2)), (W - 0.48, 0.08, TOPZ - 0.7), mi('peg'))
    back_ob = part('back', box_uv(back, 0.5), bevel=0.0)

    # header sign with a pictogram, price tags
    h = core.new_bm()
    geo.box(h, V((0, -D / 2 + 0.3, TOPZ + 0.75)), (W * 0.82, 0.16, 1.3), mi('header'))
    geo.box(h, V((0, -D / 2 + 0.4, TOPZ + 0.75)), (1.3, 0.06, 1.0), mi('sign'))
    tags = core.new_bm()
    for z in LEVELS[1:]:
        geo.box(tags, V((0, D / 2 - 0.1, z - 0.14)), (W - 0.2, 0.02, 0.2), mi('tag'))
        x = -W / 2 + 0.6
        while x < W / 2 - 0.5:
            geo.box(tags, V((x, D / 2 - 0.085, z - 0.14)), (0.32, 0.02, 0.14), mi('tagw'))
            x += rnd.uniform(0.9, 1.5)
    pic = core.new_bm()
    c = V((0, -D / 2 + 0.44, TOPZ + 0.75))
    if pictogram == 'bear':
        geo.sphere(pic, c + V((0, 0, -0.08)), 0.28, mi('header'), n=16, rings=8, scale=(1, 0.15, 1))
        for sx in (-1, 1):
            geo.sphere(pic, c + V((sx * 0.25, 0, 0.17)), 0.12, mi('header'), n=12, rings=6,
                       scale=(1, 0.15, 1))
    else:  # star
        relief(pic, star2(), c, (0, 1, 0), (0, 0, 1), 0.75, 0.04, mi('header'))
    header = part('header', h, bevel=0.03, segments=2)
    tag_ob = part('tags', tags, bevel=0.0)
    pic_ob = smooth('pictogram', pic)

    # --- merchandise ------------------------------------------------------------------------
    boxes, round_ = core.new_bm(), core.new_bm()

    def toy_box(x, y, z, w, d, ht, col):
        geo.box(boxes, V((x, y, z + ht / 2)), (w, d, ht), mi(f'box{col}'))
        geo.box(boxes, V((x, y + d / 2 + 0.005, z + ht * 0.45)), (w * 0.62, 0.02, ht * 0.5),
                mi('window'))
        toy = (col + rnd.randint(1, 7)) % len(BOX_COLS)  # the toy behind the window
        geo.box(boxes, V((x, y + d / 2 + 0.012, z + ht * 0.4)), (w * 0.36, 0.02, ht * 0.22),
                mi(f'box{toy}'))
        geo.box(boxes, V((x, y + d / 2 + 0.012, z + ht * 0.55)), (w * 0.2, 0.02, ht * 0.12),
                mi(f'box{toy}'))
        band = (col + rnd.randint(2, 5)) % len(BOX_COLS)
        geo.box(boxes, V((x, y + d / 2 + 0.008, z + ht * 0.86)), (w * 0.9, 0.02, ht * 0.16),
                mi(f'box{band}'))

    def bear(x, y, z, s, mat):
        geo.sphere(round_, V((x, y, z + 0.42 * s)), 0.42 * s, mi(mat), n=12, rings=8,
                   scale=(1, 0.9, 1.05))
        geo.sphere(round_, V((x, y + 0.05 * s, z + 1.05 * s)), 0.33 * s, mi(mat), n=12, rings=8)
        for sx in (-1, 1):
            geo.sphere(round_, V((x + sx * 0.25 * s, y, z + 1.33 * s)), 0.12 * s, mi(mat), n=8,
                       rings=5)
            geo.sphere(round_, V((x + sx * 0.12 * s, y + 0.33 * s, z + 1.12 * s)), 0.045 * s,
                       mi('dark'), n=6, rings=4)
        geo.sphere(round_, V((x, y + 0.32 * s, z + 1.0 * s)), 0.11 * s, mi('plush2'), n=8, rings=5)

    for li, z in enumerate(LEVELS):
        room = (LEVELS[li + 1] if li + 1 < len(LEVELS) else TOPZ) - z - 0.25
        kind = rnd.choice(['boxes', 'boxes', 'plush', 'balls']) if li else 'big'
        x = -W / 2 + 0.35
        while x < W / 2 - 0.6:
            if kind in ('boxes', 'big'):
                w = rnd.uniform(0.8, 1.4) if kind == 'boxes' else rnd.uniform(1.2, 1.7)
                w = min(w, W / 2 - 0.3 - x)
                if w < 0.5:
                    break
                ht = min(room, rnd.uniform(0.9, 1.5) if kind == 'boxes' else rnd.uniform(1.2, 1.7))
                toy_box(x + w / 2, 0.25, z, w - 0.08, 1.2, ht, rnd.randrange(len(BOX_COLS)))
                x += w
            elif kind == 'plush':
                s = rnd.uniform(0.8, 1.05)
                bear(x + 0.5 * s, 0.35, z, s, rnd.choice(['plush', 'plush2', 'plush3']))
                x += 1.0 * s + 0.1
            else:
                r = rnd.uniform(0.35, 0.5)
                geo.sphere(round_, V((x + r, 0.4, z + r)), r, mi(f'box{rnd.randrange(8)}'), n=14,
                           rings=8)
                x += 2 * r + 0.08
    box_ob = part('boxes', boxes, bevel=0.02, segments=1)
    round_ob = smooth('round', round_)
    propkit.finish(ENV, name, [gondola, back_ob, header, tag_ob, pic_ob, box_ob, round_ob],
                   budget=propkit.HERO_TRIS, car=(1.0, 3.4, 200))
