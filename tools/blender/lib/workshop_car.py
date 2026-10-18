"""Generic, brand-free full-size cars for environment props (T16 workshop: cars raised on lifts;
usable for any parked car): a section-loft body (lib/shell.py) with real wheel wells, glass,
lamps, trim and a detailed underbody (seen from below by the RC cars), plus revolve / tyre
helpers shared by the workshop props.

The car is modelled at 1/S (RC-car size, where shell.py and geo.py tolerances are tuned) and the
finished parts are scaled up to real size x 4.44:

    from lib import workshop_car as wc
    wc.materials(paint='#4E6A82')            # adds the car_* materials that are missing
    parts = wc.build('hatch')                # objects at full scale, wheels on the ground
    ...                                      # move / raise them, add your prop, then bake

Blender axes: X right, Y forward (nose), Z up; origin at the ground-contact centre.
Material names: car_paint, car_glass, car_trim, car_lens, car_lens_red, car_lens_amber,
car_plate, car_under, car_rim, car_disc, car_exhaust, chrome, tyre (`tyre` and `chrome` are
reused when the scene already has them, e.g. a tread-textured tyre).
"""
import math

import bmesh
import bpy
from mathutils import Matrix, Vector

from . import core, geo, parts, shell

V = Vector
S = 9.0  # modelling scale: 1/S

MATERIALS = {
    'car_glass': dict(base='#141A22', rough=0.05),
    'car_trim': dict(base='#1E1F21', rough=0.6),
    'car_lens': dict(base='#D6DCE2', rough=0.08),
    'car_lens_red': dict(base='#8E1714', rough=0.15),
    'car_lens_amber': dict(base='#C06A14', rough=0.15),
    'car_plate': dict(base='#E6E5DE', rough=0.45),
    'car_under': dict(base='#4A4B4F', rough=0.78),
    'car_rim': dict(base='#C4C8CC', metallic=1.0, rough=0.34),
    'car_disc': dict(base='#7A7D81', metallic=1.0, rough=0.5),
    'car_exhaust': dict(base='#8A7A6C', metallic=0.6, rough=0.6),
    'chrome': dict(base='#F2F2F2', metallic=1.0, rough=0.08),
    'tyre': dict(base='#2A2A2C', rough=0.8),
}


def materials(paint, paint_rough=0.4, **overrides):
    """Create the car materials not yet in the scene (paint: desaturated sRGB hex, with the
    art bible's car clearcoat)."""
    spec = dict(MATERIALS, car_paint=dict(base=paint, rough=paint_rough, coat=1.0,
                                          coat_rough=0.07), **overrides)
    for name, m in spec.items():
        if name not in core.MATS:
            core.material(name, m['base'], m.get('metallic', 0.0), m.get('rough', 0.5),
                          coat=m.get('coat', 0.0), coat_rough=m.get('coat_rough', 0.04))


# ---------------------------------------------------------------------------------------------
# revolve / tyre helpers (full scale or model scale alike)

def place(bm, verts, center=(0, 0, 0), axis=(0, 0, 1), spin=0.0):
    """Rotate verts built about +Z so the axis points along `axis` (after spinning `spin` deg
    about Z), then move them to `center`."""
    rot = V((0, 0, 1)).rotation_difference(V(axis).normalized()).to_matrix().to_4x4()
    m = Matrix.Translation(V(center)) @ rot @ Matrix.Rotation(math.radians(spin), 4, 'Z')
    bmesh.ops.transform(bm, matrix=m, verts=list(verts))


