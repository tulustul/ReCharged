"""Offline DSP kit for the ReCharged music generator (numpy only, no scipy).

Shapes: mono (n,), stereo (2, n), float64, SR = 44100.

Loops: LTI effects (EQ, ping-pong delay, convolution reverb, oversampling) are circular FFT
operations on the loop-length buffer -- the exact steady state of an endlessly repeated loop,
so the seam is clean by construction. Stateful dynamics (compressor, limiter) and time-varying
filters run over a wrap-around warm-up instead (see `periodic`).
"""
from __future__ import annotations

import zlib

import numpy as np

SR = 44100


def rng(*key) -> np.random.Generator:
    """Deterministic generator from a repr-able key (str hash() is salted per process)."""
    return np.random.default_rng(zlib.crc32(repr(key).encode()))


def undb(d):
    return 10.0 ** (np.asarray(d, float) / 20.0)


def todb(x):
    return 20.0 * np.log10(np.maximum(np.abs(x), 1e-12))


def mtof(m):
    return 440.0 * 2.0 ** ((np.asarray(m, float) - 69.0) / 12.0)


def stereo(x):
    return np.vstack([x, x]) if x.ndim == 1 else x


def pan(x, p):
    """Constant-power pan of mono or stereo x; p in -1..1."""
    a = (p + 1.0) * np.pi / 4.0
    x = stereo(x)
    return np.vstack([x[0] * np.cos(a), x[1] * np.sin(a)]) * np.sqrt(2.0)


def width(x, w):
    """Mid/side width: 0 = mono, 1 = unchanged, >1 wider."""
    m, s = 0.5 * (x[0] + x[1]), 0.5 * (x[0] - x[1]) * w
    return np.vstack([m + s, m - s])


# ------------------------------------------------------------------ oscillators

def phase(freq, n, ph0=0.0, sr=SR):
    """Phase in cycles [0, 1) and per-sample increment for a constant or per-sample freq."""
    dt = np.array(np.broadcast_to(np.asarray(freq, float) / sr, (n,)))
    ph = np.empty(n)
    ph[0] = 0.0
    np.cumsum(dt[:-1], out=ph[1:])
    return (ph + ph0) % 1.0, dt


def _blep(t, dt):
    r = np.zeros_like(t)
    m = t < dt
    u = t[m] / dt[m]
    r[m] = u + u - u * u - 1.0
    m = t > 1.0 - dt
    u = (t[m] - 1.0) / dt[m]
    r[m] = u * u + u + u + 1.0
    return r


def saw(freq, n, ph0=0.0, sr=SR):
    t, dt = phase(freq, n, ph0, sr)
    return 2.0 * t - 1.0 - _blep(t, dt)


def pulse(freq, n, ph0=0.0, sr=SR, pw=0.5):
    """Band-limited pulse; pw may be an array (PWM). DC-corrected."""
    t, dt = phase(freq, n, ph0, sr)
    y = np.where(t < pw, 1.0, -1.0) + _blep(t, dt) - _blep((t - pw) % 1.0, dt)
    return y - (2.0 * pw - 1.0)


def tri(freq, n, ph0=0.0, sr=SR):
    t, _ = phase(freq, n, ph0, sr)
    return 4.0 * np.abs(t - 0.5) - 1.0


def sine(freq, n, ph0=0.0, sr=SR):
    t, _ = phase(freq, n, ph0, sr)
    return np.sin(2.0 * np.pi * t)


OSC = {"saw": saw, "pulse": pulse, "square": pulse, "tri": tri, "sine": sine}


def unison(wave, freq, n, voices=1, cents=0.0, spread=0.8, r=None, sr=SR, **kw):
    """Detuned unison stack, voices spread across the stereo field -> (2, n)."""
    offs = np.linspace(-1.0, 1.0, voices) if voices > 1 else np.zeros(1)
    if r is not None and voices > 2:
        offs = offs + r.uniform(-0.12, 0.12, voices) * (np.abs(offs) > 0)
    out = np.zeros((2, n))
    for o in offs:
        v = OSC[wave](freq * 2.0 ** (o * cents / 1200.0), n,
                      r.random() if r is not None else 0.0, sr, **kw)
        a = (o * spread + 1.0) * np.pi / 4.0
        out[0] += v * np.cos(a)
        out[1] += v * np.sin(a)
    return out * np.sqrt(2.0 / len(offs))


# ------------------------------------------------------------------ envelopes

