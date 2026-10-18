#!/usr/bin/env python3
"""Backyard Bash layout generator (pure Python, no Blender): the garden plan — lawn, patio, deck,
dirt path, puddles, raised beds, sandbox — the race line and the props, written to
assets/data/tracks/backyard_bash.ron; decorative (physics-free) dressing goes to
assets/data/decor/backyard_bash.ron (decor.py).

    python3 tools/blender/props/garden/plan.py
    cargo run -p rc_track --example plot -- backyard_bash [--rev] [--mirror]

World axes (Bevy): x east, z south (plots: north up), y up; 1 u = 22.5 cm. Heights: lawn 0,
dirt path 0.03, puddles 0.06, patio 0.05, raised beds and sand 1.0, deck 2.0.
Headings: degrees, 0 = north (-z), positive turns left (counter-clockwise from above).
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
TRACK_OUT = os.path.join(ROOT, 'assets', 'data', 'tracks', 'backyard_bash.ron')

# --- garden plan (u) ----------------------------------------------------------------------------
BOUNDS = ((-56.0, -64.0), (56.0, 64.0))           # fences east/west/south, house wall north
LAWN, DIRT, PUDDLE, PATIO, BED, DECK = 0.0, 0.03, 0.06, 0.05, 1.0, 2.0
DECK_POLY = 1.8                                   # hidden inside garden_deck (surface marker)
EAST_X, WEST_X, SOUTH_Z, NORTH_Z = 44.0, -44.0, 48.0, -54.0  # centrelines of the four sides
START_Z = 8.0                                     # start / finish line on the east side
DECK_EDGE_Z, DECK_WEST_X = -28.0, 14.0            # deck: x 14..56, z -64..-28
DECK_CENTRE = ((DECK_WEST_X + BOUNDS[1][0]) / 2, (BOUNDS[0][1] + DECK_EDGE_Z) / 2)
RAMP_RUN = 7.5                                    # deck ramps: 2.0 u rise over 7.5 u
RAMP_FOOT_Z = DECK_EDGE_Z + RAMP_RUN              # forward ramp foot (= reverse landing)
LANDING_X = DECK_WEST_X - RAMP_RUN                # forward landing (= reverse ramp foot)
NE_R, CORNER_R, JUNCTION_R = 12.0, 12.0, 9.0      # deck turn, lawn corners, return junction
DECK_RUN = 14.0                                   # deck straight before the turn (reverse run-up)
CHICANE_IN, CHICANE_R, CHICANE_DEG, CHICANE_MID = 9.5, 8.0, 45.0, 7.5
STICK_Z, LANE_GAP, LANE_W = -18.0, 14.0, 9.0      # lollipop outbound lane, lane spacing, width
BENCH_AT, BENCH_W, BENCH_D = 6.5, 7.0, 3.0        # bench along the outbound lane, passage, depth
STICK_LEN = 13.0                                  # outbound lane before the spiral
EDGING = 1.2                                      # garden_edging block length
SANDBOX_X = 0.0                                   # sandbox centre on the south side
SANDBOX_D, KICK_RUN, KICK_TOP = 6.2, 5.29, 1.18   # sandbox depth along the track, kicker run, lip
LOOP = [(0.4, 15.0), (0.3, 12.5), (0.3, 10.0)]   # spiral: share of the turn, radius
VEER_R = 12.0
WALL_T = 0.6                                      # rc_track barrier thickness
BED_END_Z = 22.0                                  # the west border bed ends here (behind the wall)
SW_R = 14.0                                       # SW corner radius
KERB_END_X = -15.0                                # the wall along the dirt path ends here


def rad(a):
    return math.radians(a)


def fwd(h):
    return (-math.sin(rad(h)), -math.cos(rad(h)))


def right_of(h):
    fx, fz = fwd(h)
    return (-fz, fx)


def yaw_facing(dx, dz):
    """Prop yaw (degrees) that turns its front (-Z) toward direction (dx, dz)."""
    return math.degrees(math.atan2(-dx, -dz))


class Path:
    """Turtle that records control points (with segment fields) and a dense polyline."""

    def __init__(self, x, z, h, y, width):
        self.x, self.z, self.h, self.y, self.w = x, z, h, y, width
        self.pts = []    # [x, y, z, fields, note]
        self.dense = []  # (x, y, z, heading, width)
        self.at = []     # dense index of every control point

    def mark(self, note='', **f):
        self.pts.append([self.x, self.y, self.z, dict(f), note])
        self.at.append(max(0, len(self.dense) - 1))
        if 'width' in f:
            self.w = f['width']
        return len(self.pts) - 1

    def set(self, **f):
        """Fields of the last point (they apply from it to the next point)."""
        self.pts[-1][3].update(f)
        if 'width' in f:
            self.w = f['width']

    def _move(self, x, z, h, y):
        self.dense.append((x, y, z, h, self.w))
        self.x, self.z, self.h, self.y = x, z, h, y

    def straight(self, length, y1=None, at=(), note=''):
        """Straight; `at` = distances (from the start) for extra control points; the end gets
        a point with `note`. Heights change linearly to y1. Returns the end point index."""
        x0, z0, y0 = self.x, self.z, self.y
        y1 = y0 if y1 is None else y1
        dx, dz = fwd(self.h)
        n = max(1, int(length / 0.5))
        marks = sorted(a for a in at if 0.5 < a < length - 0.5)
        for i in range(1, n + 1):
            s0, s1 = (i - 1) / n * length, i / n * length
            for a in marks:
                if s0 < a <= s1:
                    self._move(x0 + dx * a, z0 + dz * a, self.h, y0 + (y1 - y0) * a / length)
                    self.mark()
            self._move(x0 + dx * s1, z0 + dz * s1, self.h, y0 + (y1 - y0) * s1 / length)
        return self.mark(note)

    def arc(self, radius, ang, step=22.5, note=''):
        """Arc of `ang` degrees (positive = left) with control points every <= `step`.
        Returns (centre, index of the last point)."""
        side = 1 if ang > 0 else -1
        lx, lz = fwd(self.h + 90 * side)
        cx, cz = self.x + lx * radius, self.z + lz * radius
        h0 = self.h
        k = max(1, math.ceil(abs(ang) / step))
        per = max(2, int(abs(rad(ang)) * radius / 0.5 / k))
        for j in range(1, k + 1):
            for i in range(1, per + 1):
                h = h0 + ang * ((j - 1) + i / per) / k
                ox, oz = fwd(h + 90 * side)
                self._move(cx - ox * radius, cz - oz * radius, h, self.y)
            last = self.mark(note if j == k else '')
        return (cx, cz), last

    def straight_to(self, axis, target, **kw):
        """Straight along the current (axis-aligned) heading until x or z = target."""
        dx, dz = fwd(self.h)
        d = (target - self.x) / dx if axis == 'x' else (target - self.z) / dz
        assert d > 0.5, f'straight_to {axis}={target}: {d:.2f}'
        return self.straight(d, **kw)

    def offset(self, i0, i1, side, extra):
        """Polyline of dense samples i0..i1 pushed `side` (+1 right, -1 left) by half width +
        `extra` (x, z)."""
        out = []
        for x, _, z, h, w in self.dense[i0:i1 + 1]:
            rx, rz = right_of(h)
            d = side * (w / 2 + extra)
            out.append((x + rx * d, z + rz * d))
        return out

    def pose(self, k, lateral=0.0, ahead=0.0):
        """(x, z, heading) of dense sample k moved `lateral` to the right and `ahead`."""
        x, _, z, h, _ = self.dense[k % len(self.dense)]
        rx, rz = right_of(h)
        fx, fz = fwd(h)
        return x + rx * lateral + fx * ahead, z + rz * lateral + fz * ahead, h


def loop_turns(alpha):
    """The spiral's arcs: veer left, decreasing-radius right turn, veer left."""
    total = 180 + 2 * alpha
    return [(VEER_R, alpha)] + [(r, -total * f) for f, r in LOOP] + [(VEER_R, alpha)]


