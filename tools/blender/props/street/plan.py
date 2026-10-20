"""Maple Street layout generator (pure Python): the street plan — kerbs, driveways, ground
polygons, the raised yard, the storm drain, dressing — and the race line, written to
assets/data/tracks/maple_street.ron; also writes the prop catalog (catalog.py + the kerb arcs
this plan needs) to assets/data/props/street.ron.

    python3 tools/blender/props/street/plan.py
    cargo run -p rc_track --example plot -- maple_street [--rev] [--mirror]

World axes (Bevy): x east, z south (plots: north up), y up; 1 u = 22.5 cm. Heights: road 0,
lawns 0.6, sidewalks 0.62, driveways 0.64, kerb tops 0.65, raised yard 3.0, drain bottom -2.
Angles of arcs: degrees from +x toward +z (clockwise in the plots).

Since the track editor (docs/design/rc_editor.md) the track and decor files are editor-owned: this
script writes them only with --layout, which replaces every edit made in the editor.
"""
import math
import os
import sys

import catalog as C

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(C.ROOT, 'assets', 'data', 'tracks', 'maple_street.ron')

# ---------------------------------------------------------------------------------------------
# street plan (u)
KB = C.KERB_D
MAPLE_N, MAPLE_S = -20.0, 20.0           # Maple Street kerb faces
STRIP, WALK = 8.0, 8.0                   # planting strip and sidewalk widths behind the kerb
WALK_C = KB + STRIP + WALK / 2           # sidewalk centre behind a kerb face
XC, RB, RF, RI = 260.0, 46.0, 12.0, 14.0  # cul-de-sac centre x, bulb / fillet / island radii
BIRCH_W, MED_W, MED_E, BIRCH_E = -36.0, -8.0, 8.0, 36.0  # Birch Lane boulevard kerb faces
RC = 16.0                                # corner kerb radius
MED_N, MED_S = 48.0, 146.0               # median nose centres (z); tips 8 u beyond
BOUNDS = ((-100.0, -110.0), (375.0, 220.0))
APRON_BULB = 124.0                       # driveway apron on the bulb (angle)
DRAIN_Z = 115.0                          # storm drain in the median's east gutter
CAR_Z = 82.0                             # SUV at Birch's east kerb, driven under after the hop
CORNER_SE, CORNER_SW = (BIRCH_E + RC, MAPLE_S + RC), (BIRCH_W - RC, MAPLE_S + RC)
TURN_C, TURN_R = (BIRCH_E + WALK_C + 9.0, MAPLE_S + WALK_C + 9.0), 9.0  # sidewalk corner turn
HOP_TURN_R = 11.0                        # turn between the Birch sidewalk and the curb hop
BEYOND = 120.0                           # lawn plates reach this far beyond the bounds (decor)
GATE_A = (79.0, 90.0)                    # Maple-side opening of the corner lot wall (x range)
GATE_B = (58.0, 67.0)                    # Birch-side opening (z range)

# fillet between the straight kerbs and the bulb
FX = XC - math.sqrt((RB + RF) ** 2 - (RF + MAPLE_S) ** 2)
FILLET_A = math.degrees(math.atan2(RF + MAPLE_S, XC - FX))  # bulb tangent angle offset from 180

# raised yard (plateau) and its ramps, forward = westbound
YARD_Z = 48.0
PA, PB, PC, PD, PE, PF = 195.0, 183.0, 175.0, 161.0, 153.0, 141.0
LAWN, YARD, TOP = C.LAWN_H, 3.0, 3.8
YARD_N, YARD_S = YARD_Z - 7.0, YARD_Z + 7.0

# parked cars the track drives under
SUV_TUNNEL = (165.0, MAPLE_N + 0.5 + C.SUV['width'] / 2)  # along the north kerb (lengthwise)
SUV_CROSS = (BIRCH_E - C.ROLLED_IN - 0.3 - C.SUV['width'] / 2, CAR_Z)  # across, after the hop

# houses (prop kind, x, z, driveway x) facing Maple Street from the north (yaw 180); garages:
# house_a +18 u, house_b +20 u along their local +X (mirrored *_m: -X), front walls 12 u ahead
NORTH_HOUSES = [('street_house_b', -60.0, -85.0, -80.0), ('street_house_a', 12.0, -86.0, -6.0),
                ('street_house_b_m', 88.0, -85.0, 108.0), ('street_house_a', 162.0, -86.0, 144.0)]
CORNER_DRIVE = 82.0                                     # corner lot driveway (= Maple-side gate)
CORNER_HOUSE = ('street_house_a_m', 100.0, 100.0, 0.0)  # faces Maple, garage at CORNER_DRIVE
BULB_HOUSE = ('street_house_a', XC + 110 * math.cos(math.radians(APRON_BULB)),
              110 * math.sin(math.radians(APRON_BULB)),
              math.degrees(math.atan2(math.cos(math.radians(APRON_BULB)),
                                      math.sin(math.radians(APRON_BULB)))))
BIRCH_HOUSE = ('street_house_a', 98.0, 172.0, 90.0)     # faces Birch Lane (west)


def editor_owned(path):
    """Track and decor files are editor-owned (docs/design/rc_editor.md): this script writes them
    only with --layout, which replaces every edit made in the editor with its own layout."""
    if '--layout' in sys.argv:
        return False
    print(f'skipped {path}: editor-owned (cargo run -p rc_editor); --layout overwrites it')
    return True


def rad(a):
    return math.radians(a)


def pol(c, r, a):
    return (c[0] + r * math.cos(rad(a)), c[1] + r * math.sin(rad(a)))


def yaw_facing(dx, dz):
    """Prop yaw (degrees) that turns its front (-Z) toward direction (dx, dz)."""
    return math.degrees(math.atan2(-dx, -dz))