def adsr(n, gate, a=0.005, d=0.1, s=1.0, r=0.1, sr=SR):
    """n samples; gate = note-on seconds. Linear attack, exponential decay (tau d/3) to s,
    release that lands exactly on 0 after r seconds."""
    t = np.arange(n) / sr
    a, dtau, r = max(a, 1e-4), max(d / 3.0, 1e-4), max(r, 1e-4)
    on = np.where(t < a, t / a, s + (1.0 - s) * np.exp(-(t - a) / dtau))
    lvl = gate / a if gate < a else s + (1.0 - s) * np.exp(-(gate - a) / dtau)
    e = np.exp(-4.6 * (t - gate) / r)
    return np.where(t < gate, on, lvl * np.clip((e - 0.01) / 0.99, 0.0, None))


def fade_tail(x, sec=0.005, sr=SR):
    """Force the last `sec` to zero (clickless note ends)."""
    k = min(int(sec * sr), x.shape[-1])
    if k:
        x[..., -k:] *= np.linspace(1.0, 0.0, k)
    return x


# ------------------------------------------------------------------ time-varying filters

def svf(x, fc, q=0.7071, mode="lp", sr=SR):
    """Zavalishin TPT state-variable filter; fc and q may be per-sample arrays.
    mode: lp | bp (unity peak) | hp | notch."""
    x = np.asarray(x, float)
    xs = np.atleast_2d(x)
    n = xs.shape[1]
    g = np.tan(np.pi * np.clip(np.broadcast_to(fc, (n,)), 8.0, 0.45 * sr) / sr)
    k = 1.0 / np.broadcast_to(np.asarray(q, float), (n,))
    a1 = 1.0 / (1.0 + g * (g + k))
    a2 = g * a1
    a3 = g * a2
    cl = (a1.tolist(), a2.tolist(), a3.tolist())
    out = np.empty_like(xs)
    for ch in range(xs.shape[0]):
        v1, v2 = _svf_run(xs[ch].tolist(), *cl)
        v1, v2 = np.array(v1), np.array(v2)
        out[ch] = {"lp": lambda: v2, "bp": lambda: k * v1, "hp": lambda: xs[ch] - k * v1 - v2,
                   "notch": lambda: xs[ch] - k * v1}[mode]()
    return out if x.ndim == 2 else out[0]


def _svf_run(x, a1, a2, a3):
    s1 = s2 = 0.0
    o1 = [0.0] * len(x)
    o2 = [0.0] * len(x)
    i = 0
    for xi, b1, b2, b3 in zip(x, a1, a2, a3):
        v3 = xi - s2
        v1 = b1 * s1 + b2 * v3
        v2 = s2 + b2 * s1 + b3 * v3
        s1 = v1 + v1 - s1
        s2 = v2 + v2 - s2
        o1[i] = v1
        o2[i] = v2
        i += 1
    return o1, o2


def lp24(x, fc, res=0.0, sr=SR):
    """4-pole low-pass: two SVFs with Butterworth Qs; res 0..1 adds a resonant peak."""
    return svf(svf(x, fc, 0.5412, "lp", sr), fc, 1.3066 + 7.0 * res, "lp", sr)


# ------------------------------------------------------------------ LTI effects (FFT)

def biquad(kind, f0, q=0.7071, gain=0.0, sr=SR):
    """RBJ cookbook biquad (b, a). kind: lp hp bp peak low high."""
    A = 10.0 ** (gain / 40.0)
    w = 2.0 * np.pi * f0 / sr
    c, s = np.cos(w), np.sin(w)
    al = s / (2.0 * q)
    sa = 2.0 * np.sqrt(A) * al
    if kind == "lp":
        return [(1 - c) / 2, 1 - c, (1 - c) / 2], [1 + al, -2 * c, 1 - al]
    if kind == "hp":
        return [(1 + c) / 2, -(1 + c), (1 + c) / 2], [1 + al, -2 * c, 1 - al]
    if kind == "bp":
        return [al, 0.0, -al], [1 + al, -2 * c, 1 - al]
    if kind == "peak":
        return [1 + al * A, -2 * c, 1 - al * A], [1 + al / A, -2 * c, 1 - al / A]
    if kind == "low":
        return ([A * ((A + 1) - (A - 1) * c + sa), 2 * A * ((A - 1) - (A + 1) * c),
                 A * ((A + 1) - (A - 1) * c - sa)],
                [(A + 1) + (A - 1) * c + sa, -2 * ((A - 1) + (A + 1) * c), (A + 1) + (A - 1) * c - sa])
    if kind == "high":
        return ([A * ((A + 1) + (A - 1) * c + sa), -2 * A * ((A - 1) + (A + 1) * c),
                 A * ((A + 1) + (A - 1) * c - sa)],
                [(A + 1) - (A - 1) * c + sa, 2 * ((A - 1) - (A + 1) * c), (A + 1) - (A - 1) * c - sa])
    raise ValueError(kind)


