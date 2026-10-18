"""Objective checks for rendered cues: decode the OGG, measure loudness/peaks (own BS.1770 meter
and ffmpeg ebur128), loop-seam continuity, and draw waveform/spectrogram/loudness PNGs."""
from __future__ import annotations

import re
import subprocess

import numpy as np
from PIL import Image, ImageDraw, ImageFont

import dsp
from dsp import SR

FONT = ImageFont.load_default(size=13)
CMAP = np.array([(0, 0, 4), (40, 11, 84), (101, 21, 110), (159, 42, 99), (212, 72, 66),
                 (245, 125, 21), (250, 193, 39), (252, 255, 164)], float)


def decode(path):
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le", "-ac", "2",
                          "-ar", str(SR), "-"], capture_output=True, check=True).stdout
    return np.frombuffer(raw, "<f4").reshape(-1, 2).T.astype(float)


def ffmpeg_loudness(path):
    err = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-i", str(path), "-af",
                          "ebur128=peak=true", "-f", "null", "-"], capture_output=True).stderr.decode()
    summ = err[err.rfind("Summary:"):]
    get = lambda k: float(re.search(k + r":\s+(-?[\d.]+|-inf)", summ).group(1).replace("-inf", "-200"))
    return {"I": get("I"), "LRA": get("LRA"), "TP": get("Peak")}


def seam(x):
    """Seam jump vs. the file's own sample-to-sample statistics (both wrap-around)."""
    d = np.abs(np.diff(x, axis=1))
    jump = np.abs(x[:, 0] - x[:, -1]).max()
    return {"jump": float(jump), "jump_pct": float((d < jump).mean() * 100)}


def _cmap(u):
    u = np.clip(u, 0, 1) * (len(CMAP) - 1)
    i = np.minimum(u.astype(int), len(CMAP) - 2)
    f = (u - i)[..., None]
    return (CMAP[i] * (1 - f) + CMAP[i + 1] * f).astype(np.uint8)