def arc_pts(c, r, a0, a1, step=2.0):
    n = max(1, math.ceil(abs(rad(a1 - a0)) * r / step))
    return [pol(c, r, a0 + (a1 - a0) * i / n) for i in range(n + 1)]


def r1(v):
    return round(v, 2)


def local_to_world(x, z, yaw, lx, lz):
    """Prop-space (lx, lz) of a prop at (x, z) with `yaw` (degrees) -> world (x, z)."""
    c, s_ = math.cos(rad(yaw)), math.sin(rad(yaw))
    return (x + lx * c + lz * s_, z - lx * s_ + lz * c)


def garage_door(kind, x, z, yaw):
    """World corners of the garage door (driveway end) of a house prop placement."""
    gx = {'street_house_a': 18.0, 'street_house_a_m': -18.0, 'street_house_b': 20.0,
          'street_house_b_m': -20.0}[kind]
    return local_to_world(x, z, yaw, gx + 7, -12.0), local_to_world(x, z, yaw, gx - 7, -12.0)


# ---------------------------------------------------------------------------------------------
# accumulators

class Plan:
    def __init__(self):
        self.props = []      # (kind, (x, y, z), yaw, dir, comment)
        self.ground = []     # (surface, height, [(x, z)], comment)
        self.arcs = {}       # kerb arc kind -> (radius, span, face)

    def prop(self, kind, x, y, z, yaw=0.0, dir='Both', note=None):
        self.props.append((kind, (x, y, z), yaw, dir, note))

    def poly(self, surface, height, pts, note):
        self.ground.append((surface, height, [(r1(x), r1(z)) for x, z in pts], note))

    # kerbs ------------------------------------------------------------------------------------
    def kerb_line(self, p0, p1, road_left, kind='street_kerb_', aprons=()):
        """Straight kerb pieces (4/2/1 u, the last one may overlap) from p0 to p1 with the road
        on the left (or right) of the walking direction; `aprons` = distances along the line
        where a driveway apron (20 u incl. flares) replaces the kerb."""
        dx, dz = p1[0] - p0[0], p1[1] - p0[1]
        length = math.hypot(dx, dz)
        tx, tz = dx / length, dz / length
        nx, nz = (tz, -tx) if road_left else (-tz, tx)  # toward the road
        yaw = yaw_facing(nx, nz)
        half = C.APRON_W / 2 + C.APRON_FLARE
        cuts = sorted((a - half, a + half) for a in aprons)
        for a in aprons:
            self.prop('street_apron', p0[0] + tx * a, 0.0, p0[1] + tz * a, yaw)
        starts = [0.0] + [b for _, b in cuts]
        ends = [a for a, _ in cuts] + [length]
        for s0, s1 in zip(starts, ends):
            s = s0
            while s < s1 - 0.05:
                rest = s1 - s
                n = 4 if rest > 3.0 else 2 if rest > 1.5 else 1
                mid = s + n / 2 if rest >= n else s1 - n / 2
                self.prop(f'{kind}{n}', p0[0] + tx * mid, 0.0, p0[1] + tz * mid, yaw)
                s += n

    def rolled_line(self, p0, p1, road_left):
        """Rolled kerb (4 u pieces), toe 1 u in front of the kerb face line."""
        dx, dz = p1[0] - p0[0], p1[1] - p0[1]
        length = math.hypot(dx, dz)
        tx, tz = dx / length, dz / length
        nx, nz = (tz, -tx) if road_left else (-tz, tx)
        yaw = yaw_facing(nx, nz)
        n = max(1, round(length / 4))
        step = length / n
        for i in range(n):
            mid = step * (i + 0.5)
            self.prop('street_kerb_rolled', p0[0] + tx * mid, 0.0, p0[1] + tz * mid, yaw)

    def kerb_arc(self, c, r, a0, a1, face, max_len=80.0):
        """Arc kerb props (face 'out': road outside the circle; 'in': road inside); arcs
        longer than `max_len` u are split into equal pieces sharing one model."""
        n = max(1, math.ceil(rad(abs(a1 - a0)) * r / max_len))
        span = abs(a1 - a0) / n
        kind = f'street_kerb_arc_{r:g}_{span:.1f}{face}'.replace('.', 'p')
        self.arcs[kind] = (r, round(span, 1), face)
        for i in range(n):
            am = a0 + (a1 - a0) * (i + 0.5) / n
            m = pol(c, r, am)
            ux, uz = math.cos(rad(am)), math.sin(rad(am))
            nx, nz = (ux, uz) if face == 'out' else (-ux, -uz)
            self.prop(kind, m[0], 0.0, m[1], yaw_facing(nx, nz))


def kerb_back_arc(c, r, a0, a1, face, step=2.0):
    """Kerb back edge of an arc kerb (for ground polygons)."""
    return arc_pts(c, r - C.KERB_D if face == 'out' else r + C.KERB_D, a0, a1, step)


# ---------------------------------------------------------------------------------------------
# the street


