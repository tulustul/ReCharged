"""Wrench Works layout generator (pure Python, no Blender): the race line, ground polygons, item
rows, props, Golden Battery and the decorative dressing of the auto-repair workshop, written to
assets/data/tracks/wrench_works.ron and assets/data/decor/wrench_works.ron; also rewrites the prop
catalog assets/data/props/workshop.ron (catalog.py).

    python3 tools/blender/props/workshop/plan.py
    cargo run -p rc_track --example plot -- wrench_works [--rev] [--mirror]

World axes (Bevy): x east, z south (plots: north up), y up; 1 u = 22.5 cm (real x 4.44).
Bearings: 0 = north (-Z), 90 = east (+X), 180 = south, 270 = west.

Floor plan (forward = clockwise): roller doors in the north wall; the N lane runs east under
the tails of two cars raised on two-post lifts (bays perpendicular to the doors); U-turn; the
M lane runs west over the workbench island (plywood ramp up, kicker, jump onto the far ramp);
U-turn; the S lane runs east through the tyre-stack chicane; U-turn; the S3 lane runs west
along the south wall through the wash bay (water) and the paint booth; the W lane runs north
along the west wall over rubber mats back to the start. Oil stains sit on the U-turn lines.
"""
import math
import os

import catalog as C

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = C.ROOT
TRACK_OUT = os.path.join(ROOT, 'assets', 'data', 'tracks', 'wrench_works.ron')
DECOR_OUT = os.path.join(ROOT, 'assets', 'data', 'decor', 'wrench_works.ron')

# ---------------------------------------------------------------------------------------------
# floor plan (u)
BOUNDS = ((-88.0, -60.0), (88.0, 60.0))
Z_N, Z_M, Z_S, Z_S3 = -42.0, -14.0, 14.0, 42.0     # east-west lane centres
X_W = -78.0                                         # west lane centre
X_E, X_WT = 60.0, -48.0                             # U-turn centres (east, west)
R_U, R_C = 14.0, 14.0                               # U-turn and corner radii
W = 10.0                                            # standard racing width
START_X = -30.0                                     # start / finish line on the N lane
LIFT_Z = Z_N + 8.0                                  # lift column line (cars centred on it)
LIFTS = ((6.0, 'workshop_lift_a'), (32.0, 'workshop_lift_b'))  # car centre x
BENCH_X = 6.0                                       # workbench island centre on the M lane
CHICANE = [(-30.0, 0.0), (-14.0, -4.0), (2.0, 4.0), (18.0, -4.0), (34.0, 0.0)]  # (x, lateral)
ALLEY = dict(x0=-26.0, x1=-2.0, gaps=(-21.5, -6.5), width=4.4)  # GB alley behind apex A
CHICANE_W = 8.0
CHICANE_OPEN = 5.0                                  # the tyre walls start this far into the chicane
DRESS_BEHIND = 1.5                                  # knockables stand this far behind a wall
WASH_X = 27.0                                       # wash bay centre on the S3 lane
BOOTH_X = -24.0                                     # paint booth centre on the S3 lane
WALLS = dict(wall_left=True, wall_right=True)
OPEN = dict(wall_left=False, wall_right=False)


def rad(a):
    return math.radians(a)


class Route:
    """Turtle that emits control points: straights and arcs (a point every <= `step` deg)."""

    def __init__(self, x, z, bearing):
        self.x, self.z, self.b = x, z, bearing
        self.pts = []

    def point(self, y=0.0, note='', l=0.0, **kw):
        rx, rz = math.cos(rad(self.b)), math.sin(rad(self.b))  # right of the heading
        pos = (round(self.x + rx * l, 3), round(y, 3), round(self.z + rz * l, 3))
        self.pts.append(dict(pos=pos, note=note, **kw))
        return len(self.pts) - 1

    def go(self, d):
        self.x += d * math.sin(rad(self.b))
        self.z -= d * math.cos(rad(self.b))

    def lane(self, to, marks=()):
        """Straight along the current axis to coordinate `to` (x heading E/W, z heading N/S);
        marks [(coord, y, note, attrs)] are emitted on the way (attrs may hold `l`)."""
        ew = round(math.sin(rad(self.b))) != 0
        here = self.x if ew else self.z
        sgn = 1 if to > here else -1
        done = 0.0
        for c, y, note, kw in sorted(marks, key=lambda m: sgn * m[0]):
            d = abs(c - here)
            self.go(d - done)
            done = d
            self.point(y, note, **kw)
        self.go(abs(to - here) - done)

    def arc(self, radius, angle, step=30.0, note='', **kw):
        """Turn by `angle` degrees (positive = right / clockwise) on `radius`."""
        sgn = 1 if angle > 0 else -1
        cx = self.x + sgn * radius * math.cos(rad(self.b))
        cz = self.z + sgn * radius * math.sin(rad(self.b))
        n = max(1, math.ceil(abs(angle) / step - 1e-9))
        a0 = math.atan2(self.z - cz, self.x - cx)
        for i in range(1, n + 1):
            a = a0 + sgn * rad(abs(angle)) * i / n
            self.x, self.z = cx + radius * math.cos(a), cz + radius * math.sin(a)
            self.point(0.0, note if i == n else '', **(kw if i == 1 else {}))
        self.b = (self.b + angle) % 360
        return (cx, cz)


