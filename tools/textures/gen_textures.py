#!/usr/bin/env python3
"""Procedural tileable PBR texture sets for rc_render's MaterialLibrary.

    python3 tools/textures/gen_textures.py                 # every set → assets/textures/<set>/
    python3 tools/textures/gen_textures.py --only tile,grass --preview /tmp/tex

Each set writes `albedo.ktx2` (RGBA8 sRGB), `normal.ktx2` (RG8, OpenGL convention) and
`rough.ktx2` (RG8: R = roughness, G = 255 metallic pass-through), all with full mip chains and
Zstandard supercompression. The world size one texture covers (`tile_u`) must match
`TEXTURE_SETS` in crates/rc_render/src/materials/mod.rs. Art references: docs/design/art-bible.md
§1.1 (PBR ranges) and §3 (environment palettes). Deterministic: re-running reproduces the files.
"""

import argparse
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from texlib import (  # noqa: E402
    VK_R8G8_UNORM, VK_R8G8B8A8_SRGB, albedo_levels, blur, fractal, lerp, mix_colors,
    normal_rough_levels, normals_from_height, normalize, rgb, rng, smoothstep, worley, write_ktx2,
)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "textures"


def coords(n):
    """Pixel centre coordinates in 0..1 (x right, y down)."""
    ys, xs = np.mgrid[0:n, 0:n].astype(np.float64)
    return (xs + 0.5) / n, (ys + 0.5) / n


# ---------------------------------------------------------------------------------------------
# Ground sets (hero: 1024²)


def asphalt(n=1024, tile=3.0):
    tone = fractal(n, 11, beta=2.2, lo=2, hi=40)
    grain = fractal(n, 12, beta=0.6, lo=60)
    binder = lerp(rgb((48, 49, 52)), rgb((62, 63, 66)), 0.0)
    albedo = mix_colors(binder, binder * 1.18, tone)
    height = grain * 0.0006
    rough = 0.9 + 0.05 * grain
    for cells, seed, cover, size in [(55, 13, 0.4, 0.40), (110, 14, 0.55, 0.36), (220, 15, 0.7, 0.33)]:
        f1, f2, cid = worley(n, cells, seed, jitter=0.9)
        stone = smoothstep(size, size - 0.12, f1) * (cid < cover)
        pick = rng(seed).random(cells * cells)[(cid * cells * cells).astype(int) % (cells * cells)]
        shade = 0.2 + 0.2 * pick + 0.18 * (pick > 0.93)
        stone_rgb = np.stack([shade * 0.98, shade, shade * 1.03], -1)
        albedo = albedo * (1 - stone[..., None]) + stone_rgb * stone[..., None]
        dome = np.clip(1.0 - (f1 / size) ** 2, 0, 1) * (cid < cover)
        height = np.maximum(height, dome * 0.006 * (110 / cells))
        rough = np.where(stone > 0.5, 0.62 + 0.15 * grain, rough)
    # Hairline cracks.
    f1, f2, _ = worley(n, 4, 16, jitter=1.0)
    crack = smoothstep(0.02, 0.0, f2 - f1) * smoothstep(0.62, 0.8, fractal(n, 17, beta=2, lo=1, hi=12))
    albedo *= (1 - 0.3 * crack)[..., None]
    height -= crack * 0.004
    rough = np.maximum(rough, crack)
    return albedo, height, rough, tile