def build_streets(pl):
    (x0, z0), (x1, z1) = BOUNDS
    kb = KB
    bulb_a0 = 180 + FILLET_A           # NW tangent point on the bulb
    bulb_a1 = 540 - FILLET_A           # SW tangent point (> 360)
    half_gap = math.degrees((C.APRON_W / 2 + C.APRON_FLARE) / RB)
    ap0, ap1 = APRON_BULB - half_gap, APRON_BULB + half_gap

    # Maple north kerb: west edge -> NW fillet -> bulb -> SW fillet -> south kerb
    pl.kerb_line((x0, MAPLE_N), (FX, MAPLE_N), road_left=False,
                 aprons=[h[3] - x0 for h in NORTH_HOUSES])
    pl.kerb_arc((FX, MAPLE_N - RF), RF, 90, FILLET_A, 'out')
    pl.kerb_arc((XC, 0), RB, bulb_a0, ap0 + 360, 'in')
    pl.kerb_arc((XC, 0), RB, ap1, bulb_a1 - 360, 'in')
    pl.kerb_arc((FX, MAPLE_S + RF), RF, 360 - FILLET_A, 270, 'out')
    pl.kerb_line((FX, MAPLE_S), (CORNER_SE[0], MAPLE_S), road_left=False,
                 aprons=[FX - CORNER_DRIVE])
    pl.kerb_arc(CORNER_SE, RC, 180, 270, 'out')
    pl.kerb_line((CORNER_SW[0], MAPLE_S), (x0, MAPLE_S), road_left=False)
    pl.kerb_arc(CORNER_SW, RC, 270, 360, 'out')
    # driveway apron on the bulb + the island
    a = pol((XC, 0), RB, APRON_BULB)
    pl.prop('street_apron', a[0], 0.0, a[1],
            yaw_facing(-math.cos(rad(APRON_BULB)), -math.sin(rad(APRON_BULB))),
            note='cul-de-sac driveway apron (the exit)')
    pl.kerb_arc((XC, 0), RI, 0, 360, 'out')

    # Birch Lane boulevard: rolled outer kerbs (curb hops), barrier median with the drain
    pl.rolled_line((BIRCH_W, CORNER_SW[1]), (BIRCH_W, z1), road_left=True)
    pl.rolled_line((BIRCH_E, CORNER_SE[1]), (BIRCH_E, z1), road_left=False)
    d0, d1 = DRAIN_Z - C.DRAIN['length'] / 2, DRAIN_Z + C.DRAIN['length'] / 2
    pl.kerb_line((MED_E, MED_N), (MED_E, d0), road_left=True)
    pl.kerb_line((MED_E, d1), (MED_E, MED_S), road_left=True)
    pl.prop('street_storm_drain', MED_E, 0.0, DRAIN_Z, yaw_facing(1, 0),
            note='storm-drain dip (Golden Battery in its inlet)')
    pl.kerb_line((MED_W, MED_S), (MED_W, MED_N), road_left=True)
    pl.kerb_arc((0, MED_N), 8, 180, 360, 'out')
    pl.kerb_arc((0, MED_S), 8, 0, 180, 'out')

    # ---- ground ----
    # road: the whole area at 0, minus the storm-drain dish (its own floor at -2.05)
    dx1 = MED_E + C.DRAIN['width']
    pl.poly('Asphalt', 0.0, [(x0, z0), (MED_E, z0), (MED_E, z1), (x0, z1)], 'road (west part)')
    pl.poly('Asphalt', 0.0, [(MED_E, z0), (dx1, z0), (dx1, d0), (MED_E, d0)],
            'road (north of the drain)')
    pl.poly('Asphalt', 0.0, [(MED_E, d1), (dx1, d1), (dx1, z1), (MED_E, z1)],
            'road (south of the drain)')
    pl.poly('Asphalt', 0.0, [(dx1, z0), (x1, z0), (x1, z1), (dx1, z1)], 'road (east part)')
    pl.poly('Asphalt', -2.05, [(MED_E, d0), (dx1, d0), (dx1, d1), (MED_E, d1)],
            'storm-drain dish floor')
    # lawn blocks (behind the kerb backs, 0.6)
    north = [(x0, MAPLE_N - kb), (FX, MAPLE_N - kb)]
    north += kerb_back_arc((FX, MAPLE_N - RF), RF, 90, FILLET_A, 'out')[1:]
    north += kerb_back_arc((XC, 0), RB, bulb_a0, 360, 'in')[1:]
    north += [(x1, 0.0), (x1, z0), (x0, z0)]
    pl.poly('Grass', LAWN, north, 'north lawns')
    south = [(x1, 0.0)]
    south += kerb_back_arc((XC, 0), RB, 360, bulb_a1, 'in')[1:]
    south += kerb_back_arc((FX, MAPLE_S + RF), RF, 360 - FILLET_A, 270, 'out')[1:]
    south += [(CORNER_SE[0], MAPLE_S + kb)]
    south += kerb_back_arc(CORNER_SE, RC, 270, 180, 'out')[1:]
    south += [(BIRCH_E + kb, z1), (x1, z1)]
    pl.poly('Grass', LAWN, south, 'south-east lawns (corner lot, raised-yard house, bulb lots)')
    sw = [(x0, MAPLE_S + kb), (CORNER_SW[0], MAPLE_S + kb)]
    sw += kerb_back_arc(CORNER_SW, RC, 270, 360, 'out')[1:]
    sw += [(BIRCH_W - kb, z1), (x0, z1)]
    pl.poly('Grass', LAWN, sw, 'south-west lawns')
    med = arc_pts((0, MED_N), 8 - kb, 180, 360) + arc_pts((0, MED_S), 8 - kb, 0, 180)
    pl.poly('Grass', LAWN, med, 'Birch Lane median')
    pl.poly('Grass', LAWN, arc_pts((XC, 0), RI - kb, 0, 360, 3.0)[:-1], 'cul-de-sac island')
    # sidewalks (0.62)
    w0, w1 = KB + STRIP, KB + STRIP + WALK
    pl.poly('Stone', C.WALK_H, [(x0, MAPLE_N - w0), (FX - 4, MAPLE_N - w0),
                                (FX - 4, MAPLE_N - w1), (x0, MAPLE_N - w1)], 'north sidewalk')
    pl.poly('Stone', C.WALK_H, [(CORNER_SE[0] + 2.6, MAPLE_S + w0), (FX - 4, MAPLE_S + w0),
                                (FX - 4, MAPLE_S + w1), (CORNER_SE[0] + 2.6, MAPLE_S + w1)],
            'south sidewalk')
    pl.poly('Stone', C.WALK_H, [(BIRCH_E + w0, CORNER_SE[1] + 2.6), (BIRCH_E + w1,
                                CORNER_SE[1] + 2.6), (BIRCH_E + w1, z1), (BIRCH_E + w0, z1)],
            'Birch east sidewalk')
    cp = [(CORNER_SE[0] + 2.6, MAPLE_S + w1), (CORNER_SE[0] + 2.6, MAPLE_S + kb)]
    cp += kerb_back_arc(CORNER_SE, RC, 270, 180, 'out')
    cp += [(BIRCH_E + kb, CORNER_SE[1] + 2.6), (BIRCH_E + w1, CORNER_SE[1] + 2.6),
           (BIRCH_E + w1, MAPLE_S + w1)]
    pl.poly('Stone', C.WALK_H, cp, 'corner paving (SE)')
    pl.poly('Stone', C.WALK_H, [(BIRCH_W - w1, CORNER_SW[1] + 2.6), (BIRCH_W - w0,
                                CORNER_SW[1] + 2.6), (BIRCH_W - w0, z1), (BIRCH_W - w1, z1)],
            'Birch west sidewalk')
    # driveways (0.64): north houses, the corner lot, the cul-de-sac house (bent to its garage)
    for kind, x, z, drive in NORTH_HOUSES:
        pl.poly('Asphalt', C.DRIVE_H, [(drive - 7, MAPLE_N - kb), (drive + 7, MAPLE_N - kb),
                                       (drive + 7, z + 12), (drive - 7, z + 12)],
                f'driveway of the {kind} at x {x:g}')
    pl.poly('Asphalt', C.DRIVE_H, [(CORNER_DRIVE - 7, MAPLE_S + kb), (CORNER_DRIVE + 7,
                                   MAPLE_S + kb), (CORNER_DRIVE + 7, 88), (CORNER_DRIVE - 7, 88)],
            'corner lot driveway (the Maple-side gate)')
    u = (math.cos(rad(APRON_BULB)), math.sin(rad(APRON_BULB)))
    v = (-u[1], u[0])
    hw = C.APRON_W / 2
    r0 = RB + kb
    g0, g1 = garage_door(*BULB_HOUSE)
    pl.poly('Asphalt', C.DRIVE_H, [(XC + u[0] * r0 + v[0] * hw, u[1] * r0 + v[1] * hw),
                                   (XC + u[0] * r0 - v[0] * hw, u[1] * r0 - v[1] * hw), g0, g1],
            'cul-de-sac driveway (to the garage)')
    x, z, yaw = BIRCH_HOUSE[1:]
    pl.poly('Asphalt', C.DRIVE_H, [(BIRCH_E + kb, z - 25), (x - 12, z - 25), (x - 12, z - 11),
                                   (BIRCH_E + kb, z - 11)], 'Birch Lane driveway')


