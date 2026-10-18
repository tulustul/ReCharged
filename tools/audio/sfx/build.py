#!/usr/bin/env python3
"""Render, master, encode (OGG Vorbis) and verify every ReCharged SFX and ambience sound.

    python3 tools/audio/sfx/build.py                      # everything
    python3 tools/audio/sfx/build.py rocket_ impact_hard  # names or name prefixes
    python3 tools/audio/sfx/build.py --previews DIR       # PNG previews (default target/audio-previews/sfx)

Per sound: render the recipe (rng seeded from the name) → remove DC, trim/fade one-shots →
normalise to the category loudness (≤ MAX_GR_DB of limiting) → brick-wall limit → encode →
decode and verify: sample-exact length, true peak ≤ -1 dBTP (gain is lowered and the file
re-encoded if needed), loop seam continuity → waveform/spectrogram PNG → summary table on stdout.
Exit code 1 if any check fails.
"""
from __future__ import annotations

import argparse
import importlib
import subprocess
import sys
import zlib
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.dont_write_bytecode = True  # no __pycache__ in the repo
sys.path.insert(0, str(Path(__file__).parent))
import synth as s  # noqa: E402

BROKEN = []  # recipe modules that failed to import: the others still build, the run fails
for _mod in ("recipes_car", "recipes_items", "recipes_race_ui", "recipes_ambience"):
    try:
        importlib.import_module(_mod)  # registers its sounds in synth.SOUNDS
    except Exception as e:  # noqa: BLE001
        BROKEN.append(f"{_mod}: {e!r}")
        print(f"WARNING: {_mod} failed to import: {e!r}", file=sys.stderr)

ROOT = Path(__file__).resolve().parents[3]
OUT = {"sfx": ROOT / "assets/audio/sfx", "ambience": ROOT / "assets/audio/ambience"}
QUALITY = {"sfx": 6, "loop": 8, "ambience": 5}  # libvorbis -q (loops: seam error = mid-file error)
CEILING_DB = -1.3   # limiter ceiling (sample peak); the decoded file must stay ≤ PEAK_MAX_DB
PEAK_MAX_DB = -1.0
MAX_GR_DB = 4.0     # never limit more than this to reach a loudness target
MIN_ONESHOT_SEC = 0.15  # shorter one-shots get a silent tail (see encode)
# Loudness targets: loops = integrated LUFS (BS.1770, gated); one-shots = loudest 100 ms
# K-weighted window ("L100"). Mono files are measured as played: dual-mono (+3 dB).
TARGETS = {
    "engine": -17.0,   # engine loops
    "loop": -19.0,     # tyre, surface, boost, drift, projectile and hazard loops
    "car": -11.0,      # impacts, landings, boost whoosh
    "item": -11.0,     # weapon and item one-shots
    "race": -12.0,     # countdown, lap, finish, jingles
    "ui": -16.0,       # menu cues
    "amb": -25.0,      # ambience beds
    "thunder": -13.0,  # thunder one-shots
}


# --------------------------------------------------------------------------- measuring
def _kmag(f):
    z = np.exp(-2j * np.pi * f / s.SR)
    h1 = (1.53512485958697 - 2.69169618940638 * z + 1.19839281085285 * z * z) / (
        1 - 1.69065929318241 * z + 0.73248077421585 * z * z)
    h2 = (1 - 2 * z + z * z) / (1 - 1.99004745483398 * z + 0.99007225036621 * z * z)
    return np.abs(h1 * h2)


def _chan_power(x, circular):
    """Per-sample K-weighted power summed over the channels as played (mono = dual-mono)."""
    k = s.eq(x, _kmag, circular)
    return np.sum(k * k, axis=1) if k.ndim == 2 else 2 * k * k


def _windows(p, win, hop):
    c = np.concatenate([[0.0], np.cumsum(p)])
    st = np.arange(0, max(len(p) - win, 0) + 1, hop)
    return (c[np.minimum(st + win, len(p))] - c[st]) / win


def loudness(x, loop):
    """Integrated LUFS for loops, loudest-100 ms K-weighted loudness for one-shots."""
    p = _chan_power(x, loop)
    if not loop:
        return -0.691 + 10 * np.log10(max(_windows(p, s.n_(0.1), s.n_(0.01)).max(), 1e-12))
    z = _windows(p, s.n_(0.4), s.n_(0.1))
    lj = -0.691 + 10 * np.log10(np.maximum(z, 1e-12))
    z = z[lj > -70]
    rel = -0.691 + 10 * np.log10(max(z.mean(), 1e-12)) - 10
    z = z[-0.691 + 10 * np.log10(z) > rel]
    return -0.691 + 10 * np.log10(max(z.mean(), 1e-12))


