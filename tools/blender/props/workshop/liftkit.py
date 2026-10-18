"""Two-post lift with a raised generic car (hero props lift_a / lift_b): blue clear-floor lift
(columns at x = +-7.1 u, overhead beam, carriages, telescopic arms with rubber pads under the
sills, power unit, hazard-wrapped column feet) and a lib/workshop_car body lifted by its sills,
wheels hanging. Origin at the lift centre on the floor; the car faces +Y (Bevy -Z).
"""
from wkit import *  # noqa: F401,F403
from lib import workshop_car as wc

L = C.LIFT
MID_WB = -0.3  # car mid-wheelbase y relative to the column line (the CG sits between columns)


def lift(sill):
    """Lift parts for a car whose sill pinch welds sit at height `sill`."""
    cx, cw, cd, ch = L['col_x'], L['col_w'], L['col_d'], L['col_h']
    arm_z = sill - 0.55
    blue, dark = mi('lift_blue'), mi('steel_dark')
    bm = core.new_bm()
    for s in (-1, 1):
        x = s * cx
        geo.box(bm, V((x, 0, (1.55 + ch - 0.4) / 2)), (cw, cd, ch - 0.4 - 1.55), blue)
        geo.box(bm, V((x, 0, ch - 0.2)), (cw + 0.12, cd + 0.12, 0.4), blue)  # top cap
        geo.box(bm, V((x - s * (cw / 2 + 0.02), 0, ch / 2)), (0.06, 0.9, ch - 0.3), dark)  # track
        geo.box(bm, V((x - s * (cw / 2 + 0.05), 0.62, ch / 2)), (0.06, 0.18, ch - 2.5),
                mi('steel'))  # safety lock rack
        # carriage on the inner face with the two arm pivots
        xc = x - s * (cw / 2 + 0.3)
        geo.box(bm, V((xc, 0, arm_z - 0.6)), (0.6, 1.9, 2.6), blue)
    hz = core.obj_from_bm('lift_cols', bm, bevel=0.05, segments=1, angle=40)

    bm = core.new_bm()
    for s in (-1, 1):  # hazard-wrapped column feet + base plates
        x = s * cx
        f = geo.box(bm, V((x, 0, 0.09 + 0.73)), (cw + 0.04, cd + 0.04, 1.46), mi('hazard'))
        box_uv(bm, f, scale=0.35)
        geo.box(bm, V((x, 0, 0.045)), (L['base_w'], L['base_d'], 0.09), dark)
    # overhead beam (clear-floor lift), power unit boxes, arm tubes
    geo.box(bm, V((0, 0, ch + 0.45)), (2 * cx + cw + 0.3, 1.0, 0.9), blue)
    ox = -cx - cw / 2
    geo.box(bm, V((ox - 0.33, 0.45, 4.9)), (0.55, 0.55, 1.1), mi('plastic_black'))
    geo.box(bm, V((ox - 0.15, -0.55, 6.0)), (0.3, 0.45, 0.55), mi('paint_grey'))
    pads = []
    for s in (-1, 1):
        piv_x = s * (cx - cw / 2 - 0.5)
        for fy in (-1, 1):
            piv = V((piv_x, fy * 0.6, arm_z))
            pad = V((s * 2.9, MID_WB + fy * (4.6 if fy > 0 else 4.2), arm_z))
            d = (pad - piv).normalized()
            mid = piv.lerp(pad, 0.55)
            geo.prism(bm, geo.rrect(0.62, 0.44, 0.0, 1)[::2], piv, mid, blue)
            geo.prism(bm, geo.rrect(0.48, 0.34, 0.0, 1)[::2], mid - d * 0.3, pad + d * 0.35,
                      mi('steel'))
            pads.append((piv, pad))
    feet = core.obj_from_bm('lift_boxes', bm, bevel=0.03, segments=1, angle=40)

    bm = core.new_bm()  # round parts: no bevel (their rounding is in the profile)
    for s in (-1, 1):
        x = s * cx
        for bx in (-1, 1):
            for by in (-1, 1):
                p = V((x + bx * (L['base_w'] / 2 - 0.25), by * (L['base_d'] / 2 - 0.25), 0.09))
                geo.cylinder(bm, p, p + V((0, 0, 0.12)), 0.09, mi('steel'), n=6)
    geo.tube(bm, [V((-cx, 0.62, ch - 0.6)), V((-cx + 0.4, 0.62, ch + 0.95)),
                  V((cx - 0.4, 0.62, ch + 0.95)), V((cx, 0.62, ch - 0.6))], 0.09,
             mi('plastic_black'), n=6, corner=0.5, cseg=2)
    geo.cylinder(bm, V((-cx + 1.2, 0, ch - 0.25)), V((cx - 1.2, 0, ch - 0.25)), 0.16,
                 mi('safety_yellow'), n=10)
    for piv, pad in pads:
        geo.cylinder(bm, piv - V((0, 0, 0.3)), piv + V((0, 0, 0.3)), 0.26, dark, n=10)
        geo.cylinder(bm, pad + V((0, 0, 0.2)), pad + V((0, 0, 0.42)), 0.26, mi('steel'), n=10)
        geo.cylinder(bm, pad + V((0, 0, 0.42)), pad + V((0, 0, 0.56)), 0.36, mi('rubber'), n=12)
    geo.cylinder(bm, V((ox - 0.35, -0.2, 4.2)), V((ox - 0.35, -0.2, 5.4)), 0.34, blue, n=12)
    geo.cylinder(bm, V((ox - 0.3, -0.55, 6.0)), V((ox - 0.65, -0.55, 6.25)), 0.04,
                 mi('plastic_red'), n=6)
    arms = core.obj_from_bm('lift_round', bm, weighted=False, angle=40)
    return [hz, feet, arms]


def build(key):
    """Raised car `key` (catalog CARS) on its lift; returns the parts for propkit.finish."""
    car = C.CARS[key]
    start('lift_blue', 'steel_dark', 'steel', 'hazard', 'plastic_black', 'safety_yellow',
          'rubber', 'paint_grey', 'plastic_red', 'tyre', 'chrome')
    wc.materials(paint=car['paint'])
    parts = wc.build(car['kind'], droop=0.22)
    raise_by = car['sill'] - 0.105 * wc.S
    for o in parts:
        o.location.z += raise_by
        o.location.y += MID_WB - (wc.PRESETS[car['kind']]['front_axle']
                                  - wc.PRESETS[car['kind']]['wheelbase'] / 2) * wc.S
    body = bake_scaled(parts, 'car')
    return [body] + lift(car['sill'])


def previews(name, key):
    """RC-eye views: approaching under the tail and looking up from beneath the car."""
    sill = C.CARS[key]['sill']
    extra_shots(name, [
        ('rc_under', (3.0, -15.0, 1.1), (0.0, -2.0, sill + 0.8), 24),
        ('rc_up', (1.5, -6.0, 0.9), (0.0, 2.0, sill + 2.0), 18),
        ('rc_side', (-16.0, -9.0, 1.2), (0.0, -2.0, sill), 24),
    ])
