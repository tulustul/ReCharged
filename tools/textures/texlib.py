"""Helpers for the procedural PBR texture generator (gen_textures.py).

Everything here is tileable by construction (spectral noise, periodic Worley cells, wrapped
gradients) and deterministic (explicit seeds). Output: KTX2 files with a full mip chain and
Zstandard supercompression (Python 3.14 stdlib `compression.zstd`), which Bevy's KTX2 loader reads
with the workspace's `zstd_rust` feature.

Conventions
- Albedo: sRGB floats 0..1, shape (n, n, 3). Mips are averaged in linear light.
- Height: world units (u), shape (n, n). Normals derive from it with the real texel size, so bump
  depths in u give physically scaled slopes.
- Normal maps: tangent space, OpenGL / glTF convention (+X right, +Y toward the image top),
  stored as two channels (RG8, Bevy reconstructs Z).
- Roughness: perceptual roughness 0..1, stored in the R channel of an RG8 metallic-roughness map
  (Bevy reads RG maps as R = roughness, G = metallic); G = 255 so the material's `metallic`
  scalar passes through. Mips add normal variance (Toksvig) so bumpy glossy surfaces do not
  sparkle at a distance.
"""

import struct
from pathlib import Path

import numpy as np
from compression import zstd

# ---------------------------------------------------------------------------------------------
# Noise


def rng(seed: int) -> np.random.Generator:
    return np.random.default_rng(seed)


def normalize(a: np.ndarray) -> np.ndarray:
    a = a - a.min()
    return a / max(float(a.max()), 1e-9)


def _freqs(n: int):
    fy = np.fft.fftfreq(n)[:, None] * n
    fx = np.fft.rfftfreq(n)[None, :] * n
    return fx, fy


def fractal(n: int, seed: int, beta: float = 2.0, lo: float = 1.0, hi: float | None = None,
            sx: float = 1.0, sy: float = 1.0) -> np.ndarray:
    """Tileable fractal noise in 0..1: white noise shaped to power ~ f^-beta between `lo` and
    `hi` cycles per tile. `sx`/`sy` stretch frequencies (sx < sy → streaks along x)."""
    white = rng(seed).standard_normal((n, n))
    fx, fy = _freqs(n)
    f = np.sqrt((fx * sx) ** 2 + (fy * sy) ** 2)
    f[0, 0] = 1.0
    amp = f ** (-beta / 2.0)
    amp[f < lo] = 0.0
    if hi is not None:
        amp *= np.exp(-((f / hi) ** 4))
    amp[0, 0] = 0.0
    return normalize(np.fft.irfft2(np.fft.rfft2(white) * amp, s=(n, n)))


def worley(n: int, cells: int, seed: int, jitter: float = 1.0, aspect: float = 1.0):
    """Tileable cellular noise with `cells` cells per side: (F1, F2) distances in cell units and
    a random value 0..1 per cell. `aspect` > 1 squashes cells vertically."""
    r = rng(seed)
    pts = (r.random((cells, cells, 2)) - 0.5) * jitter + 0.5
    ids = r.random((cells, cells))
    ys, xs = np.mgrid[0:n, 0:n].astype(np.float64) / n * cells
    cx = np.floor(xs).astype(int)
    cy = np.floor(ys).astype(int)
    f1 = np.full((n, n), 1e9)
    f2 = np.full((n, n), 1e9)
    idv = np.zeros((n, n))
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            gx = (cx + dx) % cells
            gy = (cy + dy) % cells
            px = cx + dx + pts[gy, gx, 0]
            py = cy + dy + pts[gy, gx, 1]
            d = np.hypot(xs - px, (ys - py) * aspect)
            closer = d < f1
            f2 = np.where(closer, f1, np.minimum(f2, d))
            idv = np.where(closer, ids[gy, gx], idv)
            f1 = np.where(closer, d, f1)
    return f1, f2, idv


def smoothstep(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def blur(a: np.ndarray, radius_px: float) -> np.ndarray:
    """Tileable Gaussian blur (FFT)."""
    n = a.shape[0]
    fx, fy = _freqs(n)
    sigma_f = n / (2.0 * np.pi * max(radius_px, 1e-3))
    g = np.exp(-(fx ** 2 + fy ** 2) / (2.0 * sigma_f ** 2))
    if a.ndim == 2:
        return np.fft.irfft2(np.fft.rfft2(a) * g, s=a.shape)
    return np.stack([blur(a[..., c], radius_px) for c in range(a.shape[2])], -1)


def lerp(a, b, t):
    return a + (b - a) * t


def rgb(hex_or_tuple) -> np.ndarray:
    """sRGB 0..1 colour from '#rrggbb' or 0..255 tuple."""
    if isinstance(hex_or_tuple, str):
        h = hex_or_tuple.lstrip('#')
        return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], dtype=np.float64) / 255.0
    return np.array(hex_or_tuple, dtype=np.float64) / 255.0


def mix_colors(c0, c1, t):
    """Per-pixel blend of two colours by t (n, n) → (n, n, 3)."""
    return c0[None, None, :] + (c1 - c0)[None, None, :] * t[..., None]


# ---------------------------------------------------------------------------------------------
# Colour space


def srgb_to_linear(c):
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c):
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1.0 / 2.4) - 0.055)


# ---------------------------------------------------------------------------------------------
# Normals & mips