def rms_db(x, loop):
    m = x if x.ndim == 1 else x.mean(1)
    if loop:
        return s.db(np.sqrt(np.mean(m * m)))
    return s.db(np.sqrt(_windows(m * m, s.n_(0.1), s.n_(0.01)).max()))


def true_peak_db(x):
    """4× oversampled peak (FFT interpolation)."""
    m = x if x.ndim == 2 else x[:, None]
    n = len(m)
    X = np.fft.rfft(np.pad(m, ((0, n % 2), (0, 0))), axis=0)
    up = np.fft.irfft(X, 4 * (n + n % 2), axis=0) * 4
    return s.db(np.max(np.abs(up)))


def seam_check(y):
    """(jump ratio, flux ratio, level step dB) at the loop wrap of decoded audio; ratios ≤ 1.5 pass.
    jump: |x[0]-x[-1]| vs the 99.9th percentile sample step; flux: spectral change (dB, bins more
    than 60 dB under the loudest ignored) across the seam vs its 99th percentile elsewhere."""
    m = y if y.ndim == 1 else y.mean(1)
    n = len(m)
    jump = abs(m[0] - m[-1]) / (np.percentile(np.abs(np.diff(m)), 99.9) + 1e-12)
    r = np.roll(m, n // 2)  # the seam now sits at index n//2
    frame, hop = 1024, 256
    st = np.arange(0, n - frame, hop)
    S = 20 * np.log10(np.abs(np.fft.rfft(r[st[:, None] + np.arange(frame)] * np.hanning(frame), axis=1)) + 1e-9)
    S = np.maximum(S, S.max() - 60)
    flux = np.sqrt(np.mean(np.diff(S, axis=0) ** 2, axis=1))
    mid = (st[:-1] + st[1:]) / 2 + frame / 2
    at = np.abs(mid - n // 2) < frame
    ratio = flux[at].max() / (np.percentile(flux[~at], 99) + 1e-12)
    a, b = m[: s.n_(0.05)], m[-s.n_(0.05):]
    step = s.db(np.sqrt(np.mean(a * a))) - s.db(np.sqrt(np.mean(b * b)))
    return jump, ratio, step


# --------------------------------------------------------------------------- mastering
def _lag_min(g, w, circular):
    """m[i] = min(g[i-w+1 .. i])."""
    m, k = g.copy(), 1
    while k < w:
        sh = min(k, w - k)
        prev = np.roll(m, sh)
        if not circular:
            prev[:sh] = 1.0
        m = np.minimum(m, prev)
        k += sh
    return m


def _lead_mean(m, w, circular):
    """a[i] = mean(m[i .. i+w-1])."""
    e = np.concatenate([m, m[: w - 1] if circular else np.ones(w - 1)])
    c = np.concatenate([[0.0], np.cumsum(e)])
    return (c[w: w + len(m)] - c[: len(m)]) / w


def limit(x, ceiling_db, circular):
    """Look-ahead brick-wall limiter (5 ms attack ramp, 20 ms hold); never exceeds the ceiling."""
    a = np.abs(x) if x.ndim == 1 else np.max(np.abs(x), axis=1)
    g = np.minimum(1.0, s.amp(ceiling_db) / np.maximum(a, 1e-12))
    gain = _lead_mean(_lag_min(g, s.n_(0.02), circular), s.n_(0.005), circular)
    return x * (gain if x.ndim == 1 else gain[:, None])


def trim_tail(x, floor_db=-66.0):
    m = x if x.ndim == 1 else np.max(np.abs(x), axis=1)
    e = np.sqrt(_windows(m * m, s.n_(0.01), s.n_(0.0025)))
    loud = np.nonzero(e > np.max(np.abs(m)) * s.amp(floor_db))[0]
    end = min(len(x), (loud[-1] if len(loud) else 0) * s.n_(0.0025) + s.n_(0.03))
    return x[:end]


def master(x, snd):
    x = s.eq(np.asarray(x, dtype=float), lambda f: s.hp(f, 22, 2), snd.loop)
    if not snd.loop:
        x = s.fades(trim_tail(x), 0.0003, 0.01)
        x = s.pad(x, max(0.0, MIN_ONESHOT_SEC - len(x) / s.SR))  # silent tail → multi-page OGG
    target = TARGETS[snd.cat] + snd.gain
    gain_db = target - loudness(x, snd.loop)
    headroom = CEILING_DB + MAX_GR_DB - s.db(np.max(np.abs(x)))
    gain_db = min(gain_db, headroom)
    return limit(x * s.amp(gain_db), CEILING_DB, snd.loop), target


# --------------------------------------------------------------------------- encoding
def encode(x, path, q):
    ch = 1 if x.ndim == 1 else x.shape[1]
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "f32le", "-ar", str(s.SR), "-ac", str(ch),
                    "-i", "pipe:0", "-c:a", "libvorbis", "-q:a", str(q), "-map_metadata", "-1",
                    # ≥ 2 audio pages: ffmpeg mis-trims single-page streams by up to 128 samples
                    "-page_duration", "50000",
                    "-fflags", "+bitexact", "-flags:a", "+bitexact", str(path)],
                   input=x.astype("<f4").tobytes(), check=True)