def revolve(bm, profile, mat, n=32, uv_rep=None):
    """Revolve a (radius, z) polyline about +Z; a profile walking up the outside faces out.
    uv: u = fraction of the turn x uv_rep, v = normalised arc length. Returns (faces, verts)."""
    rings = []
    for k in range(n + 1):  # duplicate seam column for clean UVs (welded on object creation)
        a = 2 * math.pi * k / n
        c, s = math.cos(a), math.sin(a)
        rings.append([bm.verts.new((r * c, r * s, z)) for r, z in profile])
    lens = [0.0]
    for (r0, z0), (r1, z1) in zip(profile, profile[1:]):
        lens.append(lens[-1] + math.hypot(r1 - r0, z1 - z0))
    total = lens[-1] or 1.0
    uv = bm.loops.layers.uv.verify() if uv_rep else None
    fs = []
    for k in range(n):
        a, b = rings[k], rings[k + 1]
        for j in range(len(profile) - 1):
            f = bm.faces.new((a[j], b[j], b[j + 1], a[j + 1]))
            f.material_index = mat
            if uv:
                for lp, (kk, jj) in zip(f.loops, ((k, j), (k + 1, j), (k + 1, j + 1),
                                                  (k, j + 1))):
                    lp[uv].uv = (kk / n * uv_rep, lens[jj] / total)
            fs.append(f)
    verts = [v for r in rings for v in r]
    axis = [v for v in verts if v.co.x ** 2 + v.co.y ** 2 < 1e-12]
    if axis:
        bmesh.ops.remove_doubles(bm, verts=axis, dist=1e-6)
    return [f for f in fs if f.is_valid], [v for v in verts if v.is_valid]


def tyre_profile(r, w, rim_r, seg=3, bottom=True, top=True):
    """(radius, z) tyre section from the inner bead (z = -w/2) round the tread to the outer
    bead (z = +w/2): bulged sidewalls, rounded shoulders. Without `bottom` / `top` the section
    starts / ends at the sidewall bulge (faces hidden between stacked tyres are left out)."""
    h = r - rim_r
    sh = min(0.17 * w, 0.32 * h)
    hw = w / 2
    pts = [(rim_r + 0.02 * h, -hw * 0.86), (rim_r + 0.45 * h, -hw * 1.02),
           (r - sh * 0.35, -hw * 0.98), (r, -hw * 0.62), (r, hw * 0.62),
           (r - sh * 0.35, hw * 0.98), (rim_r + 0.45 * h, hw * 1.02),
           (rim_r + 0.02 * h, hw * 0.86)]
    out = geo.rounded_poly([V(p) for p in pts],
                           [0, 0.4 * h, sh * 0.8, sh, sh, sh * 0.8, 0.4 * h, 0],
                           seg=[0, 2, seg, seg, seg, seg, 2, 0], closed=False)
    out = [(p.x, p.y) for p in out]
    out = out[(0 if bottom else 2):len(out) - (0 if top else 2)]
    clean = [out[0]]
    for p in out[1:]:
        if math.dist(p, clean[-1]) > 1e-7:
            clean.append(p)
    return clean