def freqz(b, a, f, sr=SR):
    z = np.exp(-2j * np.pi * np.asarray(f) / sr)
    return (b[0] + z * (b[1] + z * b[2])) / (a[0] + z * (a[1] + z * a[2]))


def response(bands, f, sr=SR):
    """Product of biquad responses; band = (kind, f0[, q[, gain_db]])."""
    h = np.ones(len(f), complex)
    for band in bands:
        h *= freqz(*biquad(*band, sr=sr), f, sr)
    return h


def eq(x, bands, sr=SR):
    """Apply biquads as a circular FFT filter: exact steady state for a loop buffer."""
    if not bands:
        return x
    n = x.shape[-1]
    h = response(bands, np.fft.rfftfreq(n, 1.0 / sr), sr)
    return np.fft.irfft(np.fft.rfft(x, axis=-1) * h, n, axis=-1)


def pingpong(x, delay, fb=0.35, hp=300.0, lp=5000.0, sr=SR):
    """Ping-pong echo (input summed to mono, first repeat left), filtered feedback, solved in
    closed form per FFT bin: d1 = zF(M + fb d2), d2 = zF d1."""
    n = x.shape[-1]
    f = np.fft.rfftfreq(n, 1.0 / sr)
    zf = np.exp(-2j * np.pi * f * delay) * response([("hp", hp), ("lp", lp)], f, sr)
    d1 = zf * np.fft.rfft(0.5 * (x[0] + x[1])) / (1.0 - fb * zf * zf)
    return np.fft.irfft(np.vstack([d1, zf * d1]), n, axis=-1)


def convolve(x, ir):
    """Circular convolution of stereo x with stereo ir (len(ir) <= len(x))."""
    n = x.shape[-1]
    return np.fft.irfft(np.fft.rfft(x, axis=-1) * np.fft.rfft(ir, n, axis=-1), n, axis=-1)


def reverb_ir(rt=2.0, damp=7000.0, predelay=0.015, gate=None, early=0.0, seed=0, sr=SR):
    """Synthetic stereo plate/room impulse response: decorrelated noise with an octave-band
    exponential decay (RT60 = rt at low/mid frequencies, shorter above `damp`), a short
    density build-up, optional early reflections and an optional gate (80s gated snare).
    Normalised to unit energy per channel."""
    r = rng("ir", rt, damp, gate, early, seed)
    n = int((predelay + (gate + 0.05 if gate else rt * 1.25)) * sr)
    t = np.arange(n) / sr
    spec = np.fft.rfft(r.standard_normal((2, n)), axis=-1)
    f = np.maximum(np.fft.rfftfreq(n, 1.0 / sr), 1.0)
    out = np.zeros((2, n))
    centers = 62.5 * 2.0 ** np.arange(9)
    for i, c in enumerate(centers):
        u = np.log2(f / c)
        w = np.where(np.abs(u) < 1.0, np.cos(0.5 * np.pi * u) ** 2, 0.0)
        if i == 0:
            w[u <= 0] = 1.0
        if i == len(centers) - 1:
            w[u >= 0] = 1.0
        band_rt = rt / np.sqrt(1.0 + (c / damp) ** 2) * (0.75 if c < 150 else 1.0)
        out += np.fft.irfft(spec * w, n, axis=-1) * np.exp(-6.91 * t / band_rt)
    out *= 1.0 - np.exp(-t / 0.008)
    if gate:
        out *= np.clip((gate - t) / 0.02, 0.0, 1.0)
    if early:
        for k in range(12):
            tt = r.uniform(0.004, 0.06)
            out[k % 2, int(tt * sr)] += early * (0.6 + 0.4 * r.random()) * (1 - tt / 0.07) * r.choice((-1, 1))
    out = np.concatenate([np.zeros((2, int(predelay * sr))), out], axis=1)
    return out / np.sqrt(np.mean(np.sum(out ** 2, axis=1)))


