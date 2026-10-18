"""Race-event cues and the menu UI set.

Race cues are bold synth sounds (beeps, FM bells, synth brass); the UI cues form one family of
short glassy electric blips in A major (sine body + a fast-fading inharmonic FM shimmer).
Every pitch is equal-tempered (A4 = 440 Hz); attacks are 2–8 ms, decays exponential. Each voice
is ~4 decay constants long and ends in a raised-cosine fade, so nothing is cut off with a click.
"""
import numpy as np

from synth import (SR, TAU, adsr, bell, bp, drive, eq, expo, fm, harmonics, hp, line, lp, n_, noise,
                   phase, place, pulse, reverb, saw, sine, smooth_rand, sound, svf, t_)


def _hz(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


def _end(n, frac=0.3):
    """Raised-cosine fade over the last `frac` of a buffer (ends exactly at 0)."""
    w = np.ones(n)
    k = max(1, int(n * frac))
    w[-k:] = 0.5 * (1 + np.cos(np.pi * np.arange(1, k + 1) / k))
    return w


def _len(decay, attack=0.005):
    return n_(attack + 4.2 * decay)


def _perc(n, attack, decay):
    t = t_(n)
    return np.where(t < attack, t / attack, np.exp(-(t - attack) / decay)) * _end(n)


def _chime(f, decay, bright=1.5):
    """FM bell: harmonic 1:1 FM whose brightness fades fast + a faint 2.76× strike partial."""
    n = _len(decay)
    t = t_(n)
    body = fm(f, n, ratio=1.0, index=bright * np.exp(-t / (0.25 * decay)))
    strike = 0.15 * sine(2.76 * f, n) * np.exp(-t / (0.15 * decay))
    return (body + strike) * _perc(n, 0.003, decay)


def _marimba(f, decay):
    """Sine bar + a 4th-harmonic mallet 'tock' that dies in a few ms."""
    n = _len(decay)
    return sine(f, n) * _perc(n, 0.002, decay) + 0.4 * sine(4 * f, n) * _perc(n, 0.001, 0.12 * decay)


def _glass(f, decay, shimmer=0.3):
    """UI voice: sine + soft 2nd harmonic + fast-fading inharmonic FM shimmer (3f carrier, 1.5f mod)."""
    n = _len(decay)
    t = t_(n)
    tone = sine(f, n) + 0.12 * sine(2 * f, n)
    tone += shimmer * fm(3 * f, n, ratio=0.5, index=1.0) * np.exp(-t / (0.3 * decay))
    return tone * _perc(n, 0.003, decay)


def _held(n, a, d, s, r, gate):
    return adsr(n, a, d, s, r, gate) * _end(n)


def _brass(rng, notes, gate, rel, cut=(500, 4500, 1800)):
    """Synth brass: 3 detuned (±7 cent) saws per note through a low-pass filter envelope."""
    dur = gate + 4 * rel
    n = n_(dur)
    x = sum(saw(f * 2 ** (c / 1200), n, ph0=rng.uniform()) for f in notes for c in (-7, 0, 7))
    fc = expo([(0, cut[0]), (0.04, cut[1]), (0.35, cut[2]), (max(dur, 0.36), 0.8 * cut[2])], n)
    return svf(x, fc, q=0.9) * _held(n, 0.008, 0.25, 0.75, rel, gate)


def _sweep(rng, dur, f0, f1, q=2.0):
    """Band-passed noise whose centre glides from f0 to f1."""
    n = n_(dur)
    return svf(noise(n, rng), expo([(0, f0), (dur, f1)], n), q, "bpn")


def _sparkle(rng, dur, count, notes, t0, t1, decay=0.03):
    """Short in-key sine pings scattered between t0 and t1 (glitter)."""
    out = np.zeros(n_(dur))
    m = _len(decay)
    for _ in range(count):
        ping = sine(_hz(rng.choice(notes)), m) * _perc(m, 0.001, decay) * rng.uniform(0.3, 1.0)
        place(out, ping, n_(rng.uniform(t0, t1)))
    return out


# --------------------------------------------------------------------------- race
@sound("race")
def countdown_beep(rng):
    """D5 synth beep: sine + soft odd harmonics (gentle square), 5 ms attack, 0.2 s body, short room."""
    n = n_(0.2 + 4 * 0.035)
    tone = harmonics(_hz(74), n, [1, 0, 0.2, 0, 0.08, 0, 0.035])
    return reverb(tone * _held(n, 0.005, 0.15, 0.85, 0.035, 0.2), rng, t60=0.35, wet=0.1)


@sound("race", gain=1.0)
def countdown_go(rng):
    """GO: D6 + D5 body, brighter harmonics, ±6 cent chorus, 0.6 s hold, upward noise swoosh."""
    n = n_(0.6 + 4 * 0.08)
    f = _hz(86)
    amps = [1, 0.35, 0.28, 0.14, 0.12, 0.06, 0.05, 0.03]
    tone = sum(harmonics(f * 2 ** (c / 1200), n, amps) for c in (-6, 0, 6)) / 3 + 0.3 * sine(f / 2, n)
    swoosh = _sweep(rng, n / SR, 800, 7000) * line([(0, 0), (0.05, 1), (0.35, 0.3), (0.6, 0)], n)
    return reverb(tone * _held(n, 0.005, 0.3, 0.8, 0.08, 0.6) + 0.25 * swoosh, rng, t60=0.5, wet=0.15)


@sound("race")
def lap_chime(rng):
    """G5-B5-D6 FM-bell arpeggio, 75 ms apart, the top note rings longest."""
    out = np.zeros(n_(0.15) + _len(0.4))
    for i, (m, decay) in enumerate(((79, 0.25), (83, 0.25), (86, 0.4))):
        place(out, _chime(_hz(m), decay), n_(0.075 * i))
    return reverb(out, rng, t60=0.6, wet=0.15)


@sound("race")
def final_lap_fanfare(rng):
    """Synth-brass fanfare in C: staccato G3+G4 triplet pickup, then a held C major chord (C3-C5)
    with a filter swell over a soft drum hit."""
    out = np.zeros(n_(0.34 + 1.35 + 4 * 0.25))
    for i in range(3):
        place(out, _brass(rng, (_hz(55), _hz(67)), 0.085, 0.06, cut=(600, 3500, 1500)), n_(0.11 * i))
    place(out, _brass(rng, [_hz(m) for m in (48, 60, 64, 67, 72)], 1.35, 0.25, cut=(500, 5000, 2000)), n_(0.34))
    n = _len(0.12)
    thump = sine(expo([(0, 140), (0.08, 55), (0.5, 45)], n), n) * _perc(n, 0.002, 0.12)
    snap = eq(noise(n, rng), lambda f: lp(f, 3500, 2) * hp(f, 200, 1)) * _perc(n, 0.001, 0.04)
    place(out, 2.0 * (thump + 0.35 * snap), n_(0.34))
    return reverb(out, rng, t60=0.8, wet=0.15)


@sound("race")
def finish_horn(rng):
    """Two-tone air horn F4+A4 (band-limited pulses) with a 2-semitone scoop up, saturated, then
    horn-bell EQ (1 kHz bump, 2 kHz low-pass, 160 Hz high-pass trims the difference-tone rumble)."""
    n = n_(1.15 + 4 * 0.1)
    scoop = expo([(0, 0.891), (0.07, 1.0), (1.5, 1.0)], n)
    wobble = 1 + 0.003 * smooth_rand(n, 6.0, rng)
    x = drive(0.6 * (pulse(_hz(65) * scoop * wobble, n, 0.32) + pulse(_hz(69) * scoop * wobble, n, 0.38)), 1.5)
    x = eq(x, lambda f: lp(f, 2000, 2) * bell(f, 1000, 1.2, 4) * hp(f, 160, 4))
    breath = 0.03 * eq(noise(n, rng), lambda f: bp(f, 1800, 1.5))
    return reverb((x + breath) * _held(n, 0.02, 0.3, 0.9, 0.1, 1.15), rng, t60=0.9, wet=0.18)


@sound("race")
def wrong_way_buzzer(rng):
    """'Bzzt-bzzt': band-limited squares at 110 + 117 Hz (7 Hz beating) + saw buzz, low-passed,
    saturated, two 0.27 s bursts."""
    n = n_(0.75)
    x = pulse(110, n) + pulse(117, n) + 0.5 * saw(110.5, n)
    x = drive(0.5 * eq(x, lambda f: lp(f, 3000, 2) * hp(f, 60, 2)), 2.0)
    gate = line([(0, 0), (0.005, 1), (0.27, 1), (0.295, 0), (0.36, 0), (0.365, 1), (0.625, 1), (0.65, 0)], n)
    return x * gate


@sound("race")
def respawn_materialise(rng):
    """Hologram: C5-G5-C6-E6-G6-C7 sine arpeggio under an accelerating tremolo, rising band-passed
    noise sweep, in-key sparkle pings, settling C6+G6 pad."""
    n = n_(1.6)
    out = np.zeros(n)
    m = _len(0.18)
    for i, note in enumerate((72, 79, 84, 88, 91, 96)):
        f = _hz(note)
        place(out, (sine(f, m) + 0.2 * sine(2 * f, m)) * _perc(m, 0.004, 0.18), n_(0.07 * i))
    out *= 1 - 0.35 * (0.5 + 0.5 * np.sin(TAU * phase(line([(0, 12), (1.0, 28)], n), n)))
    sweep = _sweep(rng, 1.6, 400, 9000) * line([(0, 0), (0.1, 0.5), (0.8, 1), (1.1, 0)], n)
    glitter = _sparkle(rng, 1.6, 30, (96, 98, 100, 103, 105, 108), 0.2, 1.0)
    p = n_(0.45 + 4 * 0.15)
    padv = (sine(_hz(84), p) + 0.7 * sine(_hz(91), p)) * _held(p, 0.15, 0.2, 0.6, 0.15, 0.45)
    place(out, 0.5 * padv, n_(0.45))
    return reverb(out + 0.3 * sweep + 0.25 * glitter, rng, t60=0.7, wet=0.2)


@sound("race")
def golden_battery_jingle(rng):
    """C6-E6-G6-C7 marimba run, then a ringing C major bell chord (C5-E5-G5-C6 + C7) with
    pentatonic sparkle."""
    out = np.zeros(n_(0.4) + _len(0.45))
    for i, m in enumerate((84, 88, 91, 96)):
        place(out, _marimba(_hz(m), 0.18), n_(0.09 * i))
    for m in (72, 76, 79, 84):
        place(out, 0.6 * _chime(_hz(m), 0.45), n_(0.4))
    place(out, 0.35 * _chime(_hz(96), 0.25), n_(0.4))
    out += 0.2 * _sparkle(rng, len(out) / SR, 24, (96, 98, 100, 103, 105, 108), 0.4, 1.4)
    return reverb(out, rng, t60=0.9, wet=0.18)


@sound("race", gain=-2.0)
def charge_ready(rng):
    """Boost-ready: E6→B6 hollow 1:2 FM chime, each note zipping up from a semitone below, with a
    +8 cent chorus copy of B6 for shimmer."""
    out = np.zeros(n_(0.085) + _len(0.22))
    for i, (note, decay) in enumerate(((88, 0.12), (95, 0.22))):
        m = _len(decay)
        t = t_(m)
        zipup = expo([(0, 0.944), (0.015, 1.0), (1.0, 1.0)], m)
        idx = 1.2 * np.exp(-t / 0.06)
        v = fm(_hz(note) * zipup, m, ratio=2.0, index=idx)
        if i:
            v += 0.4 * fm(_hz(note) * 2 ** (8 / 1200) * zipup, m, ratio=2.0, index=idx)
        place(out, v * _perc(m, 0.003, decay), n_(0.085 * i))
    return reverb(out, rng, t60=0.5, wet=0.15)


# --------------------------------------------------------------------------- UI (A major)
@sound("ui", gain=-4.0)
def ui_hover(rng):
    """Soft G#6 glass blip, 3 ms attack, ~40 ms decay."""
    return _glass(_hz(92), 0.014, shimmer=0.15)


@sound("ui")
def ui_select(rng):
    """Confirm: A5 then E6 glass blips 50 ms apart (rising fifth)."""
    out = np.zeros(n_(0.05) + _len(0.045))
    place(out, _glass(_hz(81), 0.03), 0)
    place(out, _glass(_hz(88), 0.045), n_(0.05))
    return out


@sound("ui", gain=-2.0)
def ui_back(rng):
    """Back: C#6 then F#5 glass blips 45 ms apart (falling fifth), less shimmer."""
    out = np.zeros(n_(0.045) + _len(0.04))
    place(out, _glass(_hz(85), 0.028, shimmer=0.15), 0)
    place(out, _glass(_hz(78), 0.04, shimmer=0.15), n_(0.045))
    return out


def _click(rng, fc):
    n = n_(0.012)
    return eq(noise(n, rng), lambda f: bp(f, fc, 1.5)) * _perc(n, 0.0003, 0.0018)


@sound("ui")
def ui_toggle(rng):
    """Two-click switch: band-passed noise click + E6 glass tick, softer second click 35 ms later."""
    out = np.zeros(n_(0.035) + _len(0.018))
    place(out, 0.5 * _click(rng, 3000), 0)
    place(out, 0.8 * _glass(_hz(88), 0.018, shimmer=0.1), 0)
    place(out, 0.3 * _click(rng, 2200), n_(0.035))
    return out


@sound("ui", gain=-6.0)
def ui_slider_tick(rng):
    """Tiny tick: E7 sine blip (8 ms decay) + a narrow noise click."""
    n = _len(0.008)
    out = sine(_hz(100), n) * _perc(n, 0.001, 0.008)
    return place(out, 0.4 * _click(rng, 4000), 0)


@sound("ui")
def ui_error(rng):
    """Low double 'bonk': square-ish A3 + A#3 (minor-second rub) with a small pitch drop, low-passed."""
    m = _len(0.05)
    out = np.zeros(n_(0.12) + m)
    drop = expo([(0, 1.03), (0.05, 1.0), (1.0, 1.0)], m)
    sq = [1, 0, 0.3, 0, 0.15, 0, 0.08, 0, 0.04]
    bonk = (harmonics(_hz(57) * drop, m, sq) + 0.7 * harmonics(_hz(58) * drop, m, sq)) * _perc(m, 0.003, 0.05)
    place(out, bonk, 0)
    place(out, bonk, n_(0.12))
    return eq(out, lambda f: lp(f, 2500, 2))


@sound("ui", gain=2.0)
def ui_unlock(rng):
    """Unlock: fast rising A-major-pentatonic glass glissando (A5→A7), then an A major chord
    (A5-C#6-E6-A6) with sparkle and a soft room."""
    out = np.zeros(n_(0.3) + _len(0.35))
    run = (81, 83, 85, 88, 90, 93, 95, 97, 100, 102, 105)
    for i, m in enumerate(run):
        place(out, (0.4 + 0.6 * i / len(run)) * _glass(_hz(m), 0.06, shimmer=0.2), n_(0.025 * i))
    for m in (81, 85, 88, 93):
        place(out, 0.5 * _glass(_hz(m), 0.35, shimmer=0.25), n_(0.3))
    out += 0.2 * _sparkle(rng, len(out) / SR, 20, (93, 95, 97, 100, 102, 105), 0.3, 1.0)
    return reverb(out, rng, t60=0.8, wet=0.2)
