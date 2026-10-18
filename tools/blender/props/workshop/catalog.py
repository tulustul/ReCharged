"""Workshop prop catalog: dimensions and collider primitives of every `workshop_*` prop kind,
shared by the Blender prop scripts (pure Python, no bpy), the layout generator (plan.py) and
written to assets/data/props/workshop.ron:

    python3 tools/blender/props/workshop/catalog.py

Prop space is Bevy's: +Y up, front = -Z, origin at the ground-contact centre; units u
(1 u = 22.5 cm, real metres x 4.44). Blender scripts build with Z up and front +Y, so a Bevy
offset (x, y, z) is Blender (x, -z, y). Colliders stay inside the visual mesh; drivable tops
carry a surface.
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
OUT = os.path.join(ROOT, 'assets', 'data', 'props', 'workshop.ron')
M = 4.44  # u per real metre

# --- workbench island and its plywood ramps (the M lane jump) ---------------------------------
# Two steel-frame benches back to back under one butcher-block top (5.4 x 1.8 m, 0.9 m high);
# a plywood kicker screwed on each end; ramps of two plywood sheets end to end (2 x 2.44 m).
BENCH = dict(length=24.0, width=8.0, top=4.0, top_thick=0.24, kicker_rise=0.5,
             kicker_run=3.56, ramp_run=21.7, ramp_width=8.0)
LAND_D = 14.0  # landing control point: this far down the far ramp from the lip (plan)

# ground layers above the concrete floor (y = 0): rubber mats (1 cm), oil and water films
MAT_H, OIL_H, WATER_H = 0.045, 0.02, 0.022

# --- two-post lift with a raised car ------------------------------------------------------------
# Columns 0.35 x 0.45 m, 3.7 m tall, 2.85 m clear between them; the car's mid-wheelbase sits on
# the column line. Car A (hatchback) is raised high for underbody work, car B (saloon) at
# wheel-work height. Car dims at real size x 4.44: length, width, sill height when lifted.
LIFT = dict(col_x=7.1, col_w=1.55, col_d=2.0, col_h=16.4, base_w=2.4, base_d=2.6,
            base_h=0.09)
CARS = {
    'a': dict(kind='hatch', length=18.9, width=7.95, sill=6.9, paint='#4E6A82'),
    'b': dict(kind='saloon', length=20.5, width=8.0, sill=4.3, paint='#7E2F2B'),
}

# --- tyres: 205/55 R16 car tyre (0.632 m x 0.205 m) and its steel wheel -------------------------
TYRE = dict(r=0.632 * M / 2, w=0.205 * M)          # r 1.403, w 0.910
STACKS = {2: 'workshop_tyre_stack2', 4: 'workshop_tyre_stack4'}

# --- paint booth (drive-through, 8 x 4 x 3 m) and wash bay (gantry + tiled splash walls) ------
BOOTH = dict(length=36.0, width=18.0, height=13.0, wall=0.7, door_w=10.6, door_h=10.0)
BOOTH_STAND_X, BOOTH_STANDS_Z = -6.9, (-8.0, 6.5)   # paint stands (prop space, Bevy)
WASH = dict(length=26.0, inner=11.6, wall_h=2.6, wall_t=0.5, post_x=7.4, post=0.7,
            beam_y=9.0)

# --- red roll cab + top chest -----------------------------------------------------------------
CHEST = dict(w=6.2, d=2.9, h=6.8)


# Collider `rot` is degrees (pitch about X, yaw about Y, roll about Z), applied yaw -> pitch ->
# roll: a yaw is (0, angle, 0).
def box(half, offset=(0, 0, 0), rot=None):
    return ('Box', half, offset, rot)


def wedge(half, offset=(0, 0, 0), rot=None):
    return ('Wedge', half, offset, rot)


def cyl(radius, half_height, offset=(0, 0, 0), rot=None):
    return ('Cylinder', radius, half_height, offset, rot)


def ball(radius, offset=(0, 0, 0)):
    return ('Ball', radius, offset)


def props():
    """kind -> dict(model, colliders, dynamic, mass, surface, lights, note)."""
    P = {}

    def add(kind, note, colliders, model=None, **kw):
        P[kind] = dict(model=f'models/props/workshop/{model or kind[len("workshop_"):]}.glb',
                       colliders=colliders, note=note, **kw)

    b = BENCH
    hl, hw = b['length'] / 2, b['width'] / 2
    kr, kz = b['kicker_rise'], b['kicker_run']
    add('workshop_workbench', 'Workbench island (hero): two steel benches back to back under '
        'one butcher-block top, 24 x 8 u, top at 4.0 u (0.9 m); a plywood kicker (0.5 u over '
        '3.56 u, 8 deg) on each end rises toward the edge; open underneath. Long axis along z.',
        [box((hw, b['top_thick'] / 2, hl), (0, b['top'] - b['top_thick'] / 2, 0)),
         wedge((hw, kr / 2, kz / 2), (0, b['top'] + kr / 2, -hl + kz / 2)),
         wedge((hw, kr / 2, kz / 2), (0, b['top'] + kr / 2, hl - kz / 2), (0, 180, 0))]
        + [box((0.2, (b['top'] - b['top_thick']) / 2, 0.2),
               (sx * (hw - 0.35), (b['top'] - b['top_thick']) / 2, sz * (hl - 0.35)))
           for sx in (-1, 1) for sz in (-1, 0, 1)],
        surface='Wood')
    rise = b['top'] + kr
    add('workshop_ramp', 'Plywood ramp: two 2.44 m sheets end to end on timber bearers, 8 u '
        'wide, rising 4.5 u over 21.7 u (11.7 deg) toward its front (-Z), where it rests on the '
        'workbench edge.',
        [wedge((b['ramp_width'] / 2, rise / 2, b['ramp_run'] / 2), (0, rise / 2, 0))],
        surface='Wood')
    L = LIFT
    for k, car in CARS.items():
        name = 'hatchback raised high' if k == 'a' else 'saloon at wheel-work height'
        add(f'workshop_lift_{k}', f'Two-post lift (hero) with a generic {name}: blue columns '
            f'at x = +-{L["col_x"]} u (12.65 u clear), 16.4 u tall with an overhead beam; car '
            f'{car["length"]} u long, sills at {car["sill"]} u; the car faces -Z.',
            [box((L['col_w'] / 2, L['col_h'] / 2, L['col_d'] / 2),
                 (sx * L['col_x'], L['col_h'] / 2, 0)) for sx in (-1, 1)]
            + [box((L['base_w'] / 2, L['base_h'] / 2, L['base_d'] / 2),
                   (sx * L['col_x'], L['base_h'] / 2, 0)) for sx in (-1, 1)]
            + [box((car['width'] / 2 - 0.6, 1.0, car['length'] / 2 - 2.0),
                   (0, car['sill'] + 1.4, 0))])

    t = TYRE
    for n, kind in STACKS.items():
        add(kind, f'Static stack of {n} used car tyres lying flat (205/55 R16, 0.63 m): '
            f'{n * t["w"]:.2f} u tall; the chicane walls and tyre walls.',
            [cyl(t['r'] - 0.06, n * t['w'] / 2, (0, n * t['w'] / 2, 0))])
    add('workshop_tyre', 'A loose used tyre lying flat (knockable).',
        [cyl(t['r'] - 0.06, t['w'] / 2 - 0.02, (0, t['w'] / 2, 0))], dynamic=True, mass=0.9)
    add('workshop_wheel', 'Spare wheel: tyre on a black steel rim, lying flat (knockable).',
        [cyl(t['r'] - 0.06, t['w'] / 2 - 0.02, (0, t['w'] / 2, 0))], dynamic=True, mass=1.2)

    bo = BOOTH
    hl, hw, h, wt = bo['length'] / 2, bo['width'] / 2, bo['height'], bo['wall']
    side = (hw - bo['door_w'] / 2) / 2
    add('workshop_paint_booth', 'Drive-through spray booth (hero), 36 x 18 x 13 u (8 x 4 x 3 m '
        'inside): white panel walls, angled light boxes along the roof edges, open bi-fold '
        f'doors at both ends ({bo["door_w"]} u clear), filter plenum and exhaust duct on top, '
        'two paint stands with primered panels by the left wall; long axis along z.',
        [box((wt / 2, h / 2, hl), (sx * (hw - wt / 2), h / 2, 0)) for sx in (-1, 1)]
        + [box((side, h / 2, wt / 2), (sx * (hw - side), h / 2, sz * (hl - wt / 2)))
           for sx in (-1, 1) for sz in (-1, 1)]
        + [box((0.9, 1.4, 1.5), (BOOTH_STAND_X, 1.4, z)) for z in BOOTH_STANDS_Z],
        lights=[dict(offset=(sx * (hw - 1.6), h - 1.0, z), color=(0.95, 0.97, 1.0),
                     lumens=1800, range=22, spot_deg=70, dir=(-sx * 0.35, -1, 0))
                for sx in (-1, 1) for z in (-9.0, 9.0)])
    wa = WASH
    hl, hi = wa['length'] / 2, wa['inner'] / 2
    add('workshop_wash_bay', 'Wash bay (hero): tiled splash walls on both sides (11.6 u '
        'apart, 2.6 u tall), a blue gantry with spray bars over the middle, pressure washer '
        'and hose reel on the walls; long axis along z.',
        [box((wa['wall_t'] / 2, wa['wall_h'] / 2, hl),
             (sx * (hi + wa['wall_t'] / 2), wa['wall_h'] / 2, 0)) for sx in (-1, 1)]
        + [box((wa['post'] / 2, wa['beam_y'] / 2, wa['post'] / 2),
               (sx * wa['post_x'], wa['beam_y'] / 2, 0)) for sx in (-1, 1)])
    ch = CHEST
    add('workshop_tool_chest', 'Red roll cab with a top chest (hero), 1.4 x 0.65 x 1.5 m: '
        'drawers with aluminium pulls, side handle, casters; front = drawer fronts.',
        [box((ch['w'] / 2 - 0.1, ch['h'] / 2 - 0.15, ch['d'] / 2 - 0.1),
             (0, ch['h'] / 2 - 0.05, 0))])

    # --- small kit (knockables are light: cars barely notice them) -----------------------------
    add('workshop_cone', 'Orange traffic cone, 0.5 m, with a reflective band (knockable).',
        [box((0.78, 0.05, 0.78), (0, 0.05, 0)), cyl(0.34, 0.5, (0, 0.6, 0)),
         cyl(0.17, 0.35, (0, 1.45, 0))], dynamic=True, mass=0.08)
    add('workshop_bucket', 'Grey 12 l bucket with a wire handle (knockable).',
        [cyl(0.58, 0.62, (0, 0.62, 0))], dynamic=True, mass=0.12)
    add('workshop_oil_can', '5 l engine-oil jug, translucent amber plastic (knockable).',
        [box((0.44, 0.62, 0.25), (0, 0.62, 0))], dynamic=True, mass=0.12)
    add('workshop_jerry_can', 'Red steel 20 l jerry can (knockable).',
        [box((0.7, 1.0, 0.34), (0, 1.0, 0))], dynamic=True, mass=0.3)
    add('workshop_toolbox', 'Red cantilever toolbox, 0.5 m (knockable).',
        [box((1.05, 0.5, 0.48), (0, 0.5, 0))], dynamic=True, mass=0.3)
    add('workshop_battery', 'Car battery, black case, red and black terminal caps '
        '(knockable).', [box((0.6, 0.4, 0.37), (0, 0.4, 0))], dynamic=True, mass=0.45)
    add('workshop_drain_pan', 'Black 15 l oil drain pan under a lifted car (knockable).',
        [cyl(1.2, 0.22, (0, 0.22, 0))], dynamic=True, mass=0.25)
    add('workshop_creeper', 'Mechanic\'s creeper board on six casters, 1 m long along z '
        '(knockable: it rolls).', [box((0.95, 0.2, 2.15), (0, 0.24, 0))], dynamic=True,
        mass=0.5)
    add('workshop_wrench', 'Chrome combination spanner, 45 cm, lying flat (knockable).',
        [box((0.95, 0.05, 0.13), (0, 0.05, 0))], dynamic=True, mass=0.1)
    add('workshop_jack', 'Red trolley jack, 0.7 m, handle raised toward +z.',
        [box((0.75, 0.4, 1.5), (0, 0.4, 0))])
    add('workshop_oil_drum', 'Blue 205 l steel oil drum with a hand pump.',
        [cyl(1.2, 1.94, (0, 1.94, 0))])
    add('workshop_tyre_changer', 'Tyre changer: grey cabinet with a red turntable clamping a '
        'wheel, a tilting column with the mount head; 4 x 3.2 u footprint.',
        [box((1.9, 1.6, 1.5), (0, 1.6, 0)), box((0.45, 4.1, 0.45), (0, 4.1, 1.15))])
    add('workshop_wheel_balancer', 'Wheel balancer: grey body with a display, a wheel on the '
        'shaft (+x side) under a raised hood; 6 x 2.6 u footprint.',
        [box((1.45, 2.0, 1.2), (-1.1, 2.0, 0)), cyl(1.3, 0.55, (1.3, 2.1, 0), (0, 0, 90))])
    add('workshop_engine_stand', 'Engine on a rotating stand: a four-cylinder block with head, '
        'rocker cover, pulleys and manifold on a castored T-stand; 5 x 3 u footprint.',
        [box((1.2, 1.5, 1.5), (0, 2.6, 0)), box((0.25, 1.1, 0.25), (0, 1.1, 1.6))])
    add('workshop_engine_crane', 'Folding workshop crane (engine hoist): red A-frame legs on '
        'casters, mast, boom with chain and hook; boom toward -z.',
        [box((0.3, 3.9, 0.3), (0, 3.9, 1.5)), box((1.55, 0.2, 2.0), (0, 0.32, -0.5))])
    add('workshop_hanging_lamp', 'Hanging twin-tube fluorescent work light, neutral-cool white (1.5 m batten on cables to the '
        'ceiling), placed at 14.85 u; its tubes glow all day, the spot light comes on at night.',
        [box((3.3, 0.15, 0.38), (0, 0.4, 0))],
        lights=[dict(offset=(0, -0.1, 0), color=(0.88, 0.94, 1.0), lumens=1400, range=30,
                     spot_deg=72, dir=(0, -1, 0))])
    add('workshop_pallet', 'Wooden pallet (1.2 x 0.8 m) with a few boxes of parts; long axis '
        'along x.', [box((2.62, 0.3, 1.74), (0, 0.3, 0))], surface='Wood')
    return P


# ---------------------------------------------------------------------------------------------
# RON

def _f(v):
    s = f'{float(v):.3f}'.rstrip('0').rstrip('.')
    return '0' if s in ('-0', '') else s


def _v(t):
    return '(' + ', '.join(_f(x) for x in t) + ')'


def _prim(c):
    kind = c[0]
    if kind in ('Box', 'Wedge'):
        _, half, off, rot = c
        s = f'{kind}(half: {_v(half)}, offset: {_v(off)}'
    elif kind == 'Cylinder':
        _, r, hh, off, rot = c
        s = f'Cylinder(radius: {_f(r)}, half_height: {_f(hh)}, offset: {_v(off)}'
    else:
        _, r, off = c
        return f'Ball(radius: {_f(r)}, offset: {_v(off)})'
    return s + (f', rot: {_v(rot)})' if rot else ')')


def write_catalog(path=OUT):
    lines = ['// Prop catalog — Wrench Works (auto-repair workshop, art bible §3.6). Kinds are',
             '// prefixed `workshop_`. Units: u (1 u = 22.5 cm), real size × 4.44; origin =',
             '// ground-contact centre, front faces −Z. Models: models/props/workshop/<name>.glb,',
             '// generated by tools/blender/props/workshop/<name>.py (notes: docs/design/',
             '// env-workshop.md). GENERATED by tools/blender/props/workshop/catalog.py.',
             '// Format: docs/design/rc_track.md §A9; `lights` are read by rc_render.',
             '(', '    props: {']
    for kind, p in props().items():
        lines.append(f'        // {p["note"]}')
        lines.append(f'        "{kind}": (')
        lines.append(f'            model: "{p["model"]}",')
        if p.get('dynamic'):
            lines.append('            dynamic: true,')
            lines.append(f'            mass: {_f(p["mass"])},')
        if p.get('surface'):
            lines.append(f'            surface: {p["surface"]},')
        cs = p['colliders']
        if len(cs) == 1:
            lines.append(f'            colliders: [{_prim(cs[0])}],')
        else:
            lines.append('            colliders: [')
            lines += [f'                {_prim(c)},' for c in cs]
            lines.append('            ],')
        if p.get('lights'):
            lines.append('            lights: [')
            for li in p['lights']:
                parts = [f'offset: {_v(li["offset"])}']
                if 'color' in li:
                    parts.append(f'color: {_v(li["color"])}')
                parts += [f'lumens: {_f(li["lumens"])}', f'range: {_f(li["range"])}']
                if li.get('spot_deg'):
                    parts += [f'spot_deg: {_f(li["spot_deg"])}', f'dir: {_v(li["dir"])}']
                if li.get('shadows'):
                    parts.append('shadows: true')
                lines.append(f'                ({", ".join(parts)}),')
            lines.append('            ],')
        lines.append('        ),')
    lines += ['    },', ')', '']
    with open(path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))
    print(f'wrote {path} ({len(props())} kinds)')


if __name__ == '__main__':
    write_catalog()
