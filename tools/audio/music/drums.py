"""Synthesised drum kit and transition FX. fn(notes, **params) -> stereo (2, n); one hit per call
(notes[0]; p = MIDI pitch where it matters, d = length for risers / choke gate for open hats)."""
from __future__ import annotations

import numpy as np

from dsp import SR, convolve, eq, fade_tail, mtof, pulse, reverb_ir, rng, stereo, svf, unison


def _t(sec):
    return np.arange(int(sec * SR) + 1) / SR


def _eq(x, bands):
    """EQ a one-shot with zero padding so the circular FFT filter cannot wrap the tail."""
    n = x.shape[-1]
    pad = np.zeros(x.shape[:-1] + (n // 2 + 2048,))
    return eq(np.concatenate([x, pad], axis=-1), bands)[..., :n]


def _sweep(f):
    return np.sin(2 * np.pi * np.cumsum(f) / SR)


def _sat(x, drive):
    return np.tanh(drive * x) / np.tanh(drive)


def kick(notes, tune=48.0, punch=3.5, pdecay=0.032, decay=0.4, click=0.5, drive=2.2):
    """Sine body with an exponential pitch drop, click transient, tanh drive (harmonics so the kick
    still reads on small speakers)."""
    v = notes[0].v
    t = _t(decay * 1.6)
    r = rng("kick", tune)
    body = _sweep(tune * (1 + (punch - 1) * np.exp(-t / pdecay))) * np.exp(-t / (decay / 2.6))
    clk = _eq(r.standard_normal(len(t)), [("hp", 1800), ("lp", 9000)]) * np.exp(-t / 0.0018)
    clk += 0.6 * np.sin(2 * np.pi * 2600 * t) * np.exp(-t / 0.003)
    y = _sat(body + click * clk, drive) * np.minimum(1.0, t / 0.0006)
    y = _eq(y, [("hp", 28, 0.7), ("peak", 320, 1.4, -4.0)])
    return stereo(fade_tail(y * v))


def snare(notes, tone=190.0, decay=0.17, snap=0.7, body=0.8, gated=0.0, verb_mix=0.5):
    """Two-mode drum body + filtered noise; gated > 0 adds an 80s gated plate of that length."""
    v = notes[0].v
    t = _t(max(decay * 2.5, gated + 0.1))
    r = rng("snare", tone, decay)
    f1 = tone * (1 + 0.5 * np.exp(-t / 0.01))
    b = _sweep(f1) * np.exp(-t / 0.05) + 0.45 * _sweep(f1 * 1.63) * np.exp(-t / 0.03)
    nz = _eq(r.standard_normal(len(t)), [("hp", 1100), ("peak", 5000, 0.8, 4.0), ("lp", 13000)])
    sn = _eq(r.standard_normal(len(t)), [("hp", 3500)]) * np.exp(-t / 0.004)
    y = _sat(body * b + 0.55 * nz * np.exp(-t / (decay / 3)) + snap * sn, 1.4)
    y = stereo(y * np.minimum(1.0, t / 0.0005))
    if gated:
        ir = reverb_ir(rt=1.6, damp=6000.0, predelay=0.004, gate=gated, seed=3)
        n = y.shape[1]
        wet = convolve(np.concatenate([y, np.zeros((2, ir.shape[1]))], axis=1), ir)[:, :n]
        y = y + verb_mix * 2.0 * wet
    return fade_tail(_eq(y, [("hp", 120)]) * v)


def clap(notes, decay=0.2, spread=0.009, tone=1300.0):
    """Three short noise bursts then a tail, band-passed, decorrelated L/R."""
    v = notes[0].v
    t = _t(decay * 2.2 + 0.04)
    r = rng("clap", tone)
    env = sum(np.where(t >= k * spread, np.exp(-(t - k * spread) / 0.0035), 0.0) for k in range(3))
    env = env + np.where(t >= 3 * spread, np.exp(-(t - 3 * spread) / (decay / 3)), 0.0)
    nz = r.standard_normal((2, len(t)))
    y = _eq(nz, [("bp", tone, 0.9), ("hp", 600), ("high", 6000, 0.7, 3.0)]) * env
    return fade_tail(y / np.abs(y).max() * v)


HAT_F = (205.3, 304.4, 369.6, 522.7, 540.0, 800.0)


def hat(notes, decay=0.05, tone=1.0, open_=False):
    """808-style metallic hat: six detuned squares + noise, double high-pass. Open hats are choked
    by the note length (the sequencer shortens them at the next closed hat)."""
    nt = notes[0]
    dec = decay if not open_ else max(decay, 0.25)
    t = _t(min(dec * 2.5, nt.d + 0.03) if open_ else dec * 2.5)
    r = rng("hat", tone, open_)
    metal = sum(pulse(f * tone * 1.5, len(t), r.random()) for f in HAT_F) / 6.0
    x = 0.8 * metal + 0.5 * r.standard_normal(len(t))
    x = _eq(x, [("hp", 7000), ("hp", 7000), ("peak", 10500, 1.0, 3.0), ("lp", 17000)])
    env = np.exp(-t / (dec / 3)) * np.minimum(1.0, t / 0.0004)
    if open_:
        env *= np.clip((nt.d + 0.03 - t) / 0.03, 0.0, 1.0)
    return stereo(fade_tail(x * env / 0.35 * nt.v))


def tom(notes, decay=0.35, bend=0.7, drive=1.8):
    """Synth tom (Simmons flavour): sine with a deep pitch drop, noise skin."""
    nt = notes[0]
    f0 = float(mtof(nt.p))
    t = _t(decay * 1.8)
    r = rng("tom", nt.p)
    y = _sweep(f0 * (1 + bend * np.exp(-t / 0.06))) * np.exp(-t / (decay / 3))
    y += 0.25 * _eq(r.standard_normal(len(t)), [("bp", f0 * 4, 1.0)]) * np.exp(-t / 0.02)
    return stereo(fade_tail(_sat(y, drive) * nt.v))


def crash(notes, decay=1.8, tone=1.0):
    """Dense inharmonic metal (ring-modulated squares) + noise, high-passed, wide stereo."""
    nt = notes[0]
    t = _t(decay * 1.4)
    out = np.empty((2, len(t)))
    for ch in range(2):
        r = rng("crash", tone, ch)
        m = sum(pulse(f * tone * 2.3, len(t), r.random()) * pulse(f * tone * 3.7, len(t), r.random())
                for f in HAT_F) / 6.0
        out[ch] = 0.7 * m + 0.6 * r.standard_normal(len(t))
    out = _eq(out, [("hp", 4000), ("hp", 4000), ("peak", 8000, 0.7, 3.0), ("lp", 15000)])
    env = np.exp(-t / (decay / 4.0)) * (0.6 + 0.4 * np.exp(-t / 0.08)) * np.minimum(1.0, t / 0.001)
    return fade_tail(out * env / 0.5 * nt.v)


def ride(notes, decay=1.1):
    """Bell-ish ride ping over soft wash."""
    nt = notes[0]
    t = _t(decay * 1.3)
    r = rng("ride")
    ping = sum(np.sin(2 * np.pi * f * t + r.random() * 6.3) for f in (3150, 4260, 5130, 6970)) / 4
    wash = _eq(r.standard_normal(len(t)), [("hp", 6000), ("lp", 14000)])
    y = (0.6 * ping * np.exp(-t / 0.25) + 0.4 * wash * np.exp(-t / (decay / 4))) * np.minimum(1, t / 0.0005)
    return stereo(fade_tail(y * nt.v))


def shaker(notes, decay=0.06, tone=7000.0):
    nt = notes[0]
    t = _t(decay * 3 + 0.01)
    y = _eq(rng("shaker", tone).standard_normal(len(t)), [("bp", tone, 1.2), ("hp", 4000)])
    env = np.minimum(1.0, t / 0.008) * np.exp(-np.maximum(t - 0.008, 0) / (decay / 3))
    return stereo(fade_tail(y * env / 0.3 * nt.v))


def cowbell(notes, decay=0.12):
    nt = notes[0]
    t = _t(decay * 3)
    r = rng("cowbell")
    y = pulse(540.0, len(t), r.random()) + pulse(800.0, len(t), r.random())
    y = _eq(y, [("bp", 2600, 1.4), ("hp", 700)]) * (0.4 * np.exp(-t / 0.01) + 0.6 * np.exp(-t / decay))
    return stereo(fade_tail(y / np.abs(y).max() * nt.v))


def riser(notes, start=300.0, end=9000.0, pitch=0.0):
    """Noise sweep (time-varying band-pass) swelling over the note length; optional saw sweep
    (pitch = MIDI start note, rising an octave)."""
    nt = notes[0]
    t = _t(nt.d)
    u = t / nt.d
    r = rng("riser", nt.d, start, end)
    nz = r.standard_normal((2, len(t)))
    y = svf(nz, start * (end / start) ** u, 2.5, "bp")
    if pitch:
        y += 0.25 * unison("saw", float(mtof(pitch)) * 2.0 ** u, len(t), 5, 30.0, 1.0, r)
    env = u ** 2 * np.clip((nt.d - t) / 0.02, 0.0, 1.0)
    return fade_tail(y * env * nt.v)


def downlifter(notes, start=7000.0, end=250.0):
    nt = notes[0]
    t = _t(nt.d)
    u = t / nt.d
    y = svf(rng("down", nt.d).standard_normal((2, len(t))), start * (end / start) ** u, 2.0, "bp")
    return fade_tail(y * (1 - u) ** 2 * np.minimum(1.0, t / 0.01) * nt.v)


def impact(notes, decay=1.2):
    """Downbeat boom: sub sine drop + low noise burst."""
    nt = notes[0]
    t = _t(decay * 1.3)
    sub = _sweep(32 + 60 * np.exp(-t / 0.12)) * np.exp(-t / (decay / 3))
    nz = _eq(rng("impact").standard_normal(len(t)), [("lp", 900), ("hp", 60)]) * np.exp(-t / 0.08)
    return stereo(fade_tail(_sat(sub + 0.4 * nz, 1.5) * nt.v))


def revcrash(notes, decay=1.8):
    """Reversed crash swelling into the end of the note (place it so it ends on the downbeat)."""
    nt = notes[0]
    x = crash(notes, decay)[:, ::-1]
    n = int(nt.d * SR)
    x = x[:, -n:] if x.shape[1] >= n else np.pad(x, ((0, 0), (n - x.shape[1], 0)))
    return fade_tail(x * np.minimum(1.0, np.arange(n) / (0.05 * SR)), 0.003)