def spectrogram(x, w, h, fmin=30.0, fmax=18000.0, nfft=4096):
    mono = 0.5 * (x[0] + x[1])
    n = len(mono)
    pad = np.concatenate([mono[-nfft // 2:], mono, mono[: nfft // 2]])
    starts = (np.arange(w) * n / w).astype(int)
    frames = pad[starts[:, None] + np.arange(nfft)] * np.hanning(nfft)
    p = 10 * np.log10(np.abs(np.fft.rfft(frames, axis=1)) ** 2 + 1e-12)
    fb = np.fft.rfftfreq(nfft, 1.0 / SR)
    rows = fmin * (fmax / fmin) ** (np.arange(h)[::-1] / (h - 1))
    img = np.array([np.interp(rows, fb, col) for col in p]).T
    return _cmap((img - (img.max() - 90.0)) / 90.0)


def plot(x, path, title, sections=(), bpm=None, lines=()):
    W, L = 1500, 70
    hw, hs, hl = 140, 380, 110
    y0 = 34 + 15 * len(lines)
    im = Image.new("RGB", (W + L + 20, y0 + hw + hs + hl + 40), (18, 18, 24))
    dr = ImageDraw.Draw(im)
    dr.text((L, 8), title, fill=(230, 230, 240), font=FONT)
    for i, ln in enumerate(lines):
        dr.text((L, 26 + 15 * i), ln, fill=(170, 200, 255), font=FONT)
    n = x.shape[1]
    cols = (np.arange(W + 1) * n / W).astype(int)
    for ch, col in enumerate(((90, 200, 255), (255, 140, 200))):
        mid = y0 + hw * (ch * 2 + 1) / 4
        for c in range(W):
            seg = x[ch, cols[c]: max(cols[c] + 1, cols[c + 1])]
            dr.line([(L + c, mid - seg.max() * hw / 4), (L + c, mid - seg.min() * hw / 4)], fill=col)
    ys = y0 + hw
    im.paste(Image.fromarray(spectrogram(x, W, hs)), (L, ys))
    for f in (50, 100, 200, 500, 1000, 2000, 5000, 10000):
        yy = ys + hs - 1 - (hs - 1) * np.log(f / 30.0) / np.log(18000.0 / 30.0)
        dr.text((4, yy - 7), f"{f if f < 1000 else str(f // 1000) + 'k'}", fill=(200, 200, 200), font=FONT)
    yl = ys + hs + 8
    st = dsp.short_term(x, hop=0.25)
    dr.rectangle([L, yl, L + W, yl + hl], outline=(60, 60, 70))
    lv = lambda v: yl + hl * (-6 - v) / 34.0
    for ref, colr in ((-14, (120, 255, 120)), (-24, (90, 90, 110))):
        dr.line([(L, lv(ref)), (L + W, lv(ref))], fill=colr)
        dr.text((8, lv(ref) - 7), f"{ref} LU", fill=colr, font=FONT)
    if len(st) > 1:
        pts = [(L + W * (i * 0.25 + 1.5) * SR / n, lv(max(-40.0, v))) for i, v in enumerate(st)]
        dr.line(pts, fill=(255, 220, 90), width=2)
    if bpm:
        for name, start, bars, _ in sections:
            xx = L + W * (start * 4 * 60.0 / bpm * SR) / n
            dr.line([(xx, y0), (xx, yl + hl)], fill=(255, 255, 255))
            dr.text((xx + 3, y0 + 2), name, fill=(255, 255, 255), font=FONT)
    dr.text((L, yl + hl + 8), f"0 s  ...  {n / SR:.2f} s   (waveform L/R, log spectrogram 30 Hz-18 kHz, "
            "short-term loudness 3 s)", fill=(160, 160, 170), font=FONT)
    im.save(path)


def plot_seam(x, path, k=1500):
    """End of the file followed by its start: a clean loop shows no break at the centre line."""
    y = np.concatenate([x[:, -k:], x[:, :k]], axis=1)
    W, H = 1200, 320
    im = Image.new("RGB", (W, H), (18, 18, 24))
    dr = ImageDraw.Draw(im)
    s = 0.9 / max(1e-6, np.abs(y).max())
    for ch, col in enumerate(((90, 200, 255), (255, 140, 200))):
        mid = 40 + 140 * ch + 70
        pts = [(i * W / y.shape[1], mid - y[ch, i] * s * 65) for i in range(y.shape[1])]
        dr.line(pts, fill=col)
    dr.line([(W / 2, 30), (W / 2, H)], fill=(255, 255, 255))
    dr.text((8, 8), f"loop seam: last {k} | first {k} samples (L top, R bottom)", fill=(230, 230, 240), font=FONT)
    im.save(path)


def ltas(x, nfft=32768):
    """Long-term average spectrum (dB), 1/6-octave smoothed, at 1/12-octave points 25 Hz-20 kHz."""
    mono = 0.5 * (x[0] + x[1])
    m = len(mono) // nfft
    p = np.mean(np.abs(np.fft.rfft(mono[: m * nfft].reshape(m, nfft) * np.hanning(nfft), axis=1)) ** 2, 0)
    fb = np.fft.rfftfreq(nfft, 1.0 / SR)
    fs = 25.0 * 2 ** (np.arange(0, 9.65, 1 / 12))
    return fs, np.array([10 * np.log10(p[(fb >= f * 2 ** -(1 / 12)) & (fb < f * 2 ** (1 / 12))].mean() + 1e-20)
                         for f in fs])


def plot_ltas(curves, path):
    W, H, L = 1100, 520, 60
    im = Image.new("RGB", (W + L + 200, H + 60), (18, 18, 24))
    dr = ImageDraw.Draw(im)
    fx = lambda f: L + W * np.log(f / 25.0) / np.log(20000 / 25.0)
    fy = lambda d: 20 + H * (24 - d) / 84.0
    for f in (50, 100, 200, 500, 1000, 2000, 5000, 10000):
        dr.line([(fx(f), 20), (fx(f), 20 + H)], fill=(45, 45, 55))
        dr.text((fx(f) - 10, H + 26), str(f), fill=(180, 180, 180), font=FONT)
    for d in range(-60, 25, 12):
        dr.line([(L, fy(d)), (L + W, fy(d))], fill=(45, 45, 55))
        dr.text((8, fy(d) - 7), f"{d} dB", fill=(180, 180, 180), font=FONT)
    dr.line([(fx(100), fy(4.5 * np.log2(10))), (fx(10000), fy(-4.5 * np.log2(10)))], fill=(120, 120, 120))
    dr.text((fx(3000), fy(-4.5 * np.log2(3)) - 18), "-4.5 dB/oct", fill=(140, 140, 140), font=FONT)
    for i, (name, (fs, db)) in enumerate(curves.items()):
        col = tuple(int(c) for c in _cmap(np.array([0.25 + 0.7 * i / max(1, len(curves) - 1)]))[0])
        db = db - np.interp(1000, fs, db)
        dr.line([(fx(f), fy(d)) for f, d in zip(fs, db)], fill=col, width=2)
        dr.text((L + W + 15, 30 + 18 * i), name, fill=col, font=FONT)
    dr.text((L, 2), "long-term average spectrum, 0 dB at 1 kHz", fill=(230, 230, 240), font=FONT)
    im.save(path)
