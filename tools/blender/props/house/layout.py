#!/usr/bin/env python3
"""Sofa Circuit layout generator (plain Python, no Blender): writes
assets/maps/sofa_circuit/track.ron and assets/maps/sofa_circuit/decor.ron.

The centreline is built from straights and arcs (points every 20-30 deg), the chair slalom from
R = 8 arcs; every original prop placement lives here. Check the result with the plot tool in all
four variants:

    python3 tools/blender/props/house/layout.py [--out-dir DIR]
    cargo run -p rc_track --example plot -- sofa_circuit [--rev] [--mirror]

Coordinates: x east, z south (plots: -Z up), y up. Forward = the order of the points. Yaw:
positive turns a prop's front (-Z) towards -X (0 = facing north, 180 = south, 90 = west).

Since the track editor (docs/design/rc_editor.md) the track and decor files are editor-owned: this
script writes them only with --layout, which replaces every edit made in the editor.
"""
import math
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..', '..'))
RUG = 0.05          # rug surface height
TABLE = 1.6         # coffee-table top (rug 0.05 + table 1.55)
TX = 1.3            # coffee-table centre (x); ramps 8 u long each side
LAND = 2.3          # stair landing
W = 9               # standard racing width
STEP20 = math.radians(20)

pts = []  # control points: dict(pos, optional fields, c = comment)


def editor_owned(path):
    """Track and decor files are editor-owned (docs/design/rc_editor.md): this script writes them
    only with --layout, which replaces every edit made in the editor with its own layout."""
    if '--layout' in sys.argv:
        return False
    print(f'skipped {path}: editor-owned (cargo run -p rc_editor); --layout overwrites it')
    return True


def P(x, y, z, c=None, **kw):
    d = dict(pos=(x, y, z), **kw)
    if c:
        d['c'] = c
    pts.append(d)


def arc(cx, cz, r, a0, a1, y, steps, c=None, **kw):
    """Points on a circle around (cx, cz) from angle a0 (excluded) to a1 (included), degrees;
    0 = +x, 90 = +z (south)."""
    for i in range(1, steps + 1):
        a = math.radians(a0 + (a1 - a0) * i / steps)
        P(round(cx + r * math.cos(a), 3), y, round(cz + r * math.sin(a), 3),
          c if i == 1 else None, **(kw if i == 1 else {}))


def fmt(v):
    s = f'{v:.3f}'.rstrip('0').rstrip('.')
    return '0' if s == '-0' else s


# --- C (west part), heading west into the kitchen ---------------------------------------------
P(-3, 0, -1, 'start / finish: C straight, heading west into the kitchen',
  width=W, surface='Tile', wall_left=False, wall_right=False, offroad_margin=6)
P(-14.5, 0, -1, 'island-to-bench gap: barrier on the right', wall_right=True)
P(-24, 0, -1, None, wall_right=False)
P(-42 + 8 * STEP20, 0, -1)
P(-42, 0, -1, 'U CD: right hairpin around the kitchen island, r = 8')
arc(-42, -9, 8, 90, 270, 0, 9)
# --- D, heading east along the counters, then the chair slalom in the family corner -----------
P(-42 + 8 * STEP20, 0, -17, 'D straight (east)')
P(-31, 0, -17)
P(-24, 0, -17, 'island-to-table gap: barrier on the right', wall_right=True)
P(-16.2, 0, -17, None, wall_right=False)
P(-14, 0, -17, 'chair slalom: narrows (chokepoint), R = 8 weave around three chairs', width=6)
x, z, h = -12.0, -17.0, 0.0  # heading in degrees, + = towards +z (south)
P(x, 0, z)
for h1 in (30, -50, 50, -30, 0):  # inflection headings of the constant-radius weave
    n = max(2, round(abs(h1 - h) / 16))
    d = 1 if h1 > h else -1
    for k in range(1, n + 1):
        a0, a1 = math.radians(h + (h1 - h) * (k - 1) / n), math.radians(h + (h1 - h) * k / n)
        x += 8 * d * (math.sin(a1) - math.sin(a0))  # arc of radius 8, d = turn direction
        z += 8 * d * (math.cos(a0) - math.cos(a1))
        P(round(x, 3), 0, round(z, 3))
    h = h1