def build_beyond(pl):
    """Lawn plates beyond the bounds at the edge-lawn height, where the decor houses and maples
    stand (rc_render's outdoor ground is at road level); Maple Street (west) and Birch Lane
    (south) leave between them at road level. Physics never reaches them."""
    (x0, z0), (x1, z1) = BOUNDS
    b, kb = BEYOND, C.KERB_D
    for pts, note in (
            ([(x0 - b, z0 - b), (x1 + b, z0 - b), (x1 + b, z0), (x0 - b, z0)], 'beyond: north'),
            ([(x1, z0), (x1 + b, z0), (x1 + b, z1 + b), (x1, z1 + b)], 'beyond: east'),
            ([(BIRCH_E + kb, z1), (x1, z1), (x1, z1 + b), (BIRCH_E + kb, z1 + b)],
             'beyond: south-east'),
            ([(x0 - b, z1), (BIRCH_W - kb, z1), (BIRCH_W - kb, z1 + b), (x0 - b, z1 + b)],
             'beyond: south-west'),
            ([(x0 - b, MAPLE_S + kb), (x0, MAPLE_S + kb), (x0, z1), (x0 - b, z1)],
             'beyond: west, south of Maple Street'),
            ([(x0 - b, z0), (x0, z0), (x0, MAPLE_N - kb), (x0 - b, MAPLE_N - kb)],
             'beyond: west, north of Maple Street')):
        pl.poly('Grass', LAWN, pts, f'lawn plate {note} (decor stands on it)')


def build_yard(pl):
    """The raised front yard: retaining walls, plateau lawn, kickers and the two kids' ramps."""
    wl = C.WALL
    t = wl['thick']
    xe, xw = PB - t, PE + t                       # inner faces of the end walls
    pl.poly('Grass', YARD, [(xw, YARD_N), (xe, YARD_N), (xe, YARD_S), (xw, YARD_S)],
            'raised front yard (3.0)')
    # end walls (east at PB, west at PE) and the long walls
    for x, yaw in ((PB - t / 2, 90), (PE + t / 2, -90)):
        for dz in (-3.9, 3.9):
            pl.prop('street_retaining_wall', x, LAWN, YARD_Z + dz, yaw)
    n = round((xe - xw) / wl['length'] + 0.49)
    step = (xe - xw + 2 * t) / n
    for i in range(n):
        x = xw - t + step * (i + 0.5)
        pl.prop('street_retaining_wall', x, LAWN, YARD_N - t / 2, 0)
        pl.prop('street_retaining_wall', x, LAWN, YARD_S + t / 2, 180)
    k, p = C.KICKER, C.PLANK
    pl.prop('street_kicker', PB - k['run'] / 2, YARD, YARD_Z, -90, note='east kicker')
    pl.prop('street_kicker', PE + k['run'] / 2, YARD, YARD_Z, 90, note='west kicker')
    pl.prop('street_plank_ramp', PB + p['run'] / 2, LAWN, YARD_Z, 90, 'Fwd',
            note='forward climb onto the yard')
    pl.prop('street_plank_ramp', PE - p['run'] / 2, LAWN, YARD_Z, -90, 'Rev',
            note='reverse climb onto the yard')