def step(r):
    """One turn-step on a straight before / after a turn (authoring guide A5)."""
    return r * math.pi / 6


# ---------------------------------------------------------------------------------------------
# the race line

def route():
    r = Route(START_X, Z_N, 90)
    ix = {}
    ix['start'] = r.point(note='start / finish', width=W, surface='Asphalt', offroad_margin=6,
                          **WALLS)
    # N lane east: under the tails of the two cars on the lifts
    r.lane(X_E, [(-10.0, 0, '', {}), (LIFTS[0][0], 0, 'under lift A', {}),
                 (LIFTS[1][0], 0, 'under lift B', {}), (X_E - step(R_U), 0, '', {})])
    r.point(note='E1 U-turn')
    r.arc(R_U, 180, step=22.5)
    # M lane west: the workbench island (ramp up, kicker, bench, kicker, jump, ramp down)
    B = C.BENCH
    e_edge, w_edge = BENCH_X + B['length'] / 2, BENCH_X - B['length'] / 2
    foot, kick, land = B['ramp_run'], B['kicker_run'], C.LAND_D
    lip_y, top_y = B['top'] + B['kicker_rise'], B['top']
    land_y = lip_y - land * lip_y / foot
    names = {}
    marks = [
        (X_E - step(R_U), 0, '', {}),
        # the walls stop 7 u before the east ramp foot and start again 7 u after the west one:
        # a car landing a little off line (either direction) runs out on open floor
        (e_edge + foot + 7, 0, '', dict(width=8.0, **OPEN)),
        (e_edge + foot, 0, 'east ramp foot', {}),
        (e_edge + land, land_y, 'east ramp (reverse landing)', {}),
        (e_edge, lip_y, 'east bench edge (reverse lip)', {}),
        (e_edge - kick, top_y, 'east kicker foot', {}),
        (w_edge + kick, top_y, 'west kicker foot', {}),
        (w_edge, lip_y, 'west bench edge (forward lip)', {}),
        (w_edge - land, land_y, 'west ramp (forward landing)', {}),
        (w_edge - foot, 0, 'west ramp foot', {}),
        (w_edge - foot - 7, 0, '', dict(width=W, **WALLS)),
        (X_WT + step(R_U), 0, '', {}),
    ]
    r.lane(X_WT, marks)
    for i, p in enumerate(r.pts):
        names[p['note']] = i
    ix['rev_land'] = names['east ramp (reverse landing)']
    ix['rev_lip'] = names['east bench edge (reverse lip)']
    ix['fwd_lip'] = names['west bench edge (forward lip)']
    ix['fwd_land'] = names['west ramp (forward landing)']
    r.point(note='W U-turn')
    r.arc(R_U, -180, step=22.5)
    # S lane east: the tyre-stack chicane (walls replaced by tyre stacks)
    ch = CHICANE
    marks = [(X_WT + step(R_U), 0, '', {}), (ch[0][0] - 9, 0, '', {})]
    marks.append((ch[0][0], 0, 'tyre-stack chicane', dict(width=CHICANE_W, **OPEN)))
    marks += [(x, 0, '', dict(l=l)) for x, l in ch[1:-1]]
    marks.append((ch[-1][0], 0, '', dict(width=W, **WALLS)))
    marks += [(ch[-1][0] + 9, 0, '', {}), (X_E - step(R_U), 0, '', {})]
    r.lane(X_E, marks)
    r.point(note='E2 U-turn')
    r.arc(R_U, 180, step=22.5)
    # S3 lane west: wash bay (splash walls), paint booth (its own walls)
    wl, bl = C.WASH['length'] / 2, C.BOOTH['length'] / 2
    r.lane(X_W + R_C, [(X_E - step(R_U), 0, '', {}),
                       (WASH_X + wl, 0, 'wash bay', OPEN),
                       (WASH_X - wl, 0, '', WALLS),
                       (BOOTH_X + bl, 0, 'paint booth', OPEN),
                       (BOOTH_X - bl, 0, '', WALLS),
                       (X_W + R_C + step(R_C), 0, '', {})])
    r.point(note='SW corner')
    r.arc(R_C, 90)
    # W lane north: rubber mats
    r.lane(Z_N + R_C, [(Z_S3 - R_C - step(R_C), 0, '', {}), (0.0, 0, '', {}),
                       (Z_N + R_C + step(R_C), 0, '', {})])
    r.point(note='NW corner')
    r.arc(R_C, 90)
    r.lane(START_X, [(X_W + R_C + step(R_C), 0, '', {})])
    return r.pts, ix