def solve_alpha(gap):
    """Veer angle that puts the return lane `gap` u to the right of the outbound lane."""
    def offset(a):
        p = Path(0.0, 0.0, -90.0, 0.0, 10.0)
        for r, ang in loop_turns(a):
            p.arc(r, ang)
        return p.z
    lo, hi = 0.0, 80.0
    for _ in range(60):
        mid = (lo + hi) / 2
        lo, hi = (mid, hi) if offset(mid) > gap else (lo, mid)
    return (lo + hi) / 2


# --- the race line ------------------------------------------------------------------------------

def build():
    """Forward layout: start on the east lawn heading north, up the deck, drop onto the patio,
    flower-pot chicane, hairpin into the lollipop (bench, hose-reel spiral), west lawn, south
    dirt path (sandbox jump, puddles), back to the start. Returns (path, named indices)."""
    p = Path(EAST_X, START_Z, 0.0, LAWN, 10.0)
    i = {}
    p.mark('start / finish (east lawn, heading north)', width=10, surface='Grass',
           wall_left=False, wall_right=False, offroad_margin=6)
    p.straight_to('z', RAMP_FOOT_Z + 1.5, at=(12,), note='before the deck ramp')
    i['ramp_foot'] = p.straight(1.5, note='deck ramp foot (forward ramp up; reverse landing)')
    p.set(surface='Wood')
    i['deck_edge'] = p.straight_to('z', DECK_EDGE_Z, y1=DECK,
                                   note='deck edge (forward ramp top; reverse lip)')
    p.straight(1.5, note='on the deck')
    p.set(offroad_margin=10)  # the deck is fenced by the house and the fence: wide run-off
    p.straight(DECK_RUN - 1.5, note='deck turn entry')
    i['ne_centre'], _ = p.arc(NE_R, 90, note='deck turn exit')
    p.straight_to('x', DECK_WEST_X + 1.5, at=(6.3,), note='deck straight')
    i['deck_lip'] = p.straight(1.5, note='deck west edge: forward lip')
    i['patio_landing'] = p.straight_to('x', LANDING_X, y1=PATIO,
                                       note='patio: forward landing (reverse ramp foot)')
    p.set(surface='Stone', wall_left=True, offroad_margin=6)
    p.straight(1.5, note='patio (low border wall on the lawn side)')
    # flower-pot chicane on the patio: a 10 u jog to the left between two pot clusters
    p.straight(CHICANE_IN - 1.5, note='chicane entry')
    p.set(width=9)
    i['kink1'], _ = p.arc(CHICANE_R, CHICANE_DEG, note='chicane kink 1')
    i['kink1_k'] = len(p.dense) - 1
    p.straight(CHICANE_MID, note='chicane kink 2')
    i['kink2'], _ = p.arc(CHICANE_R, -CHICANE_DEG, note='chicane exit')
    i['kink2_k'] = len(p.dense) - 1
    p.set(width=10)
    hairpin_r = (STICK_Z - p.z) / 2
    i['hairpin_in'] = p.straight_to('x', WEST_X + hairpin_r, note='hairpin entry (patio west end)')
    p.set(surface='Grass', wall_right=True, offroad_margin=4)
    i['hairpin_centre'], i['outbound'] = p.arc(hairpin_r, 180,
                                               note='hairpin exit: outbound lane (heading east)')
    p.set(width=LANE_W)
    p.straight(BENCH_AT - BENCH_D / 2, y1=LAWN, note='bench: its end frames are the walls')
    p.set(width=BENCH_W, wall_left=False, wall_right=False)
    i['bench_k'] = len(p.dense) - 1 + round(BENCH_D)  # mid-bench (dense samples are 0.5 u)
    p.straight(BENCH_D, note='bench exit')
    p.set(width=LANE_W, wall_left=True, wall_right=True)
    i['veer_out'] = p.straight(STICK_LEN - BENCH_AT - BENCH_D / 2, note='spiral entry')
    alpha = solve_alpha(LANE_GAP)
    turns = loop_turns(alpha)
    p.arc(*turns[0], note='veer out')
    i['spiral'] = []
    for r, ang in turns[1:-1]:
        c, _ = p.arc(r, ang, note=f'spiral R{r:g}')
        i['spiral'].append((c, r, len(p.dense) - 1))
    i['veer_in'] = p.arc(*turns[-1], note='veer in: return lane (heading west)')[1]
    # the loop's outer wall runs on along the return lane, so in reverse its end is not at the
    # apex of the entry veer (cars cutting it slipped outside the loop)
    p.straight(8.0, note='outer wall ends')
    p.set(wall_left=False)
    i['junction'] = p.straight_to('x', WEST_X + JUNCTION_R, note='return junction')
    p.arc(JUNCTION_R, 90, note='west lawn (heading south)')
    p.set(width=10)
    # the bed ends behind the wall; the wall itself runs on round the outside of the SW corner and
    # along the dirt path's south edge (a kerb to the vegetable patch), so cars sliding wide out of
    # the corner (reverse) glance along it instead of hitting an exposed bed end
    i['bed_end'] = p.straight_to('z', BED_END_Z, note='west bed ends (the wall runs on)')
    p.set(offroad_margin=6)
    i['sw_in'] = p.straight_to('z', SOUTH_Z - SW_R, note='SW corner entry')
    p.arc(SW_R, 90, note='south dirt path (heading east)')
    p.set(surface='Dirt')
    x_foot = SANDBOX_X - SANDBOX_D / 2 - KICK_RUN
    p.straight_to('x', KERB_END_X, y1=DIRT, note='veg-patch kerb ends')
    p.set(wall_right=False)
    p.straight_to('x', x_foot - 1.2, note='sandbox run-up')
    p.set(width=9)
    p.straight(1.2, note='kicker foot (reverse: landing slope foot)')
    i['sand_lip'] = p.straight(KICK_RUN, y1=KICK_TOP, note='sandbox jump lip (both directions)')
    i['sand_landing'] = p.straight(SANDBOX_D, note='sandbox jump landing (reverse: lip)')
    p.straight(KICK_RUN, y1=DIRT, note='landing kicker foot')
    p.straight(1.2, note='dirt path (puddles)')
    p.set(width=10)
    i['se_in'] = p.straight_to('x', EAST_X - CORNER_R, at=(8,), note='SE corner entry')
    p.arc(CORNER_R, 90, note='east lawn (heading north)')
    p.set(surface='Grass')
    p.straight_to('z', START_Z + 14, y1=LAWN, note='start straight (the loop closes to point 0)')
    i['alpha'] = alpha
    return p, i


