"""Supermarket prop catalog (Aisle Rush, art bible §3.5): dimensions, collider primitives, mass,
drivable surfaces and night lights of every `supermarket_*` prop kind. Pure Python (no bpy):
the Blender prop scripts and plan.py import the dimensions from here, and

    python3 tools/blender/props/supermarket/catalog.py [--assets DIR]

writes <assets>/data/props/supermarket.ron (default: the repo's assets).

Prop space is Bevy's: +Y up, front = -Z, origin at the ground-contact centre; units u
(1 u = 22.5 cm, real metres x 4.44). Colliders stay inside the visual mesh; drivable tops carry a
surface. Lights (rc_render, Night only): offset in prop space, linear colour, real lumens.
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
M = 4.44  # u per real metre

# --- shelving gondola (double-sided, 2 bays of 1.25 m) ------------------------------------
BAY = 5.55            # bay width along X
GONDOLA_BAYS = 2
GONDOLA_LEN = BAY * GONDOLA_BAYS
GONDOLA_DEPTH = 4.8   # base deck, both sides (1.08 m)
GONDOLA_UPPER = 3.4   # upper shelves depth, both sides
GONDOLA_H = 8.9       # 2.0 m
PLINTH_H = 1.2        # kick plate = the track barrier height
BASE_TOP = 2.6        # base shelf + its products (collider top of the base deck)
ENDCAP_DEPTH = 2.4    # endcap unit at a run end (faces the cross aisle)
GONDOLA_VARIANTS = ['a', 'b', 'c', 'd']

# --- refrigeration ----------------------------------------------------------------------------
FREEZER_W, FREEZER_D, FREEZER_H = 10.0, 4.2, 9.3    # 3-door upright glass-door freezer
COOLER_W, COOLER_D, COOLER_H = 11.1, 4.4, 9.3       # open multi-deck dairy cooler
COLD = (0.21, 0.69, 0.87)                           # freezer glow #7FD8F0 (linear)

# --- front end ----------------------------------------------------------------------------------
STAND_L, STAND_W, STAND_H = 15.0, 6.0, 4.0          # checkout stand (along Z), belt on -X side
BELT_W = 2.2                                        # conveyor belt width (on the -X edge)
LANES = [1, 2, 3, 4, 5, 6, 7, 8]                    # lane numbers (1-3 open, 4-8 gated)
GATE_W, GATE_H = 6.0, 2.7                           # closed-lane gate panel
DESK_L, DESK_W, DESK_H = 20.6, 6.0, 4.5             # customer-service desk
RAIL_L, RAIL_PANEL_H, RAIL_H = 8.0, 1.6, 4.4        # entrance railing segment

# --- cardboard-box jump (one prop: ramp, table, kicker, long down-ramp; front = takeoff) -----
JUMP_W = 9.8
JUMP = dict(ramp=12.0, table=6.0, kicker=4.0, down=18.0, top=2.2, lip=2.7)
JUMP_LEN = JUMP['ramp'] + JUMP['table'] + JUMP['kicker'] + JUMP['down']   # 40
WATER_L, WATER_W, WATER_H = 5.33, 3.55, 3.6        # pallet of shrink-wrapped water packs
WATER_RAMP = dict(width=WATER_W, rise=WATER_H, run=12.0)  # box ramp up onto the water pallets

# --- loose stock and dressing ----------------------------------------------------------------
CAN_R, CAN_H = 0.335, 0.8          # catering tin 15 x 18 cm (pyramid can)
CAN_LABELS = ['tomato', 'beans', 'peach']   # one product per pyramid
PALLET_L, PALLET_W, PALLET_H = 5.33, 3.55, 0.64    # EUR pallet 1.2 x 0.8 x 0.144 m
STACK_H = 5.6                       # pallet + shrink-wrapped cartons (1.26 m)
CRATE_L, CRATE_W, CRATE_H = 2.66, 1.78, 1.07        # produce crate 60 x 40 x 24 cm
TABLE_L, TABLE_W, TABLE_H = 9.0, 5.3, 3.8           # produce display table (top of the fruit)
BIN_L, BIN_W, BIN_H = 5.3, 4.4, 4.0                 # pallet bin of melons
RACK_L, RACK_D, RACK_H = 11.1, 4.0, 8.0             # wall produce rack
TROLLEY_L, TROLLEY_W, TROLLEY_H = 4.4, 2.7, 4.6     # shopping trolley
SIGN_W, SIGN_D, SIGN_H = 1.4, 1.0, 2.7              # wet-floor A-frame



def box(half, offset=(0, 0, 0), rot=None):
    return ('Box', half, offset, rot)


def wedge(half, offset=(0, 0, 0), rot=None):
    return ('Wedge', half, offset, rot)


def cyl(radius, half_height, offset=(0, 0, 0), rot=None):
    return ('Cylinder', radius, half_height, offset, rot)


def light(offset, lumens, rng, color=(1.0, 0.82, 0.6), spot=0.0, d=None, shadows=False):
    return dict(offset=offset, lumens=lumens, range=rng, color=color, spot=spot, dir=d,
                shadows=shadows)


def gondola_colliders(length):
    return [box((length / 2, BASE_TOP / 2, GONDOLA_DEPTH / 2), (0, BASE_TOP / 2, 0)),
            box((length / 2, (GONDOLA_H - BASE_TOP) / 2, GONDOLA_UPPER / 2),
                (0, (GONDOLA_H + BASE_TOP) / 2, 0))]


def jump_colliders():
    """Prop space: back (+Z) = ramp foot, front (-Z) = takeoff side; origin at the middle."""
    j, w = JUMP, JUMP_W / 2
    z0 = JUMP_LEN / 2                      # back end
    z1 = z0 - j['ramp']                    # ramp top
    z2 = z1 - j['table']                   # kicker foot
    z3 = z2 - j['kicker']                  # lip
    z4 = z3 - j['down']                    # front end
    return [
        wedge((w, j['top'] / 2, j['ramp'] / 2), (0, j['top'] / 2, (z0 + z1) / 2)),
        box((w, j['top'] / 2, (j['table'] + j['kicker']) / 2), (0, j['top'] / 2, (z1 + z3) / 2)),
        wedge((w, (j['lip'] - j['top']) / 2, j['kicker'] / 2),
              (0, (j['lip'] + j['top']) / 2, (z2 + z3) / 2)),
        # rot = (pitch X, yaw Y, roll Z) degrees (rc_track Primitive::pose): high end at +Z
        wedge((w, j['lip'] / 2, j['down'] / 2), (0, j['lip'] / 2, (z3 + z4) / 2), (0, 180, 0)),
    ]


def jump_profile():
    """[(z, height)] along the top centre line, back (+Z) to front (-Z)."""
    j = JUMP
    z0 = JUMP_LEN / 2
    z1 = z0 - j['ramp']
    z2 = z1 - j['table']
    z3 = z2 - j['kicker']
    z4 = z3 - j['down']
    return [(z0, 0.0), (z1, j['top']), (z2, j['top']), (z3, j['lip']), (z4, 0.0)]


def kinds():
    """kind -> dict(model, colliders, dynamic, mass, surface, lights, note)."""
    k = {}

    def add(name, note, colliders, model=None, dynamic=False, mass=None, surface=None,
            lights=()):
        k['supermarket_' + name] = dict(
            model=f'models/props/supermarket/{model or name}.glb', colliders=colliders,
            dynamic=dynamic, mass=mass, surface=surface, lights=list(lights), note=note)

    for v in GONDOLA_VARIANTS:
        add(f'gondola_{v}', f'Double-sided shelving gondola, 2 bays ({GONDOLA_LEN:g} x '
            f'{GONDOLA_DEPTH:g} x {GONDOLA_H:g} u) stocked both sides, variant {v}; length '
            'along X.', gondola_colliders(GONDOLA_LEN))
        add(f'endcap_{v}', f'Gondola endcap display ({GONDOLA_DEPTH:g} wide, {ENDCAP_DEPTH:g} '
            'deep), front -Z faces the cross aisle.',
            [box((GONDOLA_DEPTH / 2, BASE_TOP / 2, ENDCAP_DEPTH / 2), (0, BASE_TOP / 2, 0)),
             box((GONDOLA_DEPTH / 2, (GONDOLA_H - BASE_TOP) / 2, ENDCAP_DEPTH * 0.35),
                 (0, (GONDOLA_H + BASE_TOP) / 2, ENDCAP_DEPTH * 0.15))])
    fz = [box((FREEZER_W / 2, FREEZER_H / 2, FREEZER_D / 2), (0, FREEZER_H / 2, 0))]
    add('freezer', f'Upright 3-door glass freezer ({FREEZER_W:g} x {FREEZER_D:g} x '
        f'{FREEZER_H:g} u), doors toward -Z; cyan night glow.', fz,
        lights=[light((0, 5.0, -3.4), 160, 13, COLD)])
    add('freezer_dark', 'The upright freezer without its night light (every other unit).', fz,
        model='freezer')
    cl = [box((COOLER_W / 2, COOLER_H / 2, COOLER_D / 2), (0, COOLER_H / 2, 0))]
    add('cooler', f'Open multi-deck dairy cooler ({COOLER_W:g} x {COOLER_D:g} x {COOLER_H:g} u),'
        ' open front toward -Z; cool night glow.', cl,
        lights=[light((0, 6.0, -3.6), 180, 14, (0.6, 0.85, 1.0))])
    add('cooler_dark', 'The dairy cooler without its night light.', cl, model='cooler')
    for n in LANES:
        add(f'checkout_{n}', f'Checkout stand, lane {n} ({STAND_W:g} x {STAND_H:g} x '
            f'{STAND_L:g} u): conveyor belt on the -X edge, scanner, register, bagging shelf; '
            'the lane light glows when open.',
            [box((STAND_W / 2, STAND_H / 2, STAND_L / 2), (0, STAND_H / 2, 0))])
    add('lane_gate', f'Closed-lane gate: stainless frame, {GATE_W:g} x {GATE_H:g} u panel with '
        'a CLOSED sign.', [box((GATE_W / 2, GATE_H / 2, 0.15), (0, GATE_H / 2, 0))])
    add('service_desk', f'Customer-service desk ({DESK_L:g} x {DESK_W:g} x {DESK_H:g} u), '
        'front -Z.', [box((DESK_L / 2, DESK_H / 2, DESK_W / 2), (0, DESK_H / 2, 0))])
    add('railing', f'Entrance railing, {RAIL_L:g} u: posts, frosted kick panel '
        f'({RAIL_PANEL_H:g} u) and handrail ({RAIL_H:g} u).',
        [box((RAIL_L / 2, RAIL_PANEL_H / 2, 0.12), (0, RAIL_PANEL_H / 2, 0))])
    add('box_jump', f'Cardboard-box jump ({JUMP_W:g} x {JUMP_LEN:g} u): flattened-box ramp up '
        f'to a carton table ({JUMP["top"]:g}), kicker to the {JUMP["lip"]:g} u lip, long '
        'down-ramp; takeoff toward -Z.', jump_colliders(), surface='Cardboard')
    r = WATER_RAMP
    add('water_ramp', f'Narrow flattened-box ramp up onto the water pallets ({r["width"]:g} wide,'
        f' {r["rise"]:g} u rise over {r["run"]:g}), climbing toward -Z.',
        [wedge((r['width'] / 2, r['rise'] / 2, r['run'] / 2), (0, r['rise'] / 2, 0))],
        surface='Cardboard')
    add('water_pallet', f'Pallet of shrink-wrapped bottled-water packs ({WATER_L:g} x {WATER_W:g}'
        f' x {WATER_H:g} u), drivable top; length along X.',
        [box((WATER_L / 2, WATER_H / 2, WATER_W / 2), (0, WATER_H / 2, 0))],
        surface='Cardboard')
    for label in CAN_LABELS:
        add(f'can_{label}', f'Catering tin of {label} ({CAN_R * 2:g} x {CAN_H:g} u): stacked '
            'into knockable pyramids.', [cyl(CAN_R, CAN_H / 2, (0, CAN_H / 2, 0))],
            dynamic=True, mass=0.12)
    for v in ('a', 'b'):
        add(f'pallet_stack_{v}', f'Pallet of shrink-wrapped cartons ({PALLET_L:g} x '
            f'{PALLET_W:g} x {STACK_H:g} u), variant {v}; length along X.',
            [box((PALLET_L / 2, STACK_H / 2, PALLET_W / 2), (0, STACK_H / 2, 0))])
    add('pallet', f'Empty wooden EUR pallet ({PALLET_L:g} x {PALLET_W:g} x {PALLET_H:g} u).',
        [box((PALLET_L / 2, PALLET_H / 2, PALLET_W / 2), (0, PALLET_H / 2, 0))],
        surface='Wood')
    add('crate_stack', f'Two stacked produce crates of fruit ({CRATE_L:g} x {CRATE_W:g} x '
        f'{CRATE_H * 2:g} u).',
        [box((CRATE_L / 2, CRATE_H, CRATE_W / 2), (0, CRATE_H, 0))])
    add('produce_table', f'Produce display table with tilted crates of fruit ({TABLE_L:g} x '
        f'{TABLE_W:g} x {TABLE_H:g} u).',
        [box((TABLE_L / 2, TABLE_H / 2, TABLE_W / 2), (0, TABLE_H / 2, 0))])
    add('produce_bin', f'Pallet bin of melons ({BIN_L:g} x {BIN_W:g} x {BIN_H:g} u).',
        [box((BIN_L / 2, BIN_H / 2, BIN_W / 2), (0, BIN_H / 2, 0))])
    add('produce_rack', f'Wall produce rack with misting bar ({RACK_L:g} x {RACK_D:g} x '
        f'{RACK_H:g} u), front -Z.',
        [box((RACK_L / 2, RACK_H / 2, RACK_D / 2), (0, RACK_H / 2, 0))])
    add('trolley', f'Shopping trolley ({TROLLEY_W:g} x {TROLLEY_H:g} x {TROLLEY_L:g} u), '
        'handle at +Z.',
        [box((TROLLEY_W / 2, 0.45, TROLLEY_L / 2 - 0.2), (0, 0.45, 0)),
         box((TROLLEY_W / 2, 1.3, TROLLEY_L / 2), (0, 3.1, 0))])
    add('basket', 'Red plastic shopping basket (2.0 x 1.1 x 1.33 u).',
        [box((1.0, 0.55, 0.66), (0, 0.55, 0))], dynamic=True, mass=0.25)
    add('wet_floor_sign', f'Yellow wet-floor A-frame ({SIGN_W:g} x {SIGN_H:g} u).',
        [box((SIGN_W / 2, SIGN_H / 2, SIGN_D / 2), (0, SIGN_H / 2, 0))], dynamic=True,
        mass=0.2)
    add('mop_bucket', 'Mop bucket with wringer and mop (2.2 x 2.0 u, handle 5.5 u).',
        [box((1.1, 1.0, 0.9), (0, 1.0, 0))], dynamic=True, mass=1.2)
    for name, size, mass in (('box_small', (1.78, 1.33, 1.33), 0.3),
                             ('box_large', (2.66, 1.78, 1.78), 0.5)):
        add(name, f'Printed shipping carton ({size[0]:g} x {size[1]:g} x {size[2]:g} u).',
            [box((size[0] / 2, size[1] / 2, size[2] / 2), (0, size[1] / 2, 0))],
            dynamic=True, mass=mass)
    add('exit_sign', 'Illuminated EXIT sign with a twin-head emergency light, wall-mounted at '
        '12 u (origin on the floor at the wall, front -Z).',
        [box((1.3, 0.6, 0.3), (0, 12.6, -0.3))],
        lights=[light((0, 12.0, -1.0), 30, 6, (0.35, 1.0, 0.45)),
                light((-1.0, 13.6, -0.8), 450, 34, (1.0, 0.9, 0.75), 50, (-0.25, -0.7, -1)),
                light((1.0, 13.6, -0.8), 450, 34, (1.0, 0.9, 0.75), 50, (0.25, -0.7, -1))])
    return k


def _f(v):
    s = f'{v:.4f}'.rstrip('0').rstrip('.')
    return '0' if s in ('-0', '') else s


def _t(v):
    return '(' + ', '.join(_f(x) for x in v) + ')'


def _prim(p):
    kind = p[0]
    if kind in ('Box', 'Wedge'):
        _, half, off, rot = p
        s = f'{kind}(half: {_t(half)}, offset: {_t(off)}'
        return s + (f', rot: {_t(rot)})' if rot else ')')
    _, r, hh, off, rot = p   # Cylinder
    s = f'Cylinder(radius: {_f(r)}, half_height: {_f(hh)}, offset: {_t(off)}'
    return s + (f', rot: {_t(rot)})' if rot else ')')


def ron():
    out = ['// Prop catalog — Aisle Rush supermarket (art bible §3.5). Kinds are prefixed',
           '// `supermarket_`. GENERATED by tools/blender/props/supermarket/catalog.py: edit',
           '// that and re-run. Units: u (1 u = 22.5 cm, real size x 4.44); origin = ground-',
           '// contact centre, front -Z. `lights` (rc_render, Night only): offset (prop space),',
           '// linear colour, real lumens, range (u), spot_deg (0 = point light), dir.',
           '(', '    props: {']
    for name, d in sorted(kinds().items()):
        out.append(f'        // {d["note"]}')
        out.append(f'        "{name}": (')
        out.append(f'            model: "{d["model"]}",')
        if d['dynamic']:
            out.append('            dynamic: true,')
            out.append(f'            mass: {_f(d["mass"])},')
        if d['surface']:
            out.append(f'            surface: {d["surface"]},')
        prims = [_prim(p) for p in d['colliders']]
        if len(prims) == 1:
            out.append(f'            colliders: [{prims[0]}],')
        else:
            out.append('            colliders: [')
            out += [f'                {p},' for p in prims]
            out.append('            ],')
        if d['lights']:
            out.append('            lights: [')
            for li in d['lights']:
                s = (f'(offset: {_t(li["offset"])}, color: {_t(li["color"])}, '
                     f'lumens: {_f(li["lumens"])}, range: {_f(li["range"])}')
                if li['spot']:
                    s += f', spot_deg: {_f(li["spot"])}, dir: {_t(li["dir"])}'
                if li['shadows']:
                    s += ', shadows: true'
                out.append(f'                {s}),')
            out.append('            ],')
        out.append('        ),')
    out += ['    },', ')', '']
    return '\n'.join(out)


def footprint(kind):
    """Plan-view rectangles [(cx, cz, hx, hz, yaw_deg)] of a kind's colliders (prop space)."""
    out = []
    for p in kinds()[kind]['colliders']:
        if p[0] in ('Box', 'Wedge'):
            _, half, off, rot = p
            out.append((off[0], off[2], half[0], half[2], (rot or (0, 0, 0))[1]))
        else:   # Cylinder
            _, r, _, off, _ = p
            out.append((off[0], off[2], r, r, 0.0))
    return out


def main():
    a = sys.argv[1:]
    assets = a[a.index('--assets') + 1] if '--assets' in a else os.path.join(ROOT, 'assets')
    path = os.path.join(assets, 'data', 'props', 'supermarket.ron')
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(ron())
    print(f'wrote {path} ({len(kinds())} kinds)')


if __name__ == '__main__':
    # self-check: the jump profile matches its colliders' top heights
    prof = jump_profile()
    assert abs(prof[0][0] - JUMP_LEN / 2) < 1e-9 and abs(prof[-1][0] + JUMP_LEN / 2) < 1e-9
    assert all(math.isfinite(v) for z, h in prof for v in (z, h))
    main()