# ---------------------------------------------------------------------------------------------
# centreline sampling (centripetal Catmull-Rom like rc_track, gaps ignored) for placements

def _cr(p, u):
    def knot(a, b):
        return max(math.dist(a, b), 1e-4) ** 0.5
    t1 = knot(p[0], p[1])
    t2 = t1 + knot(p[1], p[2])
    t3 = t2 + knot(p[2], p[3])
    t = t1 + (t2 - t1) * u

    def mix(a, b, ta, tb):
        return tuple(x * (tb - t) / (tb - ta) + y * (t - ta) / (tb - ta) for x, y in zip(a, b))
    a1, a2, a3 = mix(p[0], p[1], 0, t1), mix(p[1], p[2], t1, t2), mix(p[2], p[3], t2, t3)
    return mix(mix(a1, a2, 0, t2), mix(a2, a3, t1, t3), t1, t2)


def samples(pts, i0, i1, n=12):
    """Dense samples (x, y, z, half_width) of segments i0 .. i1-1 (forward order)."""
    P = [p['pos'] for p in pts]
    widths, w = [], None
    for p in pts:
        w = p.get('width', w)
        widths.append(w)
    m = len(P)
    out = []
    for i in range(i0, i1):
        quad = [P[(i + k - 1) % m] for k in range(4)]
        wa, wb = widths[i % m], widths[(i + 1) % m]
        for k in range(n):
            u = k / n
            s = u * u * (3 - 2 * u)
            out.append((*_cr(quad, u), (wa + (wb - wa) * s) / 2))
    return out


def edge_line(pts, i0, i1, offset, spacing):
    """Points every `spacing` along the corridor edge offset (+ right / - left) by
    half width + |offset| - ... : `offset` is measured from the edge outward."""
    sm = samples(pts, i0, i1, 24)
    out, acc, last = [], 0.0, None
    for k in range(len(sm) - 1):
        x, y, z, hw = sm[k]
        dx, dz = sm[k + 1][0] - x, sm[k + 1][2] - z
        ln = math.hypot(dx, dz)
        if ln < 1e-9:
            continue
        rx, rz = -dz / ln, dx / ln  # right of travel (x east, z south)
        sgn = 1 if offset > 0 else -1
        q = (x + rx * sgn * (hw + abs(offset)), z + rz * sgn * (hw + abs(offset)))
        if last is None or math.dist(q, last) >= spacing:
            out.append(q)
            last = q
    return out


# ---------------------------------------------------------------------------------------------
# ground

def blob(cx, cz, rx, rz, rot=0.0, seed=1, n=28, rough=0.22):
    """Organic stain outline: an ellipse with smooth deterministic wobble (star-shaped)."""
    pts = []
    for i in range(n):
        a = 2 * math.pi * i / n
        k = 1 + rough * (0.55 * math.sin(3 * a + seed) + 0.3 * math.sin(5 * a + 2.1 * seed)
                         + 0.15 * math.sin(9 * a + 0.7 * seed))
        x, z = rx * k * math.cos(a), rz * k * math.sin(a)
        c, s = math.cos(rad(rot)), math.sin(rad(rot))
        pts.append((cx + x * c - z * s, cz + x * s + z * c))
    return pts