# --- ground --------------------------------------------------------------------------------------

def blob(cx, cz, rx, rz, n=14, seed=0.0, wobble=0.18):
    """Organic closed outline (puddles): an ellipse with a smooth deterministic wobble."""
    pts = []
    for k in range(n):
        a = 2 * math.pi * k / n
        r = 1 + wobble * math.sin(3 * a + seed) * math.cos(2 * a - seed * 0.7)
        pts.append((cx + rx * r * math.cos(a), cz + rz * r * math.sin(a)))
    return pts


PUDDLES = ((14.5, SOUTH_Z + 1.5, 3.2, 2.0), (21.0, SOUTH_Z - 1.8, 2.6, 1.7),
           (26.5, SOUTH_Z + 2.4, 2.2, 1.4))  # centre x, z, radii (puddle_rims.py matches them)


def puddle(k, grow=1.0):
    """Outline of puddle k (x, z), optionally grown about its centre."""
    cx, cz, rx, rz = PUDDLES[k]
    return blob(cx, cz, rx * grow, rz * grow, seed=1.7 * k + 0.4)


def thin(pts, step=1.5):
    """Drop polyline points closer than `step` to the previous kept one."""
    out = [pts[0]]
    for q in pts[1:]:
        if math.dist(q, out[-1]) >= step:
            out.append(q)
    return out


