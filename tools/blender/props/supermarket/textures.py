"""Supermarket textures (PIL): the packaging atlas for every product on the shelves and the
signage sheet. All brands, logos and graphics are invented (art bible §3.5: no real brands);
text uses the project's OFL font assets/fonts/Saira-Variable.ttf.

    python3 tools/blender/props/supermarket/textures.py

writes tex/packaging.png (1024², 8 x 8 cells of 128 px), tex/signs.png (1024²), both 256-colour,
and tex/atlas.json (cell / region rectangles in pixels) next to this script. Prop scripts embed
the PNGs into their glb files (sRGB base colour).
"""
import json
import math
import os
import random

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
OUT = os.path.join(HERE, 'tex')
FONT = os.path.join(ROOT, 'assets', 'fonts', 'Saira-Variable.ttf')
CELL, GRID, SS = 128, 8, 4          # final cell size, cells per side, supersampling

# art bible §3.5 accents + neutrals (packaging may be saturated in small areas)
RED, GREEN, YELLOW, CYAN = '#D7352B', '#3E9B4F', '#F4D03F', '#7FD8F0'
PALETTE = ['#D7352B', '#3E9B4F', '#2F6FD6', '#F2A33A', '#7A4FC2', '#E8739E', '#1F8A8A',
           '#B5553C', '#2C3E66', '#C9A227', '#5B8C2A', '#8E2C48', '#E0E0D8', '#262A33']


def font(size, weight=700, width=100):
    f = ImageFont.truetype(FONT, size)
    f.set_variation_by_axes([weight, width])
    return f


def text_fit(d, xy, text, max_w, size, weight=800, width=90, fill='white', anchor='mm',
             stroke=0, stroke_fill=None):
    """Draw text centred at xy, shrinking the font until it fits max_w."""
    while size > 8:
        f = font(size, weight, width)
        if d.textlength(text, font=f) <= max_w:
            break
        size -= 2
    d.text(xy, text, font=f, fill=fill, anchor=anchor, stroke_width=stroke,
           stroke_fill=stroke_fill)


def shade(hexc, k):
    """Darken (k < 1) or lighten (k > 1) a hex colour."""
    h = hexc.lstrip('#')
    rgb = [int(h[i:i + 2], 16) for i in (0, 2, 4)]
    if k < 1:
        rgb = [int(c * k) for c in rgb]
    else:
        rgb = [int(c + (255 - c) * (k - 1)) for c in rgb]
    return '#%02X%02X%02X' % tuple(max(0, min(255, c)) for c in rgb)


def contrast_ink(hexc):
    h = hexc.lstrip('#')
    r, g, b = (int(h[i:i + 2], 16) for i in (0, 2, 4))
    return '#1E2024' if 0.299 * r + 0.587 * g + 0.114 * b > 150 else '#F6F3EA'


# --- product art (drawn on an S x S canvas, S = CELL * SS) ------------------------------------------
S = CELL * SS