def tyre(bm, center, axis, r, w, rim_r, n=32, mat='tyre', inner_mat='car_under', inner=True,
         bottom=True, top=True, spin=0.0, seg=3):
    """Tyre about `axis`; with `inner`, a dark inner casing fills the bead hole's view."""
    fs, vs = revolve(bm, tyre_profile(r, w, rim_r, seg, bottom, top), core.mi(mat), n=n,
                     uv_rep=12)
    if inner:
        ip = [(rim_r + 0.02 * (r - rim_r), w / 2 * 0.86), (r - 0.09 * w, w * 0.3),
              (r - 0.09 * w, -w * 0.3), (rim_r + 0.02 * (r - rim_r), -w / 2 * 0.86)]
        f2, v2 = revolve(bm, ip, core.mi(inner_mat), n=max(12, n // 2))
        fs += f2
        vs += v2
    place(bm, vs, center, axis, spin)
    return fs


# ---------------------------------------------------------------------------------------------
# car presets (model scale, metres x 4.44 / S)

def _m(x):
    return x * 4.44 / S


PRESETS = {
    # compact five-door hatchback, 4.26 x 1.79 x 1.45 m, alloy wheels
    'hatch': dict(
        length=_m(4.26), wheelbase=_m(2.62), front_axle=_m(4.26 / 2 - 0.88), track=_m(1.54),
        tyre_r=_m(0.316), tyre_w=_m(0.205), rim_r=_m(0.203), clearance=_m(0.14),
        rim='alloy', ends=(0.03, 0.05), end_scale=0.9,
        keys=[
            (-1.05, dict(hw=0.40, z0=0.16, zb=0.40, hg=0.39, zg=0.405, hr=0.37, zt=0.415,
                         rs=0.03, rb=0.04, rm=0.25)),
            (-1.01, dict(hw=0.425, zb=0.47, hg=0.40, zg=0.475, hr=0.36, zt=0.49)),
            (-0.95, dict(zt=0.555, hr=0.345)),
            (-0.89, dict(zt=0.625, hr=0.33)),
            (-0.82, dict(hw=0.44, z0=0.105, zt=0.655, hr=0.322, crown=0.006)),
            (-0.675, dict(hw=0.446, zb=0.50, hg=0.40, zg=0.505, hr=0.325, zt=0.684,
                          bulge=0.004)),
            (-0.40, dict(zt=0.71)),
            (-0.30, dict(hw=0.443, zb=0.488, hg=0.395, zg=0.493, hr=0.335, zt=0.714,
                         crown=0.012)),
            (0.00, dict(zt=0.712, hr=0.333)),
            (0.05, dict(zt=0.70, hr=0.325)),
            (0.25, dict(zt=0.60, hr=0.36, hg=0.405, zg=0.483)),
            (0.42, dict(zt=0.497, zg=0.488, hg=0.415, hr=0.40, zb=0.482)),
            (0.616, dict(hw=0.443, zb=0.478, hg=0.42, zg=0.482, hr=0.395, zt=0.488,
                         bulge=0.004)),
            (0.86, dict(hw=0.435, z0=0.10, zb=0.458, hg=0.405, zg=0.463, hr=0.375, zt=0.472)),
            (1.05, dict(hw=0.405, z0=0.09, zb=0.418, hg=0.38, zg=0.423, hr=0.35, zt=0.436,
                        rs=0.04)),
        ],
        windshield=(0.06, 0.41), side=((-0.04, 0.39), (-0.50, -0.08), (-0.72, -0.545)),
        rear=(-0.985, -0.84), pillars=(0.42, -0.06, -0.52),
        head=dict(x=0.295, z=0.375, w=0.17, h=0.068), tail=dict(x=0.31, z=0.44, w=0.11,
                                                                  h=0.07),
        plate_f=0.235, plate_r=0.33, mirror_y=0.40, grille_z=0.37),
    # four-door saloon, 4.62 x 1.81 x 1.44 m, steel wheels with plastic trims
    'saloon': dict(
        length=_m(4.62), wheelbase=_m(2.71), front_axle=_m(4.62 / 2 - 0.92), track=_m(1.55),
        tyre_r=_m(0.316), tyre_w=_m(0.205), rim_r=_m(0.203), clearance=_m(0.14),
        rim='steel', ends=(0.04, 0.05), end_scale=0.88,
        keys=[
            (-1.14, dict(hw=0.41, z0=0.16, zb=0.43, hg=0.40, zg=0.435, hr=0.38, zt=0.445,
                         rs=0.035, rb=0.04, rm=0.25)),
            (-1.08, dict(hw=0.43, zb=0.475, hg=0.41, zg=0.48, hr=0.385, zt=0.488)),
            (-0.90, dict(hw=0.445, z0=0.105, zb=0.49, hg=0.415, zg=0.495, hr=0.39, zt=0.505)),
            (-0.70, dict(zb=0.495, zg=0.50, hg=0.405, hr=0.37, zt=0.52)),
            (-0.652, dict(hw=0.452, bulge=0.004)),
            (-0.55, dict(zt=0.61, hr=0.35, hg=0.40)),
            (-0.42, dict(zt=0.69, hr=0.335, crown=0.01)),
            (-0.10, dict(hw=0.448, zb=0.483, hg=0.395, zg=0.488, hr=0.336, zt=0.712,
                         crown=0.012)),
            (0.10, dict(zt=0.70, hr=0.33)),
            (0.30, dict(zt=0.60, hr=0.36, hg=0.405, zg=0.483)),
            (0.50, dict(zt=0.497, zg=0.49, hg=0.415, hr=0.40, zb=0.485)),
            (0.686, dict(hw=0.45, zb=0.48, hg=0.42, zg=0.485, hr=0.395, zt=0.49,
                         bulge=0.004)),
            (0.96, dict(hw=0.437, z0=0.10, zb=0.46, hg=0.405, zg=0.465, hr=0.375, zt=0.474)),
            (1.14, dict(hw=0.405, z0=0.09, zb=0.418, hg=0.38, zg=0.423, hr=0.35, zt=0.436,
                        rs=0.04)),
        ],
        windshield=(0.12, 0.49), side=((0.06, 0.48), (-0.40, 0.02), (-0.56, -0.43)),
        rear=(-0.69, -0.445), pillars=(0.50, 0.04, -0.45),
        head=dict(x=0.30, z=0.375, w=0.18, h=0.066), tail=dict(x=0.31, z=0.445, w=0.16,
                                                                 h=0.055),
        plate_f=0.235, plate_r=0.355, mirror_y=0.48, grille_z=0.37),
}


def _glass(bm, surf, prof, windshield=None, side=None, rear=None, m=0.02, r=0.025, h=0.045):
    """shell.glass with a coarser decal fill (`h`): windshield / rear window projected from
    above between the roof-edge lines, side windows from the sides between belt and roof."""
    mat = core.mi('car_glass')

    def hr(y):
        return prof.at(y)['hr']

    for rng in (windshield, rear):
        if rng:
            ys = [rng[0] + (rng[1] - rng[0]) * i / 6 for i in range(7)]
            right = [V((hr(y) - m, y)) for y in ys]
            poly = right + [V((-p.x, p.y)) for p in reversed(right)]
            poly = geo.rounded_poly(geo.ccw(poly), r, 2)
            geo.decal(bm, surf, poly, V((0, 0, 2.0)), V((1, 0, 0)), V((0, 1, 0)), mat, back=0.2,
                      h=h)
    if side:
        ys = [side[0] + (side[1] - side[0]) * i / 8 for i in range(9)]
        bot = [V((y, prof.at(y)['zg'] + m * 0.7)) for y in ys]
        top = [V((y, prof.at(y)['zt'] - prof.at(y)['rr'] * 0.6 - m * 0.5)) for y in ys]
        ok = [t.y - b.y >= 0.012 for t, b in zip(top, bot)]
        i0 = ok.index(True)
        i1 = len(ok) - ok[::-1].index(True)
        bot, top = bot[i0:i1], top[i0:i1]
        for s in (-1, 1):
            poly = [V((s * p.x, p.y)) for p in bot + top[::-1]]
            poly = geo.rounded_poly(geo.ccw(poly), r, 2)
            geo.decal(bm, surf, poly, V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)), mat,
                      back=0.2, h=h)