def ground(p, i):
    (x0, z0), (x1, z1) = BOUNDS
    g = [('Grass', LAWN, [(x0, z0), (x1, z0), (x1, z1), (x0, z1)], 'the lawn'),
         ('Stone', PATIO, [(x0, z0), (DECK_WEST_X + 1, z0), (DECK_WEST_X + 1, -38), (-20, -38),
                           (-26, -36), (x0, -36)], 'patio slabs along the house'),
         ('Wood', DECK_POLY, [(DECK_WEST_X, z0), (x1, z0), (x1, DECK_EDGE_Z),
                         (DECK_WEST_X, DECK_EDGE_Z)], 'raised deck (garden_deck holds it up)'),
         ('Dirt', DIRT, [(-36, SOUTH_Z - 6.5), (36, SOUTH_Z - 6.5), (38, SOUTH_Z + 7),
                         (-38, SOUTH_Z + 7)], 'dirt path by the vegetable patch')]
    # the west border bed: behind the right-hand wall from the hairpin to the SW corner, minus
    # the spiral's island (lawn); its east end is closed by a sleeper
    side = bed_edge(p, i)
    bed = thin(side) + [(x0, side[-1][1]), (x0, side[0][1])]
    g.append(('Dirt', BED, bed, 'west border bed and lane divider (raised, stone edging)'))
    # the corner bed inside the hairpin, closed by a sleeper on its east side
    hp = thin(p.offset(p.at[i['hairpin_in']], p.at[i['outbound']], -1, WALL_T / 2))
    g.append(('Dirt', BED, hp, 'corner bed inside the hairpin (raised)'))
    # puddles on the dirt path after the sandbox jump (forward)
    for k, (cx, cz, rx, rz) in enumerate(PUDDLES):
        g.append(('Water', PUDDLE, puddle(k), f'puddle {k + 1}'))
    # sand in the sandbox (inner faces of the boards)
    sx, sz = SANDBOX_D / 2 - 0.18, 5.35 - 0.18
    g.append(('Dirt', BED, [(SANDBOX_X - sx, SOUTH_Z - sz), (SANDBOX_X + sx, SOUTH_Z - sz),
                            (SANDBOX_X + sx, SOUTH_Z + sz), (SANDBOX_X - sx, SOUTH_Z + sz)],
              'sand in the sandbox'))
    return g