P(32, 0, -17, None, width=W)
P(34.86, 0, -17)
arc(38, -9, 8, 270, 360, 0, 4, 'NE turn, r = 8')
# --- return, heading south under the sofa into the living room --------------------------------
P(46, 0, -5.86, 'return straight (south): under the sofa')
P(46, 0, 2)
P(46, 0, 8.1, 'sofa tunnel')
P(46, RUG, 15.86)
arc(38, 19, 8, 0, 90, RUG, 4, 'SE turn on the rug in front of the TV, r = 8')
# --- A, heading west along the patio windows: the coffee-table jump ---------------------------
P(34.86, RUG, 27, 'A straight (west)')
P(29, 0, 27)
P(21, 0, 27)
P(15.3, RUG, 27, 'narrows to the coffee table (chokepoint)', width=6)
P(TX + 10.775, RUG, 27, 'foot of the east book ramp (Fwd prop)')
P(TX + 2.775, TABLE, 27, 'table east edge: forward lip, reverse landing')
P(TX - 2.775, TABLE, 27, 'table west edge: reverse lip, forward landing')
P(TX - 10.775, RUG, 27, 'foot of the west book ramp (Rev prop)')
P(-15.3, 0, 27, None, width=6)
P(-24, 0, 27, 'through the hall, past the stairs', width=W)
P(-34, 0, 27, 'plank pocket west of the stairs: barrier on the right', wall_right=True)
P(-42 + 7 * STEP20, 0, 27)
P(-42, 0, 27, 'U AB: right hairpin around the foot of the stairs, r = 7', wall_right=False)
arc(-42, 20, 7, 90, 270, 0, 9)
# --- B, heading east past the stairs into the living room -------------------------------------
P(-42 + 7 * STEP20, 0, 13, 'B straight (east): plank pocket barrier on the right', wall_right=True)
P(-34, 0, 13, 'stairs from here on', wall_right=False)
P(-24, 0, 13)
P(-10, 0, 13)
P(4, 0, 13)
P(22 - 7 * STEP20, 0, 13)
P(22, 0, 13, 'U BC: left hairpin into the family area, r = 7')
arc(22, 6, 7, 90, -90, 0, 9)
P(22 - 7 * STEP20, 0, -1, 'C straight (west)')
P(8, 0, -1)


def find(x, z):
    for i, p in enumerate(pts):
        if abs(p['pos'][0] - x) < 1e-3 and abs(p['pos'][2] - z) < 1e-3:
            return i
    raise KeyError((x, z))


def ron_points():
    out = []
    for i, p in enumerate(pts):
        x, y, z = p['pos']
        fields = [f'pos: ({fmt(x)}, {fmt(y)}, {fmt(z)})']
        for k in ('width', 'surface', 'wall_left', 'wall_right', 'offroad_margin', 'bank_deg'):
            if k in p:
                v = p[k]
                fields.append(f'{k}: ' + (str(v).lower() if isinstance(v, bool)
                                          else fmt(v) if isinstance(v, (int, float)) else v))
        line = f'        ({", ".join(fields)}), // {i}'
        if 'c' in p:
            line += f': {p["c"]}'
        out.append(line)
    return '\n'.join(out)