def build_cars(pl):
    pl.prop('street_parked_suv_a', SUV_TUNNEL[0], 0.0, SUV_TUNNEL[1], yaw_facing(1, 0),
            note='parked along the north kerb: drive under it lengthwise')
    pl.prop('street_parked_suv_b', SUV_CROSS[0], 0.0, SUV_CROSS[1], yaw_facing(0, -1),
            note='parked at the east kerb below the curb hop: drive under it between the axles')


def build_dressing(pl):
    """Houses, trees, lamps, parked cars and street furniture: the neighbourhood."""
    P = pl.prop
    strip_n, strip_s = MAPLE_N - KB - STRIP / 2, MAPLE_S + KB + STRIP / 2  # planting strips
    strip_e, strip_w = BIRCH_E + KB + STRIP / 2, BIRCH_W - KB - STRIP / 2
    # houses
    for kind, x, z, drive in NORTH_HOUSES:
        P(kind, x, LAWN, z, 180)
        P('street_mailbox', drive + 9.5, LAWN, strip_n, 180)
    kind, x, z, yaw = CORNER_HOUSE
    P(kind, x, LAWN, z, yaw, note='corner lot (lawn shortcut through its front yard)')
    P(BULB_HOUSE[0], BULB_HOUSE[1], LAWN, BULB_HOUSE[2], BULB_HOUSE[3],
      note='cul-de-sac house; the raised yard is its side garden')
    P(BIRCH_HOUSE[0], BIRCH_HOUSE[1], LAWN, BIRCH_HOUSE[2], BIRCH_HOUSE[3])
    for th, r, kind in ((295, 86, 'street_house_b'), (18, 84, 'street_house_a_m'),
                        (72, 86, 'street_house_b_m')):
        q = pol((XC, 0), r, th)
        P(kind, q[0], LAWN, q[1], yaw_facing(-math.cos(rad(th)), -math.sin(rad(th))))
    P('street_mailbox', CORNER_DRIVE - 9.5, LAWN, strip_s, 0)
    # maples: planting strips, median, island, a few front yards (the corner lot's on the
    # shortcut line)
    trees = [(x, strip_n, 'a' if i % 2 else 'b') for i, x in
             enumerate((-94, -40, 28, 64, 124, 196))]
    trees += [(x, strip_s, 'b' if i % 2 else 'a') for i, x in enumerate((64, 128, 168))]
    trees += [(strip_e, z, 'a') for z in (52, 112, 150, 196)]
    trees += [(strip_w, z, 'b' if i % 2 else 'a') for i, z in enumerate((54, 102, 150, 198))]
    trees += [(0, z, 'b') for z in (64, 100, 132)]
    trees += [(XC, 0, 'a'), (69, 45, 'b'), (-28, -56, 'a'), (46, -58, 'b'), (122, -54, 'a'),
              (300, -42, 'b'), (318, 42, 'a'), (115, 132, 'b')]
    for i, (x, z, v) in enumerate(trees):
        P(f'street_maple_{v}', x, LAWN, z, (i * 47) % 360)
    # street lamps (arms over the road)
    for x in (-66, 0, 94, 160):
        P('street_lamp', x, LAWN, strip_n, 180)
    for th in (250, 345, 60):
        q = pol((XC, 0), RB + 4.6, th)
        P('street_lamp', q[0], LAWN, q[1], yaw_facing(-math.cos(rad(th)), -math.sin(rad(th))))
    P('street_lamp', MED_W + 4.2, LAWN, 82, 90)
    P('street_lamp', MED_E - 4.2, LAWN, 118, -90)
    P('street_lamp', strip_s + 38, LAWN, strip_s, 0)
    # parked cars (dressing) along Maple's south kerb and in a driveway
    P('street_parked_suv_c', 132, 0.0, MAPLE_S - 0.5 - C.SUV['width'] / 2, 90)
    P('street_parked_suv_a', 30, 0.0, MAPLE_S - 0.5 - C.SUV['width'] / 2, 90)
    P('street_parked_suv_b', -6, C.DRIVE_H, -46, 180, note='parked in a driveway')
    # bins out for collection, hydrants, signs
    for x, z in ((-71, strip_n), (3, strip_n), (116, strip_n), (CORNER_DRIVE + 10, strip_s)):
        P('street_trash_can', x, LAWN, z, 180 if z < 0 else 0)
        P('street_recycling_bin', x + 3.2, LAWN, z, 180 if z < 0 else 0)
    P('street_hydrant_cluster', 118, LAWN, strip_n - 0.5, 180)
    P('street_hydrant', strip_e, LAWN, 128, 90)
    P('street_hydrant', XC + 44 * math.cos(rad(320)) * 1.12, LAWN, 44 * math.sin(rad(320)) * 1.12)
    P('street_sign_stop', MED_W - 2.0, LAWN, MED_N - 4.0, 180, note='stop for Birch northbound')
    P('street_sign_street', CORNER_SW[0] + 5.5, LAWN, CORNER_SW[1] - 5.5, 45)
    P('street_sign_street', CORNER_SE[0] + 8.5, LAWN, CORNER_SE[1] - 9.0, -45)
    # road paint: yellow centre dashes, a crosswalk west of the grid, a stop line
    x = -96.0
    while x < FX - 10:
        P('street_paint_dash', x, 0.0, 0.0)
        x += 20.0
    for i in range(9):
        P('street_paint_bar', 26.0, 0.0, -17.6 + 4.4 * i)
    P('street_paint_stop', (BIRCH_W + MED_W) / 2, 0.0, MED_N - 10.0)
    for x, z in ((40, 6), (118, -4), (-22, 116), (238, 12)):
        P('street_manhole', x, 0.0, z)
    # the kids' race: cones marking the tunnel, the apron, the curb hop and the U-turn
    cones = [(136, -9.5), (142, -9.5), (148, -10.0), (184, -9.8), (190, -9.5),
             (255, 38.0), (247, 41.5), (BIRCH_E - 2.5, CAR_Z - 7.0), (BIRCH_E - 2.5, CAR_Z + 7.0),
             (BIRCH_E - 5.5, CAR_Z + 7.5), (0, MED_S + 11.5), (-6.0, MED_S + 10.0),
             (6.0, MED_S + 10.0), (90, -20.0 + 2.2), (90, -4.5), (MED_E + 2.0, DRAIN_Z - 16),
             (MED_E + 2.0, DRAIN_Z + 16), (FX + 1.0, -5.0)]
    for i, (x, z) in enumerate(cones):
        P('street_cone', x, 0.0, z, (i * 31) % 90)
    # road closed at Birch's south end: barricades across both roadways
    for x in (-26.0, -14.0, 14.0, 26.0):
        P('street_barricade', x, 0.0, MED_S + 52.0)
    # autumn leaves: piles in gutters, scatter under trees and along kerbs
    for x, z in ((96, -17.4), (BIRCH_W + 3.6, 64), (MED_E + 3.4, 170), (215, -40)):
        P('street_leaf_pile', x, 0.0 if abs(z) < 60 or x < 40 else LAWN, z)
    rnd_leaves = [(x + 3, z + 2) for x, z, _ in trees[::2]]
    rnd_leaves += [(x, -18.2) for x in (-50, 10, 80, 120, 190)]
    rnd_leaves += [(x, 18.0) for x in (-60, 40, 100, 150)]
    rnd_leaves += [(BIRCH_W + 2.2, z) for z in (90, 130)] + [(MED_E + 2.0, z) for z in (70, 150)]
    for i, (x, z) in enumerate(rnd_leaves):
        y = 0.0 if (abs(z) < 20 or (MED_W < x < MED_E + 4) or (BIRCH_W < x < MED_W)) else LAWN
        if BIRCH_W < x < BIRCH_E and z > 20:
            y = LAWN if MED_W - KB < x < MED_E + KB else 0.0
        P('street_leaves', x, y, z, (i * 53) % 360)
    # gardens: hedges between north lots, a picket-fenced yard, the corner lot's details
    for x in (-24.0, 50.0, 126.0):
        for z in (-48.0, -56.0, -64.0):
            P('street_hedge', x, LAWN, z, 90)
    P('street_fence_corner', 58.0, LAWN, MAPLE_N - KB - STRIP - WALK - 2.0, 90)
    for i in range(4):
        P('street_fence', 62.2 + 8.0 * i, LAWN, MAPLE_N - KB - STRIP - WALK - 1.95)
    for z in (-50.5, -58.5):
        P('street_fence', 57.95, LAWN, z, 90)
    P('street_bike', 79.0, LAWN, 59.0, 25, note='on the lawn shortcut')
    P('street_sprinkler', 60.0, LAWN, 48.0, 30)
    for z in (46, 56, 66, 76):
        P('street_garden_light', CORNER_DRIVE + 8.4, LAWN, z)
    for x, y, z in ((CORNER_DRIVE + 2, C.DRIVE_H, 30.0), (-8, C.DRIVE_H, -30.0),
                    (110, C.WALK_H, -31.0), (BIRCH_E + 12.0, C.WALK_H, 120.0)):
        P('street_newspaper', x, y, z, 70)