def bed_edge(p, i):
    """Right-hand wall line (wall centre) from the hairpin apex past the spiral to the SW
    corner, skipping the island: the outline of the west border bed."""
    k_in, k_out = p.at[i['hairpin_in']], p.at[i['veer_out']]
    k_apex = k_in + (p.at[i['outbound']] - k_in) // 2
    k_back, k_sw = p.at[i['veer_in']], p.at[i['bed_end']]
    return p.offset(k_apex, k_out, +1, WALL_T / 2) + p.offset(k_back, k_sw, +1, WALL_T / 2)


def edging_row(a, b):
    """garden_edging blocks along the segment a -> b (x, z): (x, z, yaw) per block."""
    n = max(1, math.ceil(math.dist(a, b) / EDGING - 0.05))
    yaw = yaw_facing(b[1] - a[1], -(b[0] - a[0]))  # front faces sideways: long side along a-b
    return [(a[0] + (b[0] - a[0]) * (k + 0.5) / n, a[1] + (b[1] - a[1]) * (k + 0.5) / n, yaw)
            for k in range(n)]


# --- props ---------------------------------------------------------------------------------------

def props(p, i):
    out = []

    def add(kind, x, y, z, yaw=0.0, d='Both', note=''):
        out.append((kind, x, y, z, yaw, d, note))

    dx, dz = DECK_CENTRE
    add('garden_deck', dx, 0, dz, 0, note='deck block (top = the Wood ground polygon)')
    add('garden_deck_ramp', EAST_X, 0, DECK_EDGE_Z + RAMP_RUN / 2, 0, 'Fwd',
        'forward ramp up the deck (reverse: the drop zone)')
    add('garden_deck_ramp', DECK_WEST_X - RAMP_RUN / 2, 0, NORTH_Z, -90, 'Rev',
        'reverse ramp up the deck (forward: the landing)')
    # the barbecue stands inside the deck turn (south-west quadrant): the east and north-east of
    # the deck are run-off for cars leaving the turn (wide in one direction or the other)
    add('garden_bbq', 25.0, DECK, -38.5, yaw_facing(1, -1), note='on the deck, inside the turn')
    add('garden_fairy_lights', 24.0, DECK, DECK_EDGE_Z - 1.2, 0, note='along the deck edge')
    # bench across the outbound lane, facing back toward the hairpin
    bx, bz, bh = p.pose(i['bench_k'])
    add('garden_bench', bx, 0, bz, bh + 180, note='the track passes under the seat')
    # sandbox jump: kickers lean on the rims
    add('garden_sandbox', SANDBOX_X, 0, SOUTH_Z, 90, note='long side across the track')
    run = SANDBOX_D / 2 + KICK_RUN / 2
    add('garden_kicker', SANDBOX_X - run, 0, SOUTH_Z, -90, note='west kicker (rises east)')
    add('garden_kicker', SANDBOX_X + run, 0, SOUTH_Z, 90, note='east kicker (rises west)')
    # hose reel in the middle of the spiral's island
    (cx, cz), _, _ = i['spiral'][1]
    add('garden_hose_reel', cx, 0, cz, 30, note='centre of the spiral')
    # flower-pot chicane: a cluster on the inside of each kink (outside the corridor)
    for key, side in (('kink1_k', -1), ('kink2_k', +1)):
        x, z, h = p.pose(i[key] - 8, side * 9.0)
        add('garden_pot_cluster', x, PATIO, z, h + 90 * side, note='chicane marker')
    add('garden_shed', 46.5, 0, 56.0, yaw_facing(-1, -1), note='SE corner, door to the garden')
    for x in (-36.0, -22.0, -8.0, 20.0):
        add('garden_veg_bed', x, LAWN, 59.6, 0, note='vegetable patch' if x < -30 else '')
    add('garden_round_bed', -8.0, LAWN, 21.0, 0, note='round bed with a Japanese maple')
    # pots on the deck and along the house wall on the patio (outside the corridors)
    for x, z, kind in ((17, -31, 'large'), (30.5, -31.5, 'medium'), (15.6, -62.8, 'medium')):
        add(f'garden_pot_{kind}', x, DECK, z, note='deck pots' if x == 17 and z < -40 else '')
    for x, z, kind in ((-30, -62, 'large'), (-26.5, -62.3, 'medium'), (-6, -62.3, 'medium'),
                       (-50, -62, 'large')):
        add(f'garden_pot_{kind}', x, PATIO, z, note='patio pots' if x == -30 else '')
    add('garden_porch_light', 21.2, 1.0, BOUNDS[0][1], 180, note='beside the kitchen door')
    # knockables near the line (light: cars barely deflect)
    for d, lat in ((-4, 4.4), (0, 5.0), (5, 4.2)):
        x, z, _ = p.pose(i['kink2_k'] - 8 + d * 2, lat)
        add('garden_pot_small', x, PATIO, z, note='chicane apex pots' if d == -4 else '')
    for x, z in ((22.4, -37.0), (27.6, -36.4)):
        add('garden_pot_small', x, DECK, z, note='by the barbecue' if x < 25 else '')
    add('garden_football', 49.6, LAWN, 18.0, note='footballs')
    add('garden_football', -2.0, LAWN, 13.5)
    for k, (x, z) in enumerate(((-12, 32), (6, 30), (-40.5, 27), (30, 10))):
        add('garden_tennis_ball', x, LAWN, z, note='tennis balls' if k == 0 else '')
    add('garden_bucket', -4.5, DIRT, 55.4, note='buckets')
    add('garden_bucket', 28.0, LAWN, 56.2)
    add('garden_watering_can', 13.0, DIRT, 55.2, yaw=60.0)
    (cx, cz), _, _ = i['spiral'][1]
    for x, y, z, note in ((cx + 2.4, LAWN, cz + 1.2, 'gnomes'), (-13.6, LAWN, 24.0, ''),
                          (-14.5, PATIO, -61.6, '')):
        add('garden_gnome', x, y, z, yaw=180.0, note=note)
    # close the pocket between the patio, the bench lane and the spiral (a dead end): a pot
    # cluster south of the deck drop's landing and edging from the deck's corner to the spiral
    add('garden_pot_cluster', 10.6, PATIO, -46.4, 200, note='closes the pocket by the landing')
    corner = (DECK_WEST_X + 0.3, DECK_EDGE_Z + 0.3)
    outer = p.offset(p.at[i['veer_out']], p.at[i['veer_in']], -1, WALL_T / 2)
    near = min(outer, key=lambda q: math.dist(q, corner))
    note = 'deck corner to the spiral wall'
    for x, z, yaw in edging_row(corner, near):
        add('garden_edging', x, LAWN, z, yaw, note=note)
        note = ''
    # edging blocks on the open ends of the raised beds
    (x0, _), _ = BOUNDS
    side = bed_edge(p, i)
    k_out = p.at[i['veer_out']] - (p.at[i['hairpin_in']] + (p.at[i['outbound']]
                                                             - p.at[i['hairpin_in']]) // 2)
    hp = p.offset(p.at[i['hairpin_in']], p.at[i['outbound']], -1, WALL_T / 2)
    for a, b, note in ((side[0], (x0, side[0][1]), 'west bed, north end'),
                       (side[-1], (x0, side[-1][1]), 'west bed, south end (behind the wall)'),
                       (side[k_out], side[k_out + 1], 'lane divider, east end'),
                       (hp[0], hp[-1], 'corner bed inside the hairpin, east side')):
        for x, z, yaw in edging_row(a, b):
            add('garden_edging', x, LAWN, z, yaw, note=note)
            note = ''
    return out


def item_rows(p, i):
    _, _, k = i['spiral'][1]
    sx, sz, _ = p.pose(k - 12)
    return [((EAST_X, LAWN, -8.0), 'start straight'),
            ((25.0, DECK, NORTH_Z), 'deck straight'),
            ((sx, LAWN, sz), 'hose-reel spiral'),
            ((WEST_X, LAWN, 20.0), 'west lawn'),
            ((-20.0, DIRT, SOUTH_Z), 'sandbox run-up')]


# --- output --------------------------------------------------------------------------------------

def fmt(v):
    s = f'{v:.2f}'.rstrip('0').rstrip('.')
    return '0' if s in ('-0', '') else s


def write_track(p, i):
    lines = []
    for k, (x, y, z, f, note) in enumerate(p.pts):
        fields = [f'pos: ({fmt(x)}, {fmt(y)}, {fmt(z)})']
        for key in ('width', 'surface', 'wall_left', 'wall_right', 'offroad_margin'):
            if key in f:
                v = f[key]
                fields.append(f'{key}: {"true" if v is True else "false" if v is False else v}')
        lines.append(f'        ({", ".join(fields)}), // {k}' + (f': {note}' if note else ''))
    gaps = [(i['deck_lip'], i['patio_landing'], 'Fwd', 'drop off the deck onto the patio'),
            (i['ramp_foot'], i['deck_edge'], 'Rev', 'reverse: drop off the deck onto the lawn'),
            (i['sand_lip'], i['sand_landing'], 'Both', 'over the sandbox (kickers both sides)')]
    grounds = [f'        (surface: {s}, height: {fmt(h)}, points: ['
               + ', '.join(f'({fmt(x)}, {fmt(z)})' for x, z in pts) + f']), // {n}'
               for s, h, pts, n in ground(p, i)]
    prop_lines = []
    for kind, x, y, z, yaw, d, note in props(p, i):
        yaw = (yaw + 180) % 360 - 180
        extra = f', dir: {d}' if d != 'Both' else ''
        prop_lines.append(f'        (kind: "{kind}", pos: ({fmt(x)}, {fmt(y)}, {fmt(z)}), '
                          f'yaw: {fmt(yaw)}{extra}),' + (f' // {note}' if note else ''))
    rows = [f'        (pos: ({fmt(x)}, {fmt(y)}, {fmt(z)})), // {n}'
            for (x, y, z), n in item_rows(p, i)]
    out = [
        '// Backyard Bash — back garden (GDD §7.1, art bible §3.4). ~460 u, 5 laps, outdoor ★★.',
        '// GENERATED by tools/blender/props/garden/plan.py — edit that and re-run; plot:',
        '//   cargo run -p rc_track --example plot -- backyard_bash [--rev] [--mirror]',
        '// Invisible ribbon: lawn (0), dirt path (0.03), puddles (0.06), patio (0.05), raised beds',
        '// and sand (1.0) and the deck (2.0) are ground polygons; ramps, kickers, pots, the bench',
        '// and the hose reel are props. Forward (counter-clockwise from above): east lawn start →',
        '// ramp up the deck, drop onto the patio → flower-pot chicane → hairpin into the',
        '// lollipop: under the garden bench → hose-reel spiral → west lawn → south dirt path:',
        '// sandbox jump, puddles → east lawn. Reverse: the same loop, ramp up from the patio and',
        '// drop off the deck onto the east lawn just before the line.',
        '(',
        '    style: Invisible,',
        '    start: 0,',
        '    kill_height: -6,',
        '    wall_height: 1.2,',
        f'    bounds: (min: ({fmt(BOUNDS[0][0])}, {fmt(BOUNDS[0][1])}), '
        f'max: ({fmt(BOUNDS[1][0])}, {fmt(BOUNDS[1][1])})),',
        '    points: [', *lines, '    ],',
        '    gaps: [', *[f'        (from: {a}, to: {b}, dir: {d}), // {n}' for a, b, d, n in gaps],
        '    ],',
        '    ground: [', *grounds, '    ],',
        '    item_rows: [', *rows, '    ],',
        '    // Above the sandbox, off the line: take the jump on the outer edge of the kicker.',
        f'    golden_battery: (pos: ({fmt(SANDBOX_X + 0.5)}, 1.9, {fmt(SOUTH_Z + 3.8)})),',
        '    props: [', *prop_lines, '    ],',
        ')',
        '',
    ]
    with open(TRACK_OUT, 'w', encoding='utf-8') as fh:
        fh.write('\n'.join(out))


def main():
    p, i = build()
    gap = math.hypot(p.x - EAST_X, p.z - START_Z)
    length = sum(math.dist(a[:3], b[:3]) for a, b in zip(p.dense, p.dense[1:])) + gap
    print(f'spiral veer {i["alpha"]:.2f} deg; closure {gap:.2f} u (straight); path ~{length:.0f} u;'
          f' {len(p.pts)} points')
    write_track(p, i)
    print(f'wrote {os.path.relpath(TRACK_OUT, ROOT)}')


if __name__ == '__main__':
    main()