def rect(x0, z0, x1, z1):
    return [(x0, z0), (x1, z0), (x1, z1), (x0, z1)]


def ground():
    (x0, z0), (x1, z1) = BOUNDS
    g = [dict(note='stained concrete floor', surface='Asphalt', height=0.0,
              points=rect(x0, z0, x1, z1))]
    mat = C.MAT_H
    for za, zb in ((-24.0, -3.0), (3.0, 24.0)):
        for xa, xb in ((X_W - 5.6, X_W - 0.15), (X_W + 0.15, X_W + 5.6)):
            g.append(dict(note='ribbed rubber mat (W lane)', surface='Carpet', height=mat,
                          points=rect(xa, za, xb, zb)))
    hi = C.WASH['inner'] / 2
    wl = C.WASH['length'] / 2
    g.append(dict(note='wash bay: wet tiles under running water', surface='Water',
                  height=C.WATER_H, points=rect(WASH_X - wl, Z_S3 - hi, WASH_X + wl, Z_S3 + hi)))
    oil = C.OIL_H
    for note, b in (
            ('oil slick on the E1 U-turn apex', blob(70.5, -28.5, 5.5, 7.5, 10, seed=1)),
            ('oil slick on the outside of the W U-turn exit', blob(-55.0, 17.5, 7.5, 3.4, 12,
                                                                   seed=4)),
            ('oil spill on the S3 lane exit of the E2 U-turn', blob(52.0, 40.0, 7.0, 3.6, 8,
                                                                   seed=7)),
            ('drips under lift A', blob(LIFTS[0][0] + 0.5, Z_N + 1.8, 2.2, 1.6, 20, seed=2)),
            ('drips under lift B', blob(LIFTS[1][0] - 0.8, Z_N + 2.2, 2.6, 1.8, -15, seed=3))):
        g.append(dict(note=note, surface='Oil', height=oil, points=b))
    return g


# ---------------------------------------------------------------------------------------------
# props

def props(pts, ix):
    P = []

    def add(kind, x, z, yaw=0.0, y=0.0, note='', dir='Both'):
        P.append(dict(kind=kind, pos=(x, y, z), yaw=yaw, note=note, dir=dir))

    B = C.BENCH
    add('workshop_workbench', BENCH_X, Z_M, 90, note='workbench island on the M lane')
    add('workshop_ramp', BENCH_X + B['length'] / 2 + B['ramp_run'] / 2, Z_M, 90,
        note='plywood ramps: up onto the bench, down from it (forward: east up, west down)')
    add('workshop_ramp', BENCH_X - B['length'] / 2 - B['ramp_run'] / 2, Z_M, -90)
    for x, kind in LIFTS:
        add(kind, x, LIFT_Z, 180, note='car on a two-post lift (nose south)')
    add('workshop_paint_booth', BOOTH_X, Z_S3, 90, note='paint booth on the S3 lane')
    add('workshop_wash_bay', WASH_X, Z_S3, 90, note='wash bay on the S3 lane')
    add('workshop_tool_chest', X_E + 3.2, Z_N + R_U, -90,
        note='red tool chest on the E1 island tip, drawers toward the U-turn')
    # tyre-stack chicane: stacks along both corridor edges between the chicane points; behind
    # apex A (north side) a narrow alley between two rows opens through two gaps (Golden Battery)
    i0 = next(i for i, p in enumerate(pts) if p['note'] == 'tyre-stack chicane')
    i1 = i0 + len(CHICANE) - 1
    t = C.TYRE
    pitch = 2 * t['r'] + 0.12
    first = True

    def stack(x, z, note=''):
        nonlocal first
        add('workshop_tyre_stack2', x, z, (x * 37 + z * 11) % 360,
            note=('chicane tyre walls' if first else '') + note)
        first = False

    a = ALLEY
    for side in (-1, 1):
        for x, z in edge_line(pts, i0, i1, side * (t['r'] + 0.05), pitch):
            if side < 0 and any(abs(x - g) < pitch * 0.95 for g in a['gaps']):
                continue
            if x < CHICANE[0][0] + CHICANE_OPEN:
                continue  # open entry: cars leave the W U-turn wide
            stack(x, z)
    off = t['r'] + 0.05 + 2 * t['r'] + a['width']
    outer = [q for q in edge_line(pts, i0, i1, -off, pitch) if a['x0'] <= q[0] <= a['x1']]
    for x, z in outer:
        stack(x, z)
    inner = edge_line(pts, i0, i1, -(t['r'] + 0.05), 0.25)
    for x, z in (outer[0], outer[-1]):  # end caps: close the alley between the rows
        xi, zi = min(inner, key=lambda q: abs(q[0] - x))
        n = max(1, round(abs(zi - z) / pitch))
        for k in range(1, n):
            stack(x + (xi - x) * k / n, z + (zi - z) * k / n)
    dress(add)
    return P


