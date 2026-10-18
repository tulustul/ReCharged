"""Blender diorama of a dressed track for checking the environment before the game draws props:
ribbon (ToyTrack profile of crates/rc_track/src/mesh.rs) and barriers swept along the spline of
supports.py, ground polygons, room walls at `bounds`, every prop of the variant (glb from the
catalog, or grey collider boxes while a model is missing), item rows and the Golden Battery.
Renders a top view, overviews and chase-camera views.

    blender -b -P tools/blender/props/toy/diorama.py -- OUT_DIR [--rev] [--mirror]
        [--views top,overview,chase] [--chase S1,S2,…] [--res 1280x720] [--samples 24]
        [--cam x,y,z:tx,ty,tz …]   (extra free cameras in track coordinates)

Track coordinates (Bevy: x right, y up, z back) map to Blender as (x, -z, y).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403
import bpy  # noqa: E402
import supports  # noqa: E402
from lib import render, toy_ron  # noqa: E402

ROOT = propkit.ROOT
argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
OUT = argv[0] if argv and not argv[0].startswith('--') else os.path.join(ROOT, 'target', 'diorama')
REV, MIRROR = '--rev' in argv, '--mirror' in argv


def opt(name, default):
    return argv[argv.index(name) + 1] if name in argv else default


TRACK = opt('--track', 'toy_town')
VIEWS = opt('--views', 'top,overview,chase').split(',')


def B(p):
    """Track (Bevy) point -> Blender, mirrored if asked."""
    x = -p[0] if MIRROR else p[0]
    return V((x, -p[2], p[1]))


core.reset()
track = toy_ron.load(os.path.join(ROOT, 'assets', 'data', 'tracks', f'{TRACK}.ron'))
catalog = {}
for f in sorted(os.listdir(os.path.join(ROOT, 'assets', 'data', 'props'))):
    if f.endswith('.ron'):
        catalog.update(toy_ron.load(os.path.join(ROOT, 'assets', 'data', 'props', f))['props'])

M = {}
for name, col, rough in (('ribbon', ORANGE, 0.25), ('lip', '#F58A33', 0.25),
                         ('wall', '#F5A142', 0.3), ('mat', TEAL, 0.95), ('room', WALL, 0.85),
                         ('collider', '#9A9A9A', 0.6), ('cube', '#5FE3FF', 0.2),
                         ('battery', '#FFC21A', 0.25)):
    M[name] = core.material(name, col, 0.0, rough)
M['wood'] = wood('wood', BEECH, 0.4, seed=7)
SURF = {'Carpet': 'mat', 'Wood': 'wood'}


def active(d):
    return d in ('Both', 'Rev' if REV else 'Fwd')


# --- ribbon and barriers -------------------------------------------------------------------
pts = supports.resolve(track)
n = len(pts)
walls, wl, wr = [], False, False
for p in track['points']:
    wl, wr = p.get('wall_left', wl), p.get('wall_right', wr)
    walls.append((wl, wr))
gaps = [(g['from'], g['to']) for g in track.get('gaps', []) if active(g.get('dir', 'Both'))]
at, gap, dense = supports.centreline(pts, gaps)
T, L, W, C = 0.25, 0.25, 0.35, 0.7  # thickness, lip height, lip width, chamfer
rib, wal = core.new_bm(), core.new_bm()


def frame_at(i, u):
    p, fwd, hw, bank = at(i, u)
    right, up = supports.frame(fwd, bank)
    return V(p), V(right), V(up), hw


def sweep(bm, i, profile, mats):
    """profile: [(k, x, y)] lateral k*hw + x, height y (CCW); one ring per ~0.75 u."""
    steps = max(4, int(math.ceil(supports.dist(pts[i][0], pts[(i + 1) % n][0]) / 0.75)))
    rings = []
    for s in range(steps + 1):
        p, r, up, hw = frame_at(i, s / steps)
        rings.append([bm.verts.new(B(p + r * (k * hw + x) + up * y)) for k, x, y in profile])
    m = len(profile)
    for a, b in zip(rings, rings[1:]):
        for j in range(m):
            f = bm.faces.new((a[j], a[(j + 1) % m], b[(j + 1) % m], b[j]))
            f.material_index = core.mi(mats[j])
    return rings


PROFILE = [(-1, -W - C, -T), (1, W + C, -T), (1, W, L), (1, 0, L), (1, 0, 0), (-1, 0, 0),
           (-1, 0, L), (-1, -W, L)]
PMATS = ['ribbon', 'ribbon', 'lip', 'lip', 'ribbon', 'lip', 'lip', 'ribbon']
for i in range(n):
    if gap[i]:
        continue
    sweep(rib, i, PROFILE, PMATS)
    hgt = track.get('wall_height', 1.2)
    if walls[i][0]:
        sweep(wal, i, [(-1, -W - 0.6, -T), (-1, -W, -T), (-1, -W, hgt), (-1, -W - 0.6, hgt)],
              ['wall'] * 4)
    if walls[i][1]:
        sweep(wal, i, [(1, W, -T), (1, W + 0.6, -T), (1, W + 0.6, hgt), (1, W, hgt)],
              ['wall'] * 4)
for bm in (rib, wal):
    geo.bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
core.obj_from_bm('ribbon', rib, weighted=False, angle=40)
core.obj_from_bm('walls', wal, weighted=False, angle=40)

# --- ground, room ------------------------------------------------------------------------------
for g in track.get('ground', []):
    if not active(g.get('dir', 'Both')):
        continue
    bm = core.new_bm()
    vs = [bm.verts.new(B((x, g['height'], z))) for x, z in g['points']]
    f = bm.faces.new(vs)
    f.material_index = core.mi(SURF.get(g['surface'], 'mat'))
    geo.bmesh.ops.triangulate(bm, faces=[f])
    geo.bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    for fc in bm.faces:
        if fc.normal.z < 0:
            fc.normal_flip()
    core.obj_from_bm('ground', box_uv(bm, 0.25), weighted=False)
b = track.get('bounds')
if b:
    (x0, z0), (x1, z1) = b['min'], b['max']
    bm = core.new_bm()
    H = 18.0  # ceiling height of the room (4 m)
    for (ax, az), (bx, bz) in (((x0, z0), (x1, z0)), ((x1, z0), (x1, z1)), ((x1, z1), (x0, z1)),
                               ((x0, z1), (x0, z0))):
        a, c = B((ax, 0, az)), B((bx, 0, bz))
        mid = (a + c) / 2
        d = c - a
        geo.box(bm, V((mid.x, mid.y, H / 2)), (abs(d.x) + 1, abs(d.y) + 1, H), core.mi('room'))
    core.obj_from_bm('room', bm, weighted=False)

# --- item rows, battery --------------------------------------------------------------------
cubes = core.new_bm()
for row in track.get('item_rows', []):
    if not active(row.get('dir', 'Both')):
        continue
    q = row['pos']
    best = min(((supports.dist(d[0], q), d) for d in dense), key=lambda e: e[0])[1]
    i = best[3]
    us = [k / 40 for k in range(41)]
    u = min(us, key=lambda uu: supports.dist(at(i, uu)[0], q))
    p, r, up, hw = frame_at(i, u)
    cnt = row.get('count', 5)
    for k in range(cnt):
        lat = -(hw - 1.5) + 2 * (hw - 1.5) * k / (cnt - 1)
        c = B(p + r * lat + up * 0.8)
        geo.box(cubes, c, (0.7, 0.7, 0.7), core.mi('cube'))
if cubes.faces:
    core.obj_from_bm('cubes', cubes, weighted=False)
gb = track.get('golden_battery')
if gb and not REV:
    bm = core.new_bm()
    c = B(gb['pos'])
    geo.cylinder(bm, c - V((0, 0, 0.5)), c + V((0, 0, 0.5)), 0.32, core.mi('battery'), n=16)
    core.obj_from_bm('battery', bm, weighted=False)

# --- props ---------------------------------------------------------------------------------
templates = {}


def template(kind):
    if kind in templates:
        return templates[kind]
    d = catalog[kind]
    path = os.path.join(ROOT, 'assets', d['model'])
    if os.path.exists(path):
        before = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=path)
        objs = [o for o in bpy.data.objects if o not in before]
    else:
        bm = core.new_bm()
        for c in d['colliders']:
            off = c.get('offset', [0, 0, 0])
            if c['_name'] in ('Box', 'Wedge'):
                h = c['half']
                geo.box(bm, V((off[0], -off[2], off[1])), (2 * h[0], 2 * h[2], 2 * h[1]),
                        core.mi('collider'))
            elif c['_name'] == 'Cylinder':
                geo.cylinder(bm, V((off[0], -off[2], off[1] - c['half_height'])),
                             V((off[0], -off[2], off[1] + c['half_height'])), c['radius'],
                             core.mi('collider'), n=20)
            else:
                geo.sphere(bm, V((off[0], -off[2], off[1])), c['radius'], core.mi('collider'))
        objs = [core.obj_from_bm(kind, bm, weighted=False)]
    for o in objs:
        o.hide_render = True
    templates[kind] = objs
    return objs


for pl in track.get('props', []):
    if not active(pl.get('dir', 'Both')):
        continue
    yaw = math.radians(pl.get('yaw', 0.0))
    if MIRROR:
        yaw = -yaw
    root = bpy.data.objects.new(pl['kind'], None)
    bpy.context.scene.collection.objects.link(root)
    for o in template(pl['kind']):
        if o.type != 'MESH':
            continue
        c = o.copy()
        c.hide_render = False
        bpy.context.scene.collection.objects.link(c)
        c.parent = root
    root.location = B(pl['pos'])
    root.rotation_euler = (0, 0, yaw)

decor_path = os.path.join(ROOT, 'assets', 'data', 'decor', f'{TRACK}.ron')
if os.path.exists(decor_path):  # physics-free dressing drawn by rc_render
    for d in toy_ron.load(decor_path)['decor']:
        if not active(d.get('dir', 'Both')):
            continue
        key = 'decor:' + d['model']
        if key not in templates:
            before = set(bpy.data.objects)
            bpy.ops.import_scene.gltf(filepath=os.path.join(ROOT, 'assets', d['model']))
            templates[key] = [o for o in bpy.data.objects if o not in before]
            for o in templates[key]:
                o.hide_render = True
        root = bpy.data.objects.new('decor', None)
        bpy.context.scene.collection.objects.link(root)
        for o in templates[key]:
            if o.type == 'MESH':
                c = o.copy()
                c.hide_render = False
                bpy.context.scene.collection.objects.link(c)
                c.parent = root
        yaw = math.radians(d.get('yaw', 0.0))
        root.location = B(d['pos'])
        root.rotation_euler = (0, 0, -yaw if MIRROR else yaw)
        root.scale = (d.get('scale', 1.0),) * 3

# --- cameras -------------------------------------------------------------------------------
propkit.stage(floor='#8A6A48')
scene = bpy.context.scene
res = [int(v) for v in opt('--res', '1280x720').split('x')]
scene.render.resolution_x, scene.render.resolution_y = res
scene.eevee.taa_render_samples = int(opt('--samples', '24'))
os.makedirs(OUT, exist_ok=True)
stem = TRACK + ('_rev' if REV else '') + ('_mirror' if MIRROR else '')


def shoot(name, loc, target, lens=None, fov=None, ortho=None):
    cam = render._camera(V(loc), V(target), lens, fov)
    cam.data.clip_end = 3000
    cam.data.clip_start = 0.05
    if ortho:
        cam.data.type = 'ORTHO'
        cam.data.ortho_scale = ortho
    p = os.path.join(OUT, f'{stem}_{name}.png')
    scene.render.filepath = p
    bpy.ops.render.render(write_still=True)
    bpy.data.objects.remove(cam)
    print(f'[diorama] {p}')


xs = [B(d[0]).x for d in dense]
ys = [B(d[0]).y for d in dense]
cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
span = max(max(xs) - min(xs), max(ys) - min(ys)) + 30
if 'top' in VIEWS:  # north up; the ortho scale spans the wider image axis
    w, h = max(xs) - min(xs) + 30, max(ys) - min(ys) + 30
    shoot('top', (cx, cy - 0.01, 400), (cx, cy, 0), ortho=max(w, h * res[0] / res[1]))
if 'overview' in VIEWS:
    shoot('overview_se', (cx + span * 0.55, cy - span * 0.75, span * 0.5), (cx, cy, 0), lens=30)
    shoot('overview_nw', (cx - span * 0.55, cy + span * 0.75, span * 0.5), (cx, cy, 0), lens=30)


def chase(s):
    """Camera 4.2 u behind / 1.7 u above the centreline at arc length s (travel direction)."""
    total = dense[-1][2]
    s %= total
    k = min(range(len(dense)), key=lambda j: abs(dense[j][2] - s))
    j = min(range(len(dense)), key=lambda j: abs(dense[j][2] - ((s + 6) % total)))
    here, ahead = V(dense[k][0]), V(dense[j][0])
    fwd = (ahead - here).normalized()
    if REV:
        j = min(range(len(dense)), key=lambda j: abs(dense[j][2] - ((s - 6) % total)))
        ahead = V(dense[j][0])
        fwd = (ahead - here).normalized()
    eye = here - fwd * 4.2 + V((0, 1.7, 0))
    look = here + fwd * 6.0 + V((0, 0.6, 0))
    return B(eye), B(look)


if 'chase' in VIEWS:
    total = dense[-1][2]
    ss = [float(v) for v in opt('--chase', ','.join(str(int(total * k / 12)) for k in range(12)))
          .split(',')]
    for s in ss:
        eye, look = chase(s)
        shoot(f'chase_{int(s):03d}', eye, look, fov=70)
for spec in [a for a in argv if ':' in a and ',' in a]:
    e, t = spec.split(':')
    shoot('cam_' + e.replace(',', '_'), B([float(v) for v in e.split(',')]),
          B([float(v) for v in t.split(',')]), fov=70)