def normals_from_height(h: np.ndarray, tile_u: float) -> np.ndarray:
    """Unit tangent-space normals (n, n, 3) of a height map in u covering `tile_u` u."""
    n = h.shape[0]
    texel = tile_u / n
    dx = (np.roll(h, -1, 1) - np.roll(h, 1, 1)) / (2.0 * texel)
    dy = (np.roll(h, -1, 0) - np.roll(h, 1, 0)) / (2.0 * texel)  # +row = down
    nx, ny, nz = -dx, dy, np.ones_like(h)
    length = np.sqrt(nx * nx + ny * ny + nz * nz)
    return np.stack([nx / length, ny / length, nz / length], -1)


def box(a: np.ndarray) -> np.ndarray:
    return 0.25 * (a[0::2, 0::2] + a[1::2, 0::2] + a[0::2, 1::2] + a[1::2, 1::2])


def mip_chain(level0: np.ndarray, reduce) -> list:
    levels = [level0]
    while levels[-1].shape[0] > 1:
        levels.append(reduce(levels[-1]))
    return levels


def albedo_levels(srgb: np.ndarray) -> list:
    lin = mip_chain(srgb_to_linear(srgb), box)
    out = []
    for level in lin:
        s = linear_to_srgb(level)
        a = np.ones(s.shape[:2] + (1,))
        out.append(to_u8(np.concatenate([s, a], -1)))
    return out


def normal_rough_levels(normals: np.ndarray, rough: np.ndarray):
    """Normal (RG8) and roughness (RG8, G = 255) mip chains with Toksvig roughness widening."""
    n_levels = mip_chain(normals, box)
    r_levels = mip_chain(rough, box)
    normal_out, rough_out = [], []
    for nl, rl in zip(n_levels, r_levels):
        length = np.clip(np.linalg.norm(nl, axis=-1), 1e-4, 1.0)
        unit = nl / length[..., None]
        variance = (1.0 - length) / length
        r = np.clip(np.sqrt(rl * rl + variance * 1.5), 0.0, 1.0)
        normal_out.append(to_u8(unit[..., :2] * 0.5 + 0.5))
        rough_out.append(to_u8(np.stack([r, np.ones_like(r)], -1)))
    return normal_out, rough_out


def to_u8(a: np.ndarray) -> np.ndarray:
    return np.clip(np.round(a * 255.0), 0, 255).astype(np.uint8)


# ---------------------------------------------------------------------------------------------
# KTX2


KTX2_ID = bytes([0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A])
VK_R8G8_UNORM = 16
VK_R8G8B8A8_UNORM = 37
VK_R8G8B8A8_SRGB = 43
ZSTD = 2


def _dfd(channels: int, srgb: bool) -> bytes:
    """Basic data format descriptor of an 8-bit-per-channel RGBA / RG format."""
    ids = [0, 1, 2, 15][:channels]
    samples = b''
    for i, cid in enumerate(ids):
        qualifier = 0x10 if (srgb and cid == 15) else 0  # alpha stays linear
        samples += struct.pack('<HBB4BII', i * 8, 7, cid | qualifier, 0, 0, 0, 0, 0, 255)
    block_size = 24 + len(samples)
    block = struct.pack('<IHH', 0, 2, block_size)
    block += struct.pack('<4B', 1, 1, 2 if srgb else 1, 0)  # RGBSDA, BT709, transfer, flags
    block += struct.pack('<4B', 0, 0, 0, 0)  # texel block 1x1x1x1
    block += struct.pack('<8B', channels, 0, 0, 0, 0, 0, 0, 0)
    block += samples
    return struct.pack('<I', 4 + len(block)) + block


def _kvd() -> bytes:
    entry = b'KTXwriter\x00rc_render tools/textures/gen_textures.py\x00'
    data = struct.pack('<I', len(entry)) + entry
    return data + b'\x00' * (-len(data) % 4)


def write_ktx2(path: Path, levels: list, vk_format: int, srgb: bool):
    """Writes a 2D texture with `levels` (uint8 arrays, level 0 first), zstd per level."""
    channels = levels[0].shape[2]
    width = levels[0].shape[1]
    height = levels[0].shape[0]
    dfd = _dfd(channels, srgb)
    kvd = _kvd()
    count = len(levels)
    header_len = 12 + 9 * 4 + 4 * 4 + 2 * 8
    index_len = 24 * count
    dfd_offset = header_len + index_len
    kvd_offset = dfd_offset + len(dfd)
    data_offset = kvd_offset + len(kvd)
    raws = [np.ascontiguousarray(l).tobytes() for l in levels]
    packed = [zstd.compress(r, level=19) for r in raws]
    # Mip data is stored smallest level first; the index lists level 0 first.
    offsets = [0] * count
    cursor = data_offset
    for i in reversed(range(count)):
        offsets[i] = cursor
        cursor += len(packed[i])
    out = bytearray(KTX2_ID)
    out += struct.pack('<9I', vk_format, 1, width, height, 0, 0, 1, count, ZSTD)
    out += struct.pack('<4I2Q', dfd_offset, len(dfd), kvd_offset, len(kvd), 0, 0)
    for i in range(count):
        out += struct.pack('<3Q', offsets[i], len(packed[i]), len(raws[i]))
    out += dfd
    out += kvd
    for i in reversed(range(count)):
        out += packed[i]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(bytes(out))