DECOR = [  # physics-free houses and trees down the streets, on lawns at LAWN height
    ('house_b', -140.0, -86.0, 180, 1.0), ('house_a', -205.0, -87.0, 180, 1.0),
    ('house_a_m', -140.0, 72.0, 180 - 90, 1.0), ('house_b_m', -140.0, 150.0, -90, 1.0),
    ('house_a', -205.0, 70.0, 0, 1.0), ('house_b', -60.0, 262.0, 180 + 180, 1.0),
    ('house_a_m', 75.0, 262.0, 0, 1.0), ('house_b', 405.0, -30.0, 90, 1.0),
    ('house_a', 410.0, 70.0, 90, 1.0), ('maple_a', -120.0, -24.6, 0, 1.1),
    ('maple_b', -170.0, -24.6, 40, 0.95), ('maple_a', -120.0, 24.6, 80, 1.0),
    ('maple_b', -175.0, 26.0, 120, 1.15), ('maple_a', -45.0, 235.0, 10, 1.0),
    ('maple_b', 45.0, 240.0, 60, 0.9), ('maple_a', 0.0, -135.0, 0, 1.2),
    ('maple_b', 90.0, -140.0, 0, 1.0), ('maple_a', 210.0, -130.0, 0, 1.1),
    ('maple_b', 395.0, 25.0, 0, 1.0), ('maple_a', 300.0, 150.0, 0, 1.2),
    ('maple_b', 180.0, 160.0, 0, 1.05), ('maple_a', 240.0, 240.0, 0, 1.0),
]


def write_decor():
    path = os.path.join(C.ROOT, 'assets', 'data', 'decor', 'maple_street.ron')
    os.makedirs(os.path.dirname(path), exist_ok=True)
    lines = ['// Decor (physics-free) for Maple Street: houses and trees beyond the play area.',
             '// GENERATED by tools/blender/props/street/plan.py. Format: rc_render props/decor.rs.',
             '(', '    decor: [']
    for model, x, z, yaw, scale in DECOR:
        lines.append(f'        (model: "models/props/street/{model}.glb", pos: ({fmt(x)}, {fmt(LAWN)}, '
                     f'{fmt(z)}), yaw: {fmt(((yaw + 180) % 360) - 180)}, scale: {fmt(scale)}),')
    lines += ['    ],', ')', '']
    if editor_owned(path):
        return
    with open(path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))
    print(f'wrote {path} ({len(DECOR)} items)')


