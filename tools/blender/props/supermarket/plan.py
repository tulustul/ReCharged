"""Aisle Rush layout (GDD §7.1, art bible §3.5): writes assets/data/tracks/aisle_rush.ron (and
assets/data/decor/aisle_rush.ron) from the store plan below, after checking that no static
obstacle reaches into the racing corridor (a Python copy of rc_track's centripetal Catmull-Rom).

    python3 tools/blender/props/supermarket/plan.py [--assets DIR]

World axes as in the game: +X east, +Z south (plots: north up), Y up; u = 22.5 cm.
Store plan (north = back wall): dairy and deli coolers along the north wall, the back aisle, a
block of 8 gondola rows (aisles run north-south), the promenade, the checkout bank and the lobby
along the south shopfront. The freezer aisle runs along the west wall, produce along the east.

Forward lap (clockwise from above): lobby westbound (start) -> freezer aisle north (ice) ->
back aisle east -> aisle A south (box hump) -> U-turn around two endcaps -> aisle B north (box
jump) -> back aisle east -> produce south (S-bends around the water pallets: Golden Battery on
top, up a narrow box ramp) -> promenade west -> checkout lane 2 south -> lobby. Lanes 1 and 3
are alternative routes. Reverse: the same loop backwards, the box jump in aisle A.

Since the track editor (docs/design/rc_editor.md) the track and decor files are editor-owned: this
script writes them only with --layout, which replaces every edit made in the editor.
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import catalog as C  # noqa: E402

ROOT = C.ROOT
KINDS = C.kinds()

# --- store plan (u) ----------------------------------------------------------------------------
X_MIN, X_MAX, Z_MIN, Z_MAX = -111.0, 96.0, -84.0, 86.0   # room walls (track bounds)
XF, HW_F = -100.0, 6.0          # freezer aisle centre line, half width
ZB, HW_B = -68.0, 7.0           # back aisle
XA, XB, HW_A = -48.0, -16.0, 5.0  # aisles A and B (the U-turn joins them)
ZGN = -55.5                     # gondola runs: north end ...
RUN = 2 * C.ENDCAP_DEPTH + 7 * C.GONDOLA_LEN
ZGS = ZGN + RUN                 # ... south end (27)
GONDOLA_X = [-72.0 + 16.0 * k for k in range(8)]  # row centres; aisles between them
AISLE_X = [(a + b) / 2 for a, b in zip(GONDOLA_X, GONDOLA_X[1:])]
XE, HW_E = 64.0, 7.0            # produce route (north and south ends)
ZP, HW_P = 41.0, 6.0            # promenade (east part)
XL, HW_L = 12.0, 4.0            # main checkout lane (lane 2)
ZL, HW_LOBBY = 76.0, 6.0        # lobby
STAND_Z = 58.5                  # checkout stand centres (z 51..66, between T9 and T10)
XS = -26.0                      # start / finish line on the lobby
JUMP_Z = -13.0                  # centre of both box jumps (aisles A and B)
R1, R2, R3, R6, R7, R8, R9, R10 = 14.0, 14.0, 12.0, 12.0, 12.0, 12.0, 10.0, 10.0
RU = (XB - XA) / 2              # U-turn radius (16)
S_DX, S_LEN = 12.0, 30.0        # produce S-bends: lateral shift over length
FREEZER_FACE_W = XF - HW_F - 0.6   # wall freezer fronts (behind the barrier)
ISLAND_W = XF + HW_F + 0.6         # freezer island west face
ISLAND_Z = (-54.0, 56.0)
LANE_W = {1: 6.0, 2: 2 * HW_L + 0.6, 3: 6.0}  # open lanes; closed ones are 6 u wide, gated

# --- control points -----------------------------------------------------------------------------
P = []  # dicts: x, y, z, + optional width, walls (left, right), margin, note


def editor_owned(path):
    """Track and decor files are editor-owned (docs/design/rc_editor.md): this script writes them
    only with --layout, which replaces every edit made in the editor with its own layout."""
    if '--layout' in sys.argv:
        return False
    print(f'skipped {path}: editor-owned (cargo run -p rc_editor); --layout overwrites it')
    return True


def pt(x, z, y=0.0, **kw):
    P.append(dict(x=x, y=y, z=z, **kw))
    return len(P) - 1


def arc(cx, cz, r, a0, a1, step=30.0, **kw):
    """Interior points of the arc from angle a0 to a1 (degrees, x = cx + r cos a,
    z = cz + r sin a); the end points are added separately by the straights."""
    n = max(1, round(abs(a1 - a0) / step))
    for i in range(1, n):
        a = math.radians(a0 + (a1 - a0) * i / n)
        pt(cx + r * math.cos(a), cz + r * math.sin(a), **kw)


def s_bend(x0, z0, dx, length, step=15.0):
    """Southbound S-bend from (x0, z0) shifting by dx over `length`: two tangent arcs. Adds the
    interior points and the end point; returns the end."""
    phi = 2 * math.atan2(abs(dx), length)
    r = length / (2 * math.sin(phi))
    sgn = 1 if dx > 0 else -1
    n = max(2, math.ceil(math.degrees(phi) / step))
    for i in range(1, n + 1):          # first arc (toward dx), ends at the inflection point
        a = phi * i / n
        pt(x0 + sgn * r * (1 - math.cos(a)), z0 + r * math.sin(a))
    for i in range(1, n + 1):          # second arc, back to heading south
        a = phi * (n - i) / n
        pt(x0 + dx - sgn * r * (1 - math.cos(a)), z0 + length - r * math.sin(a))
    return x0 + dx, z0 + length


def jump_points(x, travel_north, rev_gap):
    """Box-jump control points in forward order; returns (gap_from, gap_to) indices. Extra
    points keep the spline on the ramps (straight between collinear points)."""
    prof = C.jump_profile()          # prop z (back +, front -) -> height
    z_back, z_top, z_kick, z_lip, z_front = (JUMP_Z + z for z, _ in prof)
    h_top, h_lip = C.JUMP['top'], C.JUMP['lip']
    g = 10.0                         # landing: 10 u down the long ramp
    z_land = z_lip - g
    h_land = h_lip * (1 - g / C.JUMP['down'])
    seq = [(z_back + 4, 0.0, None), (z_back, 0.0, 'jump: ramp foot'),
           ((z_back + z_top) / 2, h_top / 2, None), (z_top, h_top, 'jump: ramp top'),
           (z_kick, h_top, 'jump: kicker foot'), (z_lip, h_lip, 'jump: lip'),
           (z_land, h_land, 'jump: landing'), (z_front, 0.0, 'jump: down-ramp foot'),
           (z_front - 4, 0.0, None)]
    if not travel_north:
        seq = seq[::-1]
    idx = {}
    for z, h, note in seq:
        i = pt(x, z, h, **({'note': note} if note else {}))
        if note:
            idx[note] = i
    if rev_gap:
        return idx['jump: landing'], idx['jump: lip']
    return idx['jump: lip'], idx['jump: landing']


def build_points():
    P.clear()
    gaps = []
    w = lambda hw: 2 * hw  # noqa: E731
    # Lobby, westbound from the start line (sunset windows on the left).
    pt(XS, ZL, width=w(HW_LOBBY), walls=(False, False), margin=8, note='start / finish')
    pt(-62, ZL)
    pt(XF + R1 * 1.52, ZL, note='T1')
    pt(XF + R1, ZL)
    arc(XF + R1, ZL - R1, R1, 90, 180)
    # Freezer aisle, northbound: glass-door freezers both sides, an icy floor.
    pt(XF, ZL - R1, width=w(HW_F))
    pt(XF, ZL - R1 * 1.52, walls=(True, True), margin=4, note='freezer aisle')
    pt(XF, 30)
    pt(XF, -10)
    pt(XF, ZB + R2 * 1.52)
    pt(XF, ZB + R2, walls=(False, False), margin=6, note='T2')
    arc(XF + R2, ZB + R2, R2, 180, 270)
    # Back aisle west, eastbound: dairy coolers on the left.
    pt(XF + R2, ZB, width=w(HW_B), walls=(True, False), note='back aisle (dairy)')
    pt(XA - R3 * 1.52, ZB)
    pt(XA - R3, ZB, width=w(HW_A), walls=(False, False), note='T3')
    arc(XA - R3, ZB + R3, R3, 270, 360)
    # Aisle A, southbound: the box jump is a hump this way (the reverse jump).
    pt(XA, ZB + R3, walls=(True, True), margin=3, note='aisle A')
    pt(XA, -45)
    gaps.append(jump_points(XA, travel_north=False, rev_gap=True) + ('Rev',))
    pt(XA, 17)
    pt(XA, ZGS, walls=(False, False), margin=6, note='U-turn')
    arc(XA + RU, ZGS, RU, 180, 0, step=22.5)   # around the two endcaps
    # Aisle B, northbound: the forward box jump.
    pt(XB, ZGS, walls=(True, True), margin=3, note='aisle B')
    pt(XB, 17)
    gaps.append(jump_points(XB, travel_north=True, rev_gap=False) + ('Fwd',))
    pt(XB, -45)
    pt(XB, ZB + R6, walls=(False, False), margin=6, note='T6')
    arc(XB + R6, ZB + R6, R6, 180, 270)
    # Back aisle east, eastbound: meat and deli coolers on the left.
    pt(XB + R6, ZB, width=w(HW_B), walls=(True, False), note='back aisle (deli)')
    pt(XE - R7 * 1.52, ZB)
    pt(XE - R7, ZB, walls=(False, False), note='T7')
    arc(XE - R7, ZB + R7, R7, 270, 360)
    # Produce, southbound: S-bends around the water pallets (Golden Battery shortcut).
    pt(XE, ZB + R7, width=w(HW_E), margin=14, note='produce')
    x, z = XE, -50.0
    pt(x, z)
    x, z = s_bend(x, z, S_DX, S_LEN)
    z += 12.0
    pt(x, z)
    x, z = s_bend(x, z, -S_DX, S_LEN)
    P[-1].update(width=w(HW_P), margin=6)       # the S ends one turn-step before T8
    pt(XE, ZP - R8, note='T8')
    arc(XE - R8, ZP - R8, R8, 0, 90)
    # Promenade, westbound, into checkout lane 2.
    pt(XE - R8, ZP, note='promenade')
    pt(XL + R9 * 1.52, ZP, margin=12)
    pt(XL + R9, ZP, width=w(HW_L), note='T9')
    arc(XL + R9, ZP + R9, R9, 270, 180)
    pt(XL, ZP + R9, note='checkout lane 2')
    pt(XL, ZL - R10)
    arc(XL - R10, ZL - R10, R10, 0, 90)
    pt(XL - R10, ZL, width=w(HW_LOBBY), margin=8, note='lobby')
    return gaps


# --- centripetal Catmull-Rom (rc_track spline.rs) -------------------------------------------------

def _cr(p0, p1, p2, p3, t):
    def tj(ti, a, b):
        return ti + math.dist(a, b) ** 0.5
    t0 = 0.0
    t1 = tj(t0, p0, p1)
    t2 = tj(t1, p1, p2)
    t3 = tj(t2, p2, p3)
    u = t1 + (t2 - t1) * t

    def lerp(a, b, ta, tb):
        k = (u - ta) / (tb - ta)
        return [x + (y - x) * k for x, y in zip(a, b)]
    a1, a2, a3 = lerp(p0, p1, t0, t1), lerp(p1, p2, t1, t2), lerp(p2, p3, t2, t3)
    b1, b2 = lerp(a1, a2, t0, t2), lerp(a2, a3, t1, t3)
    return lerp(b1, b2, t1, t2)


def resolved():
    """Points with inherited fields filled in (file order)."""
    out, cur = [], dict(width=10.0, walls=(False, False), margin=6.0)
    for p in P:
        for k in cur:
            if k in p:
                cur[k] = p[k]
        out.append(dict(p, **cur))
    return out


def samples(step=0.5):
    """[(x, y, z, half_width, seg)] along the closed loop."""
    pts = resolved()
    n = len(pts)
    out = []
    for i in range(n):
        q = [[pts[(i + k) % n][c] for c in 'xyz'] for k in (-1, 0, 1, 2)]
        ln = math.dist(q[1], q[2])
        m = max(2, int(ln / step * 2))
        wa, wb = pts[i]['width'], pts[(i + 1) % n]['width']
        for j in range(m):
            t = j / m
            s = t * t * (3 - 2 * t)
            x, y, z = _cr(*q, t)
            out.append((x, y, z, (wa + (wb - wa) * s) / 2, i))
    return out


# --- props ------------------------------------------------------------------------------------------
PROPS = []   # (kind, x, y, z, yaw, dir)


def prop(kind, x, z, yaw=0.0, y=0.0, d='Both'):
    assert kind in KINDS, kind
    PROPS.append((kind, x, y, z, yaw, d))


def rot(x, z, yaw):
    c, s = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
    return x * c + z * s, -x * s + z * c


def footprints(kind, x, z, yaw):
    """World plan rectangles as corner lists."""
    out = []
    for cx, cz, hx, hz, ryaw in C.footprint(kind):
        corners = []
        for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            lx, lz = rot(sx * hx, sz * hz, ryaw)
            lx, lz = cx + lx, cz + lz
            wx, wz = rot(lx, lz, yaw)
            corners.append((x + wx, z + wz))
        out.append(corners)
    return out


def _seg_dist(p, a, b):
    ax, az = b[0] - a[0], b[1] - a[1]
    t = max(0.0, min(1.0, ((p[0] - a[0]) * ax + (p[1] - a[1]) * az) / max(ax * ax + az * az,
                                                                         1e-12)))
    return math.hypot(a[0] + ax * t - p[0], a[1] + az * t - p[1])


def _inside(p, poly):
    c = False
    for a, b in zip(poly, poly[1:] + poly[:1]):
        if (a[1] > p[1]) != (b[1] > p[1]) and \
                p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
            c = not c
    return c


ON_LINE = {'supermarket_box_jump'}  # props the racing line is meant to drive over


def corridor_hits(margin=0.05):
    """Static obstacles reaching into the corridor: [(kind, x, z, depth)]."""
    ss = samples()
    hits = []
    for kind, x, y, z, yaw, d in PROPS:
        k = KINDS[kind]
        if k['dynamic'] or kind in ON_LINE or y > 3.0:
            continue
        worst = 0.0
        for poly in footprints(kind, x, z, yaw):
            for sx, sy, sz, hw, _ in ss[::2]:
                p = (sx, sz)
                dist = 0.0 if _inside(p, poly) else min(
                    _seg_dist(p, a, b) for a, b in zip(poly, poly[1:] + poly[:1]))
                worst = max(worst, hw + margin - dist)
        if worst > 0:
            hits.append((kind, x, z, worst))
    return hits


def pyramid(x, z, yaw=0.0, d='Both', label='tomato'):
    """10 catering tins stacked 6-3-1 (tetrahedral), centred on (x, z)."""
    s = 2 * C.CAN_R + 0.01
    h = s * math.sqrt(3) / 2
    layers = [[(0, 0), (s, 0), (2 * s, 0), (s / 2, h), (1.5 * s, h), (s, 2 * h)],
              [(s / 2, h / 3), (1.5 * s, h / 3), (s, 4 * h / 3)],
              [(s, 2 * h / 3)]]
    for k, layer in enumerate(layers):
        for lx, lz in layer:
            rx, rz = rot(lx - s, lz - 2 * h / 3, yaw)
            prop(f'supermarket_can_{label}', x + rx, z + rz, yaw, y=k * C.CAN_H, d=d)


def place_props():
    PROPS.clear()
    # Gondola runs: endcap, 7 two-bay gondolas, endcap; length along Z (yaw +-90).
    for gi, gx in enumerate(GONDOLA_X):
        prop(f'supermarket_endcap_{"abcd"[gi % 4]}', gx, ZGN + C.ENDCAP_DEPTH / 2, 0)
        for k in range(7):
            v = C.GONDOLA_VARIANTS[(gi * 3 + k) % 4]
            zc = ZGN + C.ENDCAP_DEPTH + C.GONDOLA_LEN * (k + 0.5)
            prop(f'supermarket_gondola_{v}', gx, zc, 90 if (gi + k) % 2 else -90)
        prop(f'supermarket_endcap_{"cdab"[gi % 4]}', gx, ZGS - C.ENDCAP_DEPTH / 2, 180)
    # Freezer island (back to back) and the wall freezers of the freezer aisle.
    n_isl = round((ISLAND_Z[1] - ISLAND_Z[0]) / C.FREEZER_W)
    for k in range(n_isl):
        zc = ISLAND_Z[0] + C.FREEZER_W * (k + 0.5)
        prop('supermarket_freezer' if k % 2 else 'supermarket_freezer_dark',
             ISLAND_W + C.FREEZER_D / 2, zc, 90)
        prop('supermarket_freezer_dark', ISLAND_W + C.FREEZER_D * 1.5, zc, -90)
    for k in range(12):
        zc = -57.0 + C.FREEZER_W * k
        prop('supermarket_freezer' if k % 2 == 0 else 'supermarket_freezer_dark',
             FREEZER_FACE_W - C.FREEZER_D / 2, zc, -90)
    # Dairy, meat and deli coolers along the north wall.
    for k in range(18):
        xc = X_MIN + 5.0 + C.COOLER_W * (k + 0.5)
        if xc + C.COOLER_W / 2 > X_MAX - 1:
            break
        prop('supermarket_cooler' if k % 2 == 0 else 'supermarket_cooler_dark',
             xc, Z_MIN + C.COOLER_D / 2, 180)
    # Checkout bank: each stand serves the lane on its west side; lanes 1-3 open, the rest gated;
    # the customer-service desk (facing the lobby) closes the line up to the freezer island.
    xs = stand_x(len(C.LANES))
    for lane, xc in zip(C.LANES, xs):
        prop(f'supermarket_checkout_{lane}', xc, STAND_Z, 0)
        if lane > 3:
            prop('supermarket_lane_gate', xc - (C.STAND_W + LANE_W.get(lane, 6.0)) / 2,
                 STAND_Z - C.STAND_L / 2 + 0.2, 0)
    x_east = xs[0] + C.STAND_W / 2
    x = xs[-1] - C.STAND_W / 2 - LANE_W.get(C.LANES[-1], 6.0)
    desk_x0 = ISLAND_W + 2 * C.FREEZER_D
    prop('supermarket_service_desk', (desk_x0 + x) / 2, STAND_Z, 180)
    # Entrance railing from the bank's east end to the east wall.
    xr = x_east
    while xr < X_MAX - 0.5:
        prop('supermarket_railing', xr + C.RAIL_L / 2, STAND_Z, 0)
        xr += C.RAIL_L
    # Box jumps (takeoff toward the north in both aisles).
    prop('supermarket_box_jump', XA, JUMP_Z, 0)
    prop('supermarket_box_jump', XB, JUMP_Z, 0)

    # Closed aisles: two pallets of stock across the mouth.
    def close(xc, zc):
        for s in (-1, 1):
            prop(f'supermarket_pallet_stack_{"ab"[s > 0]}', xc + s * C.PALLET_L / 2, zc, 0)
    north_mouth = ZGN - C.PALLET_W / 2
    xm = (XA + XB) / 2
    for ax in AISLE_X:
        if ax in (XA, XB):
            continue
        close(ax, north_mouth)
    close(xm, ZGS - C.PALLET_W / 2)               # aisle M, inside the U
    close(GONDOLA_X[0] - 8.0 - 0.3, north_mouth)  # aisle 0' between the island and row 1
    # Back aisle block between aisles A and B: pallets from M's mouth to the coolers.
    for k in range(4):
        prop(f'supermarket_pallet_stack_{"ab"[k % 2]}', xm,
             north_mouth - C.PALLET_W / 2 - C.PALLET_L * (k + 0.5) + 0.2, 90)
    # Promo wall between the U and the checkout chicane.
    xw = GONDOLA_X[4]
    for k in range(5):
        prop(f'supermarket_pallet_stack_{"ba"[k % 2]}', xw, ZGS + C.PALLET_L * (k + 0.5) - 0.6 * k,
             90)
    # Produce island: the water-pallet display (Golden Battery on top, narrow ramp forward
    # only) and two rows of fruit tables out to the last gondola row, so the S is forced.
    xg, z0, z1 = gb_line()
    prop('supermarket_water_ramp', xg, z0 - C.WATER_RAMP['run'] / 2, 180, d='Fwd')
    for k in range(3):
        prop('supermarket_water_pallet', xg, z0 + C.WATER_L * (k + 0.5), 90)
    x_t = xg - C.WATER_W / 2 - C.TABLE_W / 2
    for row in range(2):
        for k in range(3):
            prop('supermarket_produce_table', x_t - row * C.TABLE_W,
                 z1 - C.TABLE_L * (k + 0.5), 90)
    dress()
    # Knockable can pyramids: both endcaps inside the U, the outside of T1.
    for gx, label in ((GONDOLA_X[2], 'tomato'), (GONDOLA_X[3], 'beans')):
        pyramid(gx, ZGS + 1.6, 30.0, label=label)
    pyramid(XF - 3.0, ZL + 3.5, 0.0, label='peach')
    # Wet-floor signs where the ice starts and ends, the mop bucket that caused it.
    for sx, sz in ((XF - 4.8, 36.0), (XF + 4.6, 34.0), (XF - 4.6, -50.0), (XF + 4.8, -48.0)):
        prop('supermarket_wet_floor_sign', sx, sz, 90 if sx < XF else -90)
    prop('supermarket_mop_bucket', XF + 4.4, 40.0, 30)
    return PROPS


def gb_line():
    """Golden Battery line: x of the water pallets, z of the ramp top and of the far end."""
    z0 = -48.0 + C.WATER_RAMP['run']
    return XE - 9.0, z0, z0 + 3 * C.WATER_L


def dress():
    """Department dressing outside the corridor (checked by corridor_hits like the rest)."""
    # East wall: misted produce racks, bins of melons in front of them.
    for k in range(7):
        prop('supermarket_produce_rack', X_MAX - C.RACK_D / 2, -46.5 + C.RACK_L * k, 90)
    for bx, bz, yaw in ((86.5, -40.0, 90), (86.5, -26.0, 84), (88.0, 6.0, 90), (88.0, 16.0, 95),
                        (80.0, -62.0, 10), (88.0, -62.0, -5)):
        prop('supermarket_produce_bin', bx, bz, yaw)
    for cx, cz, yaw in ((84.0, 30.0, 0), (78.0, 31.0, 8), (76.0, -74.0, 0), (82.0, -75.0, 12)):
        prop('supermarket_crate_stack', cx, cz, yaw)
    # Lobby: trolley corral along the shopfront by the entrance, one stray trolley.
    for k in range(6):
        prop('supermarket_trolley', 44.0 + 1.25 * k, 82.6, 90)
    prop('supermarket_trolley', 74.0, 72.0, 160)
    prop('supermarket_trolley', -66.0, 64.5, 75)
    # Knockables: baskets left by the checkouts, cartons waiting by the back-aisle pallets.
    prop('supermarket_basket', -3.0, 68.5, 20)
    prop('supermarket_basket', 30.0, 68.8, -35)
    prop('supermarket_box_large', -27.0, -62.5, 12)
    prop('supermarket_box_small', -25.5, -60.2, -20)
    # EXIT signs with twin emergency heads (the night's light), on every wall.
    for x, z, yaw in ((-32.0, Z_MIN, 180), (40.0, Z_MIN, 180), (-38.0, Z_MAX, 0),
                      (23.0, Z_MAX, 0), (X_MIN, 66.0, -90), (X_MAX, 40.0, 90)):
        prop('supermarket_exit_sign', x, z, yaw)
    # Empty pallets stacked by the back-aisle block and in the west pocket.
    for x, z, yaw, n in ((-40.5, -77.4, 3, 3), (-62.0, 45.0, 80, 2)):
        for k in range(n):
            prop('supermarket_pallet', x, z, yaw + 4 * k, y=k * C.PALLET_H)
    # Seasonal stock in the dead-end pocket west of the U.
    for x, z, yaw in ((-80.0, 34.0, 8), (-74.0, 34.5, -6), (-68.0, 33.0, 90)):
        prop(f'supermarket_pallet_stack_{"ab"[int(x) % 2]}', x, z, yaw)
    # Produce floor: tables and bins between the route and the wall racks, bins of melons by
    # the last gondola row, a fruit table and watermelons greeting customers at the entrance.
    for x, z, yaw in ((86.0, -48.0, 0), (87.5, -6.0, 90), (87.0, 25.5, 0), (79.5, 22.0, 90)):
        prop('supermarket_produce_table', x, z, yaw)
    for x, z, yaw in ((46.0, -6.0, 3), (46.5, 2.5, -4), (46.0, 11.0, 6), (79.0, 45.0, 15),
                      (86.0, 47.0, -10)):
        prop('supermarket_produce_bin', x, z, yaw)
    for x, z, yaw in ((52.0, 18.0, 20), (50.0, 21.5, -12), (77.0, 40.0, 30)):
        prop('supermarket_crate_stack', x, z, yaw)
    # Entrance and lobby: water and soda promotions by the shopfront, a second trolley corral.
    for x, z, yaw in ((-108.0, 72.0, 90), (-96.0, 84.0, 0)):
        prop('supermarket_water_pallet', x, z, yaw)
    for x, z, yaw in ((34.0, 80.5, 0), (40.0, 80.5, 2), (88.0, 66.0, 90)):
        prop('supermarket_pallet_stack_b', x, z, yaw)
    for k in range(5):
        prop('supermarket_trolley', 84.0 - 1.25 * k, 82.6, -90)
    # Promenade: stock waiting in front of the closed aisles.
    for x, z, yaw in ((AISLE_X[5], ZGS + 2.4, 0), (AISLE_X[6], ZGS + 2.6, 3)):
        prop('supermarket_pallet_stack_a', x - 2.7, z, yaw)
        prop('supermarket_water_pallet', x + 2.7, z, yaw)


def stand_x(n):
    """Centres of the first n checkout stands (lane 1 = east), as place_props lays them out."""
    x = XL + LANE_W[2] / 2 + C.STAND_W + LANE_W[1] + C.STAND_W
    out = []
    for lane in C.LANES[:n]:
        out.append(x - C.STAND_W / 2)
        x -= C.STAND_W + LANE_W.get(lane, 6.0)
    return out


# --- decor (physics-free: assets/data/decor/aisle_rush.ron) -------------------------------
DECOR = []   # (model file stem, x, y, z, yaw)


def decor(stem, x, z, yaw=0.0, y=0.0):
    DECOR.append((stem, x, y, z, yaw))


def place_decor():
    DECOR.clear()
    on_route = {XA, XB}
    for n, ax in enumerate(AISLE_X, start=1):
        for zs in (ZGN + 12.0, ZGS - 12.0):
            decor(f'hanging_sign_{n}', ax, zs)
        # light rows: the route's aisles stay lit after closing, the closed aisles are dark
        for k in range(7):
            decor('ceiling_light_on' if ax in on_route or k % 3 == 1 else 'ceiling_light_off',
                  ax, ZGN + C.ENDCAP_DEPTH + C.GONDOLA_LEN * (k + 0.5))
    for k in range(10):                                  # freezer aisle: lit
        decor('ceiling_light_on', XF, -50.0 + C.GONDOLA_LEN * k)
    for k in range(17):                                  # back aisle and promenade rows
        x = X_MIN + 8.0 + C.GONDOLA_LEN * k
        if x > X_MAX - 6:
            break
        decor('ceiling_light_on' if k % 2 == 0 else 'ceiling_light_off', x, ZB, 90)
        decor('ceiling_light_off' if k % 3 else 'ceiling_light_on', x, ZP, 90)
        decor('ceiling_light_on' if k % 2 else 'ceiling_light_off', x, ZL, 90)
    for k in range(8):                                   # produce
        decor('ceiling_light_on' if k % 2 == 0 else 'ceiling_light_off', XE + 6.0,
              -50.0 + C.GONDOLA_LEN * k)
    # automatic doors inside the shopfront: the entrance by the trolleys, the exit behind the
    # checkouts; paper bags waiting on the open lanes' bagging shelves
    span = X_MAX - X_MIN
    decor('entrance_doors', 64.0, Z_MAX - 0.5)
    decor('entrance_doors', X_MIN + 0.5 * span, Z_MAX - 0.5)
    for k, xc in enumerate(stand_x(3)):
        decor('paper_bag', xc - C.STAND_W / 2 + C.BELT_W / 2 + 0.2, STAND_Z + 4.0, 15 * k - 10,
              y=C.STAND_H - 0.01)
        decor('paper_bag', xc - C.STAND_W / 2 + C.BELT_W / 2 + 0.5, STAND_Z + 6.1, 25 - 20 * k,
              y=C.STAND_H - 0.01)
    decor('hanging_sign_checkout', 6.0, 46.5)
    decor('hanging_sign_checkout', -36.0, 46.5)
    band = 2 * C.GONDOLA_LEN

    def bands(colour, x0, x1, z, yaw, along_x):
        n = max(1, round((x1 - x0) / band))
        for k in range(n):
            c = x0 + (x1 - x0) * (k + 0.5) / n
            decor(f'wall_band_{colour}', c if along_x else z, z if along_x else c, yaw)
    bands('blue', -62.0, 62.0, X_MIN, -90, False)          # west wall: frozen
    bands('teal', X_MIN + 2, -12.0, Z_MIN, 180, True)       # north wall: dairy
    bands('red', -12.0, X_MAX - 2, Z_MIN, 180, True)        # north wall: deli & meat
    bands('green', -62.0, 40.0, X_MAX, 90, False)           # east wall: produce
    decor('wall_sign_frozen', X_MIN, -4.0, -90)
    decor('wall_sign_dairy', -70.0, Z_MIN, 180)
    decor('wall_sign_deli', 30.0, Z_MIN, 180)
    decor('wall_sign_produce', X_MAX, -10.0, 90)
    decor('wall_sign_service', -74.6, Z_MAX, 0)
    # overstock on a few gondola tops (same pose as the gondola under it)
    for kind, x, y, z, yaw, d in PROPS:
        if kind.startswith('supermarket_gondola_') and (int(x) + int(z)) % 3 == 0:
            decor(f'topstock_{"ab"[int(z) % 2]}', x, z, yaw)


def decor_ron():
    out = ['// Aisle Rush decor: physics-free models (rc_render props/decor.rs). GENERATED by',
           '// tools/blender/props/supermarket/plan.py: edit that and re-run. Hanging aisle signs,',
           '// ceiling light rows (lit along the route), department signs, overstock on gondolas.',
           '(', '    decor: [']
    for stem, x, y, z, yaw in DECOR:
        s = (f'        (model: "models/props/supermarket/{stem}.glb", pos: ({_f(x)}, {_f(y)}, '
             f'{_f(z)})')
        if abs(yaw) > 1e-6:
            s += f', yaw: {_f(yaw)}'
        out.append(s + '),')
    out += ['    ],', ')', '']
    return '\n'.join(out)


# --- RON ---------------------------------------------------------------------------------------------

def _f(v):
    s = f'{v:.2f}'.rstrip('0').rstrip('.')
    return '0' if s in ('-0', '') else s


def track_ron(gaps):
    out = ['// Aisle Rush — supermarket after closing (GDD §7.1, art bible §3.5). ~800 u, 3 laps,',
           '// indoor ★★, Sunset and Night. GENERATED by tools/blender/props/supermarket/plan.py:',
           '// edit that and re-run. Plot: cargo run -p rc_track --example plot -- aisle_rush',
           '// [--rev] [--mirror]. Invisible ribbon on the vinyl floor (Tile, 0); the freezer aisle',
           '// floor is iced over (Ice ground); the box jumps, shelves, freezers and checkouts are',
           '// props (assets/data/props/supermarket.ron). Barriers = shelf / freezer kick plates.',
           '(', '    style: Invisible,', '    start: 0,', '    kill_height: -6,',
           '    wall_height: 1.2,',
           f'    bounds: (min: ({_f(X_MIN)}, {_f(Z_MIN)}), max: ({_f(X_MAX)}, {_f(Z_MAX)})),',
           '    points: [']
    cur = {}
    for i, p in enumerate(P):
        fields = [f'pos: ({_f(p["x"])}, {_f(p["y"])}, {_f(p["z"])})']
        if i == 0:
            fields.append('surface: Tile')
        for key, name in (('width', 'width'), ('margin', 'offroad_margin')):
            if key in p and cur.get(key) != p[key]:
                fields.append(f'{name}: {_f(p[key])}')
                cur[key] = p[key]
        if 'walls' in p and cur.get('walls') != p['walls']:
            fields.append(f'wall_left: {str(p["walls"][0]).lower()}, '
                          f'wall_right: {str(p["walls"][1]).lower()}')
            cur['walls'] = p['walls']
        note = f' // {i}' + (f': {p["note"]}' if 'note' in p else '')
        out.append(f'        ({", ".join(fields)}),{note}')
    out.append('    ],')
    out.append('    gaps: [')
    for a, b, d in gaps:
        out.append(f'        (from: {a}, to: {b}, dir: {d}),')
    out.append('    ],')
    out.append('    ground: [')
    for surface, h, poly, note in GROUND:
        pts = ', '.join(f'({_f(x)}, {_f(z)})' for x, z in poly)
        out.append(f'        // {note}')
        out.append(f'        (surface: {surface}, height: {_f(h)}, points: [{pts}]),')
    out.append('    ],')
    out.append('    item_rows: [')
    for x, y, z, d in ITEM_ROWS:
        out.append(f'        (pos: ({_f(x)}, {_f(y)}, {_f(z)})' +
                   (f', dir: {d}' if d != 'Both' else '') + '),')
    out.append('    ],')
    out.append('    shortcuts: [')
    for path, width, d, note in SHORTCUTS:
        pts = ', '.join(f'({_f(x)}, {_f(y)}, {_f(z)})' for x, y, z in path)
        out.append(f'        // {note}')
        out.append(f'        (path: [{pts}], width: {_f(width)}, dir: {d}),')
    out.append('    ],')
    gx, gy, gz = GOLDEN
    out.append(f'    golden_battery: (pos: ({_f(gx)}, {_f(gy)}, {_f(gz)})),')
    out.append('    props: [')
    for kind, x, y, z, yaw, d in PROPS:
        s = f'(kind: "{kind}", pos: ({_f(x)}, {_f(y)}, {_f(z)})'
        if abs(yaw) > 1e-6:
            s += f', yaw: {_f(yaw)}'
        if d != 'Both':
            s += f', dir: {d}'
        out.append(f'        {s}),')
    out += ['    ],', ')', '']
    return '\n'.join(out)


def blob(cx, cz, rx, rz, n=28, wobble=0.12, seed=3):
    """Organic closed outline (ice puddles): an ellipse with low-frequency wobble."""
    import random
    rnd = random.Random(seed)
    ph = [rnd.uniform(0, 2 * math.pi) for _ in range(3)]
    out = []
    for i in range(n):
        a = 2 * math.pi * i / n
        k = 1 + wobble * (math.sin(2 * a + ph[0]) * 0.6 + math.sin(3 * a + ph[1]) * 0.3 +
                          math.sin(5 * a + ph[2]) * 0.2)
        out.append((cx + rx * k * math.cos(a), cz + rz * k * math.sin(a)))
    return out


def clip_x(poly, x0, x1):
    return [(min(max(x, x0), x1), z) for x, z in poly]


GROUND, ITEM_ROWS, SHORTCUTS, GOLDEN = [], [], [], None


def features():
    global GOLDEN
    GROUND.clear()
    ITEM_ROWS.clear()
    SHORTCUTS.clear()
    GROUND.append(('Tile', 0.0, [(X_MIN, Z_MIN), (X_MAX, Z_MIN), (X_MAX, Z_MAX), (X_MIN, Z_MAX)],
                   'polished vinyl tile, wall to wall'))
    ice = clip_x(blob(XF, -8.0, 9.0, 50.0, n=36, wobble=0.1, seed=7),
                 FREEZER_FACE_W - 0.05, ISLAND_W + 0.05)
    GROUND.append(('Ice', 0.02, ice, 'frozen-over floor of the freezer aisle (a freezer leaks)'))
    ITEM_ROWS.extend([(-60.0, 0.0, ZL, 'Both'), (XF, 0.0, 22.0, 'Both'),
                      (XA, 0.0, -44.0, 'Both'), (20.0, 0.0, ZB, 'Both'),
                      (XE + S_DX, 0.0, -14.0, 'Both')])
    lane1 = XL + HW_L + C.STAND_W + LANE_W[1] / 2
    lane3 = XL - HW_L - C.STAND_W - LANE_W[3] / 2
    SHORTCUTS.append(([(XL + R9 + 2, 0, ZP), (lane1 + 1, 0, ZP + 6), (lane1, 0, 52),
                       (lane1, 0, 64), (lane1 - 4, 0, ZL - 1), (XL - R10 - 4, 0, ZL)],
                      5.0, 'Both', 'checkout lane 1 (east of lane 2)'))
    SHORTCUTS.append(([(XL + R9 + 2, 0, ZP), (lane3 + 3, 0, ZP + 1), (lane3, 0, 52),
                       (lane3, 0, 64), (lane3 - 3, 0, ZL - 1), (XL - R10 - 10, 0, ZL)],
                      5.0, 'Both', 'checkout lane 3 (west of lane 2)'))
    xg, z0, z1 = gb_line()
    top = C.WATER_H
    SHORTCUTS.append(([(XE - 2, 0, -54), (xg, 0, -48), (xg, top, z0), (xg, top, z1),
                       (xg + 1, 0, z1 + 12), (XE + 2, 0, z1 + 24)],
                      3.0, 'Fwd', 'over the water pallets (Golden Battery)'))
    GOLDEN = (xg, top + 1.0, (z0 + z1) / 2)


def main():
    a = sys.argv[1:]
    assets = a[a.index('--assets') + 1] if '--assets' in a else os.path.join(ROOT, 'assets')
    gaps = build_points()
    place_props()
    features()
    hits = corridor_hits()
    for kind, x, z, depth in hits:
        print(f'corridor: {kind} at ({x:.1f}, {z:.1f}) reaches {depth:.2f} u into the corridor')
    ss = samples(0.25)
    length = sum(math.dist(a[:3], b[:3]) for a, b in zip(ss, ss[1:] + ss[:1]))
    dyn = sum(1 for p in PROPS if KINDS[p[0]]['dynamic'])
    print(f'{len(P)} points, lap ≈ {length:.0f} u, {len(PROPS)} props ({dyn} dynamic)')
    place_decor()
    for name, text in (('tracks', track_ron(gaps)), ('decor', decor_ron())):
        path = os.path.join(assets, 'data', name, 'aisle_rush.ron')
        if editor_owned(path):
            continue
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w', encoding='utf-8') as f:
            f.write(text)
        print(f'wrote {path}')
    print(f'{len(DECOR)} decor models')
    return 1 if hits else 0


if __name__ == '__main__':
    sys.exit(main())