def arc_pt(c, r, deg):
    """Point at `deg` (0 = +x, clockwise in plots: +z) on a circle around c."""
    return (c[0] + r * math.cos(rad(deg)), c[1] + r * math.sin(rad(deg)))


def dress(add):
    """Knockables beside the racing width (<= 40 dynamic bodies) and statics in the islands.
    Nothing knockable stands on the racing width: behind the walls on walled lanes, just outside
    the width where the walls are open (cars knocked them onto the line or tripped over them)."""
    bx = BENCH_X
    behind = W / 2 + DRESS_BEHIND  # centre line of the dressing behind a lane wall
    # N lane: drain pans under the car bodies (lift bays), a creeper, the wheel off car B, jugs
    add('workshop_drain_pan', LIFTS[0][0] - 2.5, Z_N + behind + 1.2,
        note='N lane dressing (knockable, behind the walls)')
    add('workshop_drain_pan', LIFTS[1][0] + 2.0, Z_N + behind + 1.2, 40)
    add('workshop_creeper', 19.0, Z_N - behind - 1.0, 80)
    add('workshop_wheel', LIFTS[1][0] - 4.0, Z_N - behind - 1.4, 25)
    for x, z, yaw in ((-2.0, Z_N - behind - 0.5, 10), (-1.0, Z_N - behind - 0.6, -25)):
        add('workshop_oil_can', x, z, yaw)
    # E1 U-turn island (inside the inner wall): a toolbox and a battery by the tool chest
    for kind, deg, r, yaw in (('workshop_toolbox', 25, 6.0, -60), ('workshop_battery', 42, 6.2, 20)):
        x, z = arc_pt((X_E, Z_N + R_U), r, deg)
        add(kind, x, z, yaw)
    # M lane: cones on the open floor around both ramp feet (the walls open there), spanners on
    # the bench top
    e_foot = bx + C.BENCH['length'] / 2 + C.BENCH['ramp_run']
    w_foot = bx - C.BENCH['length'] / 2 - C.BENCH['ramp_run']
    for x in (e_foot + 5.0, e_foot + 2.0, w_foot - 2.0, w_foot - 5.0):
        for side in (-1, 1):
            add('workshop_cone', x, Z_M + side * 5.3, (x * 13) % 90,
                note='cones around the ramp feet' if (x, side) == (e_foot + 5.0, -1) else '')
    add('workshop_wrench', bx + 4.0, Z_M + 2.6, 30, y=C.BENCH['top'],
        note='spanners left on the bench top (knock them off)')
    add('workshop_wrench', bx - 3.0, Z_M - 2.7, -50, y=C.BENCH['top'])
    # S lane: cones at the open chicane entry, loose tyres behind the walls after its exit
    for side in (-1, 1):
        add('workshop_cone', CHICANE[0][0] + 1.5, Z_S + side * (CHICANE_W / 2 + 1.2), 21)
    for x, z in ((45.0, Z_S - behind - 1.4), (48.5, Z_S + behind + 1.4)):
        add('workshop_tyre', x, z, (x * 31) % 360, note='loose tyres' if x == 45.0 else '')
    x, z = arc_pt((X_E, Z_S + R_U), 6.0, 0)
    add('workshop_tyre', x, z, 70)
    # S3 lane: buckets in the wash bay, oil jugs and jerry cans by the booth
    add('workshop_bucket', WASH_X + 9.0, Z_S3 + 4.9, 0, note='wash bay buckets, jugs, cans')
    add('workshop_bucket', WASH_X - 8.5, Z_S3 - 4.9, 0)
    for x, z, yaw in ((-3.0, Z_S3 - behind - 0.5, 15), (-4.0, Z_S3 - behind - 0.6, -35)):
        add('workshop_oil_can', x, z, yaw)
    add('workshop_jerry_can', BOOTH_X - 21.0, Z_S3 + behind + 0.6, 80)
    add('workshop_jerry_can', BOOTH_X - 22.6, Z_S3 + behind + 0.7, 95)
    # W lane: cones at the mat ends, behind the walls
    for z, side in ((-26.0, -1), (26.0, 1)):
        add('workshop_cone', X_W + side * (behind + 0.8), z, 20)
    # islands (reachable only by falling off the bench): equipment and stock, static
    add('workshop_tyre_changer', 45.0, -1.5, 180, note='island equipment (static)')
    add('workshop_wheel_balancer', 53.0, 2.0, -90)
    for x, z in ((-30.0, -28.0), (-27.3, -28.6), (-28.6, -25.6)):
        add('workshop_oil_drum', x, z, (x * 17) % 360)
    for x, z in ((8.0, 27.5), (10.7, 27.9), (62.6, Z_S + R_U)):
        add('workshop_oil_drum', x, z, (z * 23) % 360)
    # hanging work lights over the lanes (cables reach the 24 u ceiling; spot lights at night)
    ly = 24.0 - 9.15
    rows = [(x, Z_N, 0) for x in (-50.0, -26.0, -2.0, 22.0, 46.0)]
    rows += [(x, Z_M, 0) for x in (50.0, 26.0, 2.0, -22.0, -42.0)]
    rows += [(x, Z_S, 0) for x in (-40.0, -16.0, 8.0, 32.0, 52.0)]
    rows += [(x, Z_S3, 0) for x in (54.0, WASH_X, 2.0, -58.0)]
    rows += [(X_W, z, 90) for z in (-20.0, 0.0, 20.0)]
    for i, (x, z, yaw) in enumerate(rows):
        add('workshop_hanging_lamp', x, z, yaw, y=ly, note='hanging work lights' if i == 0 else '')
    add('workshop_engine_stand', -44.0, -27.0, 160)
    add('workshop_engine_crane', -50.0, -27.5, 120)
    add('workshop_pallet', -20.0, 28.0, 10)
    add('workshop_pallet', 22.0, 28.5, -6)
    add('workshop_jack', 22.0, -27.0, 35)
    for x, z in ((-12.0, -27.0), (-9.2, -27.8), (26.0, -2.0), (28.9, -1.6)):
        add('workshop_tyre_stack4', x, z, (x * 41) % 360)


