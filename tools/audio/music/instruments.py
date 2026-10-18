"""Synth voices. Every instrument is fn(notes, **params) -> stereo (2, n).

`notes` is a tuple of song.Note (t, d, p, v, slide) with times in seconds relative to the phrase
start. Poly instruments are called with one note at a time (the sequencer caches identical
calls); mono instruments get a whole phrase so they can glide and play legato.
"""
from __future__ import annotations

import numpy as np

from dsp import SR, adsr, fade_tail, lp24, mtof, pulse, rng, saw, sine, stereo, svf, unison


def _len(sec):
    return int(sec * SR) + 1


def _vel(v, amt=0.7):
    return 1.0 - amt + amt * v


def _key(f, amt=0.5):
    """Filter key tracking relative to middle C."""
    return (f / 261.63) ** amt


# ------------------------------------------------------------------ poly voices

def pad(notes, voices=7, detune=18.0, cutoff=1800.0, fenv=1.8, q=0.8, attack=0.35, decay=1.2,
        sustain=0.85, release=1.2, wave="saw", var=0):
    """Supersaw pad: detuned unison, filter opening with the attack."""
    nt = notes[0]
    f = float(mtof(nt.p))
    n = _len(nt.d + release)
    x = unison(wave, f, n, voices, detune, 0.95, rng("pad", nt.p, var))
    fe = adsr(n, nt.d, attack * 1.5, decay * 2, 0.45, release)
    y = svf(x, cutoff * _key(f) * fenv ** fe, q)
    return fade_tail(y * adsr(n, nt.d, attack, decay, sustain, release) * _vel(nt.v, 0.4))


def pluck(notes, wave="saw", voices=3, detune=10.0, cutoff=300.0, peak=6000.0, fdecay=0.12, res=0.2,
          decay=0.35, release=0.1, pw=0.5, var=0):
    """Plucked subtractive voice: fast 24 dB filter envelope, velocity -> brightness."""
    nt = notes[0]
    f = float(mtof(nt.p))
    n = _len(nt.d + release)
    kw = {"pw": pw} if wave == "pulse" else {}
    x = unison(wave, f, n, voices, detune, 0.7, rng("pluck", nt.p, var), **kw)
    t = np.arange(n) / SR
    fc = cutoff * _key(f, 0.3) * (peak / cutoff) ** (np.exp(-t / fdecay) * _vel(nt.v, 0.5))
    y = lp24(x, fc, res)
    return fade_tail(y * adsr(n, nt.d, 0.002, decay, 0.0, release) * _vel(nt.v))


def fm(notes, ratio=3.5, index=2.5, idecay=0.4, decay=1.6, sustain=0.0, release=0.4, detune=4.0,
       tine=0.0, var=0):
    """Two-operator FM (bells: ratio 3.5; e-piano: ratio 1 + tine), detuned L/R pair."""
    nt = notes[0]
    f = float(mtof(nt.p))
    n = _len(nt.d + release)
    t = np.arange(n) / SR
    idx = index * np.exp(-t / idecay) * _vel(nt.v, 0.6)
    out = np.empty((2, n))
    for ch, c in enumerate((-detune, detune)):
        fc = f * 2.0 ** (c / 1200.0)
        y = np.sin(2 * np.pi * fc * t + idx * np.sin(2 * np.pi * fc * ratio * t))
        if tine:
            y += tine * np.sin(2 * np.pi * fc * 7.0 * t) * np.exp(-t / 0.03)
        out[ch] = y
    return fade_tail(out * adsr(n, nt.d, 0.001, decay, sustain, release) * _vel(nt.v))


def brass(notes, voices=5, detune=12.0, cutoff=500.0, peak=4000.0, fattack=0.05, fdecay=0.3,
          sustain=0.65, release=0.15, var=0):
    """Synth brass: filter swells open then settles (the 'bwah')."""
    nt = notes[0]
    f = float(mtof(nt.p))
    n = _len(nt.d + release)
    x = unison("saw", f, n, voices, detune, 0.8, rng("brass", nt.p, var))
    fe = adsr(n, nt.d, fattack, fdecay, 0.35, release) * _vel(nt.v, 0.5)
    y = lp24(x, cutoff * _key(f, 0.4) * (peak / cutoff) ** fe, 0.1)
    return fade_tail(y * adsr(n, nt.d, 0.008, 0.25, sustain, release) * _vel(nt.v))