def art_box(d, rnd, bg, brand, sub, motif):
    """Box front (cereal, crackers, pasta, tea...): band, motif, brand."""
    d.rectangle([0, 0, S, S], fill=bg)
    band = shade(bg, 0.7)
    d.rectangle([0, 0, S, int(S * 0.24)], fill=band)
    d.rectangle([0, int(S * 0.9), S, S], fill=band)
    ink = contrast_ink(bg)
    cx, cy = S // 2, int(S * 0.6)
    if motif == 'bowl':
        d.ellipse([cx - 170, cy - 120, cx + 170, cy - 40], fill='#F2EEE4')
        for _ in range(26):
            x, y = cx + rnd.randint(-140, 140), cy - 80 + rnd.randint(-40, 10)
            r = rnd.randint(14, 26)
            d.ellipse([x - r, y - r, x + r, y + r], fill=rnd.choice(['#E8B34A', '#C98A2E',
                                                                      '#F2D27A']))
        d.chord([cx - 180, cy - 110, cx + 180, cy + 150], 0, 180, fill='#FFFFFF')
        d.chord([cx - 180, cy - 110, cx + 180, cy + 150], 0, 180, outline=band, width=10)
    elif motif == 'rings':
        for _ in range(14):
            x, y = cx + rnd.randint(-170, 170), cy + rnd.randint(-110, 110)
            r = rnd.randint(28, 44)
            c = rnd.choice(['#F4D03F', '#E8739E', '#7FD8F0', '#F2A33A', '#9BD06A'])
            d.ellipse([x - r, y - r, x + r, y + r], outline=c, width=22)
    elif motif == 'crackers':
        for row in range(2):
            for col in range(3):
                x, y = cx - 150 + col * 150, cy - 70 + row * 140
                d.ellipse([x - 62, y - 62, x + 62, y + 62], fill='#E2B36A', outline='#B07A34',
                          width=8)
                for k in range(5):
                    a = k * 2 * math.pi / 5
                    d.ellipse([x + 30 * math.cos(a) - 6, y + 30 * math.sin(a) - 6,
                               x + 30 * math.cos(a) + 6, y + 30 * math.sin(a) + 6],
                              fill='#9C6A2A')
    elif motif == 'pasta':
        d.rounded_rectangle([cx - 190, cy - 130, cx + 190, cy + 130], 40, fill='#FFFDF4')
        for _ in range(9):
            x, y = cx + rnd.randint(-150, 150), cy + rnd.randint(-95, 95)
            for k in range(4):
                d.arc([x - 40 + k * 10, y - 18, x + 40 + k * 10, y + 18], 0, 360,
                      fill='#E8C04A', width=12)
    elif motif == 'leaf':
        for k in range(5):
            a = -0.9 + k * 0.45
            x, y = cx + 120 * math.sin(a), cy + 30 - 120 * math.cos(a) * 0.6
            d.ellipse([x - 40, y - 80, x + 40, y + 80], fill=shade(GREEN, 1.15 - 0.08 * k))
    elif motif == 'bubbles':
        for _ in range(24):
            x, y = rnd.randint(0, S), rnd.randint(int(S * 0.3), S)
            r = rnd.randint(14, 50)
            d.ellipse([x - r, y - r, x + r, y + r], outline='#FFFFFF', width=8)
        d.polygon([(0, cy + 40), (S * 0.3, cy - 20), (S * 0.6, cy + 60), (S, cy - 10), (S, S),
                   (0, S)], fill=shade(bg, 1.35))
    elif motif == 'chips':
        for _ in range(9):
            x, y = cx + rnd.randint(-160, 160), cy + rnd.randint(-100, 110)
            d.ellipse([x - 70, y - 40, x + 70, y + 40], fill='#F2C14E', outline='#C9902A',
                      width=8)
    elif motif == 'cookie':
        for k in range(3):
            x, y = cx - 120 + k * 120, cy + (k % 2) * 40
            d.ellipse([x - 80, y - 80, x + 80, y + 80], fill='#C98A4B')
            for _ in range(7):
                px, py = x + rnd.randint(-50, 50), y + rnd.randint(-50, 50)
                d.ellipse([px - 12, py - 12, px + 12, py + 12], fill='#4A2A18')
    elif motif == 'sun':
        for k in range(16):
            a = k * math.pi / 8
            d.line([cx, cy, cx + 260 * math.cos(a), cy + 260 * math.sin(a)],
                   fill=shade(bg, 1.25), width=34)
        d.ellipse([cx - 100, cy - 100, cx + 100, cy + 100], fill=YELLOW)
    text_fit(d, (S // 2, int(S * 0.12)), brand, S - 40, 120, fill=contrast_ink(band))
    text_fit(d, (S // 2, int(S * 0.34)), sub, S - 60, 64, weight=600, width=100, fill=ink)


LABEL_ASPECT = 2.4   # circumference / height of a tin: labels are drawn wide, then squashed


def art_label(d, rnd, bg, brand, sub, fruit):
    """Wrap-around label (cans, bottles, jars): stripes, a fruit / product picture, brand.
    `d` draws on a LABEL_ASPECT x 1 canvas (see packaging())."""
    W = int(S * LABEL_ASPECT)
    d.rectangle([0, 0, W, S], fill=bg)
    d.rectangle([0, 0, W, 60], fill=shade(bg, 0.65))
    d.rectangle([0, S - 60, W, S], fill=shade(bg, 0.65))
    d.rectangle([0, 64, W, 84], fill='#F6F0DC')
    d.rectangle([0, S - 84, W, S - 64], fill='#F6F0DC')
    for k in range(2):                      # the picture appears twice around the can
        cx, cy = W // 4 + k * W // 2 + W // 8, int(S * 0.56)
        if fruit == 'tomato':
            d.ellipse([cx - 80, cy - 70, cx + 80, cy + 80], fill='#D7352B')
            d.polygon([(cx, cy - 70), (cx - 30, cy - 95), (cx + 30, cy - 95)], fill='#3E9B4F')
        elif fruit == 'beans':
            for _ in range(9):
                x, y = cx + rnd.randint(-70, 70), cy + rnd.randint(-50, 60)
                d.ellipse([x - 26, y - 16, x + 26, y + 16], fill='#B5553C', outline='#7A3220',
                          width=4)
        elif fruit == 'corn':
            d.ellipse([cx - 45, cy - 95, cx + 45, cy + 95], fill='#F2C230')
            for y in range(cy - 80, cy + 80, 22):
                for x in range(cx - 30, cx + 31, 22):
                    d.ellipse([x - 8, y - 8, x + 8, y + 8], fill='#D9A21E')
            d.polygon([(cx - 50, cy + 95), (cx - 90, cy - 40), (cx - 20, cy + 60)],
                      fill='#5B8C2A')
        elif fruit == 'peach':
            d.ellipse([cx - 75, cy - 70, cx + 75, cy + 80], fill='#F2A33A')
            d.ellipse([cx - 40, cy - 50, cx + 20, cy + 40], fill='#F6C46A')
            d.ellipse([cx + 10, cy - 100, cx + 70, cy - 60], fill='#5B8C2A')
        elif fruit == 'fish':
            d.ellipse([cx - 90, cy - 40, cx + 60, cy + 40], fill='#9FB7C9')
            d.polygon([(cx + 50, cy), (cx + 100, cy - 45), (cx + 100, cy + 45)], fill='#9FB7C9')
        elif fruit == 'lemon':
            d.ellipse([cx - 80, cy - 60, cx + 80, cy + 60], fill='#F4D03F')
            d.ellipse([cx - 30, cy - 40, cx + 20, cy], fill='#FAE68A')
        elif fruit == 'cherry':
            for dx in (-40, 40):
                d.ellipse([cx + dx - 45, cy - 20, cx + dx + 45, cy + 70], fill='#8E2C48')
            d.line([cx - 40, cy - 20, cx, cy - 100, cx + 40, cy - 20], fill='#5B8C2A', width=10)
        elif fruit == 'drop':
            d.polygon([(cx, cy - 100), (cx - 60, cy + 10), (cx + 60, cy + 10)], fill=CYAN)
            d.ellipse([cx - 62, cy - 40, cx + 62, cy + 80], fill=CYAN)
        elif fruit == 'bolt':
            d.polygon([(cx + 20, cy - 110), (cx - 60, cy + 10), (cx - 5, cy + 10), (cx - 30, cy +
                       110), (cx + 60, cy - 20), (cx + 5, cy - 20)], fill=YELLOW)
        elif fruit == 'cow':
            d.rounded_rectangle([cx - 90, cy - 70, cx + 90, cy + 70], 30, fill='#FFFFFF')
            for _ in range(5):
                x, y = cx + rnd.randint(-70, 60), cy + rnd.randint(-50, 40)
                d.ellipse([x - 22, y - 16, x + 26, y + 20], fill='#262A33')
    ink = contrast_ink(bg)
    for k in range(2):
        x = W // 4 + k * W // 2 - W // 16
        text_fit(d, (x, int(S * 0.42)), brand, W // 2 - W // 6, 150, fill=ink)
        text_fit(d, (x, int(S * 0.62)), sub, W // 2 - W // 5, 80, weight=600, fill=ink)


def art_frozen(d, rnd, bg, brand, sub, motif):
    """Frozen pack (pizza, peas, ice cream, fries): icy gradient and frost flakes."""
    for y in range(0, S, 8):
        k = y / S
        d.rectangle([0, y, S, y + 8], fill=shade(bg, 1.0 + 0.35 * (1 - k)))
    cx, cy = S // 2, int(S * 0.6)
    if motif == 'pizza':
        d.ellipse([cx - 170, cy - 150, cx + 170, cy + 150], fill='#E8B34A')
        d.ellipse([cx - 150, cy - 130, cx + 150, cy + 130], fill='#C8402E')
        for _ in range(10):
            x, y = cx + rnd.randint(-110, 110), cy + rnd.randint(-100, 100)
            d.ellipse([x - 20, y - 20, x + 20, y + 20], fill=rnd.choice(['#8E2C48', '#F2EEE4',
                                                                          '#5B8C2A']))
    elif motif == 'peas':
        for _ in range(40):
            x, y = cx + rnd.randint(-180, 180), cy + rnd.randint(-130, 140)
            d.ellipse([x - 20, y - 20, x + 20, y + 20], fill=rnd.choice(['#5B8C2A', '#79A83A',
                                                                          '#4A7A22']))
    elif motif == 'icecream':
        d.polygon([(cx - 70, cy), (cx + 70, cy), (cx, cy + 190)], fill='#D9A35A')
        for k, c in enumerate(('#F2E2C4', '#E8739E', '#6B3A22')):
            x = cx - 60 + k * 60
            d.ellipse([x - 70, cy - 120 + (k % 2) * 30, x + 70, cy + 20 + (k % 2) * 30],
                      fill=c)
    elif motif == 'fries':
        for k in range(9):
            x = cx - 160 + k * 40
            d.rectangle([x, cy - 140 + (k % 3) * 25, x + 26, cy + 120], fill='#F2C230')
        d.rectangle([cx - 190, cy + 40, cx + 190, cy + 170], fill='#D7352B')
    for _ in range(14):
        x, y = rnd.randint(0, S), rnd.randint(0, S)
        for k in range(3):
            a = k * math.pi / 3
            d.line([x - 22 * math.cos(a), y - 22 * math.sin(a), x + 22 * math.cos(a),
                    y + 22 * math.sin(a)], fill='#FFFFFF', width=5)
    d.rectangle([0, 0, S, int(S * 0.23)], fill=shade(bg, 0.55))
    text_fit(d, (S // 2, int(S * 0.115)), brand, S - 40, 110, fill='#FFFFFF')
    text_fit(d, (S // 2, int(S * 0.31)), sub, S - 60, 60, weight=600, fill='#1E2A3A')


def art_carton(d, rnd, bg, brand, sub, motif):
    """Milk / juice carton face."""
    d.rectangle([0, 0, S, S], fill='#F4F2EC')
    d.rectangle([0, 0, S, int(S * 0.3)], fill=bg)
    d.ellipse([-S * 0.3, S * 0.62, S * 1.3, S * 1.4], fill=shade(bg, 1.15))
    if motif == 'cow':
        cx, cy, k = S * 0.5, S * 0.7, 0.72

        def e(x0, y0, x1, y1, **kw):
            d.ellipse([cx + x0 * k, cy + y0 * k, cx + x1 * k, cy + y1 * k], **kw)
        for sx in (-1, 1):                  # ears and horns
            e(sx * 150 - 50, -110, sx * 150 + 50, -60, fill='#F2EEE4', outline='#262A33',
              width=6)
            d.line([cx + sx * 70 * k, cy - 120 * k, cx + sx * 100 * k, cy - 170 * k],
                   fill='#C9B48A', width=16)
        e(-120, -140, 120, 90, fill='#FFFFFF', outline='#262A33', width=6)
        e(-100, -120, -20, -50, fill='#262A33')
        e(-95, 10, 95, 120, fill='#F0A6B0', outline='#262A33', width=6)
        for sx in (-1, 1):
            e(sx * 45 - 14, 50, sx * 45 + 14, 78, fill='#8E3A4A')
            e(sx * 50 - 14, -40, sx * 50 + 14, -12, fill='#262A33')
    else:
        d.ellipse([S * 0.32, S * 0.4, S * 0.68, S * 0.76], fill='#F2A33A')
    text_fit(d, (S // 2, int(S * 0.15)), brand, S - 40, 110, fill='#FFFFFF')
    text_fit(d, (S // 2, int(S * 0.36)), sub, S - 60, 56, weight=600, fill=shade(bg, 0.6))


def art_carton_box(d, rnd):
    """Generic shipping carton print: kraft with arrows and a product code."""
    d.rectangle([0, 0, S, S], fill='#C49A68')
    ink = '#5A4026'
    for k in range(2):
        x = S * 0.25 + k * S * 0.5
        d.polygon([(x, S * 0.18), (x - 40, S * 0.3), (x - 14, S * 0.3), (x - 14, S * 0.44),
                   (x + 14, S * 0.44), (x + 14, S * 0.3), (x + 40, S * 0.3)], fill=ink)
    text_fit(d, (S // 2, int(S * 0.55)), 'THIS SIDE UP', S - 80, 60, weight=700, fill=ink)
    text_fit(d, (S // 2, int(S * 0.72)), f'{rnd.randint(10, 99)} x {rnd.randint(200, 999)} g',
             S - 120, 54, weight=600, fill=ink)
    d.rectangle([S * 0.3, S * 0.82, S * 0.7, S * 0.9], outline=ink, width=6)
    for k in range(14):
        x = S * 0.32 + k * S * 0.026
        d.rectangle([x, S * 0.835, x + (6 if k % 3 else 12), S * 0.885], fill=ink)


# --- the packaging atlas -------------------------------------------------------------------------------
# (name, art function, background, brand, descriptor, motif). Invented brands only; a few nod to
# the game's cars (Volt, Sparky, Hornet, Zip, Torque, Phantom, Dumpster, Bolt).
PRODUCTS = [
    ('cereal_1', art_box, '#D7352B', 'KRUNCHLINGS', 'crispy corn flakes', 'bowl'),
    ('cereal_2', art_box, '#2F6FD6', 'VOLT FLAKES', 'charged with oats', 'bowl'),
    ('cereal_3', art_box, '#F2A33A', 'SUNNY LOOPS', 'fruity rings', 'rings'),
    ('cereal_4', art_box, '#7A4FC2', 'CHOCO COMETS', 'cocoa crunch', 'bowl'),
    ('cereal_5', art_box, '#3E9B4F', 'OAT HOLLOW', 'whole grain oats', 'sun'),
    ('cereal_6', art_box, '#C9A227', 'HORNET HONEY', 'honey loops', 'rings'),
    ('cereal_7', art_box, '#E8739E', 'BERRY BURST', 'strawberry puffs', 'rings'),
    ('cereal_8', art_box, '#2C3E66', 'MUESLI MILL', 'nuts & raisins', 'sun'),
    ('cracker_1', art_box, '#B5553C', 'CRUMBLETON', 'butter crackers', 'crackers'),
    ('cracker_2', art_box, '#1F8A8A', 'BISKIT BAY', 'sea salt crackers', 'crackers'),
    ('cookie_1', art_box, '#2C3E66', 'MOONCRUMB', 'choc chip cookies', 'cookie'),
    ('cookie_2', art_box, '#8E2C48', 'NANA JO\'S', 'double choc', 'cookie'),
    ('pasta_1', art_box, '#2F6FD6', 'SPIRALINI', 'durum wheat pasta', 'pasta'),
    ('pasta_2', art_box, '#3E9B4F', 'TORQUE TWISTS', 'fusilli 500 g', 'pasta'),
    ('tea_1', art_box, '#5B8C2A', 'LEAFWELL', 'green tea  40 bags', 'leaf'),
    ('tea_2', art_box, '#8E2C48', 'EARLY HILL', 'breakfast tea', 'leaf'),
    ('soap_1', art_box, '#1F8A8A', 'BUBBLO', 'laundry powder', 'bubbles'),
    ('soap_2', art_box, '#7A4FC2', 'SPARKLE SUDS', 'fresh lavender', 'bubbles'),
    ('soap_3', art_box, '#2F6FD6', 'TIDEWAY', 'ocean clean', 'bubbles'),
    ('soap_4', art_box, '#E8739E', 'PETAL SOFT', 'fabric softener', 'bubbles'),
    ('chips_1', art_box, '#D7352B', 'ZIP CRISPS', 'sea salt', 'chips'),
    ('chips_2', art_box, '#3E9B4F', 'CRUNCHWAVE', 'sour cream & onion', 'chips'),
    ('chips_3', art_box, '#2F6FD6', 'SALTY DOG', 'ready salted', 'chips'),
    ('chips_4', art_box, '#F2A33A', 'DUMPSTER DIPS', 'cheese puffs', 'chips'),
    ('can_tomato', art_label, '#D7352B', 'TOMATELLO', 'chopped tomatoes', 'tomato'),
    ('can_beans', art_label, '#2C3E66', 'BEANERY', 'baked beans', 'beans'),
    ('can_corn', art_label, '#3E9B4F', 'GOLDCOB', 'sweet corn', 'corn'),
    ('can_peach', art_label, '#F2A33A', 'PEACHY KEEN', 'peach halves', 'peach'),
    ('can_fish', art_label, '#1F8A8A', 'SEA SPARKY', 'tuna chunks', 'fish'),
    ('can_soup', art_label, '#8E2C48', 'SOUPLEY', 'cream of tomato', 'tomato'),
    ('soda_cola', art_label, '#B8322A', 'VOLT COLA', 'original fizz', 'bolt'),
    ('soda_lemon', art_label, '#5B8C2A', 'ZAP LEMON', 'sparkling', 'lemon'),
    ('soda_cherry', art_label, '#8E2C48', 'FIZZBANG', 'cherry soda', 'cherry'),
    ('water', art_label, '#2F6FD6', 'AQUA BROOK', 'still spring water', 'drop'),
    ('juice', art_label, '#F2A33A', 'SUNPRESS', 'orange juice', 'peach'),
    ('energy', art_label, '#262A33', 'RECHARGED', 'energy drink', 'bolt'),
    ('jar_1', art_label, '#C9A227', 'HIVE & CO', 'pure honey', 'lemon'),
    ('jar_2', art_label, '#8E2C48', 'BRAMBLE', 'cherry jam', 'cherry'),
    ('milk_1', art_carton, '#2F6FD6', 'MOO RIDGE', 'whole milk 1 L', 'cow'),
    ('milk_2', art_carton, '#3E9B4F', 'MOO RIDGE', 'semi-skimmed', 'cow'),
    ('milk_3', art_carton, '#D7352B', 'MOO RIDGE', 'skimmed', 'cow'),
    ('juice_carton', art_carton, '#F2A33A', 'SUNPRESS', 'orange, no bits', 'orange'),
    ('yogurt', art_carton, '#E8739E', 'CREAMLY', 'strawberry yogurt', 'cow'),
    ('cheese', art_carton, '#C9A227', 'HILLTOP', 'mature cheddar', 'cow'),
    ('frozen_pizza_1', art_frozen, '#2F6FD6', 'SNOWPEAK', 'stone-baked pizza', 'pizza'),
    ('frozen_pizza_2', art_frozen, '#2C3E66', 'PHANTOM PIZZA', 'pepperoni', 'pizza'),
    ('frozen_peas', art_frozen, '#1F8A8A', 'POLAR BITES', 'garden peas', 'peas'),
    ('frozen_ice_1', art_frozen, '#7A4FC2', 'GLACIER SCOOPS', 'vanilla & berry', 'icecream'),
    ('frozen_ice_2', art_frozen, '#2F6FD6', 'GLACIER SCOOPS', 'triple choc', 'icecream'),
    ('frozen_fries', art_frozen, '#1F8A8A', 'POLAR BITES', 'crinkle fries', 'fries'),
    ('frozen_veg', art_frozen, '#3E9B4F', 'FROSTFIELD', 'mixed vegetables', 'peas'),
    ('frozen_bolt', art_frozen, '#2C3E66', 'BOLT BURGERS', '8 quarter pounders', 'pizza'),
    ('carton_1', None, None, None, None, None),
    ('carton_2', None, None, None, None, None),
    ('carton_3', None, None, None, None, None),
    ('rice_1', art_box, '#E0E0D8', 'PADDY GOLD', 'long grain rice', 'sun'),
    ('flour_1', art_box, '#E0E0D8', 'MILLSTONE', 'plain flour 1 kg', 'leaf'),
    ('sugar_1', art_box, '#2F6FD6', 'SWEETDALE', 'granulated sugar', 'sun'),
    ('coffee_1', art_box, '#262A33', 'NIGHT SHIFT', 'dark roast coffee', 'sun'),
    ('tissue_1', art_box, '#7FD8F0', 'CLOUDSOFT', 'tissues 3 ply', 'bubbles'),
    ('nappy_1', art_box, '#E8739E', 'LITTLE ZIPS', 'nappies size 4', 'rings'),
    ('pet_1', art_box, '#F2A33A', 'WAGGLES', 'dog biscuits', 'cookie'),
    ('pet_2', art_box, '#7A4FC2', 'WHISKERLY', 'cat food', 'crackers'),
    ('baking_1', art_box, '#8E2C48', 'BAKEHOUSE', 'cake mix', 'cookie'),
]
assert len(PRODUCTS) <= GRID * GRID


def packaging():
    atlas = Image.new('RGB', (CELL * GRID, CELL * GRID), '#808080')
    cells = {}
    for i, (name, fn, bg, brand, sub, motif) in enumerate(PRODUCTS):
        rnd = random.Random(i * 7919 + 13)
        wide = fn is art_label
        img = Image.new('RGB', (int(S * LABEL_ASPECT) if wide else S, S), bg or '#C49A68')
        d = ImageDraw.Draw(img)
        if fn is None:
            art_carton_box(d, rnd)
        else:
            fn(d, rnd, bg, brand, sub, motif)
        img = img.resize((CELL, CELL), Image.LANCZOS)
        col, row = i % GRID, i // GRID
        atlas.paste(img, (col * CELL, row * CELL))
        # side colour: a small swatch inside the cell (for box sides, tops, lids)
        cells[name] = dict(rect=[col * CELL, row * CELL, CELL, CELL],
                           side=[col * CELL + 4, row * CELL + CELL // 2, 3, 3]
                           if fn is not art_label else [col * CELL + 4, row * CELL + 8, 3, 3])
    return atlas, cells


# --- signage sheet (1024²) ----------------------------------------------------------------------------

def signs():
    W = 1024
    sheet = Image.new('RGB', (W, W), '#E9E6DE')
    regions = {}

    def region(name, x, y, w, h, draw):
        img = Image.new('RGB', (w * 2, h * 2), '#FFFFFF')
        draw(ImageDraw.Draw(img), w * 2, h * 2)
        sheet.paste(img.resize((w, h), Image.LANCZOS), (x, y))
        regions[name] = [x, y, w, h]

    def exit_sign(d, w, h):
        d.rectangle([0, 0, w, h], fill='#1E8A4A')
        d.rectangle([10, 10, w - 10, h - 10], outline='#E8F6EC', width=8)
        text_fit(d, (w * 0.62, h * 0.52), 'EXIT', w * 0.55, int(h * 0.7), weight=800,
                 fill='#F2FFF6')
        cx, cy = w * 0.2, h * 0.5                   # running figure (generic pictogram)
        d.ellipse([cx - 16, cy - 64, cx + 16, cy - 32], fill='#F2FFF6')
        d.line([cx, cy - 30, cx - 10, cy + 20], fill='#F2FFF6', width=16)
        d.line([cx - 10, cy + 20, cx + 30, cy + 60], fill='#F2FFF6', width=14)
        d.line([cx - 10, cy + 20, cx - 40, cy + 56], fill='#F2FFF6', width=14)
        d.line([cx - 4, cy - 14, cx + 36, cy - 2], fill='#F2FFF6', width=12)
        d.line([cx - 4, cy - 14, cx - 40, cy + 4], fill='#F2FFF6', width=12)

    def wet_floor(d, w, h):
        d.rectangle([0, 0, w, h], fill='#F2C230')
        d.polygon([(w / 2, h * 0.08), (w * 0.12, h * 0.62), (w * 0.88, h * 0.62)],
                  fill='#1E2024')
        d.polygon([(w / 2, h * 0.16), (w * 0.2, h * 0.58), (w * 0.8, h * 0.58)], fill='#F2C230')
        cx, cy = w * 0.5, h * 0.42                  # slipping figure
        d.ellipse([cx - 14, cy - 70, cx + 14, cy - 42], fill='#1E2024')
        d.line([cx, cy - 40, cx + 16, cy + 6], fill='#1E2024', width=14)
        d.line([cx + 16, cy + 6, cx - 30, cy + 40], fill='#1E2024', width=12)
        d.line([cx + 16, cy + 6, cx + 50, cy + 30], fill='#1E2024', width=12)
        d.line([cx + 4, cy - 26, cx - 40, cy - 40], fill='#1E2024', width=10)
        d.line([cx + 4, cy - 26, cx + 46, cy - 50], fill='#1E2024', width=10)
        text_fit(d, (w / 2, h * 0.74), 'CAUTION', w * 0.86, int(h * 0.12), weight=800,
                 fill='#1E2024')
        text_fit(d, (w / 2, h * 0.88), 'WET FLOOR', w * 0.86, int(h * 0.1), weight=700,
                 fill='#1E2024')

    def closed(d, w, h):
        d.rectangle([0, 0, w, h], fill='#B8322A')
        text_fit(d, (w / 2, h / 2), 'LANE CLOSED', w * 0.9, int(h * 0.6), weight=800,
                 fill='#FFFFFF')

    def sale(d, w, h):
        d.rectangle([0, 0, w, h], fill=YELLOW)
        d.rectangle([0, 0, w, h * 0.12], fill=RED)
        d.rectangle([0, h * 0.88, w, h], fill=RED)
        text_fit(d, (w / 2, h * 0.42), 'SPECIAL', w * 0.86, int(h * 0.3), weight=800,
                 fill=RED)
        text_fit(d, (w / 2, h * 0.7), 'OFFER', w * 0.86, int(h * 0.26), weight=800, fill='#1E2024')

    def dept(label, bg):
        def f(d, w, h):
            d.rectangle([0, 0, w, h], fill=bg)
            d.rectangle([0, h * 0.84, w, h], fill=shade(bg, 0.7))
            text_fit(d, (w / 2, h * 0.44), label, w * 0.92, int(h * 0.62), weight=800,
                     fill='#FFFFFF')
        return f

    def aisle(n, lines):
        def f(d, w, h):
            d.rectangle([0, 0, w, h], fill='#2C3E66')
            d.rectangle([0, 0, w, h * 0.42], fill='#F4D03F')
            text_fit(d, (w / 2, h * 0.22), f'AISLE {n}', w * 0.9, int(h * 0.3), weight=800,
                     fill='#1E2024')
            for k, t in enumerate(lines):
                text_fit(d, (w / 2, h * (0.55 + 0.15 * k)), t, w * 0.9, int(h * 0.12),
                         weight=600, fill='#F6F3EA')
        return f

    def price_strip(d, w, h):
        d.rectangle([0, 0, w, h], fill='#F2F0EA')
        rnd = random.Random(5)
        x = 6
        while x < w - 60:
            tw = rnd.choice([88, 104, 120])
            col = YELLOW if rnd.random() < 0.7 else '#F6F3EA'
            d.rectangle([x, 6, x + tw, h - 6], fill=col, outline='#C8C2B0', width=2)
            price = f'{rnd.randint(0, 9)}.{rnd.choice(["49", "99", "29", "79", "19"])}'
            text_fit(d, (x + tw * 0.62, h / 2), price, tw * 0.6, int(h * 0.62), weight=800,
                     fill='#1E2024')
            d.rectangle([x + 8, h * 0.3, x + tw * 0.28, h * 0.7], fill=RED if col == YELLOW
                        else '#3E9B4F')
            x += tw + rnd.randint(30, 90)

    def lane_light(n):
        def f(d, w, h):
            d.rectangle([0, 0, w, h], fill='#F6F3EA')
            d.ellipse([w * 0.1, h * 0.1, w * 0.9, h * 0.9], fill='#2F6FD6')
            text_fit(d, (w / 2, h * 0.5), str(n), w * 0.6, int(h * 0.7), weight=800,
                     fill='#FFFFFF')
        return f

    region('exit', 0, 0, 256, 128, exit_sign)
    region('wet_floor', 256, 0, 192, 288, wet_floor)
    region('closed', 448, 0, 256, 64, closed)
    region('sale', 448, 64, 192, 192, sale)
    region('price_strip', 0, 288, 1024, 32, price_strip)
    region('carton_print', 704, 0, 192, 192, _carton)
    region('dept_frozen', 0, 320, 512, 96, dept('FROZEN FOODS', '#2F6FD6'))
    region('dept_dairy', 512, 320, 512, 96, dept('DAIRY', '#1F8A8A'))
    region('dept_produce', 0, 416, 512, 96, dept('FRESH PRODUCE', '#3E9B4F'))
    region('dept_deli', 512, 416, 512, 96, dept('DELI & MEAT', '#B8322A'))
    region('dept_checkout', 0, 512, 512, 96, dept('CHECKOUT', '#2C3E66'))
    region('dept_service', 512, 512, 512, 96, dept('CUSTOMER SERVICE', '#7A4FC2'))
    aisles = [['BREAKFAST', 'CEREAL', 'TEA & COFFEE'], ['BAKING', 'RICE & PASTA', 'SAUCES'],
              ['CANNED FOOD', 'SOUP', 'TINNED FISH'], ['SNACKS', 'CRISPS', 'BISCUITS'],
              ['DRINKS', 'WATER', 'JUICE'], ['HOUSEHOLD', 'LAUNDRY', 'PAPER'],
              ['PETS', 'BABY', 'HEALTH']]
    for k, lines in enumerate(aisles):
        region(f'aisle_{k + 1}', (k % 4) * 256, 608 + (k // 4) * 160, 256, 160,
               aisle(k + 1, lines))
    for n in range(1, 9):
        region(f'lane_{n}', 768 + ((n - 1) % 4) * 64, 768 + ((n - 1) // 4) * 64, 64, 64,
               lane_light(n))
    region('blank_white', 900, 0, 64, 64, lambda d, w, h: d.rectangle([0, 0, w, h],
                                                                       fill='#F4F2EC'))

    def sale_wide(d, w, h):
        d.rectangle([0, 0, w, h], fill=YELLOW)
        d.rectangle([0, 0, w, h * 0.14], fill=RED)
        d.rectangle([0, h * 0.86, w, h], fill=RED)
        text_fit(d, (w * 0.5, h * 0.52), 'SPECIAL OFFER', w * 0.9, int(h * 0.56), weight=800,
                 fill=RED)
    region('sale_wide', 0, 928, 512, 96, sale_wide)

    def frost(d, w, h):
        d.rectangle([0, 0, w, h], fill='#2C3E66')
        text_fit(d, (w * 0.5, h * 0.5), 'FROZEN FOODS', w * 0.6, int(h * 0.7), weight=800,
                 fill='#E8F6FF')
    region('frozen_header', 512, 928, 512, 48, frost)          # freezer header: 10.4 : 1

    def welcome(d, w, h):
        d.rectangle([0, 0, w, h], fill='#2C3E66')
        text_fit(d, (w * 0.5, h * 0.5), 'WELCOME', w * 0.8, int(h * 0.62), weight=800,
                 fill='#F6F3EA')
    region('welcome', 512, 976, 256, 48, welcome)

    def bag(d, w, h):
        d.rectangle([0, 0, w, h], fill='#B88D5C')
        d.ellipse([w * 0.3, h * 0.3, w * 0.7, h * 0.7], outline='#3E9B4F', width=10)
        text_fit(d, (w * 0.5, h * 0.5), 'FRESH', w * 0.36, int(h * 0.14), weight=800,
                 fill='#3E9B4F')
    region('paper_bag', 768, 976, 64, 48, bag)
    return sheet, regions


def _carton(d, w, h):
    d.rectangle([0, 0, w, h], fill='#C49A68')
    ink = '#5A4026'
    for k in range(2):
        x = w * 0.28 + k * w * 0.44
        d.polygon([(x, h * 0.14), (x - w * 0.09, h * 0.3), (x - w * 0.03, h * 0.3),
                   (x - w * 0.03, h * 0.46), (x + w * 0.03, h * 0.46), (x + w * 0.03, h * 0.3),
                   (x + w * 0.09, h * 0.3)], fill=ink)
    text_fit(d, (w / 2, h * 0.62), 'THIS SIDE UP', w * 0.86, int(h * 0.14), weight=700,
             fill=ink)
    text_fit(d, (w / 2, h * 0.8), 'FRAGILE', w * 0.6, int(h * 0.12), weight=700, fill=ink)


def main():
    os.makedirs(OUT, exist_ok=True)
    atlas, cells = packaging()
    sheet, regions = signs()
    # 256-colour PNGs: every glb embeds its own copy, and flat packaging art loses nothing
    for img, name in ((atlas, 'packaging.png'), (sheet, 'signs.png')):
        img.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(
            os.path.join(OUT, name), optimize=True)
    with open(os.path.join(OUT, 'atlas.json'), 'w', encoding='utf-8') as f:
        json.dump(dict(packaging=dict(size=CELL * GRID, cells=cells),
                       signs=dict(size=1024, regions=regions)), f, indent=1, sort_keys=True)
    print(f'wrote {OUT}/packaging.png ({len(cells)} products), signs.png ({len(regions)} '
          'regions), atlas.json')


if __name__ == '__main__':
    main()