def concrete(n=1024, tile=4.0):
    mottling = fractal(n, 21, beta=2.3, lo=1, hi=30)
    stains = smoothstep(0.55, 0.95, fractal(n, 22, beta=2.4, lo=1, hi=16))
    sand = fractal(n, 23, beta=0.4, lo=80)
    base = rgb((143, 140, 134))
    albedo = mix_colors(base * 0.9, base * 1.07, mottling)
    albedo *= (1.0 - 0.07 * stains)[..., None]
    albedo *= (0.96 + 0.08 * sand)[..., None]
    f1, _, cid = worley(n, 140, 24, jitter=1.0)
    pores = smoothstep(0.12, 0.05, f1) * (cid < 0.18)
    albedo *= (1 - 0.35 * pores)[..., None]
    height = sand * 0.0004 + mottling * 0.0015 - pores * 0.003
    f1, f2, _ = worley(n, 5, 25)
    crack = smoothstep(0.015, 0.0, f2 - f1) * smoothstep(0.7, 0.85, fractal(n, 26, beta=2, lo=1, hi=10))
    albedo *= (1 - 0.25 * crack)[..., None]
    height -= crack * 0.003
    rough = 0.88 + 0.06 * sand - 0.08 * stains + 0.1 * crack
    return albedo, height, rough, tile


def grass(n=1024, tile=3.0):
    r = rng(31)
    soil = mix_colors(rgb((52, 40, 26)), rgb((70, 56, 34)), fractal(n, 32, beta=2, lo=4))
    albedo = soil.copy()
    height = np.zeros((n, n))
    patches = fractal(n, 33, beta=2.4, lo=1, hi=16)
    dry = fractal(n, 34, beta=2.2, lo=2, hi=24)
    # Blades: short strokes with random lean, painted far → near (taller blades on top).
    count = 30000
    px = r.random(count) * n
    py = r.random(count) * n
    angle = r.random(count) * np.pi * 2
    length = (0.12 + 0.2 * r.random(count)) / tile * n
    tallness = r.random(count)
    order = np.argsort(tallness)
    hue = r.random(count)
    light = r.random(count)
    green_dark = rgb((38, 72, 24))
    green_light = rgb((112, 158, 54))
    yellow = rgb((150, 152, 70))
    steps = 16
    for i in order:
        t = np.linspace(0.0, 1.0, steps)
        xs = (px[i] + np.cos(angle[i]) * length[i] * t).astype(int) % n
        ys = (py[i] + np.sin(angle[i]) * length[i] * t).astype(int) % n
        base_mix = 0.25 + 0.75 * light[i] * (0.6 + 0.4 * patches[ys[0], xs[0]])
        c = lerp(green_dark, green_light, base_mix)
        c = lerp(c, yellow, 0.55 * hue[i] * dry[ys[0], xs[0]])
        shade = (0.4 + 0.6 * t)[:, None]  # tips catch more light
        albedo[ys, xs] = c[None, :] * shade
        height[ys, xs] = (0.01 + 0.02 * tallness[i]) * (0.4 + 0.6 * t)
        # One-pixel width doubling for thicker blades.
        albedo[ys, (xs + 1) % n] = c[None, :] * shade * 0.9
        height[ys, (xs + 1) % n] = height[ys, xs] * 0.9
    albedo = blur(albedo, 0.6)
    height = blur(height, 0.7)
    rough = 0.82 + 0.12 * (1 - normalize(height))
    return albedo, height, rough, tile


def dirt(n=1024, tile=3.0):
    tone = fractal(n, 41, beta=2.2, lo=1, hi=24)
    clumps = fractal(n, 42, beta=1.6, lo=6, hi=120)
    grains = fractal(n, 43, beta=0.3, lo=120)
    base = rgb((107, 76, 48))
    albedo = mix_colors(base * 0.82, base * 1.12, tone)
    albedo *= (0.9 + 0.2 * clumps)[..., None]
    albedo *= (0.94 + 0.12 * grains)[..., None]
    height = clumps * 0.012 + grains * 0.001
    rough = 0.92 + 0.06 * grains
    for cells, seed, cover in [(26, 44, 0.25), (60, 45, 0.3)]:
        f1, _, cid = worley(n, cells, seed, jitter=0.9)
        size = 0.3 if cells == 26 else 0.25
        pebble = smoothstep(size, size - 0.08, f1) * (cid < cover)
        tint = 0.6 + 0.4 * cid
        pebble_rgb = np.stack([0.42 * tint, 0.36 * tint, 0.29 * tint], -1)
        albedo = albedo * (1 - pebble[..., None]) + pebble_rgb * pebble[..., None]
        height = np.maximum(height, np.clip(1 - (f1 / size) ** 2, 0, 1) * (cid < cover) * 0.02 * (26 / cells))
        rough = np.where(pebble > 0.5, 0.7, rough)
    return albedo, height, rough, tile