# ---------------------------------------------------------------------------------------------
# the race line (forward = start eastbound on Maple Street, clockwise around the island)

def race_points():
    P = []

    def pt(x, y, z, note=None, **kw):
        P.append(dict(pos=(r1(x), y, r1(z)), note=note, **kw))

    def arc(c, r, angles, y=0.0, note=None):
        for i, a in enumerate(angles):
            p = pol(c, r, a)
            pt(p[0], y, p[1], note if i == 0 else None)

    walk_s = MAPLE_S + WALK_C           # Maple south sidewalk centre (z)
    walk_e = BIRCH_E + WALK_C           # Birch east sidewalk centre (x)
    # Maple Street eastbound (north half), start/finish at x 90: keeps the evenly spaced
    # checkpoint gates out of the curb-hop S-bend (a gate there can be skipped by a car knocked
    # wide by the parked SUV's tyres)
    pt(90, 0, -12.2, 'start / finish', width=14, surface='Asphalt', wall_left=False,
       wall_right=False, offroad_margin=6)
    pt(100, 0, -12.4)
    pt(132, 0, -14.2, width=10)  # corridor stays inside the north kerb (z -20)
    pt(150, 0, SUV_TUNNEL[1], 'under the parked SUV (6 u between its wheels)', width=6,
       offroad_margin=3)
    pt(165, 0, SUV_TUNNEL[1])
    pt(182, 0, SUV_TUNNEL[1])
    pt(198, 0, -14.5, width=10, offroad_margin=8)
    # into the cul-de-sac, clear of the kerb and the NW fillet: around the island at radius 30
    pt(214, 0, -15.0)
    pt(229, 0, -19.5, width=14)
    arc((XC, 0), 30, (240, 265, 290, 315, 340, 5, 30, 55, 80), note='cul-de-sac ring')
    # out over the driveway apron
    pt(252, 0, 32.0, width=12)
    pt(241, 0, 35.5, width=10)
    p = pol((XC, 0), RB + 1.5, APRON_BULB - 0.5)
    pt(p[0], C.DRIVE_H, p[1], 'top of the driveway apron')
    pt(224, C.DRIVE_H, 44.0)
    pt(213, LAWN, 47.5)
    pt(204, LAWN, 48.2)
    # the raised yard: forward climbs the east plank, jumps off the west end
    pt(PA, LAWN, YARD_Z, 'east plank foot (reverse: landing)')
    pt(PB, TOP, YARD_Z, 'east edge (reverse: lip)')
    pt(PC, YARD, YARD_Z, 'east kicker foot')
    pt(PD, YARD, YARD_Z, 'west kicker foot')
    pt(PE, TOP, YARD_Z, 'west edge: forward lip')
    pt(PF, LAWN, YARD_Z, 'forward landing (reverse: west plank foot)')
    pt(127, LAWN, YARD_Z)
    pt(113, C.WALK_H, 41.5)
    pt(100, C.WALK_H, walk_s + 1.0, 'corner lot garden wall (left)', width=8, wall_left=True)
    pt(GATE_A[1], C.WALK_H, walk_s, 'Maple-side gate (lawn shortcut)', wall_left=False,
       offroad_margin=17)
    pt(GATE_A[0], C.WALK_H, walk_s, wall_left=True)
    pt(TURN_C[0] + 8, C.WALK_H, walk_s, width=11)
    # around the corner lot's corner on the corner paving
    arc(TURN_C, TURN_R, (250, 220, 190), C.WALK_H)
    pt(walk_e, C.WALK_H, TURN_C[1] + 8, width=8, offroad_margin=17)
    pt(walk_e, C.WALK_H, GATE_B[0], 'Birch-side gate (lawn shortcut)', wall_left=False)
    pt(walk_e, C.WALK_H, CAR_Z - 13, 'Birch east sidewalk', offroad_margin=8)
    # right turn, curb hop down the rolled kerb, under the SUV, left turn
    # (reverse: the left turn right after climbing the hop; R 9 scrubbed cars to ~7 km/h)
    arc((walk_e - HOP_TURN_R, CAR_Z - HOP_TURN_R), HOP_TURN_R, (30, 60), C.WALK_H)
    pt(walk_e - HOP_TURN_R, C.WALK_H, CAR_Z, width=10)
    pt(BIRCH_E - 0.2, 0.3, CAR_Z, 'curb hop (rolled kerb)')
    pt(SUV_CROSS[0], 0, CAR_Z, 'under the parked SUV (across, between the axles)')
    pt(SUV_CROSS[0] - 6.5, 0, CAR_Z)
    arc((SUV_CROSS[0] - 6.5, CAR_Z + 9), 9, (240, 210))
    pt(SUV_CROSS[0] - 15.5, 0, CAR_Z + 9, width=12)
    # storm-drain dip in the median gutter
    pt(15, 0, DRAIN_Z - 14, 'storm-drain dip')
    pt(15, -2.0, DRAIN_Z - 2)
    pt(15, -2.0, DRAIN_Z + 2)
    pt(15, 0, DRAIN_Z + 14)
    pt(19, 0, MED_S - 4, width=14)
    # U-turn around the median's south nose
    arc((0, MED_S), 20, (25, 55, 90, 125, 155), note='U-turn')
    pt(-20, 0, MED_S - 8)
    pt(-20, 0, 100, 'Birch Lane northbound')
    pt(-20, 0, 65)
    pt(-20, 0, 30)
    # right turn into Maple Street
    arc((10, 18), 30, (195, 225, 255))
    pt(22, 0, -12)
    pt(60, 0, -12)
    return P


GAPS = [  # point indices are resolved by note
    ('west edge: forward lip', 'forward landing (reverse: west plank foot)', 'Fwd'),
    ('east plank foot (reverse: landing)', 'east edge (reverse: lip)', 'Rev'),
]