def chank(notes, cutoff=700.0, peak=5000.0, fdecay=0.045, decay=0.13, pw=0.3, var=0):
    """Funk 'guitar chank' / clav stab: thin pulse + saw, very fast filter snap."""
    nt = notes[0]
    f = float(mtof(nt.p))
    n = _len(nt.d + 0.06)
    t = np.arange(n) / SR
    r = rng("chank", nt.p, var)
    x = pulse(f, n, r.random(), pw=pw) + 0.5 * saw(f * 1.004, n, r.random())
    y = lp24(x, cutoff * (peak / cutoff) ** (np.exp(-t / fdecay) * _vel(nt.v, 0.4)), 0.25)
    y = stereo(y * adsr(n, nt.d, 0.001, decay, 0.0, 0.05) * _vel(nt.v))
    return fade_tail(y)


def stab(notes, voices=7, detune=22.0, cutoff=600.0, peak=7000.0, fdecay=0.12, decay=0.28,
         bend=0.0, var=0):
    """Rave/supersaw stab with optional pitch bend into the note (semitones)."""
    nt = notes[0]
    f = float(mtof(nt.p))
    n = _len(nt.d + 0.1)
    t = np.arange(n) / SR
    fr = f * 2.0 ** (bend * np.exp(-t / 0.025) / 12.0)
    x = unison("saw", fr, n, voices, detune, 1.0, rng("stab", nt.p, var))
    y = lp24(x, cutoff * (peak / cutoff) ** (np.exp(-t / fdecay) * _vel(nt.v, 0.4)), 0.15)
    return fade_tail(y * adsr(n, nt.d, 0.002, decay, 0.0, 0.08) * _vel(nt.v))


# ------------------------------------------------------------------ mono (phrase) voices

def _pitch(notes, n, glide):
    """Per-sample MIDI pitch; notes flagged slide glide in from the previous pitch."""
    p = np.full(n, float(notes[0].p))
    t = np.arange(n) / SR
    for k, nt in enumerate(notes):
        i0 = int(round(nt.t * SR))
        i1 = int(round(notes[k + 1].t * SR)) if k + 1 < len(notes) else n
        if nt.slide and k:
            start = p[i0 - 1] if i0 > 0 else notes[k - 1].p
            p[i0:i1] = nt.p + (start - nt.p) * np.exp(-(t[i0:i1] - nt.t) / glide)
        else:
            p[i0:i1] = nt.p
    return p


def _env(notes, n, a, d, s, r):
    """Mono amp envelope: retrigger from the current level, slides stay legato."""
    env = np.zeros(n)
    lvl = 0.0
    for k, nt in enumerate(notes):
        i0 = int(round(nt.t * SR))
        i1 = int(round(notes[k + 1].t * SR)) if k + 1 < len(notes) else n
        if i1 <= i0:
            continue
        tt = np.arange(i1 - i0) / SR

        def on(u, lvl=lvl, legato=nt.slide and k > 0):
            if legato:
                return s + (lvl - s) * np.exp(-u / 0.01)
            ramp = lvl + (1.0 - lvl) * np.minimum(u / a, 1.0)
            return np.where(u < a, ramp, s + (1.0 - s) * np.exp(-(u - a) / (d / 3.0)))

        lg = float(on(np.array([nt.d]))[0])
        e = np.exp(-4.6 * (tt - nt.d) / r)
        seg = np.where(tt < nt.d, on(tt), lg * np.clip((e - 0.01) / 0.99, 0.0, None))
        env[i0:i1] = seg * _vel(nt.v, 0.5)
        lvl = seg[-1]
    return env


def _fenv(notes, n, decay, accent=0.0):
    """Filter envelope retriggered by every non-slide note; accented notes (v > 0.9) hit harder."""
    e = np.zeros(n)
    t = np.arange(n) / SR
    for k, nt in enumerate(notes):
        if nt.slide and k:
            continue
        i0 = int(round(nt.t * SR))
        acc = 1.0 + accent * (nt.v > 0.9)
        e[i0:] = acc * np.exp(-(t[i0:] - nt.t) / (decay / acc))
    return e


def _phrase_len(notes, release):
    return _len(max(nt.t + nt.d for nt in notes) + release)


def bass(notes, wave="saw", cutoff=160.0, peak=1500.0, fdecay=0.1, res=0.25, sub=0.9, drive=1.6,
         glide=0.05, a=0.003, d=0.3, s=0.85, release=0.05, detune=7.0, var=0):
    """Mono synth bass: two detuned oscillators through a 24 dB envelope filter + drive, plus a
    clean sine sub at the fundamental."""
    n = _phrase_len(notes, release)
    f = mtof(_pitch(notes, n, glide))
    r = rng("bass", var)
    lo = 2.0 ** (-detune / 1200.0)
    if wave == "saw":
        x = saw(f * lo, n, r.random()) + saw(f / lo, n, r.random())
    else:
        x = pulse(f * lo, n, r.random(), pw=0.4) + saw(f / lo, n, r.random())
    y = lp24(0.5 * x, cutoff * (peak / cutoff) ** np.minimum(_fenv(notes, n, fdecay, 0.5), 1.5), res)
    y = np.tanh(drive * y) / np.tanh(drive)
    y = (0.55 * y + sub * sine(f, n)) * _env(notes, n, a, d, s, release)
    return stereo(fade_tail(y))