def decor():
    """Physics-free wall dressing, lamps and floor markings (assets/data/decor)."""
    (x0, z0), (x1, z1) = BOUNDS
    D = []

    def add(model, x, z, yaw=0.0, y=0.0, note=''):
        D.append(dict(model=model, pos=(x, y, z), yaw=yaw, note=note))

    for i, (x, _) in enumerate(LIFTS):  # roller doors behind the lift bays + the wash bay
        add('roller_door', x, z0, 180, note='roller doors (openings in workshop.ron)' if i == 0 else '')
        add('bay_marking', x, z0 + 16.0, 0, note='lift bay markings' if i == 0 else '')
    add('roller_door', WASH_X, z1, 0)
    add('start_line', START_X, Z_N, 90, note='chequered start / finish tape')
    # north wall: shelving west of the doors, a tool wall to the east, signs
    for i, x in enumerate((-58.0, -48.0, -38.0, -28.0, -18.0)):
        add('shelf_a' if i % 2 == 0 else 'shelf_b', x, z0, 180,
            note='north wall: shelving, tool wall, signs' if i == 0 else '')
    add('walkway_line', -38.0, z0 + 3.0, 0)
    add('safety_signs', -9.5, z0, 180)
    add('tool_wall', 52.0, z0, 180)
    add('shelf_b', 66.0, z0, 180)
    add('compressor', x1 - 1.5, z0 + 6.0, 90)
    # east wall: shelving and a tool wall between and beside the U-turns
    add('shelf_a', x1, -40.0, 90, note='east wall')
    add('shelf_b', x1, -13.0, 90)
    add('tool_wall', x1, 0.0, 90)
    add('shelf_a', x1, 13.0, 90)
    add('safety_signs', x1, 24.0, 90)
    add('shelf_b', x1, 40.0, 90)
    # south wall
    for i, x in enumerate((-78.0, -68.0, -58.0)):
        add('shelf_b' if i % 2 == 0 else 'shelf_a', x, z1, 0, note='south wall' if i == 0 else '')
    add('safety_signs', 46.0, z1, 0)
    add('tool_wall', 62.0, z1, 0)
    add('shelf_a', 76.0, z1, 0)
    # west wall: tyre racks behind the W lane, a shelf in each corner
    for i, z in enumerate((-31.0, -18.5, -6.0, 6.5, 19.0, 31.5)):
        add('tyre_rack', x0, z, -90, note='west wall: tyre racks' if i == 0 else '')
    add('shelf_a', x0, -52.0, -90)
    add('shelf_b', x0, 52.0, -90)
    for i, x in enumerate((-72.0, -48.0, -24.0, 0.0, 24.0, 48.0, 72.0)):  # exposed roof trusses
        add('roof_truss', x, 0.0, 0, note='roof trusses with cable trays' if i == 0 else '')
    # oil smudges and tyre scuffs on the concrete
    for i, (x, z) in enumerate(((6.0, Z_N + 1.0), (32.0, Z_N), (60.0, Z_M + 1.0), (-20.0, Z_S),
                                (0.0, Z_S3), (-60.0, 30.0), (-78.0, -36.0), (48.0, Z_N),
                                (-48.0, Z_M), (70.0, Z_S3 - 2.0), (-30.0, Z_N + 2.0),
                                (20.0, Z_M + 2.5), (72.0, -30.0), (-60.0, 2.0), (0.0, Z_S - 2.0),
                                (40.0, Z_S3 + 1.5), (-78.0, 34.0), (64.0, 34.0))):
        add('stains', x, z, (x * 53 + z * 29) % 360, note='floor stains' if i == 0 else '')
    return D