def carpet(n=1024, tile=2.0):
    f1, _, cid = worley(n, 170, 51, jitter=0.85)
    tuft = np.clip(1.0 - (f1 / 0.62) ** 2, 0, 1)
    fibres = fractal(n, 52, beta=0.2, lo=200)
    wear = fractal(n, 53, beta=2.2, lo=1, hi=10)
    base = rgb((206, 201, 196))
    shade = (0.82 + 0.18 * tuft) * (0.94 + 0.12 * cid) * (0.93 + 0.1 * fibres) * (0.96 + 0.06 * wear)
    albedo = base[None, None, :] * shade[..., None]
    height = tuft * 0.008 + fibres * 0.0015
    rough = 0.96 + 0.04 * fibres
    return albedo, height, rough, tile


def wood_planks(n=1024, tile=4.0, rows=6):
    r = rng(61)
    x, y = coords(n)
    grain = fractal(n, 62, beta=1.4, lo=20, sx=0.012, sy=1.0)
    warp = fractal(n, 63, beta=2.6, lo=1, hi=4, sx=0.25, sy=1.0)
    fine = fractal(n, 64, beta=0.8, lo=60, sx=0.01, sy=1.0)
    blotch = fractal(n, 65, beta=2.4, lo=2, hi=24, sx=0.2, sy=1.0)
    base = rgb((186, 136, 92))
    albedo = np.zeros((n, n, 3))
    height = np.zeros((n, n))
    rough = np.zeros((n, n))
    row_h = n // rows
    gap = max(2, n // 400)
    for row in range(rows):
        y0 = row * row_h
        k = int(r.integers(2, 4))
        lengths = 1.0 + r.dirichlet(np.ones(k)) * (tile - k * 1.0)
        start = r.random() * tile
        cuts = (((start + np.concatenate([[0.0], np.cumsum(lengths)[:-1]])) % tile) / tile * n).astype(int)
        cuts = np.sort(cuts)
        for k in range(len(cuts)):
            a, b = cuts[k], cuts[(k + 1) % len(cuts)]
            span = (np.arange(a, b if b > a else b + n) % n)
            shift = (r.integers(0, n), r.integers(0, n))
            g = np.roll(np.roll(grain, shift[0], 0), shift[1], 1)
            w = np.roll(np.roll(warp, shift[0], 0), shift[1], 1)
            f = np.roll(np.roll(fine, shift[0], 0), shift[1], 1)
            bl = np.roll(np.roll(blotch, shift[0], 0), shift[1], 1)
            rings = 0.5 + 0.5 * np.sin((y * rows * 12.0 + w * 1.6 + r.random()) * np.pi * 2)
            figure = np.clip(0.45 * g + 0.3 * rings ** 8 + 0.15 * f + 0.1 * bl, 0, 1)
            tint = 0.82 + 0.3 * r.random()
            hue = np.array([1.0, 0.97 + 0.05 * r.random(), 0.92 + 0.1 * r.random()])
            plank = base[None, None, :] * hue * tint * (0.78 + 0.34 * figure)[..., None]
            ys = slice(y0, y0 + row_h)
            albedo[ys, span] = plank[ys, span]
            height[ys, span] = (r.random() - 0.5) * 0.0016 + figure[ys, span] * 0.0006
            rough[ys, span] = 0.34 + 0.12 * figure[ys, span]
            # Joint at the plank's start.
            albedo[ys, a % n] *= 0.35
            height[ys, a % n] -= 0.006
            rough[ys, a % n] = 0.85
        # Gap above each row, with a small chamfer.
        for i in range(gap + 2):
            weight = 0.35 if i < gap else 0.75
            albedo[(y0 + i) % n] *= weight
            height[(y0 + i) % n] -= 0.006 if i < gap else 0.002
            rough[(y0 + i) % n] = 0.85 if i < gap else rough[(y0 + i) % n]
    return albedo, height, rough, tile


def tile(n=1024, tile_u=2.667, per_side=2):
    x, y = coords(n)
    cell = n / per_side
    grout_w = 0.035 / tile_u * n
    lx = (x * n) % cell
    ly = (y * n) % cell
    edge = np.minimum(np.minimum(lx, cell - lx), np.minimum(ly, cell - ly))
    face = smoothstep(grout_w * 0.5, grout_w * 0.5 + 6.0, edge)
    ix = (x * per_side).astype(int)
    iy = (y * per_side).astype(int)
    r = rng(71)
    tint = r.random((per_side, per_side))[iy, ix]
    glaze = fractal(n, 72, beta=2.6, lo=1, hi=10)
    sand = fractal(n, 73, beta=0.3, lo=120)
    tile_rgb = rgb((233, 227, 213)) * 1.0
    grout_rgb = rgb((150, 146, 137))
    surface = tile_rgb[None, None, :] * (0.97 + 0.04 * tint + 0.025 * glaze)[..., None]
    grout = grout_rgb[None, None, :] * (0.9 + 0.2 * sand)[..., None]
    albedo = grout * (1 - face[..., None]) + surface * face[..., None]
    wave = fractal(n, 74, beta=2.8, lo=2, hi=12)
    height = face * (0.008 + wave * 0.0015) + glaze * 0.0003 + (1 - face) * sand * 0.001
    rough = lerp(0.85 + 0.1 * sand, 0.1 + 0.05 * glaze, face)
    return albedo, height, rough, tile_u


def marble(n=1024, tile_u=4.0, per_side=2):
    x, y = coords(n)
    # Perlin's marble: a directional sine bent by turbulence (sum of |noise| octaves).
    turb = np.zeros((n, n))
    for octave in range(3):
        band = fractal(n, 81 + octave, beta=2.0, lo=2 ** octave, hi=2 ** (octave + 2))
        turb += np.abs(band * 2.0 - 1.0) / 2 ** octave
    v = np.abs(np.sin(((x + y) * 1.0 + turb * 0.9) * np.pi * 2))
    strength = smoothstep(0.25, 0.75, fractal(n, 88, beta=2.4, lo=1, hi=8))
    veins = smoothstep(0.22, 0.0, v) ** 2 * (0.3 + 0.7 * strength)
    warp = fractal(n, 87, beta=2.6, lo=1, hi=8)
    v2 = np.abs(np.sin(((x - y * 0.4) * 2.0 + turb * 2.4) * np.pi * 2))
    veins2 = smoothstep(0.05, 0.0, v2) * smoothstep(0.45, 0.75, warp)
    clouds = fractal(n, 83, beta=2.4, lo=1, hi=20)
    base = rgb((234, 232, 228))
    albedo = base[None, None, :] * (0.93 + 0.07 * clouds)[..., None]
    albedo = albedo * (1 - 0.4 * veins[..., None]) + rgb((128, 126, 126))[None, None, :] * 0.4 * veins[..., None]
    albedo *= (1 - 0.18 * veins2)[..., None]
    cell = n / per_side
    seam = np.minimum((x * n) % cell, (y * n) % cell) < 1.5
    albedo[seam] *= 0.55
    height = np.where(seam, -0.003, 0.0)
    rough = 0.09 + 0.05 * clouds + 0.04 * veins + seam * 0.5
    return albedo, height, rough, tile_u


def vinyl(n=1024, tile_u=2.667, per_side=2):
    x, y = coords(n)
    ix = (x * per_side).astype(int)
    iy = (y * per_side).astype(int)
    checker = (ix + iy) % 2
    base = rgb((217, 214, 204))
    albedo = np.broadcast_to(base, (n, n, 3)).copy()
    for cells, seed, cover, colour, aspect in [
        (90, 91, 0.35, (160, 158, 152), 2.6),
        (140, 92, 0.3, (243, 241, 236), 2.2),
        (60, 93, 0.25, (196, 186, 166), 3.0),
    ]:
        for flip in (0, 1):
            f1, _, cid = worley(n, cells, seed + flip * 7, jitter=1.0, aspect=aspect)
            chip = smoothstep(0.28, 0.2, f1) * (cid < cover)
            if flip:
                chip = chip.T
            chip *= (checker == flip)
            albedo = albedo * (1 - chip[..., None]) + rgb(colour)[None, None, :] * chip[..., None]
    tone = rng(94).random((per_side, per_side))[iy, ix]
    albedo *= (0.975 + 0.035 * tone)[..., None]
    cell = n / per_side
    seam = np.minimum((x * n) % cell, (y * n) % cell) < 1.2
    albedo[seam] *= 0.78
    scuffs = smoothstep(0.6, 0.85, fractal(n, 95, beta=2.2, lo=2, hi=40, sx=0.3, sy=1.0))
    height = np.where(seam, -0.0015, 0.0) + fractal(n, 96, beta=1.0, lo=60) * 0.0002
    rough = 0.16 + 0.22 * scuffs + seam * 0.4
    return albedo, height, rough, tile_u


def stone(n=1024, tile_u=4.0, per_side=2):
    x, y = coords(n)
    cell = n / per_side
    joint_w = 0.06 / tile_u * n
    lx = (x * n) % cell
    ly = (y * n) % cell
    chip = fractal(n, 101, beta=1.8, lo=8, hi=200) * 6.0
    edge = np.minimum(np.minimum(lx, cell - lx), np.minimum(ly, cell - ly)) - chip
    slab = smoothstep(joint_w * 0.5, joint_w * 0.5 + 4.0, edge)
    ix = (x * per_side).astype(int)
    iy = (y * per_side).astype(int)
    r = rng(102)
    shade_v = r.random((per_side, per_side))[iy, ix]
    warm = r.random((per_side, per_side))[iy, ix]
    tint = np.stack([0.5 + 0.5 * shade_v + 0.06 * warm, 0.5 + 0.5 * shade_v, 0.5 + 0.5 * shade_v - 0.06 * warm], -1)
    tilt = (r.random((per_side, per_side, 2)) - 0.5)[iy, ix]
    grain = fractal(n, 103, beta=0.6, lo=60)
    tone = fractal(n, 104, beta=2.2, lo=2, hi=30)
    base = rgb((187, 178, 164))
    face = base[None, None, :] * (0.86 + 0.16 * tint) * (0.9 + 0.1 * tone)[..., None] * (0.93 + 0.1 * grain)[..., None]
    f1, _, cid = worley(n, 160, 105)
    pits = smoothstep(0.15, 0.06, f1) * (cid < 0.15)
    face *= (1 - 0.3 * pits)[..., None]
    joint = mix_colors(rgb((96, 88, 74)), rgb((130, 120, 102)), grain)
    albedo = joint * (1 - slab[..., None]) + face * slab[..., None]
    height = slab * (0.015 + (tilt[..., 0] * (lx / cell) + tilt[..., 1] * (ly / cell)) * 0.004) + grain * 0.001 - pits * 0.002
    rough = lerp(0.95, 0.82 + 0.08 * grain, slab)
    return albedo, height, rough, tile_u


def toy_track(n=1024, tile_u=2.0):
    x, y = coords(n)
    grooves = 0.5 + 0.5 * np.cos(x * np.pi * 2 * 20)
    # Moulded stipple (injection-mould texture) and faint flow lines along the strip.
    stipple = fractal(n, 111, beta=0.9, lo=60)
    flow = fractal(n, 112, beta=2.4, lo=1, hi=10, sx=1.0, sy=0.25)
    mottle = fractal(n, 113, beta=2.2, lo=2, hi=24)
    base = rgb((238, 238, 236))
    albedo = base[None, None, :] * (0.94 + 0.05 * mottle + 0.03 * flow + 0.012 * grooves)[..., None]
    height = grooves ** 4 * 0.0012 + stipple * 0.0007 + flow * 0.0004
    rough = 0.22 + 0.08 * stipple + 0.04 * grooves + 0.04 * mottle
    return albedo, height, rough, tile_u


# ---------------------------------------------------------------------------------------------
# Props / walls (512²)


def plywood(n=512, tile_u=4.0):
    x, y = coords(n)
    warp = fractal(n, 121, beta=2.8, lo=1, hi=4, sx=0.35, sy=1.0)
    rings = 0.5 + 0.5 * np.sin((y * 20.0 + warp * 0.7) * np.pi * 2)
    fibre = fractal(n, 122, beta=1.0, lo=30, sx=0.02, sy=1.0)
    base = rgb((207, 172, 122))
    figure = 0.35 * rings ** 6 + 0.65 * fibre
    albedo = base[None, None, :] * (0.9 + 0.12 * (1 - figure))[..., None]
    d = np.hypot((x - 0.27) * 0.6, y - 0.63)
    albedo *= (1 - 0.2 * smoothstep(0.03, 0.01, d))[..., None]
    height = fibre * 0.0008
    rough = 0.66 + 0.08 * fibre
    return albedo, height, rough, tile_u


def cardboard(n=512, tile_u=2.0):
    x, y = coords(n)
    flutes = 0.5 + 0.5 * np.cos(y * np.pi * 2 * 64)
    fibre = fractal(n, 131, beta=0.8, lo=40, sx=0.05, sy=1.0)
    mottle = fractal(n, 132, beta=2.2, lo=1, hi=16)
    base = rgb((181, 141, 96))
    albedo = base[None, None, :] * (0.88 + 0.12 * mottle + 0.05 * fibre + 0.025 * flutes)[..., None]
    height = flutes * 0.0012 + fibre * 0.0004
    rough = 0.9 + 0.05 * fibre
    return albedo, height, rough, tile_u


def rubber(n=512, tile_u=2.0):
    _, y = coords(n)
    ribs = np.clip(np.cos(y * np.pi * 2 * 25), 0, 1) ** 0.5
    texture = fractal(n, 141, beta=0.5, lo=60)
    dust = fractal(n, 142, beta=2.0, lo=2, hi=30)
    base = rgb((40, 40, 42))
    valley = (1 - ribs) * smoothstep(0.4, 0.8, dust)
    albedo = base[None, None, :] * (0.92 + 0.12 * texture)[..., None] + rgb((40, 38, 35))[None, None, :] * valley[..., None] * 0.6
    height = ribs * 0.01 + texture * 0.0004
    rough = 0.9 - 0.07 * ribs + 0.05 * valley
    return albedo, height, rough, tile_u


def metal(n=512, tile_u=2.0):
    streaks = fractal(n, 151, beta=1.0, lo=4, sx=0.02, sy=1.0)
    smudge = fractal(n, 152, beta=2.4, lo=1, hi=8)
    r = rng(153)
    scratches = np.zeros((n, n))
    for _ in range(90):
        y0 = r.random() * n
        x0 = r.random() * n
        length = r.random() * n * 0.4
        slope = (r.random() - 0.5) * 0.08
        t = np.arange(int(length))
        xs = (x0 + t).astype(int) % n
        ys = (y0 + t * slope).astype(int) % n
        scratches[ys, xs] = 1.0
    scratches = blur(scratches, 0.5)
    base = rgb((190, 192, 196))
    albedo = base[None, None, :] * (0.94 + 0.06 * streaks + 0.03 * smudge)[..., None]
    height = streaks * 0.0002 - scratches * 0.0004
    rough = 0.3 + 0.08 * streaks + 0.12 * smudge + 0.1 * scratches
    return albedo, height, rough, tile_u


def plaster(n=512, tile_u=4.0):
    peel = fractal(n, 161, beta=1.4, lo=12)
    tone = fractal(n, 162, beta=2.4, lo=1, hi=8)
    base = rgb((237, 234, 228))
    albedo = base[None, None, :] * (0.975 + 0.02 * tone + 0.01 * peel)[..., None]
    height = peel * 0.0012
    rough = 0.84 + 0.06 * peel
    return albedo, height, rough, tile_u


def ice(n=512, tile_u=4.0):
    f1, f2, _ = worley(n, 7, 171)
    cracks = smoothstep(0.025, 0.0, f2 - f1)
    f1b, f2b, _ = worley(n, 19, 172)
    fine = smoothstep(0.02, 0.0, f2b - f1b) * smoothstep(0.5, 0.75, fractal(n, 173, beta=2, lo=2, hi=20))
    frost = smoothstep(0.45, 0.8, fractal(n, 174, beta=2.2, lo=1, hi=24))
    f1c, _, cid = worley(n, 90, 175)
    bubbles = smoothstep(0.1, 0.03, f1c) * (cid < 0.2)
    base = rgb((214, 233, 243))
    albedo = base[None, None, :] * (0.95 + 0.05 * frost)[..., None]
    albedo = np.clip(albedo + (0.1 * cracks + 0.06 * fine + 0.08 * bubbles + 0.04 * frost)[..., None], 0, 1)
    height = -cracks * 0.003 - fine * 0.0015 + frost * 0.002
    rough = 0.06 + 0.4 * frost + 0.2 * cracks
    return albedo, height, rough, tile_u


SETS = {
    "asphalt": asphalt,
    "cardboard": cardboard,
    "carpet": carpet,
    "concrete": concrete,
    "dirt": dirt,
    "grass": grass,
    "ice": ice,
    "marble": marble,
    "metal": metal,
    "plaster": plaster,
    "plywood": plywood,
    "rubber": rubber,
    "stone": stone,
    "tile": tile,
    "toy_track": toy_track,
    "vinyl": vinyl,
    "wood_planks": wood_planks,
}


def build(name: str, preview: Path | None):
    albedo, height, rough, tile_u = SETS[name]()
    albedo = np.clip(albedo, 0.0, 1.0)
    rough = np.clip(rough, 0.03, 1.0)
    normals = normals_from_height(height, tile_u)
    out = OUT / name
    write_ktx2(out / "albedo.ktx2", albedo_levels(albedo), VK_R8G8B8A8_SRGB, srgb=True)
    normal_levels, rough_levels = normal_rough_levels(normals, rough)
    write_ktx2(out / "normal.ktx2", normal_levels, VK_R8G8_UNORM, srgb=False)
    write_ktx2(out / "rough.ktx2", rough_levels, VK_R8G8_UNORM, srgb=False)
    size = sum(f.stat().st_size for f in out.glob("*.ktx2"))
    print(f"{name:12s} {albedo.shape[0]}² tile {tile_u} u  {size / 1e6:.2f} MB")
    if preview is not None:
        from PIL import Image
        preview.mkdir(parents=True, exist_ok=True)
        n = albedo.shape[0]
        sheet = np.zeros((n, n * 3, 3))
        sheet[:, :n] = albedo
        sheet[:, n:2 * n] = normals * 0.5 + 0.5
        sheet[:, 2 * n:] = rough[..., None]
        Image.fromarray((sheet * 255).astype(np.uint8)).save(preview / f"{name}.png")
        tiled = np.tile(albedo, (2, 2, 1))
        Image.fromarray((tiled * 255).astype(np.uint8)).resize((n, n)).save(preview / f"{name}_tiled.png")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--only", help="comma-separated set names")
    parser.add_argument("--preview", type=Path, help="write PNG contact sheets here")
    args = parser.parse_args()
    names = args.only.split(",") if args.only else list(SETS)
    for name in names:
        if name not in SETS:
            sys.exit(f"unknown set {name}; known: {', '.join(SETS)}")
        build(name, args.preview)


if __name__ == "__main__":
    main()