def acid(notes, wave="saw", cutoff=250.0, octaves=3.2, fdecay=0.2, res=0.8, glide=0.07, drive=2.2,
         release=0.03, var=0):
    """303-style line: resonant 24 dB filter, accents (v > 0.9) and slides."""
    n = _phrase_len(notes, release)
    f = mtof(_pitch(notes, n, glide))
    x = saw(f, n) if wave == "saw" else pulse(f, n, pw=0.5)
    fc = cutoff * 2.0 ** (octaves * np.minimum(_fenv(notes, n, fdecay, 0.6), 1.6))
    y = lp24(x, fc, res)
    y = np.tanh(drive * y) / np.tanh(drive)
    return stereo(fade_tail(y * _env(notes, n, 0.002, 0.2, 0.7, release)))


def reese(notes, cutoff=450.0, res=0.1, detune=16.0, sub=0.9, drive=2.0, wobble=0.0, rate=2.0,
          glide=0.04, release=0.08, var=0):
    """Reese bass: three detuned saws (slow phasing) through a low-pass with optional wobble LFO
    (rate in Hz, phrase-relative), plus sine sub."""
    n = _phrase_len(notes, release)
    f = mtof(_pitch(notes, n, glide))
    t = np.arange(n) / SR
    r = rng("reese", var)
    x = unison("saw", f, n, 3, detune, 0.6, r)
    fc = cutoff * 2.0 ** (wobble * 0.5 * (1 - np.cos(2 * np.pi * rate * t)))
    y = lp24(x, fc, res)
    y = np.tanh(drive * y) / np.tanh(drive)
    env = _env(notes, n, 0.005, 0.3, 0.9, release)
    return fade_tail((0.6 * y + sub * sine(f, n)) * env)


def lead(notes, voices=3, detune=10.0, wave="saw", cutoff=1600.0, fenv=2.2, fdecay=0.35, q=1.0,
         glide=0.06, vib=0.18, vib_rate=5.5, vib_delay=0.22, a=0.008, d=0.4, s=0.75, release=0.25,
         pw=0.5, var=0):
    """Mono lead: detuned unison with glide, delayed vibrato, filter envelope."""
    n = _phrase_len(notes, release)
    p = _pitch(notes, n, glide)
    t = np.arange(n) / SR
    since = np.zeros(n)
    for nt in notes:
        if not nt.slide:
            i0 = int(round(nt.t * SR))
            since[i0:] = t[i0:] - nt.t
    p = p + vib * np.clip((since - vib_delay) / 0.3, 0.0, 1.0) * np.sin(2 * np.pi * vib_rate * t)
    kw = {"pw": pw} if wave == "pulse" else {}
    x = unison(wave, mtof(p), n, voices, detune, 0.5, rng("lead", var), **kw)
    y = svf(x, cutoff * fenv ** np.minimum(_fenv(notes, n, fdecay), 1.0), q)
    return fade_tail(y * _env(notes, n, a, d, s, release))


VOWELS = {"a": (800, 1150, 2900), "e": (400, 1700, 2600), "i": (300, 2200, 3000),
          "o": (450, 800, 2830), "u": (325, 700, 2530)}


def talkbox(notes, vowels="ua", morph=0.1, glide=0.05, q=7.0, a=0.01, d=0.3, s=0.8, release=0.12,
            var=0):
    """Talk-box style lead: buzzy carrier through three band-pass formants that morph from the
    first to the second vowel on every attack ('wah'/'yeah')."""
    n = _phrase_len(notes, release)
    f = mtof(_pitch(notes, n, glide))
    t = np.arange(n) / SR
    r = rng("talk", var)
    x = saw(f, n, r.random()) + 0.6 * pulse(f * 1.003, n, r.random(), pw=0.3)
    mix = np.zeros(n)
    for nt in notes:
        if not nt.slide:
            i0 = int(round(nt.t * SR))
            mix[i0:] = 1.0 - np.exp(-(t[i0:] - nt.t) / morph)
    v0, v1 = VOWELS[vowels[0]], VOWELS[vowels[1]]
    y = sum(g * svf(x, a0 * (b0 / a0) ** mix, q, "bp") for a0, b0, g in zip(v0, v1, (1.0, 0.7, 0.35)))
    y = 0.8 * y + 0.1 * svf(x, 2500.0, 0.7)
    return stereo(fade_tail(y * _env(notes, n, a, d, s, release)))