def battery_pos(pts):
    """Golden Battery in the middle of the chicane alley (forward layouts)."""
    i0 = next(i for i, p in enumerate(pts) if p['note'] == 'tyre-stack chicane')
    i1 = i0 + len(CHICANE) - 1
    t, a = C.TYRE, ALLEY
    xm = (a['x0'] + a['x1']) / 2
    mid = t['r'] + 0.05 + t['r'] + a['width'] / 2
    x, z = min(edge_line(pts, i0, i1, -mid, 0.25), key=lambda q: abs(q[0] - xm))
    return (round(x, 2), 1.0, round(z, 2))
    return P


# ---------------------------------------------------------------------------------------------
# RON output

def fmt(v):
    if isinstance(v, bool):
        return 'true' if v else 'false'
    if isinstance(v, float):
        s = f'{v:.3f}'.rstrip('0').rstrip('.')
        return '0' if s in ('-0', '') else s
    if isinstance(v, (tuple, list)):
        return '(' + ', '.join(fmt(float(x) if isinstance(x, int) else x) for x in v) + ')'
    return str(v)


def point_ron(p, i):
    keys = ['width', 'bank_deg', 'surface', 'wall_left', 'wall_right', 'offroad_margin']
    attrs = ''.join(f', {k}: {fmt(p[k])}' for k in keys if k in p)
    note = f' {p["note"]}' if p['note'] else ''
    return f'        (pos: {fmt(p["pos"])}{attrs}), // {i}:{note}'