def chorus(x, rate=0.3, depth=0.0025, base=0.012, mix=0.4, loop=True, sr=SR):
    """Two modulated taps per channel (quadrature LFOs). Loop mode wraps reads and rounds the
    LFO to whole cycles per buffer so the result stays periodic."""
    n = x.shape[-1]
    if loop:
        rate = max(1, round(rate * n / sr)) * sr / n
    i = np.arange(n)
    out = (1.0 - 0.5 * mix) * x.copy()
    for ch in range(2):
        for k, ph in enumerate((0.0, np.pi)):
            d = (base * (1 + 0.3 * k) + depth * np.sin(2 * np.pi * rate * i / sr + ph + ch * np.pi / 2)) * sr
            pos = i - d
            i0 = np.floor(pos).astype(np.int64)
            fr = pos - i0
            if loop:
                y = x[ch, i0 % n] * (1 - fr) + x[ch, (i0 + 1) % n] * fr
            else:
                a, b = np.clip(i0, 0, n - 1), np.clip(i0 + 1, 0, n - 1)
                y = np.where(i0 >= 0, x[ch, a] * (1 - fr) + x[ch, b] * fr, 0.0)
            out[ch] += 0.5 * mix * y
    return out


def oversample(x, k):
    """Band-limited k-times upsampling (circular)."""
    n = x.shape[-1]
    return np.fft.irfft(np.fft.rfft(x, axis=-1), n * k, axis=-1) * k