def props():
    out = []

    def add(kind, x, y, z, yaw=0, dir=None, c=None):
        s = f'        (kind: "{kind}", pos: ({fmt(x)}, {fmt(y)}, {fmt(z)})'
        if yaw:
            s += f', yaw: {fmt(yaw)}'
        if dir:
            s += f', dir: {dir}'
        out.append(s + '),' + (f' // {c}' if c else ''))

    out.append('        // Architecture: the spine wall between the front and back halves, the hall wall.')
    add('house_wall_spine', -18, 0, 5)
    add('house_wall_hall', -10, 0, 21, -90)
    out.append('        // Living room: the jump between the armchairs, the divider bookcases, sofa, TV.')
    add('house_coffee_table', TX, RUG, 27)
    add('house_book_ramp', TX + 6.775, RUG, 27, 90, 'Fwd')
    add('house_book_ramp', TX - 6.775, RUG, 27, -90, 'Rev')
    add('house_armchair', TX, 0, 21.0, 180)
    add('house_armchair', TX, 0, 33.6)
    for i, bx in enumerate((-5.9, 1.3, 8.5, 15.7, 22.9, 30.1)):
        add('house_bookcase_b' if i % 2 else 'house_bookcase', bx, 0, 18.4, 180)
    add('house_sofa', 46, 0, 8.1, 180)
    add('house_sideboard', 44, 0, 35.9)
    add('house_tv', 44, 2.44, 35.9)
    for i, bz in enumerate((-1.5, 5.7, 12.9)):
        add('house_bookcase' if i % 2 else 'house_bookcase_b', 37.5, 0, bz, 90)
    add('house_plant_pot', 35.2, 0, 17.3, c='closes the corner between the divider rows')
    add('house_floor_lamp', 36.4, 0, 20.6)
    out.append('        // Hall: the staircase, the plank onto its landing, the closet behind its top.')
    add('house_staircase', -23.7, 0, 20, -90)
    add('house_plank', -36.865, 0, 20, -90)
    add('house_cupboard', -11.85, 0, 20, 180, c='closes the gap between the stairs and the hall wall')
    out.append('        // Kitchen-diner: island, table, bench, chairs (slalom), counters, fridge, divider.')
    add('house_kitchen_island', -28.25, 0, -9)
    add('house_dining_table', -10.25, 0, -9)
    add('house_bench', -10.25, 0, -6.6, 180)
    add('house_dining_chair', -10.25, 0, -11.4, 180)
    add('house_dining_chair', -17.4, 0, -9, -90)
    add('house_dining_chair', -3.1, 0, -9, 90)
    add('house_dining_chair', -3.7, 0, -19.3, 20, c='slalom')
    add('house_dining_chair', 8.3, 0, -16.2, 195, c='slalom')
    add('house_dining_chair', 20.2, 0, -19.3, -15, c='slalom')
    add('house_plant_pot', -1, 0, -9)
    for i, cx in enumerate((-47.65, -36.95, -26.25, -15.55, -4.85)):
        add('house_counter_hob' if i == 2 else 'house_counter', cx, 0, -25.65, 180)
    add('house_fridge', -55, 0, -25.55, 180)
    add('house_sideboard', 4.45, 0, -9)
    for i, bx in enumerate((12.55, 19.75, 26.95, 34.15)):
        add('house_bookcase_b' if i % 2 else 'house_bookcase', bx, 0, -9)
    add('house_plant_pot', 36.2, 0, -6.6)
    out.append('        // Hall dressing: shoe rack and shoes by the front door, console, laundry, plants.')
    add('house_shoe_rack', -55.6, 0, 9.5, -90)
    add('house_shoe', -53.8, 0, 12, 30)
    add('house_shoe', -53.1, 0, 12.9, 75)
    add('house_console', -40, 0, 36.25, 180)
    add('house_laundry_basket', -21, 0, 34.4, 8)
    add('house_plant_pot', -55, 0, 35)
    add('house_plant_pot', -55, 0, 6.6)
    out.append('        // Living room dressing: lamps, plants, cushion and slippers by the sofa, the remote,')
    out.append('        // magazines and a mug by the armchairs, a pile of books, toys and a cable on B.')
    add('house_floor_lamp', -6.2, 0, 33.4)
    add('house_floor_lamp', 53.6, 0, 9.5)
    add('house_plant_pot', 54, 0, 35)
    add('house_plant_pot', 54, 0, 2)
    add('house_cushion', 44.5, RUG, 13.4, 25)
    add('house_slipper', 49.5, RUG, 12.6, 15)
    add('house_slipper', 50.3, RUG, 13.3, -10)
    add('house_remote', 39.5, RUG, 23.5, 40)
    add('house_magazines', TX + 3.2, RUG, 31.0, 18)
    add('house_mug', TX - 2.0, RUG, 31.5)
    add('house_book_large', -3.5, RUG, 23.0, 30)
    add('house_book_medium', -3.4, RUG + 0.13, 23.1, 18)
    add('house_book_small', -3.3, RUG + 0.29, 23.0, 40)
    add('house_toy_truck', 14, 0, 9.6, 60)
    add('house_book_medium', -2, 0, 16.2, 15)
    add('house_book_small', -1.3, 0, 15.5, -20)
    add('house_cable', 6, 0, 13, 90)
    out.append('        // Kitchen-diner dressing: dog bowl, pendants, plants, the kids\' corner by the north wall.')
    add('house_bowl', -51.5, 0, -21.5)
    for px in (-31, -25.5, -10.25):
        add('house_pendant', px, 0, -9)
    add('house_plant_pot', -55, 0, 3)
    add('house_plant_pot', 2.6, 0, -24.8)
    add('house_plant_pot', 53.5, 0, -23)
    add('house_cushion', 17, 0, -24, 10)
    add('house_toy_truck', 25, 0, -23.2, -30)
    add('house_book_large', 11, 0, -25, 5)
    return '\n'.join(out)


# (model, x, y, z, yaw, scale): physics-free dressing for rc_render
DECOR = [
    ('front_door', -56.98, 0, 20, -90, 1.0), ('doormat', -54.6, 0, 20, -90, 1.0),
    ('rug_fringe', -9.5, 0, 29, 90, 0.886), ('rug_fringe', 17, 0, 29, -90, 0.886),
    ('rug_fringe', 36, 0, 12, 0, 0.857), ('rug_fringe', 48, 0, 12, 0, 0.857),
    ('rug_fringe', 36, 0, 32, 180, 0.857), ('rug_fringe', 48, 0, 32, 180, 0.857),
]