def _surf_dir(front):
    return (V((-1, 0, 0)), V((0, 0, 1))) if front else (V((1, 0, 0)), V((0, 0, 1)))


def _body(P):
    HL = P['length'] / 2
    prof = shell.Profile(P['keys'])
    D = dict(wheelbase=P['wheelbase'], radius=(P['tyre_r'],) * 2, wheel_width=(P['tyre_w'],) * 2,
             track=(P['track'],) * 2)
    arches = shell.arches(D, clear=0.018, gap=0.012)
    # shell.arches centres the axles on y = 0; shift them to the preset's axle positions
    yf = P['front_axle']
    yr = yf - P['wheelbase']
    arches = [(yf if a[0] > 0 else yr, a[1], a[2], a[3]) for a in arches]
    bm = core.new_bm()
    shell.shell(bm, prof, -HL, HL, arches=arches, seg=2, step=0.16, ends=P['ends'],
                end_scale=P['end_scale'], min_thick=0.02)
    for f in bm.faces:  # shell.py paints with `paint` / `plastic`: map to the car materials
        f.material_index = core.mi('car_paint') if f.material_index == core.mi('paint') \
            else core.mi('car_under')
    ob = core.obj_from_bm('car_shell', bm, weighted=False, angle=48)
    return prof, ob, arches