def downsample(y, k):
    n = y.shape[-1] // k
    return np.fft.irfft(np.fft.rfft(y, axis=-1)[..., : n // 2 + 1], n, axis=-1) / k


# ------------------------------------------------------------------ dynamics

def periodic(fn, x, warm, loop):
    """Run a stateful process on a loop: prepend the loop's last `warm` samples so the state
    at sample 0 equals the state after the loop end, then drop the warm-up."""
    if not loop or warm <= 0:
        return fn(x)
    warm = min(warm, x.shape[-1])
    return fn(np.concatenate([x[..., -warm:], x], axis=-1))[..., warm:]


def duck_env(n, onsets, depth, attack=0.003, hold=0.02, release=0.2, loop=True, sr=SR):
    """Sidechain pump gain from known kick onsets (samples): dips to 1-depth, holds, recovers
    with a raised-cosine. Overlaps use the deeper of the last two kicks, so it is continuous."""
    on = np.sort(np.asarray(onsets, float))
    if len(on) == 0 or depth <= 0:
        return np.ones(n)
    if loop:
        on = np.concatenate([on[-2:] - n, on])
    else:
        on = np.concatenate([[-1e9, -1e9], on])
    i = np.arange(n)
    j = np.searchsorted(on, i, side="right") - 1

    def shape(dt):
        dt = dt / sr
        u = np.clip((dt - attack - hold) / release, 0.0, 1.0)
        return np.where(dt < attack, dt / attack, 0.5 * (1 + np.cos(np.pi * u)))

    s = np.maximum(shape(i - on[j]), shape(i - on[np.maximum(j - 1, 0)]))
    return 1.0 - depth * s


def compress(x, thresh, ratio=2.0, attack=0.01, release=0.15, knee=6.0, hop=32, loop=True, sr=SR):
    """Feed-forward RMS-ish bus compressor; detector on hop blocks, smoothed in dB."""
    def run(y):
        m = y.shape[-1] // hop
        ms = np.mean(np.maximum(y[0] ** 2, y[1] ** 2)[: m * hop].reshape(m, hop), axis=1)
        over = 10 * np.log10(ms + 1e-12) - thresh
        slope = 1.0 - 1.0 / ratio
        gr = np.where(over <= -knee / 2, 0.0,
                      np.where(over >= knee / 2, over * slope, slope * (over + knee / 2) ** 2 / (2 * knee)))
        ca, cr = np.exp(-hop / (attack * sr)), np.exp(-hop / (release * sr))
        g, sm = 0.0, []
        for v in gr.tolist():
            g = v + (ca if v > g else cr) * (g - v)
            sm.append(g)
        gain = undb(-np.interp(np.arange(y.shape[-1]), np.arange(m) * hop + hop / 2, sm))
        return y * gain
    return periodic(run, x, int(1.5 * sr), loop)


def softclip(x, knee=-3.0):
    """Memoryless soft knee: unity below `knee` dBFS, tanh towards 0 dBFS above (2x oversampled)."""
    t = float(undb(knee))
    y = oversample(x, 2)
    a = np.abs(y)
    m = a > t
    y[m] = np.sign(y[m]) * (t + (1 - t) * np.tanh((a[m] - t) / (1 - t)))
    return downsample(y, 2)


def peak_env(x):
    """Per-sample true-peak estimate: max |x| over 4x-oversampled points in [n, n+1), all channels."""
    n = x.shape[-1]
    return np.abs(oversample(x, 4)).reshape(x.shape[0], n, 4).max(axis=(0, 2))


def _runmin(v, w):
    """m[i] = min(v[i:i+w]) (truncated at the end)."""
    m = v.copy()
    span = 1
    while span * 2 <= w:
        m[:-span] = np.minimum(m[:-span], m[span:])
        span *= 2
    k = w - span
    if k > 0:
        m[:-k] = np.minimum(m[:-k], m[k:])
    return m


def limiter(x, ceiling=-1.5, look=0.003, release=0.12, loop=True, sr=SR):
    """Look-ahead true-peak limiter. Gain = box-average(window w) of the forward running min of
    the required gain -> never above the requirement, smooth w-sample attack; plus a slower
    release computed on 64-sample blocks."""
    c = float(undb(ceiling))
    w = max(2, int(look * sr))
    p = peak_env(x)
    warm = int(0.5 * sr) if loop else 0
    if loop:
        p = np.concatenate([p[-warm:], p, p[:w]])
        xx = np.concatenate([x[:, -warm:], x, x[:, :w]], axis=1)
    else:
        p = np.concatenate([p, np.zeros(w)])
        xx = np.concatenate([x, np.zeros((2, w))], axis=1)
    req = np.minimum(1.0, c / np.maximum(p, 1e-12))
    m = _runmin(req, w)
    cs = np.cumsum(np.concatenate([np.ones(w), m]))
    g = (cs[w:] - cs[:-w]) / w
    hop = 64
    nb = len(g) // hop
    blocks = g[: nb * hop].reshape(nb, hop).min(axis=1)
    rc = np.exp(-hop / (release * sr))
    v, rel = 1.0, []
    for b in blocks.tolist():
        v = b if b < v else b - (b - v) * rc
        rel.append(v)
    r = np.interp(np.arange(len(g)), np.arange(nb) * hop + hop / 2, rel)
    y = xx * np.minimum(g, r)
    return y[:, warm: warm + x.shape[-1]]


# ------------------------------------------------------------------ loudness (BS.1770-4)

def _k_weight(f, sr):
    """BS.1770 K-weighting response at any rate (B. De Man's parametrisation)."""
    k = np.tan(np.pi * 1681.974450955533 / sr)
    q = 0.7071752369554196
    vh = 10.0 ** (3.999843853973347 / 20.0)
    vb = vh ** 0.4996667741545416
    a0 = 1.0 + k / q + k * k
    shelf = freqz([(vh + vb * k / q + k * k) / a0, 2 * (k * k - vh) / a0, (vh - vb * k / q + k * k) / a0],
                  [1.0, 2 * (k * k - 1) / a0, (1 - k / q + k * k) / a0], f, sr)
    k = np.tan(np.pi * 38.13547087602444 / sr)
    q = 0.5003270373238773
    a0 = 1.0 + k / q + k * k
    hp = freqz([1.0, -2.0, 1.0], [1.0, 2 * (k * k - 1) / a0, (1 - k / q + k * k) / a0], f, sr)
    return shelf * hp


def _block_ms(x, block, hop, sr):
    n = x.shape[-1]
    y = np.fft.irfft(np.fft.rfft(x, axis=-1) * _k_weight(np.fft.rfftfreq(n, 1.0 / sr), sr), n, axis=-1) ** 2
    cs = np.concatenate([np.zeros((y.shape[0], 1)), np.cumsum(y, axis=1)], axis=1)
    b, h = int(block * sr), int(hop * sr)
    st = np.arange(0, y.shape[1] - b + 1, h)
    return ((cs[:, st + b] - cs[:, st]) / b).sum(axis=0)


def lufs(x, sr=SR):
    """Integrated loudness with absolute (-70) and relative (-10 LU) gates."""
    z = _block_ms(stereo(x), 0.4, 0.1, sr)
    if len(z) == 0:
        return -70.0
    z = z[-0.691 + 10 * np.log10(z + 1e-20) > -70.0]
    if len(z) == 0:
        return -70.0
    rel = -0.691 + 10 * np.log10(z.mean()) - 10.0
    z = z[-0.691 + 10 * np.log10(z) > rel]
    return float(-0.691 + 10 * np.log10(z.mean()))


def short_term(x, sr=SR, hop=0.5):
    """Short-term (3 s) loudness curve, one value per `hop` seconds."""
    return -0.691 + 10 * np.log10(_block_ms(stereo(x), 3.0, hop, sr) + 1e-20)


def true_peak_db(x):
    return float(todb(peak_env(stereo(x)).max()))