def write_track(pts, ix, ground, rows, props, gaps, battery, shortcuts):
    lines = [
        '// Wrench Works — auto-repair workshop (GDD §7.1, art bible §3.6). ~700 u, 3 laps, indoor',
        '// ★★★. GENERATED by tools/blender/props/workshop/plan.py — edit that and re-run; plot:',
        '//   cargo run -p rc_track --example plot -- wrench_works [--rev] [--mirror]',
        '// Invisible ribbon on a concrete floor (Asphalt, y = 0); rubber mats (Carpet), oil stains',
        '// (Oil) and the wash bay (Water) are ground polygons just above it; the workbench, its',
        '// plywood ramps, the lifts, tyre stacks, the booth and the wash bay are props.',
        '// Forward = clockwise: start in front of the roller doors → N lane under the tails of two',
        '// cars on lifts → U-turn → M lane over the workbench (ramp up, jump off the far end onto',
        '// the other ramp) → U-turn → S lane tyre-stack chicane → U-turn → S3 lane wash bay and',
        '// paint booth → W lane over rubber mats.',
        '(',
        '    style: Invisible,',
        f'    start: {ix["start"]},',
        '    kill_height: -6,',
        '    wall_height: 1.2,',
        f'    bounds: (min: {fmt(BOUNDS[0])}, max: {fmt(BOUNDS[1])}),',
        '    points: [',
    ]
    lines += [point_ron(p, i) for i, p in enumerate(pts)]
    lines += ['    ],', '    gaps: [']
    lines += [f'        (from: {a}, to: {b}, dir: {d}), // {n}' for a, b, d, n in gaps]
    lines += ['    ],', '    ground: [']
    for g in ground:
        pts2 = ', '.join(fmt((round(x, 2), round(z, 2))) for x, z in g['points'])
        d = f', dir: {g["dir"]}' if g.get('dir') else ''
        lines.append(f'        // {g["note"]}')
        lines.append(f'        (surface: {g["surface"]}, height: {fmt(g["height"])}, '
                     f'points: [{pts2}]{d}),')
    lines += ['    ],', '    item_rows: [']
    for pos, count, note in rows:
        c = f', count: {count}' if count != 5 else ''
        lines.append(f'        (pos: {fmt(pos)}{c}), // {note}')
    lines += ['    ],', '    shortcuts: [']
    for s in shortcuts:
        path = ', '.join(fmt(p) for p in s['path'])
        lines.append(f'        // {s["note"]}')
        lines.append(f'        (path: [{path}], width: {fmt(s["width"])}, dir: {s["dir"]}),')
    lines += ['    ],']
    if battery:
        lines.append(f'    // {battery[1]}')
        lines.append(f'    golden_battery: (pos: {fmt(battery[0])}),')
    lines += ['    props: [']
    for p in props:
        if p.get('note'):
            lines.append(f'        // {p["note"]}')
        d = f', dir: {p["dir"]}' if p.get('dir', 'Both') != 'Both' else ''
        yaw = f', yaw: {fmt(round(float(p["yaw"]), 1))}' if p.get('yaw') else ''
        pos = tuple(round(float(c), 3) for c in p['pos'])
        lines.append(f'        (kind: "{p["kind"]}", pos: {fmt(pos)}{yaw}{d}),')
    lines += ['    ],', ')', '']
    with open(TRACK_OUT, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))


def write_decor(items):
    os.makedirs(os.path.dirname(DECOR_OUT), exist_ok=True)
    lines = ['// Wrench Works decorative models (physics-free, rc_render): wall dressing and',
             '// out-of-reach clutter. GENERATED by tools/blender/props/workshop/plan.py.',
             '(', '    decor: [']
    for d in items:
        if d.get('note'):
            lines.append(f'        // {d["note"]}')
        yaw = f', yaw: {fmt(round(float(d["yaw"]), 1))}' if d.get('yaw') else ''
        sc = f', scale: {fmt(float(d["scale"]))}' if d.get('scale', 1.0) != 1.0 else ''
        dd = f', dir: {d["dir"]}' if d.get('dir', 'Both') != 'Both' else ''
        pos = tuple(round(float(c), 3) for c in d['pos'])
        lines.append(f'        (model: "models/props/workshop/{d["model"]}.glb", '
                     f'pos: {fmt(pos)}{yaw}{sc}{dd}),')
    lines += ['    ],', ')', '']
    with open(DECOR_OUT, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))


# ---------------------------------------------------------------------------------------------

def main():
    pts, ix = route()
    gaps = [(ix['fwd_lip'], ix['fwd_land'], 'Fwd', 'jump off the west end of the workbench'),
            (ix['rev_land'], ix['rev_lip'], 'Rev', 'jump off the east end (reverse)')]
    rows = [((-12.0, 0.0, Z_N), 5, 'N lane, before the lifts'),
            ((BENCH_X, C.BENCH['top'], Z_M), 4, 'on the workbench top'),
            ((-38.0, 0.0, Z_S), 5, 'S lane, before the chicane'),
            ((48.0, 0.0, Z_S3), 5, 'S3 lane, before the wash bay'),
            ((X_W, C.MAT_H, -1.0), 5, 'W lane, between the mats')]
    gb = battery_pos(pts)
    battery = (gb, 'Golden Battery in the tyre alley behind the chicane\'s first apex: in through '
                   'a gap in the tyre wall, out through the next (forward layouts)')
    write_track(pts, ix, ground(), rows, props(pts, ix), gaps, battery, [])
    write_decor(decor())
    C.write_catalog()
    print(f'wrote {TRACK_OUT} ({len(pts)} points)')


if __name__ == '__main__':
    main()