def build(kind='hatch', droop=0.0):
    """The car's parts (objects) at full scale, standing on its wheels; `droop` (u, full scale)
    lowers the wheels relative to the body (a car lifted by its sills)."""
    P = PRESETS[kind]
    for n in ('paint', 'plastic'):  # slots shell.py / parts.py write: remapped below
        if n not in core.MATS:
            core.material(n, '#808080', 0.0, 0.5)
    prof, body, arches = _body(P)
    surf = geo.Surface([body])
    mi = core.mi
    HL = P['length'] / 2
    bm = core.new_bm()
    # glass
    _glass(bm, surf, prof, windshield=P['windshield'], rear=P['rear'], h=0.06)
    for rng in P['side']:
        _glass(bm, surf, prof, side=rng, m=0.016, r=0.02, h=0.065)
    # lamps, grille, plates
    h, t = P['head'], P['tail']
    parts.lamps(bm, surf, geo.ccw([V(p) for p in ((-h['w'] / 2, -h['h'] / 2),
                                                  (h['w'] / 2, -h['h'] * 0.2),
                                                  (h['w'] / 2, h['h'] / 2),
                                                  (-h['w'] / 2, h['h'] / 2))]),
                h['x'], h['z'], True, 'car_lens', height=0.004, bezel='chrome')
    parts.lamps(bm, surf, geo.rrect(t['w'], t['h'], 0.012, 2), t['x'], t['z'], False,
                'car_lens_red', height=0.004)
    fr, rr = _surf_dir(True), _surf_dir(False)
    geo.decal(bm, surf, geo.rrect(0.30, 0.052, 0.015, 2, 0, P['grille_z']), V((0, 3, 0)), *fr,
              mi('car_trim'), offset=0.003)  # grille
    geo.decal(bm, surf, geo.rrect(0.50, 0.065, 0.02, 2, 0, 0.175), V((0, 3, 0)), *fr,
              mi('car_trim'), offset=0.003)  # lower intake
    geo.decal(bm, surf, geo.rrect(0.255, 0.054, 0.006, 1, 0, P['plate_f']), V((0, 3, 0)), *fr,
              mi('car_plate'), offset=0.005)
    geo.decal(bm, surf, geo.rrect(0.255, 0.054, 0.006, 1, 0, P['plate_r']), V((0, -3, 0)), *rr,
              mi('car_plate'), offset=0.005)
    for s in (-1, 1):
        geo.decal(bm, surf, geo.rrect(0.06, 0.012, 0.004, 1, s * 0.33, 0.21), V((0, -3, 0)),
                  *rr, mi('car_lens_red'), offset=0.003)  # bumper reflectors
        side = (V((s * 2.0, 0, 0)), V((0, s, 0)), V((0, 0, 1)))
        for y in P['pillars']:  # door shut lines
            geo.strip(bm, surf, [(s * y, 0.125), (s * y, 0.468)], 0.004, *side, mi('car_trim'),
                      offset=0.0015, step=0.08, back=0.2)
        geo.strip(bm, surf, [(s * (P['pillars'][0] + 0.02), 0.13),
                             (s * (P['pillars'][2] - 0.02), 0.13)], 0.004, *side, mi('car_trim'),
                  offset=0.0015, step=0.12, back=0.2)  # sill line
        for y in (P['pillars'][0] - 0.12, P['pillars'][1] - 0.12):  # door handles
            geo.solid_decal(bm, surf, geo.rrect(0.05, 0.012, 0.005, 1, s * y, 0.432), *side,
                            mi('car_trim'), height=0.003, back=0.2)
        ya = P['front_axle'] + 0.11
        geo.decal(bm, surf, geo.rrect(0.03, 0.01, 0.004, 1, s * ya, 0.43), *side,
                  mi('car_lens_amber'), offset=0.002, back=0.2)  # side repeater
    for f in bm.faces:  # lamp side walls default to `plastic`
        if f.material_index == mi('plastic'):
            f.material_index = mi('car_trim')
    details = core.obj_from_bm('car_details', bm, weighted=False, angle=60)

    # hard trim: bumper lips, mirrors, wipers, roof stub antenna
    bm = core.new_bm()
    tr = mi('car_trim')
    for s in (-1, 1):
        my = P['mirror_y']
        z = prof.at(my)['zg'] + 0.025
        x = prof.at(my)['hg'] + 0.03
        geo.slab(bm, [(my - 0.03, 0.035, z - 0.022, z + 0.022, 0.01),
                      (my + 0.012, 0.03, z - 0.02, z + 0.02, 0.01)], mi('car_paint'),
                 cx=s * (x + 0.03))
        geo.box(bm, V((s * (x + 0.005), my, z - 0.01)), (0.03, 0.03, 0.02), tr)
    ws = P['windshield'][1] - 0.01
    zw = prof.at(ws)['zt'] + 0.006
    for x0 in (-0.26, 0.02):
        geo.box(bm, V((x0 + 0.12, ws, zw)), (0.24, 0.008, 0.006), tr)
    for y, z0 in ((HL - 0.02, 0.09), (-HL + 0.025, 0.152)):  # bumper lips
        geo.slab(bm, [(y - 0.02, 0.36, z0 - 0.012, z0 + 0.012, 0.006),
                      (y + 0.02, 0.36, z0 - 0.012, z0 + 0.012, 0.006)], tr)
    hit, _ = surf.project(V((0, -0.55, 2.0)), V((0, 0, -1)), 0.0, back=0.0)
    if hit is not None:
        geo.cylinder(bm, hit - V((0, 0, 0.004)), hit + V((0, -0.025, 0.045)), 0.006, tr, n=8,
                     r1=0.003)
    trim = core.obj_from_bm('car_trim', bm, bevel=0.003, segments=1, angle=35)

    under = _underbody(P, arches)
    wheels = _wheels(P, droop / S)
    objs = [body, details, trim, under] + wheels
    for o in objs:  # model scale -> full scale (modifiers stay in model units: scale with it)
        o.scale = (S, S, S)
        o.location = o.location * S
    return objs