HEADER = """// Sofa Circuit — family house ground floor (GDD §7.1, art bible §3.3). ~415 u, 5 laps, indoor ★.
// GENERATED by tools/blender/props/house/layout.py: edit that and re-run it. Format and
// conventions: docs/design/rc_track.md; notes: docs/design/env-house.md. Plot it:
//   cargo run -p rc_track --example plot -- sofa_circuit [--rev] [--mirror]
// Invisible ribbon over oak / tile / rug ground polygons; the furniture is the track. Forward:
// start in the kitchen-diner (C, west) -> hairpin round the island (tile) -> D east along the
// counters -> chair slalom (6 u, R = 8 weave) -> NE turn -> under the sofa -> SE turn on the rug
// -> A west: up the book ramp, over the coffee table between the armchairs, land in the hall ->
// hairpin round the foot of the stairs (plank shortcut over the landing, Golden Battery) -> B
// east behind the bookcases -> hairpin round the end of the spine wall -> C.
"""


def track_ron():
    east, west = find(TX + 2.775, 27), find(TX - 2.775, 27)
    return HEADER + f"""(
    style: Invisible,
    start: 0,
    kill_height: -3,
    wall_height: 1.2,
    bounds: (min: (-57, -27), max: (56, 37)),
    points: [
{ron_points()}
    ],
    gaps: [
        // Over the coffee table: forward up the east book ramp (a Fwd prop), reverse up the west
        // one (a Rev prop); fast cars fly well past the far edge onto the floor.
        (from: {east}, to: {west}, dir: Both),
    ],
    ground: [
        // Oak floor of the hall and the living room (front half).
        (surface: Wood, height: 0, points: [(-57, 5), (56, 5), (56, 37), (-57, 37)]),
        // Oak floor of the family corner of the kitchen-diner.
        (surface: Wood, height: 0, points: [(0, -27), (56, -27), (56, 5), (0, 5)]),
        // Kitchen tiles (and under the dining table).
        (surface: Tile, height: 0, points: [(-57, -27), (0, -27), (0, 5), (-57, 5)]),
        // Reading-corner rug under the coffee table and the armchairs.
        (surface: Carpet, height: 0.05, points: [(-9.5, 22.8), (17, 22.8), (17, 35.2), (-9.5, 35.2)]),
        // Rug in front of the sofa.
        (surface: Carpet, height: 0.05, points: [(30, 12), (54, 12), (54, 32), (30, 32)]),
    ],
    item_rows: [
        (pos: (-26, 0, -1)), // C, kitchen
        (pos: (-14, 0, -17)), // D, before the slalom
        (pos: (46, 0, -2)), // return, before the sofa
        (pos: (26, 0, 27)), // A, before the jump approach
        (pos: (6, 0, 13)), // B, living room
    ],
    shortcuts: [
        // Up the plank over the bottom steps, across the stair landing (Golden Battery), drop
        // onto B.
        (path: [(-49, 0, 20), (-43, 0, 19), (-31.6, {LAND}, 19), (-28, {LAND}, 18), (-24, 0, 14)], width: 3.5, dir: Fwd),
        // Reverse: the same plank and landing, dropping onto A.
        (path: [(-24, 0, 26), (-28, {LAND}, 22), (-31.6, {LAND}, 21), (-43, 0, 21), (-49, 0, 20)], width: 3.5, dir: Rev),
    ],
    golden_battery: (pos: (-29.1, {fmt(LAND + 1.0)}, 20)),
    props: [
{props()}
    ],
)
"""


def decor_ron():
    lines = ['// Sofa Circuit decor: physics-free dressing (rc_render, crates/rc_render/src/props/decor.rs).',
             '// GENERATED by tools/blender/props/house/layout.py. Cars never touch these: the front',
             '// door on the hall wall, the doormat, the rug fringes.',
             '(decor: [']
    for m, dx, dy, dz, yaw, sc in DECOR:
        lines.append(f'    (model: "models/props/house/{m}.glb", pos: ({fmt(dx)}, {fmt(dy)}, '
                     f'{fmt(dz)}), yaw: {fmt(yaw)}, scale: {fmt(sc)}),')
    return '\n'.join(lines + ['])', ''])


def main():
    args = sys.argv[1:]
    out = args[args.index('--out-dir') + 1] if '--out-dir' in args else os.path.join(ROOT, 'assets')
    for rel, text in (('maps/sofa_circuit/track.ron', track_ron()),
                      ('maps/sofa_circuit/decor.ron', decor_ron())):
        path = os.path.join(out, rel)
        if editor_owned(path):
            continue
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w', encoding='utf-8') as f:
            f.write(text)
        print(f'wrote {path}')
    print(f'{len(pts)} control points')


if __name__ == '__main__':
    main()