ITEM_ROWS = [(108, 0, -12.6), (288, 0, -10), (168, YARD, YARD_Z), (15, 0, 96),
             (-20, 0, 82)]

SHORTCUTS = [
    # through the corner lot's front yard: in at the Maple-side gate, out at the Birch-side gate.
    # The path starts/ends ~12 u along the sidewalk beyond the gates so the checkpoint keep-out
    # zone keeps gates where every car is still on the main line (a gate right at a gate
    # opening can be skipped by cars cutting in early).
    ([(104, C.WALK_H, 35.9), (GATE_A[1] + 1, C.WALK_H, MAPLE_S + WALK_C), (84.5, LAWN, 41.0),
      (72, LAWN, 49), (62, LAWN, 56), (55, LAWN, 61.5),
      (BIRCH_E + WALK_C, C.WALK_H, GATE_B[1] + 1), (BIRCH_E + WALK_C, C.WALK_H, CAR_Z - 7)],
     6, 'Both'),
]

GOLDEN_BATTERY = (MED_E + 1.2, -1.0, DRAIN_Z)  # in the storm-drain inlet


# ---------------------------------------------------------------------------------------------
# output

def fmt(v):
    return C.fmt(float(v)) if not isinstance(v, (tuple, list)) else \
        '(' + ', '.join(C.fmt(float(x)) for x in v) + ')'


def point_line(i, p, prev):
    fields = [f'pos: {fmt(p["pos"])}']
    for k in ('width', 'bank_deg', 'surface', 'wall_left', 'wall_right', 'offroad_margin'):
        if p.get(k) is not None:
            v = p[k]
            fields.append(f'{k}: ' + (('true' if v else 'false') if isinstance(v, bool)
                                      else v if isinstance(v, str) else fmt(v)))
    s = f'        ({", ".join(fields)}), // {i}'
    return s + (f': {p["note"]}' if p.get('note') else '')


def write_track(pl):
    pts = race_points()
    idx = {p['note']: i for i, p in enumerate(pts) if p.get('note')}
    lines = [
        '// Maple Street — suburban street & front lawns (GDD §7.1, art bible §3.7). ~900 u, 3 laps,',
        '// outdoor ★★★. GENERATED by tools/blender/props/street/plan.py — edit that and re-run;',
        '// plot: cargo run -p rc_track --example plot -- maple_street [--rev] [--mirror].',
        '// Invisible ribbon: the road (0), lawns (0.6), sidewalks (0.62), driveways (0.64) are',
        '// ground polygons; kerbs (0.65), ramps, the drain and parked cars are props.',
        '// Forward: start eastbound on Maple Street → under a parked SUV (chokepoint) → clockwise',
        '// around the cul-de-sac island → under the SUV parked across a driveway, up the apron →',
        '// the raised front yard (plank up, jump off the west end) → sidewalk (lawn shortcut',
        '// through the corner lot\'s gates) → curb hop onto Birch Lane → storm-drain dip →',
        '// U-turn around the median → Birch Lane north → right into Maple Street.',
        '(',
        '    style: Invisible,',
        '    start: 0,',
        '    kill_height: -6,',
        '    wall_height: 1.2,',
        f'    bounds: (min: {fmt(BOUNDS[0])}, max: {fmt(BOUNDS[1])}),',
        '    points: [',
    ]
    lines += [point_line(i, p, pts[i - 1] if i else None) for i, p in enumerate(pts)]
    lines.append('    ],')
    lines.append('    gaps: [')
    for a, b, d in GAPS:
        lines.append(f'        (from: {idx[a]}, to: {idx[b]}, dir: {d}), // {a} → {b}')
    lines.append('    ],')
    lines.append('    ground: [')
    for surface, h, poly, note in pl.ground:
        pts_s = ', '.join(fmt(p) for p in poly)
        lines.append(f'        // {note}')
        lines.append(f'        (surface: {surface}, height: {fmt(h)}, points: [{pts_s}]),')
    lines.append('    ],')
    lines.append('    item_rows: [')
    lines += [f'        (pos: {fmt(p)}),' for p in ITEM_ROWS]
    lines.append('    ],')
    lines.append('    shortcuts: [')
    for path, width, d in SHORTCUTS:
        lines.append(f'        (path: [{", ".join(fmt(p) for p in path)}], width: {fmt(width)}, '
                     f'dir: {d}),')
    lines.append('    ],')
    lines.append(f'    golden_battery: (pos: {fmt(GOLDEN_BATTERY)}),')
    lines.append('    props: [')
    for kind, pos, yaw, d, note in pl.props:
        s = f'        (kind: "{kind}", pos: {fmt(tuple(r1(v) for v in pos))}'
        if abs(yaw) > 1e-6:
            s += f', yaw: {fmt(r1(((yaw + 180) % 360) - 180))}'
        if d != 'Both':
            s += f', dir: {d}'
        s += '),'
        lines.append(s + (f' // {note}' if note else ''))
    lines += ['    ],', ')', '']
    if editor_owned(OUT):
        return
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))
    print(f'wrote {OUT} ({len(pts)} points, {len(pl.props)} props, {len(pl.ground)} ground)')


def arc_catalog(pl):
    out = {}
    for kind, (r, span, face) in sorted(pl.arcs.items()):
        out[kind] = dict(model=f'models/props/street/{kind[len("street_"):]}.glb',
                         colliders=C.kerb_arc_colliders(r, span, face), surface='Stone',
                         note=f'Barrier kerb arc, radius {r:g} u, {span:g}°, road '
                              f'{"outside" if face == "out" else "inside"} the circle.')
    return out


def build():
    pl = Plan()
    build_streets(pl)
    build_beyond(pl)
    build_yard(pl)
    build_cars(pl)
    build_dressing(pl)
    return pl


def main():
    pl = build()
    C.write(arc_catalog(pl))
    write_track(pl)
    write_decor()


if __name__ == '__main__':
    main()