def _underbody(P, arches):
    """Seen from below: floor tunnel, exhaust with cat and silencers, fuel tank, sump, gearbox,
    subframes, arms, struts, springs, driveshafts, rear beam axle."""
    mi = core.mi
    bm = core.new_bm()
    ub, ex, metal = mi('car_under'), mi('car_exhaust'), mi('car_disc')
    yf = P['front_axle']
    yr = yf - P['wheelbase']
    HL = P['length'] / 2
    z0 = 0.098  # floor level (shell underside is at ~0.10)
    tw = P['track'] / 2
    # engine sump and gearbox (front, transverse engine)
    geo.box(bm, V((0, yf + 0.07, z0 - 0.012)), (0.30, 0.20, 0.06), mi('car_disc'))
    geo.box(bm, V((-0.16, yf - 0.04, z0 + 0.0)), (0.20, 0.12, 0.06), ub)
    # front subframe and lower arms to the hubs
    geo.box(bm, V((0, yf - 0.12, z0 - 0.012)), (0.56, 0.035, 0.022), ub)
    geo.box(bm, V((0, yf + 0.26, z0 - 0.01)), (0.50, 0.03, 0.02), ub)
    for s in (-1, 1):
        parts.a_arm(bm, (s * 0.20, yf - 0.10, z0 - 0.01), (s * 0.20, yf + 0.04, z0 - 0.01),
                    (s * (tw - 0.05), yf, P['tyre_r'] - 0.04), w=0.018, t=0.012, mat='car_under')
        geo.cylinder(bm, V((s * 0.11, yf - 0.01, P['tyre_r'] - 0.025)),
                     V((s * (tw - 0.06), yf, P['tyre_r'] - 0.02)), 0.009, metal, n=6)  # driveshaft
        for xb in (0.16, tw - 0.1):  # CV boots
            geo.cylinder(bm, V((s * (xb - 0.02), yf - 0.01, P['tyre_r'] - 0.025)),
                         V((s * (xb + 0.02), yf - 0.01, P['tyre_r'] - 0.025)), 0.016, ub, n=8,
                         r1=0.012)
        # front strut + coil spring up into the arch
        geo.cylinder(bm, V((s * (tw - 0.09), yf - 0.02, P['tyre_r'] - 0.01)),
                     V((s * (tw - 0.12), yf - 0.04, 0.37)), 0.012, metal, n=8)
        geo.helix(bm, V((s * (tw - 0.10), yf - 0.03, P['tyre_r'] + 0.05)),
                  V((s * (tw - 0.12), yf - 0.04, 0.33)), 0.026, 0.004, 3, mi('car_rim'), n=4,
                  steps_per_turn=5)
    # exhaust: downpipe, catalytic converter, centre silencer, rear box and tailpipe
    path = [V((-0.05, yf + 0.12, z0 + 0.04)), V((-0.05, yf - 0.08, z0 - 0.035)),
            V((0.02, yf - 0.30, z0 - 0.035)), V((0.02, yr + 0.35, z0 - 0.035)),
            V((0.10, yr + 0.05, z0 - 0.03)), V((0.18, yr - 0.12, z0 - 0.025))]
    geo.tube(bm, path, 0.012, ex, n=6, corner=0.06, cseg=2)
    geo.cylinder(bm, V((-0.02, yf - 0.16, z0 - 0.035)), V((0.0, yf - 0.32, z0 - 0.035)), 0.03,
                 ex, n=10)
    geo.cylinder(bm, V((0.02, -0.02, z0 - 0.032)), V((0.02, -0.20, z0 - 0.032)), 0.026, ex,
                 n=10)
    geo.slab(bm, [(yr - 0.18, 0.13, z0 - 0.075 + 0.02, z0 + 0.0, 0.03),
                  (yr - 0.06, 0.13, z0 - 0.075 + 0.02, z0 + 0.0, 0.03)], ex, seg=2, cx=0.20)
    tip = V((0.24, -HL + 0.02, z0 - 0.02))
    geo.tube(bm, [V((0.24, yr - 0.18, z0 - 0.03)), tip], 0.012, ex, n=8)
    geo.cylinder(bm, tip, tip + V((0, -0.02, 0)), 0.016, mi('chrome'), n=12)
    for y0, y1 in ((yf - 0.14, yf - 0.34), (0.0, -0.22)):  # aluminium heat shields
        geo.box(bm, V((0.01, (y0 + y1) / 2, z0 - 0.002)), (0.11, abs(y1 - y0), 0.006),
                mi('car_rim'))
    for y in (yf - 0.25, -0.1, yr + 0.3):  # exhaust hangers
        geo.box(bm, V((0.02, y, z0 - 0.012)), (0.08, 0.012, 0.012), ub)
    # fuel tank ahead of the rear axle, floor tunnel and cross members
    geo.slab(bm, [(yr + 0.10, 0.27, z0 - 0.028, z0 + 0.0, 0.015),
                  (yr + 0.36, 0.30, z0 - 0.03, z0 + 0.0, 0.015)], mi('car_trim'), seg=1)
    geo.slab(bm, [(yf - 0.20, 0.06, z0 - 0.012, z0 + 0.005, 0.01),
                  (yr + 0.40, 0.06, z0 - 0.012, z0 + 0.005, 0.01)], ub, seg=1, cx=0.02)
    for y in (yf - 0.30, 0.0):
        geo.box(bm, V((0, y, z0 - 0.006)), (0.70, 0.02, 0.012), ub)
    # rear twist-beam axle, trailing arms, dampers and springs
    geo.box(bm, V((0, yr + 0.02, P['tyre_r'] - 0.035)), (2 * tw - 0.20, 0.03, 0.028), ub)
    for s in (-1, 1):
        geo.box(bm, V((s * (tw - 0.10), yr + 0.14, P['tyre_r'] - 0.035)), (0.035, 0.30, 0.03),
                ub)
        geo.helix(bm, V((s * (tw - 0.18), yr + 0.03, P['tyre_r'] - 0.02)),
                  V((s * (tw - 0.18), yr + 0.03, 0.30)), 0.025, 0.004, 3, mi('car_rim'), n=4,
                  steps_per_turn=5)
        geo.cylinder(bm, V((s * (tw - 0.08), yr - 0.03, P['tyre_r'] - 0.03)),
                     V((s * (tw - 0.10), yr - 0.06, 0.33)), 0.011, metal, n=8)
    return core.obj_from_bm('car_under', bm, weighted=False, angle=40)