def decode(path, ch):
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le", "-ac", str(ch),
                          "-ar", str(s.SR), "pipe:1"], capture_output=True, check=True).stdout
    y = np.frombuffer(raw, "<f4").astype(float)
    return y.reshape(-1, ch) if ch > 1 else y


# --------------------------------------------------------------------------- previews
_CMAP = np.array([(0, 0, 4), (40, 11, 84), (101, 21, 110), (159, 42, 99), (212, 72, 66),
                  (245, 125, 21), (250, 193, 39), (252, 255, 164)], dtype=float)
F_LO, F_HI = 30.0, 24000.0


def _spectro(m, width, height, ref=None):
    n = len(m)
    frame = int(np.clip(1 << int(np.log2(max(n // 16, 256))), 256, 4096))
    hop = max(1, (n - frame) // width) if n > frame else 1
    m = np.pad(m, (0, max(0, frame - n)))
    st = np.arange(0, len(m) - frame + 1, hop)[: width * 4]
    S = np.abs(np.fft.rfft(m[st[:, None] + np.arange(frame)] * np.hanning(frame), axis=1))
    fb = np.fft.rfftfreq(frame, 1 / s.SR)
    lo_bins = np.clip(np.searchsorted(fb, np.geomspace(F_LO, F_HI, height + 1)[:-1]), 0, len(fb) - 1)
    cnt = np.diff(np.append(lo_bins, len(fb)))
    S = np.sqrt(np.add.reduceat(S * S, lo_bins, axis=1) / np.maximum(cnt, 1))[:, ::-1].T  # row power, high on top
    ref = ref or S.max() + 1e-12
    v = np.clip((20 * np.log10(S / ref + 1e-12) + 80) / 80, 0, 1) * (len(_CMAP) - 1)
    i = np.minimum(v.astype(int), len(_CMAP) - 2)
    rgb = _CMAP[i] + (_CMAP[i + 1] - _CMAP[i]) * (v - i)[..., None]
    img = Image.fromarray(rgb.astype(np.uint8)).resize((width, height), Image.NEAREST)
    d = ImageDraw.Draw(img)
    for f, lab in ((100, "100"), (1000, "1k"), (10000, "10k")):
        y = int(np.log(F_HI / f) / np.log(F_HI / F_LO) * (height - 1))
        d.line([(0, y), (6, y)], fill=(255, 255, 255))
        d.text((8, y - 6), lab, fill=(200, 200, 200))
    return img, ref


def _wave(m, width, height):
    img = Image.new("RGB", (width, height), (16, 18, 28))
    d = ImageDraw.Draw(img)
    cols = np.array_split(m, width) if len(m) >= width else [m[i: i + 1] for i in range(len(m))]
    mid = height / 2
    for xpix, c in enumerate(cols):
        if len(c):
            d.line([(xpix, mid - c.max() * mid), (xpix, mid - c.min() * mid)], fill=(90, 220, 255))
    lvl = s.amp(PEAK_MAX_DB) * mid
    d.line([(0, mid - lvl), (width, mid - lvl)], fill=(120, 60, 60))
    d.line([(0, mid + lvl), (width, mid + lvl)], fill=(120, 60, 60))
    return img


def preview(y, snd, path, header):
    m = y if y.ndim == 1 else y.mean(1)
    W, WH, SH = 840, 80, 260
    panels = [(m, W, "full")]
    if snd.loop:
        k = s.n_(0.4)
        panels.append((np.concatenate([m[-k:], m[:k]]), 300, "seam"))
    width = sum(p[1] for p in panels) + 10 * (len(panels) - 1)
    extra = 200 if snd.pitch else 0
    img = Image.new("RGB", (width, 22 + WH + SH + extra), (8, 8, 12))
    d = ImageDraw.Draw(img)
    d.text((6, 5), header, fill=(235, 235, 235))
    x0, ref = 0, None
    for sig, w, kind in panels:
        img.paste(_wave(sig, w, WH), (x0, 22))
        sp, ref = _spectro(sig, w, SH, ref)
        img.paste(sp, (x0, 22 + WH))
        if kind == "seam":
            d.line([(x0 + w // 2, 22), (x0 + w // 2, 22 + WH + SH)], fill=(255, 40, 40))
            d.text((x0 + 4, 24), "seam ±0.4 s", fill=(255, 120, 120))
        x0 += w + 10
    if snd.pitch:
        for j, rate in enumerate((0.5, 1.0, 2.0)):
            r = s.hermite_rate(m, rate, s.n_(1.2))
            sp, _ = _spectro(r, width // 3 - 6, extra - 16, ref)
            img.paste(sp, (j * (width // 3), 22 + WH + SH + 16))
            d.text((j * (width // 3) + 4, 22 + WH + SH + 2), f"playback rate {rate}", fill=(235, 235, 235))
    img.save(path)


# --------------------------------------------------------------------------- main
def build(name, snd, preview_dir):
    rng = np.random.default_rng(zlib.crc32(name.encode()))
    x, target = master(snd.fn(rng), snd)
    ch = 1 if x.ndim == 1 else 2
    path = OUT[snd.folder] / f"{name}.ogg"
    for _ in range(4):
        encode(x, path, QUALITY["loop" if snd.loop and snd.folder == "sfx" else snd.folder])
        y = decode(path, ch)
        over = true_peak_db(y) - PEAK_MAX_DB  # true peak: kira's resampler interpolates between samples
        if over <= 0:
            break
        x = x * s.amp(-over - 0.05)
    row = dict(name=name, folder=snd.folder, dur=len(y) / s.SR, loop=snd.loop, ch=ch,
               peak=s.db(np.max(np.abs(y))), tp=true_peak_db(y), rms=rms_db(y, snd.loop),
               lufs=loudness(y, snd.loop), target=target, problems=[])
    if len(y) != len(x):
        row["problems"].append(f"decoded length {len(y)} != {len(x)}")
    if row["tp"] > PEAK_MAX_DB + 1e-3:
        row["problems"].append(f"true peak {row['tp']:.2f} dBTP")
    if snd.loop:
        jump, flux, step = seam_check(y)
        row["seam"] = f"{jump:.2f}/{flux:.2f}/{step:+.1f}dB"
        if jump > 1.5 or flux > 1.5:
            row["problems"].append("loop seam")
    if abs(row["lufs"] - target) > 1.5:
        row["problems"].append(f"loudness {row['lufs']:.1f} vs target {target:.1f}")
    if preview_dir:
        header = (f"{name}  {row['dur']:.2f}s  {'LOOP ' if snd.loop else ''}{ch}ch  peak {row['peak']:.1f} dBFS"
                  f"  TP {row['tp']:.1f}  RMS {row['rms']:.1f}  {'LUFS-I' if snd.loop else 'L100'} {row['lufs']:.1f}")
        preview(y, snd, preview_dir / f"{name}.png", header)
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("names", nargs="*")
    ap.add_argument("--previews", default=str(ROOT / "target/audio-previews/sfx"))
    ap.add_argument("--no-previews", action="store_true")
    a = ap.parse_args()
    todo = {k: v for k, v in s.SOUNDS.items() if not a.names or any(k.startswith(p) for p in a.names)}
    if not todo:
        sys.exit(f"no sound matches {a.names}")
    pdir = None if a.no_previews else Path(a.previews)
    if pdir:
        pdir.mkdir(parents=True, exist_ok=True)
    for d in OUT.values():
        d.mkdir(parents=True, exist_ok=True)
    print("| cue | file | dur s | loop | ch | peak dBFS | true peak | RMS dBFS | loudness | target | seam j/f/step |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    bad = list(BROKEN)
    for name, snd in todo.items():
        r = build(name, snd, pdir)
        unit = "LUFS-I" if r["loop"] else "L100"
        print(f"| {name} | {r['folder']}/{name}.ogg | {r['dur']:.2f} | {'yes' if r['loop'] else 'no'} | {r['ch']} "
              f"| {r['peak']:.1f} | {r['tp']:.1f} | {r['rms']:.1f} | {r['lufs']:.1f} {unit} | {r['target']:.0f} "
              f"| {r.get('seam', '')} |", flush=True)
        if r["problems"]:
            bad.append(f"{name}: {', '.join(r['problems'])}")
    if bad:
        print("\nPROBLEMS:\n" + "\n".join(bad))
        sys.exit(1)


if __name__ == "__main__":
    main()
