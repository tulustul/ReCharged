"""ReCharged SFX synthesis kit — numpy only (no scipy).

Signals are float64 arrays at `SR`: mono = shape (n,), stereo = shape (n, 2).
Every function taking `circular=True` treats the buffer as one period of a loop, so the result
stays seamless when played looped (filters, convolution, events and modulation wrap around).
Static filters are FFT-domain magnitude responses: minimum-phase (causal, no pre-ringing) for
one-shots, zero-phase circular for loops.  `svf` is the time-domain filter for resonant sweeps.

    python3 tools/audio/sfx/synth.py     # self-check
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Callable

import numpy as np

SR = 48_000
TAU = 2 * np.pi


@dataclass
class Sound:
    fn: Callable  # fn(rng) -> signal; rng is seeded from the sound name (reproducible)
    cat: str      # loudness category (build.TARGETS)
    loop: bool
    folder: str   # "sfx" | "ambience"
    gain: float   # dB offset from the category target
    pitch: bool   # loop meant to be pitch-shifted by playback rate (adds a rate-test preview)


SOUNDS: dict[str, Sound] = {}


def sound(cat: str, loop=False, folder="sfx", gain=0.0, name=None, pitch=False):
    """Register a recipe under its function name (or `name`, for variants)."""
    def deco(fn):
        SOUNDS[name or fn.__name__] = Sound(fn, cat, loop, folder, gain, pitch)
        return fn
    return deco


def n_(sec: float) -> int:
    return int(round(sec * SR))


def t_(n: int) -> np.ndarray:
    return np.arange(n) / SR


def _arr(v, n: int) -> np.ndarray:
    return np.broadcast_to(np.asarray(v, dtype=float), (n,))


def grid(freq: float, n: int) -> float:
    """Round `freq` to a whole number of cycles per `n` samples (loop-safe frequency)."""
    return max(1, round(freq * n / SR)) * SR / n


def db(x: float) -> float:
    return 20 * np.log10(max(float(x), 1e-12))


def amp(d: float) -> float:
    return 10 ** (d / 20)


# --------------------------------------------------------------------------- envelopes
def line(points, n: int) -> np.ndarray:
    """Piecewise-linear envelope through (time_s, value) points; holds the end values."""
    ts, vs = zip(*points)
    return np.interp(t_(n), ts, vs)


def expo(points, n: int) -> np.ndarray:
    """Piecewise-exponential envelope (values > 0) for pitch and cutoff glides."""
    ts, vs = zip(*points)
    return np.exp(np.interp(t_(n), ts, np.log(vs)))


def perc(n: int, attack=0.002, decay=0.1, hold=0.0) -> np.ndarray:
    """Linear attack, optional hold, then exponential decay with time constant `decay` (s)."""
    t = t_(n)
    e = np.exp(-np.maximum(t - attack - hold, 0) / decay)
    return np.where(t < attack, t / max(attack, 1e-9), e)


def adsr(n: int, a: float, d: float, s: float, r: float, gate: float) -> np.ndarray:
    """ADSR envelope; the note is held for `gate` seconds, then released (time constant r)."""
    def held(t):
        return np.where(t < a, t / max(a, 1e-9), s + (1 - s) * np.exp(-np.maximum(t - a, 0) / max(d, 1e-9)))
    t = t_(n)
    return np.where(t < gate, held(t), held(np.float64(gate)) * np.exp(-(t - gate) / max(r, 1e-9)))


def fades(x: np.ndarray, fin=0.0005, fout=0.005) -> np.ndarray:
    x = np.array(x, dtype=float)
    a, b = min(n_(fin), len(x)), min(n_(fout), len(x))
    if a:
        x[:a] = (x[:a].T * np.linspace(0, 1, a)).T
    if b:
        x[-b:] = (x[-b:].T * np.linspace(1, 0, b) ** 2).T
    return x


def smooth_rand(n: int, rate: float, rng, circular=False) -> np.ndarray:
    """Smooth random control signal in [-1, 1] (cosine-interpolated points `rate` per second)."""
    m = max(2, int(round(n / SR * rate)))
    v = rng.uniform(-1, 1, m + (0 if circular else 1))
    pos = np.arange(n) * m / n
    i = pos.astype(int)
    f = (1 - np.cos(np.pi * (pos - i))) / 2
    j = (i + 1) % m if circular else i + 1
    return v[i % len(v)] * (1 - f) + v[j] * f


# --------------------------------------------------------------------------- oscillators
def phase(freq, n: int, ph0=0.0) -> np.ndarray:
    """Running phase in cycles (starts at ph0); `freq` in Hz, scalar or per sample."""
    f = _arr(freq, n)
    return ph0 + np.concatenate(([0.0], np.cumsum(f[:-1]))) / SR


def sine(freq, n: int, ph0=0.0) -> np.ndarray:
    return np.sin(TAU * phase(freq, n, ph0))


def _blep(p, dt):
    out = np.zeros_like(p)
    a = p < dt
    x = p[a] / dt[a]
    out[a] = 2 * x - x * x - 1
    b = p > 1 - dt
    x = (p[b] - 1) / dt[b]
    out[b] = x * x + 2 * x + 1
    return out


def saw(freq, n: int, ph0=0.0) -> np.ndarray:
    """Band-limited (polyBLEP) sawtooth."""
    dt = np.maximum(np.abs(_arr(freq, n)) / SR, 1e-9)
    p = phase(freq, n, ph0) % 1.0
    return 2 * p - 1 - _blep(p, dt)


def pulse(freq, n: int, duty=0.5, ph0=0.0) -> np.ndarray:
    """Band-limited (polyBLEP) pulse/square; `duty` scalar or per sample."""
    dt = np.maximum(_arr(freq, n) / SR, 1e-9)
    p = phase(freq, n, ph0) % 1.0
    d = _arr(duty, n)
    return np.where(p < d, 1.0, -1.0) + _blep(p, dt) - _blep((p - d) % 1.0, dt)


def harmonics(f0, n: int, amps, fmax=0.42 * SR, phases=None) -> np.ndarray:
    """Band-limited harmonic stack sum_k amps[k-1]·sin(2πkφ); partials fade out near `fmax`."""
    ph = phase(f0, n)
    f0 = _arr(f0, n)
    out = np.zeros(n)
    for k, a in enumerate(amps, 1):
        fk = k * f0
        if fk.min() >= fmax:
            break
        if a:
            g = np.clip((fmax - fk) / (0.15 * fmax), 0, 1)
            out += a * g * np.sin(TAU * k * ph + (0 if phases is None else phases[k - 1]))
    return out


def fm(fc, n: int, ratio=1.0, index=1.0, fmod=None) -> np.ndarray:
    """2-operator phase modulation; modulator at ratio·fc (or absolute `fmod` Hz)."""
    fc = _arr(fc, n)
    fmo = fc * ratio if fmod is None else _arr(fmod, n)
    return np.sin(TAU * phase(fc, n) + _arr(index, n) * np.sin(TAU * phase(fmo, n)))


def modes(n: int, freqs, amps, decays) -> np.ndarray:
    """Bank of exponentially decaying sinusoids (modal resonator impulse response)."""
    t = t_(n)
    out = np.zeros(n)
    for f, a, d in zip(freqs, amps, decays):
        if f < 0.45 * SR:
            out += a * np.exp(-t / d) * np.sin(TAU * f * t)
    return out


# --------------------------------------------------------------------------- noise & events
def noise(n: int, rng, slope=0.0) -> np.ndarray:
    """Unit-RMS Gaussian noise tilted by `slope` dB/octave (0 white, -3 pink, -6 brown); periodic in n."""
    x = rng.standard_normal(n)
    if slope:
        X = np.fft.rfft(x)
        f = np.maximum(np.fft.rfftfreq(n, 1 / SR), 20.0)
        X *= (f / 1000.0) ** (slope / 6.0206)
        X[0] = 0
        x = np.fft.irfft(X, n)
    return x / (np.std(x) + 1e-12)


def dust(n: int, rng, rate, spread_db=6.0) -> np.ndarray:
    """Poisson impulse train (`rate` per second, scalar or per sample) with random sign and level."""
    r = _arr(rate, n)
    rmax = max(float(r.max()), 1e-9)
    pos = rng.integers(0, n, rng.poisson(rmax * n / SR))
    pos = pos[rng.uniform(0, rmax, len(pos)) < r[pos]]
    out = np.zeros(n)
    np.add.at(out, pos, rng.choice([-1.0, 1.0], len(pos)) * amp(1) ** rng.normal(0, spread_db, len(pos)))
    return out


def place(buf: np.ndarray, ev: np.ndarray, start: int, circular=False, gain=1.0) -> np.ndarray:
    """Add `ev` into `buf` (in place) at sample `start`; wraps around when circular."""
    idx = start + np.arange(len(ev))
    if circular:
        np.add.at(buf, idx % len(buf), gain * ev)
    else:
        ok = (idx >= 0) & (idx < len(buf))
        buf[idx[ok]] += gain * ev[ok]
    return buf


def mix(*xs) -> np.ndarray:
    """Sum signals of different lengths (zero-padded); any stereo input makes the result stereo."""
    stereo = any(x.ndim == 2 for x in xs)
    out = np.zeros((max(len(x) for x in xs), 2) if stereo else max(len(x) for x in xs))
    for x in xs:
        out[: len(x)] += x[:, None] if stereo and x.ndim == 1 else x
    return out


def pad(x: np.ndarray, sec: float) -> np.ndarray:
    return np.concatenate([x, np.zeros((n_(sec),) + x.shape[1:])])


# --------------------------------------------------------------------------- filters
def lp(f, fc, order=2):
    return 1 / np.sqrt(1 + (f / fc) ** (2 * order))


def hp(f, fc, order=2):
    return 1 / np.sqrt(1 + (fc / np.maximum(f, 1e-6)) ** (2 * order))


def bp(f, fc, q=1.0):
    x = q * (f / fc - fc / np.maximum(f, 1e-6))
    return 1 / np.sqrt(1 + x * x)


def bell(f, fc, q, gain_db):
    return 1 + (amp(gain_db) - 1) * bp(f, fc, q) ** 2


def _nfft(m: int) -> int:
    return 1 << int(np.ceil(np.log2(max(m, 2))))


def _minphase_fir(mag, taps: int) -> np.ndarray:
    """Minimum-phase FIR (`taps` long) with magnitude `mag(f)`. The cepstrum is taken on an 8×
    longer FFT so it cannot alias into late echoes; -100 dB floor for exact zeros (e.g. DC)."""
    m = _nfft(8 * taps)
    H = mag(np.fft.rfftfreq(m, 1 / SR))
    c = np.fft.irfft(np.log(np.maximum(H, 1e-5 * H.max())), m)
    w = np.zeros(m)
    w[0] = w[m // 2] = 1
    w[1:m // 2] = 2
    return np.fft.irfft(np.exp(np.fft.rfft(c * w, m)), m)[:taps]


def eq(x: np.ndarray, mag, circular=False) -> np.ndarray:
    """Filter by magnitude response `mag(freqs_hz)`: circular zero-phase for loops, causal
    minimum-phase (no pre-ringing; impulse response ≤ 0.5 s, output keeps len(x)) for one-shots."""
    if x.ndim == 2:
        return np.stack([eq(x[:, c], mag, circular) for c in range(x.shape[1])], 1)
    n = len(x)
    if circular:
        return np.fft.irfft(np.fft.rfft(x) * mag(np.fft.rfftfreq(n, 1 / SR)), n)
    return convolve(x, _minphase_fir(mag, min(n, n_(0.5))))[:n]


def svf(x: np.ndarray, fc, q=0.707, mode="lp") -> np.ndarray:
    """Time-varying TPT state-variable filter (fc, q scalar or per sample).
    Modes: lp, hp, bp (peak gain q), bpn (unity peak), notch."""
    n = len(x)
    g = np.tan(np.pi * np.clip(_arr(fc, n), 5.0, 0.45 * SR) / SR)
    k = 1.0 / _arr(q, n)
    a1 = 1 / (1 + g * (g + k))
    a2 = g * a1
    a3 = g * a2
    lo, ba = np.empty(n), np.empty(n)
    s1 = s2 = 0.0
    for i, (xi, b1, b2, b3) in enumerate(zip(x.tolist(), a1.tolist(), a2.tolist(), a3.tolist())):
        v3 = xi - s2
        v1 = b1 * s1 + b2 * v3
        v2 = s2 + b2 * s1 + b3 * v3
        s1, s2 = 2 * v1 - s1, 2 * v2 - s2
        lo[i], ba[i] = v2, v1
    if mode == "lp":
        return lo
    if mode == "bp":
        return ba
    if mode == "bpn":
        return ba * k
    if mode == "hp":
        return x - k * ba - lo
    return x - k * ba  # notch


def stft_eq(x: np.ndarray, gain, frame=1024, circular=False) -> np.ndarray:
    """Time-varying zero-phase EQ: gain(t_s[:, None], f_hz[None, :]) -> (frames, bins) magnitudes."""
    if x.ndim == 2:
        return np.stack([stft_eq(x[:, c], gain, frame, circular) for c in range(x.shape[1])], 1)
    n, hop = len(x), frame // 4
    win = np.hanning(frame + 1)[:-1]
    src = x if circular else np.concatenate([np.zeros(frame), x, np.zeros(frame)])
    m = len(src)
    starts = np.arange(0, m if circular else m - frame + 1, hop)
    idx = (starts[:, None] + np.arange(frame)) % m
    centre = ((starts + frame / 2) - (0 if circular else frame)) / SR
    G = gain(centre[:, None], np.fft.rfftfreq(frame, 1 / SR)[None, :])
    y = np.fft.irfft(np.fft.rfft(src[idx] * win) * G, frame) * win
    out, norm = np.zeros(m), np.zeros(m)
    np.add.at(out, idx, y)
    np.add.at(norm, idx, np.broadcast_to(win * win, idx.shape))
    out /= np.maximum(norm, 1e-9)
    return out if circular else out[frame : frame + n]


# --------------------------------------------------------------------------- layered-hit building blocks
def norm(x: np.ndarray) -> np.ndarray:
    """Peak-normalise to ±1."""
    return x / (np.max(np.abs(x)) + 1e-12)


def rms1(x: np.ndarray) -> np.ndarray:
    """Normalise to unit RMS (for balancing layers by energy)."""
    return x / (np.sqrt(np.mean(x * x)) + 1e-12)


def band(lo, hi, order=2):
    """Band-pass magnitude response for eq/stft_eq: high-pass `lo` × low-pass `hi`."""
    return lambda f: hp(f, lo, order) * lp(f, hi, order)


def burst(n: int, rng, decay, lo, hi, attack=0.0003) -> np.ndarray:
    """Band-limited noise crack with exponential decay (time constant `decay`)."""
    return eq(noise(n, rng) * perc(n, attack, decay), band(lo, hi))


def boom(n: int, f0, f1, glide, decay, attack=0.002) -> np.ndarray:
    """Sine thump with an exponential pitch drop f0→f1 over `glide` seconds."""
    return sine(expo([(0, f0), (glide, f1)], n), n) * perc(n, attack, decay)


def crackle(n: int, rng, rate, lo, hi, circular=False) -> np.ndarray:
    """Electric crackle / rattle: Poisson clicks (rate scalar or per sample) through a band-pass."""
    return eq(dust(n, rng, rate), band(lo, hi), circular)


def ring(n: int, rng, freqs, amps, decays, exciter=None) -> np.ndarray:
    """Modal resonator (metal/plastic body) struck by `exciter` (default: a 3 ms noise tap)."""
    if exciter is None:
        k = n_(0.003)
        exciter = noise(k, rng) * np.hanning(k)
    return convolve(exciter, modes(n, freqs, amps, decays))[:n]


# --------------------------------------------------------------------------- shaping, space
def drive(x: np.ndarray, k: float) -> np.ndarray:
    """tanh saturation normalised so that ±1 stays ±1."""
    return np.tanh(k * x) / np.tanh(k)


def pan(x: np.ndarray, p) -> np.ndarray:
    """Constant-power pan of a mono signal; p in [-1, 1] (scalar or per sample)."""
    a = (np.asarray(p, dtype=float) + 1) * np.pi / 4
    return np.stack([x * np.cos(a), x * np.sin(a)], 1)


def convolve(a: np.ndarray, b: np.ndarray, circular=False) -> np.ndarray:
    """Linear (or circular, period len(a)) convolution of a mono signal with mono b."""
    if circular:
        n = len(a)
        b = np.pad(b, (0, -len(b) % n)).reshape(-1, n).sum(0)  # fold a long IR onto the loop
        return np.fft.irfft(np.fft.rfft(a) * np.fft.rfft(b), n)
    n = len(a) + len(b) - 1
    m = _nfft(n)
    return np.fft.irfft(np.fft.rfft(a, m) * np.fft.rfft(b, m), m)[:n]


def reverb_ir(t60: float, rng, predelay=0.0, damp=1.0, dur=None) -> np.ndarray:
    """Synthetic room response: decaying noise whose highs die (1 + damp·f/4 kHz) times faster;
    unit energy."""
    x = noise(n_(dur or min(1.3 * t60, 8.0)), rng)
    y = stft_eq(x, lambda T, F: 10 ** (-3 * T * (1 + damp * F / 4000.0) / t60), frame=512)
    y = fades(y, 0.003, 0.05)
    y = np.concatenate([np.zeros(n_(predelay)), y])
    return y / np.sqrt(np.sum(y * y))


def reverb(x: np.ndarray, rng, t60=1.0, wet=0.2, predelay=0.01, damp=1.0, stereo=False,
           circular=False) -> np.ndarray:
    """Dry + convolution reverb. Mono in → mono or (stereo=True) decorrelated stereo out.
    One-shots grow by the tail length; loops wrap the tail (circular)."""
    chans = x.T if x.ndim == 2 else [x] * (2 if stereo else 1)
    out = []
    for c in chans:
        w = convolve(c, reverb_ir(t60, rng, predelay, damp), circular)
        out.append(mix(c, wet * w))
    return np.stack(out, 1) if len(out) == 2 else out[0]


def hermite_rate(x: np.ndarray, rate: float, n_out: int) -> np.ndarray:
    """Play a loop at `rate` exactly like kira (4-point 3rd-order Hermite, no anti-aliasing)."""
    pos = np.arange(n_out) * rate
    i = pos.astype(np.int64)
    fr = pos - i
    m = len(x)
    p, c, n1, n2 = (x[(i + o) % m] for o in (-1, 0, 1, 2))
    c1 = (n1 - p) * 0.5
    c2 = p - c * 2.5 + n1 * 2.0 - n2 * 0.5
    c3 = (n2 - p) * 0.5 + (c - n1) * 1.5
    return ((c3 * fr + c2) * fr + c1) * fr + c


# --------------------------------------------------------------------------- self-check
if __name__ == "__main__":
    rng = np.random.default_rng(1)
    f = np.array([100.0, 1000.0, 10000.0])
    assert abs(lp(f, 1000)[1] - 0.7071) < 1e-3 and abs(hp(f, 1000)[1] - 0.7071) < 1e-3
    assert abs(bp(f, 1000, 4)[1] - 1) < 1e-9 and bp(f, 1000, 4)[0] < 0.1
    n = n_(1.0)
    tone = sine(grid(997.3, n), n)
    assert abs(tone[0] - sine(grid(997.3, n), n + 1)[n]) < 1e-9, "grid frequency loops seamlessly"
    x = noise(n, rng, -3)
    y = eq(x, lambda f: lp(f, 2000, 4), circular=True)
    seam = abs(y[0] - y[-1])
    assert seam < 4 * np.percentile(np.abs(np.diff(y)), 99), "circular eq is seamless"
    z = stft_eq(x, lambda T, F: np.ones_like(T * F), circular=True)
    assert np.max(np.abs(z - x)) < 1e-9, "unity stft_eq is transparent"
    w = stft_eq(x, lambda T, F: np.ones_like(T * F))
    assert np.max(np.abs(w - x)) < 1e-9
    imp = np.zeros(4096)
    imp[100] = 1
    h = eq(imp, lambda f: lp(f, 1000, 4))
    assert np.max(np.abs(h[:99])) < 1e-3, "minimum-phase eq does not pre-ring"
    s = svf(sine(1000, n), 1000, q=5, mode="bp")
    assert abs(np.max(np.abs(s[n // 2:])) - 5) < 0.2, "svf band-pass peak gain = q"
    r = hermite_rate(tone, 1.0, n)
    assert np.max(np.abs(r - tone)) < 1e-12
    print("synth self-check ok")