def _wheels(P, droop):
    """Four wheels (tyre, rim, brake disc and caliper), axis X, lowered by `droop`."""
    mi = core.mi
    yf = P['front_axle']
    yr = yf - P['wheelbase']
    r, w, rim_r = P['tyre_r'], P['tyre_w'], P['rim_r']
    out = []
    for name, y in (('f', yf), ('r', yr)):
        for s in (-1, 1):
            c = V((s * P['track'] / 2, y, r - droop))
            bm = core.new_bm()
            tyre(bm, c, V((s, 0, 0)), r, w, rim_r, n=16, seg=1, inner=False)
            _rim(bm, c, s, rim_r, w, P['rim'])
            out.append(core.obj_from_bm(f'wheel_{name}{"lr"[s > 0]}', bm, weighted=False,
                                        angle=40))
    return out


def _rim(bm, c, s, rim_r, w, style):
    """Rim (face toward +s*X), barrel, brake disc and caliper behind it."""
    mi = core.mi
    hw = w / 2
    rim = mi('car_rim') if style == 'alloy' else mi('car_under')
    # barrel + lip (outer face at +hw*0.82)
    prof = [(rim_r * 0.97, -hw * 0.86), (rim_r * 1.01, -hw * 0.82), (rim_r * 0.92, -hw * 0.6),
            (rim_r * 0.92, hw * 0.55), (rim_r * 1.01, hw * 0.8), (rim_r * 0.99, hw * 0.86),
            (rim_r * 0.9, hw * 0.84)]
    fs, vs = revolve(bm, prof, rim, n=16)
    if style == 'alloy':  # 5 twin spokes on a hub
        hub = [(rim_r * 0.28, hw * 0.78), (rim_r * 0.24, hw * 0.86), (0.0, hw * 0.86)]
        f2, v2 = revolve(bm, hub, rim, n=16)
        vs += v2
        for i in range(5):  # five spokes
            a = 2 * math.pi * i / 5
            p0 = V((rim_r * 0.24 * math.cos(a), rim_r * 0.24 * math.sin(a), hw * 0.80))
            p1 = V((rim_r * 0.9 * math.cos(a), rim_r * 0.9 * math.sin(a), hw * 0.72))
            f3 = geo.prism(bm, geo.rrect(rim_r * 0.2, rim_r * 0.1, rim_r * 0.04, 1),
                           p0, p1, rim, up=V((0, 0, 1)), scale1=0.75)
            vs += list({v for f in f3 for v in f.verts})
        back = [(rim_r * 0.88, hw * 0.3), (0.0, hw * 0.3)]  # dark disc behind the spokes
        f4, v4 = revolve(bm, back, mi('car_under'), n=16)
        vs += v4
    else:  # steel wheel behind a silver plastic trim with vents
        trim = [(rim_r * 0.98, hw * 0.84), (rim_r * 0.9, hw * 0.95), (rim_r * 0.55, hw * 1.0),
                (rim_r * 0.2, hw * 1.04), (0.0, hw * 1.05)]
        f2, v2 = revolve(bm, trim, mi('car_rim'), n=16)
        vs += v2
        for i in range(6):
            a = 2 * math.pi * (i + 0.5) / 6
            cc = V((rim_r * 0.62 * math.cos(a), rim_r * 0.62 * math.sin(a), hw * 1.0))
            f3 = geo.cylinder(bm, cc - V((0, 0, 0.003)), cc + V((0, 0, 0.004)), rim_r * 0.14,
                              mi('car_under'), n=6)
            vs += list({v for f in f3 for v in f.verts})
    # brake disc + caliper (inside the barrel, visible through the spokes and from behind)
    disc = [(rim_r * 0.8, -hw * 0.1), (rim_r * 0.8, hw * 0.12), (0.0, hw * 0.12)]
    f5, v5 = revolve(bm, disc, mi('car_disc'), n=16)
    vs += v5
    cal = geo.box(bm, V((0, rim_r * 0.62, -hw * 0.05)), (rim_r * 0.5, rim_r * 0.28, hw * 0.55),
                  mi('car_under'))
    vs += list({v for f in cal for v in f.verts})
    m = V((s, 0, 0))
    place(bm, list(set(v for v in vs if v.is_valid)), c, m)
