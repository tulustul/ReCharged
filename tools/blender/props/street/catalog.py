"""Street prop catalog: dimensions and collider primitives of every `street_*` prop kind, shared
by the Blender prop scripts (pure Python, no bpy) and written to assets/data/props/street.ron:

    python3 tools/blender/props/street/catalog.py

Prop space is Bevy's: +Y up, front = -Z (the face toward the road / the climbing direction),
origin at the ground-contact centre; units u (1 u = 22.5 cm, real metres x 4.44). Colliders
stay inside the visual mesh; drivable tops carry a surface. Kerb arcs come from plan.py.
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
OUT = os.path.join(ROOT, 'assets', 'data', 'props', 'street.ron')

# --- street cross-section (see streetkit.py) ----------------------------------------------
KERB_H, KERB_D = 0.65, 0.6          # barrier kerb: height above the road, depth behind the face
WALK_H, DRIVE_H, LAWN_H = 0.62, 0.64, 0.6
APRON_W, APRON_FLARE, APRON_IN, APRON_OUT = 14.0, 3.0, 2.0, 0.6  # in: into the road, out: behind
ROLLED_IN = 1.0                     # rolled kerb toe in front of the kerb face line

# --- parked SUV (generic family crossover, 4.85 x 1.92 x 1.76 m) ----------------------------
SUV = dict(length=21.5, width=8.5, height=7.8, wheelbase=12.6, track=7.3, tyre_r=1.68,
           tyre_w=1.13, floor=1.8)
SUV_PAINTS = {'a': '#4F6A82', 'b': '#7C3036', 'c': '#A9ADB1'}  # slate blue, burgundy, silver

# --- kids' ramps on the raised yard ---------------------------------------------------------
PLANK = dict(width=10.8, rise=3.2, run=12.0)   # two plywood sheets on cinder blocks, 14.9°
KICKER = dict(width=10.8, rise=0.8, run=8.0)   # plywood kicker, 5.7°
WALL = dict(length=8.0, height=2.5, thick=0.8)  # stacked-stone retaining wall (lawn 0.6 -> 3.1)

# --- storm drain dip --------------------------------------------------------------------------
DRAIN = dict(length=24.0, width=18.0, depth=2.0, bottom=4.0, side=4.0)


# Collider rotations: rc_track reads `rot: (x, y, z)` as degrees about each axis (pitch X,
# yaw Y, roll Z, applied yaw -> pitch -> roll); `yaw=` is the common case.
def _rot(yaw, rot):
    return (0, yaw, 0) if yaw is not None else rot


def box(half, offset=(0, 0, 0), yaw=None, rot=None):
    return ('Box', half, offset, _rot(yaw, rot))


def wedge(half, offset=(0, 0, 0), yaw=None, rot=None):
    return ('Wedge', half, offset, _rot(yaw, rot))


def cyl(radius, half_height, offset=(0, 0, 0), yaw=None, rot=None):
    return ('Cylinder', radius, half_height, offset, _rot(yaw, rot))


def ball(radius, offset=(0, 0, 0)):
    return ('Ball', radius, offset)


def kerb_straight(length):
    return box((length / 2, KERB_H / 2, KERB_D / 2), (0, KERB_H / 2, KERB_D / 2))


def arc_points(radius, span_deg, face, step):
    """Face-line points of an arc kerb piece in prop space (x, z), origin at the arc middle,
    front -Z. face 'out': road outside the circle (centre behind, +Z); 'in': road inside."""
    n = max(1, math.ceil(math.radians(span_deg) * radius / step))
    half = math.radians(span_deg) / 2
    s = 1 if face == 'out' else -1
    pts = []
    for i in range(n + 1):
        a = -half + 2 * half * i / n
        pts.append((radius * math.sin(a), s * radius * (1 - math.cos(a)), a))
    return pts


def kerb_arc_colliders(radius, span_deg, face):
    """Chord boxes along an arc kerb (≤ 3 u chords, sagitta ≤ ~0.1 u)."""
    out = []
    pts = arc_points(radius, span_deg, face, 3.0)
    s = 1 if face == 'out' else -1
    for (x0, z0, a0), (x1, z1, a1) in zip(pts, pts[1:]):
        a = (a0 + a1) / 2
        nx, nz = s * math.sin(a), -math.cos(a)  # outward face normal (toward the road)
        length = math.hypot(x1 - x0, z1 - z0) + 0.05
        cx = (x0 + x1) / 2 - nx * KERB_D / 2
        cz = (z0 + z1) / 2 - nz * KERB_D / 2
        yaw = math.degrees(math.atan2(-nx, -nz))
        out.append(box((length / 2, KERB_H / 2, KERB_D / 2), (cx, KERB_H / 2, cz), yaw=yaw))
    return out


def props():
    """kind -> dict(model, colliders, dynamic, mass, surface, lights, note)."""
    P = {}

    def add(kind, note, colliders, model=None, **kw):
        P[kind] = dict(model=f'models/props/street/{model or kind[len("street_"):]}.glb',
                       colliders=colliders, note=note, **kw)

    # kerbs and road furniture -------------------------------------------------------------
    for n in (4, 2, 1):
        add(f'street_kerb_{n}', f'Concrete barrier kerb, {n} u long, 0.65 u (15 cm) tall, '
            '0.6 u deep, with its gutter pan; face toward -Z (the road).',
            [kerb_straight(n)], surface='Stone')
    add('street_kerb_rolled', 'Rolled (mountable) kerb, 4 u: a 21° slope from 1 u in front of '
        'the kerb line up to 0.62 at its back: the curb hop.',
        [wedge((2, WALK_H / 2, (ROLLED_IN + KERB_D) / 2),
               (0, WALK_H / 2, (KERB_D - ROLLED_IN) / 2), yaw=180)], surface='Stone')
    w = APRON_W / 2 + APRON_FLARE
    add('street_apron', 'Driveway apron: 14 u driveway + 3 u flares, from 2 u into the road up '
        'to the driveway (0.64) at the kerb back line (14°).',
        [wedge((w, DRIVE_H / 2, (APRON_IN + APRON_OUT) / 2),
               (0, DRIVE_H / 2, (APRON_OUT - APRON_IN) / 2), yaw=180)], surface='Asphalt')
    d = DRAIN
    hl, hw = d['length'] / 2, d['width'] / 2
    slope = (hl - d['bottom'] / 2)
    add('street_storm_drain', 'Storm-drain dip: a 24 x 18 u concrete dish 2 u deep along the '
        'kerb (11° slopes) with a cast-iron grate and the kerb inlet; origin on the kerb face '
        'line, road toward -Z. Includes the deep kerb over its length.',
        [wedge((hw, d['depth'] / 2, slope / 2), (-hl + slope / 2, -d['depth'] / 2, -hw),
               yaw=90),
         wedge((hw, d['depth'] / 2, slope / 2), (hl - slope / 2, -d['depth'] / 2, -hw),
               yaw=-90),
         wedge((hl, d['depth'] / 2, d['side'] / 2),
               (0, -d['depth'] / 2, -d['width'] + d['side'] / 2)),
         box((d['bottom'] / 2 + 0.2, 0.25, hw), (0, -d['depth'] - 0.25, -hw)),
         box((hl, (KERB_H + d['depth']) / 2, KERB_D / 2),
             (0, (KERB_H - d['depth']) / 2, KERB_D / 2))],
        hero=True, surface='Asphalt')
    add('street_manhole', 'Cast-iron manhole cover, 3 u across, flush with the road.',
        [box((1.4, 0.01, 1.4), (0, 0.01, 0))], surface='Asphalt')
    add('street_paint_dash', 'Yellow centre-line dash, 10 x 0.45 u.',
        [box((5, 0.01, 0.22), (0, 0.01, 0))], surface='Asphalt')
    add('street_paint_bar', 'White crosswalk bar, 2.2 x 14 u (long side along X).',
        [box((7, 0.01, 1.1), (0, 0.01, 0))], surface='Asphalt')
    add('street_paint_stop', 'White stop line, 22 x 0.9 u.',
        [box((11, 0.01, 0.45), (0, 0.01, 0))], surface='Asphalt')

    # parked cars ---------------------------------------------------------------------------
    s = SUV
    zw, xw = s['wheelbase'] / 2, s['track'] / 2
    wheels = [cyl(s['tyre_r'] - 0.02, s['tyre_w'] / 2 - 0.02, (x, s['tyre_r'], z), rot=(0, 0, 90))
              for x in (-xw, xw) for z in (-zw, zw)]
    body = [box((s['width'] / 2 - 0.15, (4.7 - s['floor']) / 2, s['length'] / 2 - 0.4),
                (0, (4.7 + s['floor']) / 2, 0)),
            box((s['width'] / 2 - 0.6, 1.4, 6.4), (0, 6.05, 2.2))]
    for v, paint in SUV_PAINTS.items():
        add(f'street_parked_suv_{v}', f'Parked family crossover ({paint}), 21.5 x 8.5 x 7.8 u. '
            'Drive under it: 1.8 u clear under the floor, 9.2 u between the axles (across), '
            '6.1 u between the wheels (along).', wheels + body, hero=True, model=f'parked_suv_{v}')

    # kids' ramps and the raised yard -------------------------------------------------------
    p = PLANK
    add('street_plank_ramp', 'Kids\' ramp: two plywood sheets on cinder blocks, 10.8 u wide, '
        'rising 3.2 u over 12 u toward -Z (14.9°).',
        [wedge((p['width'] / 2, p['rise'] / 2, p['run'] / 2), (0, p['rise'] / 2, 0))],
        surface='Wood')
    k = KICKER
    add('street_kicker', 'Plywood kicker on bricks, 10.8 u wide, rising 0.8 u over 8 u toward '
        '-Z (5.7°).', [wedge((k['width'] / 2, k['rise'] / 2, k['run'] / 2),
                             (0, k['rise'] / 2, 0))], surface='Wood')
    wl = WALL
    add('street_retaining_wall', 'Stacked-stone retaining wall, 8 u long, 0.8 u thick, 2.5 u '
        'tall (a raised front yard); its cap is drivable.',
        [box((wl['length'] / 2, wl['height'] / 2, wl['thick'] / 2), (0, wl['height'] / 2, 0))],
        surface='Stone')

    # street furniture ---------------------------------------------------------------------
    add('street_lamp', 'Street lamp: 28 u tapered steel pole, upswept arm and a sodium '
        'cobra head over the road (-Z); its bulb material is lamp_sodium.',
        [cyl(0.55, 13.5, (0, 13.5, 0)), cyl(1.0, 0.6, (0, 0.6, 0))], hero=True,
        lights=[dict(offset=(0, 28.2, -7.2), color=(1.0, 0.70, 0.38), lumens=60000, range=70,
                     spot_deg=110, dir=(0, -1, -0.12))], model='street_lamp')
    add('street_hydrant', 'Red fire hydrant with white bonnet and caps, 3.5 u tall.',
        [cyl(0.6, 1.7, (0, 1.7, 0))])
    add('street_hydrant_cluster', 'Hydrant on a concrete pad with two yellow guard bollards '
        '(toward -Z) and a marker post.',
        [cyl(0.6, 1.7, (0, 1.7, 0)), cyl(0.45, 1.95, (-2.4, 1.95, -0.8)),
         cyl(0.45, 1.95, (2.4, 1.95, -0.8))], hero=True)
    add('street_mailbox', 'Black rural mailbox with a red flag on a white 4x4 post, 5.2 u.',
        [box((0.21, 2.0, 0.21), (0, 2.0, 0)), box((0.45, 0.47, 1.03), (0, 4.57, -0.08))])
    for kind, model, note in (('street_trash_can', 'trash_can', 'Dark green wheelie bin'),
                              ('street_recycling_bin', 'recycling_bin', 'Blue recycling bin')):
        add(kind, f'{note}, 2.7 x 3.4 x 4.6 u (wheels at +Z).',
            [box((1.25, 2.25, 1.6), (0, 2.25, 0.1))], model=model)
    add('street_cone', 'Traffic cone, 3.1 u tall (knockable).',
        [box((0.75, 0.06, 0.75), (0, 0.06, 0)), cyl(0.42, 1.45, (0, 1.55, 0))],
        dynamic=True, mass=0.25)
    add('street_sign_stop', 'Stop sign on a square steel post, 10 u.',
        [cyl(0.2, 4.9, (0, 4.9, 0.12))])
    add('street_sign_street', 'Street-name blades MAPLE ST / BIRCH LN on a steel post, 12 u.',
        [cyl(0.2, 6.0, (0, 6.0, 0))])
    add('street_barricade', 'Road-closed barricade: striped rails on A-frames, amber flasher.',
        [box((5.0, 2.2, 0.75), (0, 2.2, 0))],
        lights=[dict(offset=(-4.4, 4.9, 0), color=(1.0, 0.5, 0.08), lumens=60, range=8,
                     spot_deg=0)])
    add('street_newspaper', 'Rolled newspaper in a plastic sleeve (knockable).',
        [cyl(0.25, 0.65, (0, 0.25, 0), rot=(0, 0, 90))], dynamic=True, mass=0.08)
    add('street_sprinkler', 'Oscillating lawn sprinkler.', [cyl(0.5, 0.28, (0, 0.28, 0))])
    add('street_garden_light', 'Solar path light, 1.8 u (lamp_warm lens).',
        [cyl(0.2, 0.9, (0, 0.9, 0))],
        lights=[dict(offset=(0, 1.46, 0), color=(1.0, 0.82, 0.6), lumens=60, range=6,
                     spot_deg=0)])
    add('street_bike', 'Kid\'s bike lying on its side on the lawn, 5.6 u long.',
        [box((1.45, 0.9, 2.8), (-1.36, 0.9, 0))])
    add('street_leaf_pile', 'Raked pile of maple leaves: a soft 0.5 u bump 5 u across.',
        [ball(6.0, (0, -5.5, 0))], surface='Grass')
    add('street_leaves', 'Scatter of fallen maple leaves on the ground (flat).',
        [box((2.4, 0.01, 2.1), (0, 0.01, 0.15))], surface='Asphalt')

    # garden and backdrop ------------------------------------------------------------------
    add('street_hedge', 'Clipped privet hedge block, 8 x 3.6 x 5 u.',
        [box((4.0, 2.5, 1.8), (0, 2.5, 0))])
    add('street_fence', 'White picket fence, 8 u section, 4.8 u posts.',
        [box((4.0, 2.2, 0.15), (0, 2.2, 0.05))])
    add('street_fence_corner', 'Picket fence corner with a climbing rose: two 8 u legs (along +X '
        'and +Z from the corner post).', [box((4.0, 2.2, 0.15), (4.2, 2.2, 0.05)),
                                          box((0.15, 2.2, 4.0), (-0.05, 2.2, 4.2))], hero=True)
    for v in ('a', 'b'):
        add(f'street_maple_{v}', 'Autumn maple tree, 40-50 u tall; only the trunk collides.',
            [cyl(1.2, 6.0, (0, 6.0, 0))], model=f'maple_{v}')
    # houses: front toward -Z; *_m are mirrored (garage on -X)
    for m, sx in (('', 1), ('_m', -1)):
        add(f'street_house_a{m}', 'Two-storey colonial with porch and attached two-car garage '
            f'(garage on {"+" if sx > 0 else "-"}X), 56 x 36 x 42 u.',
            [box((15.0, 11.5, 14.0), (-9.0 * sx, 11.5, 0)),
             box((12.0, 6.25, 12.0), (18.0 * sx, 6.25, 0)),
             box((8.0, 0.65, 2.5), (-11.0 * sx, 0.65, -16.5))], hero=True,
            model=f'house_a{m}',
            lights=[dict(offset=(-8.0 * sx, 8.7, -14.6), color=(1.0, 0.8, 0.55), lumens=500,
                         range=20, spot_deg=0)])
        add(f'street_house_b{m}', 'Single-storey ranch with brick wainscot and garage wing '
            f'(garage on {"+" if sx > 0 else "-"}X), 64 x 36 x 25 u.',
            [box((19.0, 6.25, 15.0), (-11.0 * sx, 6.25, 0)),
             box((12.0, 5.5, 13.5), (20.0 * sx, 5.5, 1.5)),
             box((3.25, 0.45, 1.5), (-6.0 * sx, 0.45, -17.0))], hero=True,
            model=f'house_b{m}',
            lights=[dict(offset=(-3.4 * sx, 8.1, -15.8), color=(1.0, 0.8, 0.55), lumens=500,
                         range=20, spot_deg=0)])
    return P


def fmt(v):
    if isinstance(v, (tuple, list)):
        return '(' + ', '.join(fmt(x) for x in v) + ')'
    if isinstance(v, float):
        s = f'{v:.4f}'.rstrip('0').rstrip('.')
        return s if s not in ('-0', '') else '0'
    return str(v)


def prim(c):
    kind = c[0]
    if kind in ('Box', 'Wedge'):
        _, half, off, rot = c
        s = f'{kind}(half: {fmt(half)}, offset: {fmt(off)}'
    elif kind == 'Cylinder':
        _, r, hh, off, rot = c
        s = f'Cylinder(radius: {fmt(r)}, half_height: {fmt(hh)}, offset: {fmt(off)}'
    else:
        _, r, off = c
        return f'Ball(radius: {fmt(r)}, offset: {fmt(off)})'
    if rot is not None:
        s += f', rot: {fmt(rot)}'
    return s + ')'


def ron(extra=None):
    """The catalog RON; `extra` = additional kinds (kerb arcs from plan.py)."""
    P = props()
    P.update(extra or {})
    lines = [
        '// Prop catalog — Maple Street (suburban street & front lawns, art bible §3.7).',
        '// GENERATED by tools/blender/props/street/catalog.py (+ kerb arcs from plan.py):',
        '// edit those and run `python3 tools/blender/props/street/plan.py`.',
        '// Units: u (1 u = 22.5 cm, real size x 4.44); origin = ground-contact centre, front -Z.',
        '// `lights` (rc_render): offset (prop space, u), linear colour, lumens (real-world),',
        '// range (u), spot_deg (outer cone, 0 = point light), dir (prop space, spots only).',
        '(',
        '    props: {',
    ]
    for kind in sorted(P):
        d = P[kind]
        lines.append(f'        // {d["note"]}')
        lines.append(f'        "{kind}": (')
        lines.append(f'            model: "{d["model"]}",')
        if d.get('dynamic'):
            lines.append('            dynamic: true,')
            lines.append(f'            mass: {fmt(float(d["mass"]))},')
        if d.get('surface'):
            lines.append(f'            surface: {d["surface"]},')
        cs = d['colliders']
        if len(cs) == 1:
            lines.append(f'            colliders: [{prim(cs[0])}],')
        else:
            lines.append('            colliders: [')
            lines += [f'                {prim(c)},' for c in cs]
            lines.append('            ],')
        if d.get('lights'):
            lines.append('            lights: [')
            for li in d['lights']:
                parts = [f'offset: {fmt(tuple(float(x) for x in li["offset"]))}',
                         f'color: {fmt(tuple(float(x) for x in li["color"]))}',
                         f'lumens: {fmt(float(li["lumens"]))}', f'range: {fmt(float(li["range"]))}',
                         f'spot_deg: {fmt(float(li["spot_deg"]))}']
                if 'dir' in li:
                    parts.append(f'dir: {fmt(tuple(float(x) for x in li["dir"]))}')
                lines.append(f'                ({", ".join(parts)}),')
            lines.append('            ],')
        lines.append('        ),')
    lines += ['    },', ')', '']
    return '\n'.join(lines)


def write(extra=None):
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(ron(extra))
    print(f'wrote {OUT}')


if __name__ == '__main__':
    import plan  # noqa: F401  (writes the catalog with its kerb arcs)
    plan.main()
