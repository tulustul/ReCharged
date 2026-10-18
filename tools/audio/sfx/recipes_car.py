"""Car sounds: engine loops, tyre/surface loops, impacts, landings, boost and drift.

Engine loops are 4 s, mono, built only from partials on the loop's 0.25 Hz grid and circular
noise, so they are seamless by construction. They are band-limited to ~9 kHz so kira's
playback-rate resampling (Hermite, no anti-aliasing) stays alias-free up to rate ~2.4.
Base pitch is meant for playback rate 1.0 ≈ mid revs (idle ≈ 0.55, top speed + boost ≈ 2.0).
"""
from functools import partial

import numpy as np

from synth import (SR, TAU, bell, boom, bp, burst, drive, dust, eq, expo, fades, fm, grid,
                   harmonics, hp, line, lp, mix, n_, noise, perc, phase, place, reverb, ring, rms1,
                   saw, sine, smooth_rand, sound, svf, t_)

ENGINE_SEC = 4.0
ENGINE_FMAX = 9000.0  # Hz: top partial at playback rate 1.0


def _jitter(n, rng, depth):
    """Zero-mean periodic speed wobble (keeps the loop's total phase on the grid)."""
    j = smooth_rand(n, 3.0, rng, circular=True) + 0.5 * np.sin(TAU * phase(grid(0.75, n), n))
    return 1 + depth * (j - j.mean())


def _engine(rng, f0, amps, am=(), noise_band=(2500, 7000), noise_db=-24.0, phases=None, sub=0.0,
            rough=0.04):
    n = n_(ENGINE_SEC)
    f0 = grid(f0, n)
    f = f0 * _jitter(n, rng, 0.0015)
    tone = harmonics(f, n, amps, fmax=ENGINE_FMAX, phases=phases)
    if sub:
        tone += sub * harmonics(f / 2, n, [1.0], fmax=ENGINE_FMAX)
    tone /= np.sqrt(np.mean(tone * tone))
    mod = 1 + rough * smooth_rand(n, 40.0, rng, circular=True)  # mechanical irregularity
    for rate, depth in am:  # gear / drivetrain flutter
        mod += depth * np.sin(TAU * phase(grid(rate, n), n))
    lo, hi = noise_band
    fc = np.sqrt(lo * hi)
    hiss = eq(noise(n, rng), lambda fr: bp(fr, fc, fc / (hi - lo)) * lp(fr, ENGINE_FMAX, 6), circular=True)
    hiss *= 0.6 + 0.4 * np.sin(TAU * phase(f, n))  # brush/inverter noise follows the commutation rate
    hiss /= np.sqrt(np.mean(hiss * hiss))
    return tone * mod + 10 ** (noise_db / 20) * hiss


def _amps(f0, count, tilt, odd=1.0, boosts=()):
    k = np.arange(1, count + 1)
    a = k ** -tilt * np.where(k % 2 == 1, odd, 1.0)
    for fc, q, gain in boosts:  # resonances placed on the harmonic series (they move with pitch)
        a *= bell(k * f0, fc, q, gain)
    return a


@sound("engine", loop=True, pitch=True)
def engine_buggy(rng):
    """Brushed RC buggy: bright nasal whine, gear-mesh whine around the 7th/14th harmonic,
    12.5 Hz gear flutter and commutator hiss."""
    f0 = 185.0
    a = _amps(f0, 60, 0.9, odd=1.3, boosts=((7 * f0, 6, 9), (14 * f0, 6, 4), (3000, 0.7, 3)))
    return _engine(rng, f0, a, am=((12.5, 0.08),), noise_band=(3000, 7500), noise_db=-22)


@sound("engine", loop=True, pitch=True)
def engine_truck(rng):
    """Big monster-truck motor: low pulsy buzz with a 450-700 Hz growl formant, sub-octave weight,
    23 Hz drivetrain growl and an 11th-harmonic gear grind."""
    f0 = 92.0
    a = _amps(f0, 100, 0.9, odd=1.6, boosts=((560, 1.1, 8), (11 * f0, 8, 6), (2500, 0.8, -4)))
    return _engine(rng, f0, a, am=((23.0, 0.22), (46.0, 0.06)), noise_band=(200, 1500), noise_db=-18,
                   sub=0.45, rough=0.1)


@sound("engine", loop=True, pitch=True)
def engine_sports(rng):
    """Electric hypercar scream: a bright 6th-10th harmonic scream band (1.5-2.5 kHz at rate 1) over
    smooth low partials, a thin 13th-harmonic inverter whine; random phases keep it smooth, not buzzy."""
    f0 = 245.0
    a = np.array([1.0, 0.6, 0.45, 0.35, 0.36, 0.55, 0.66, 0.5, 0.4, 0.3, 0.2, 0.12, 0.3, 0.08,
                  0.06, 0.05, 0.04, 0.03, 0.022, 0.016, 0.012, 0.009, 0.007, 0.005, 0.004, 0.003])
    ph = rng.uniform(0, TAU, len(a))
    return _engine(rng, f0, a, noise_band=(2000, 6000), noise_db=-27, phases=ph, sub=0.18, rough=0.02)


# --------------------------------------------------------------------------- tyre & surface loops
LOOP_SEC = 3.0


def _wobble(n, rng, rates_depths):
    """Zero-mean periodic frequency multiplier from (rate Hz, depth) smooth-random layers."""
    m = sum(d * smooth_rand(n, r, rng, circular=True) for r, d in rates_depths)
    return 1 + m - m.mean()


def _grains(n, rng, rate, grain, circular=True, spread_db=6.0):
    """Poisson-placed copies of `grain(rng)` (a short signal) with random level and sign."""
    out = np.zeros(n)
    for pos in rng.integers(0, n, rng.poisson(rate * n / SR)):
        g = grain(rng)
        place(out, g * rng.choice([-1, 1]) * 10 ** (rng.normal(0, spread_db) / 20), int(pos), circular)
    return out


@sound("loop", loop=True, pitch=True)
def skid_loop(rng):
    """Tyre squeal on hard floors: three wandering stick-slip squeal tones (~1.35/1.9/2.6 kHz +
    harmonics) that wax and wane with irregular chatter, over band-passed friction hiss and a
    little rubber-scrub body."""
    n = n_(LOOP_SEC)
    out = np.zeros(n)
    for f0, g in ((1350, 1.0), (1900, 0.55), (2600, 0.25)):
        f = grid(f0, n) * _wobble(n, rng, ((0.7, 0.05), (2.5, 0.06), (8.0, 0.015)))
        a = np.clip(0.55 + 0.6 * smooth_rand(n, 4.0, rng, circular=True), 0.03, 1.1)  # squeal waxes/wanes
        a *= 1 + 0.3 * smooth_rand(n, 45.0, rng, circular=True)  # stick-slip chatter
        out += g * a * harmonics(f, n, [1.0, 0.35, 0.12])
    hiss = eq(noise(n, rng), lambda f: bp(f, 3000, 0.7) * lp(f, 9000, 4), circular=True)
    hiss *= 1 + 0.3 * smooth_rand(n, 15.0, rng, circular=True)
    body = eq(noise(n, rng), lambda f: bp(f, 350, 1.0), circular=True)
    out = rms1(out) + 0.4 * rms1(hiss) + 0.2 * rms1(body)
    return eq(out, lambda f: lp(f, 10000, 4), circular=True)


@sound("loop", loop=True, pitch=True)
def scrub_loop(rng):
    """Tyre scrub on carpet, grass and dirt: grainy band-passed friction noise, fibre-scratch
    crackle and a low rumbling body — no squeal."""
    n = n_(LOOP_SEC)
    base = eq(noise(n, rng, -3), lambda f: bp(f, 1100, 0.45), circular=True)
    base *= np.clip(1 + 0.45 * smooth_rand(n, 40.0, rng, circular=True), 0.2, None)
    grain = eq(_grains(n, rng, 900, lambda r: noise(n_(0.002), r) * np.hanning(n_(0.002)), spread_db=3),
               lambda f: bp(f, 2500, 0.8), circular=True)
    body = eq(noise(n, rng, -6), lambda f: lp(f, 250, 2), circular=True)
    body *= 1 + 0.4 * smooth_rand(n, 8.0, rng, circular=True)
    out = rms1(base) + 0.5 * rms1(grain) + 0.45 * rms1(body)
    return eq(out, lambda f: lp(f, 8000, 4) * hp(f, 40, 2), circular=True)


def _thud(rng, f=(55, 95), dec=0.045):
    t = t_(n_(0.3))
    tone = sum(np.sin(TAU * fr * rng.uniform(0.9, 1.1) * t) for fr in f) * np.exp(-t / dec)
    return fades(tone + 0.3 * eq(noise(len(t), rng), lambda fr: lp(fr, 400, 2)) * np.exp(-t / 0.02), 0, 0.02)


@sound("loop", loop=True, pitch=True)
def rumble_grass_loop(rng):
    """Driving over a lawn: random low suspension thuds, swishing grass against the chassis
    (high-passed fluttering noise) and crisp blade flicks."""
    n = n_(LOOP_SEC)
    bumps = _grains(n, rng, 16, _thud, spread_db=2.5)  # dense and even: no bump stands out per loop
    swish = eq(noise(n, rng), lambda f: hp(f, 2000, 2) * lp(f, 9000, 3), circular=True)
    swish *= np.clip(0.6 + 0.5 * smooth_rand(n, 18.0, rng, circular=True)
                     + 0.3 * smooth_rand(n, 3.0, rng, circular=True), 0.05, None)
    flicks = eq(_grains(n, rng, 220, lambda r: noise(n_(0.001), r), spread_db=4), lambda f: hp(f, 3500, 2),
                circular=True)
    low = eq(noise(n, rng, -6), lambda f: lp(f, 150, 2), circular=True)
    return 0.9 * rms1(bumps) + 0.5 * rms1(swish) + 0.25 * rms1(flicks) + 0.3 * rms1(low)


def _pebble(rng):
    t = t_(n_(0.045))  # ≥ 7 decay constants: grains must not end with a click
    fr = rng.uniform(800, 2600, 3)
    return sum(np.sin(TAU * f * t) * np.exp(-t / rng.uniform(0.002, 0.006)) for f in fr)


@sound("loop", loop=True, pitch=True)
def rumble_dirt_loop(rng):
    """Dirt and gravel crunch: dense band-passed sand/grit grains, small pebble knocks
    (short damped partials), low rumble and a sandy hiss."""
    n = n_(LOOP_SEC)
    grit = eq(_grains(n, rng, 1400, lambda r: noise(n_(r.uniform(0.0005, 0.003)), r), spread_db=4),
              lambda f: bp(f, 3000, 0.7), circular=True)
    knocks = _grains(n, rng, 60, _pebble, spread_db=3)
    low = eq(noise(n, rng, -6), lambda f: lp(f, 180, 2), circular=True)
    low *= 1 + 0.5 * smooth_rand(n, 6.0, rng, circular=True)
    hiss = eq(noise(n, rng), lambda f: bp(f, 5000, 1.2), circular=True)
    out = rms1(grit) + 0.6 * rms1(knocks) + 0.55 * rms1(low) + 0.15 * rms1(hiss)
    return eq(out, lambda f: lp(f, 11000, 4), circular=True)


# --------------------------------------------------------------------------- impacts & landings
def _rattle(rng, n, rate, tau, lo=900, hi=3500):
    """Loose parts rattling: a decaying stream of clicks through small short-lived modes."""
    env = np.exp(-t_(n) / tau)
    clicks = dust(n, rng, rate * env, spread_db=4) * env  # fewer and quieter as it settles
    k = 5
    return ring(n, rng, rng.uniform(lo, hi, k), np.ones(k), rng.uniform(0.003, 0.012, k), clicks)


def _landing(rng, pitch):
    """Tyre-and-chassis landing: pitch-dropping low sine thud, low-passed body noise, tyre squish,
    decaying plastic rattle and a faint suspension spring; _2 is heavier and lower."""
    n = n_(0.6)
    thud = boom(n, 85 * pitch, 45 * pitch, 0.12, 0.09, 0.001)
    body = eq(noise(n, rng), lambda f: lp(f, 700, 2)) * perc(n, 0.001, 0.045)
    squish = eq(noise(n, rng), lambda f: bp(f, 600, 1.2)) * perc(n, 0.002, 0.03)
    spring = sine(190 * pitch * (1 + 0.03 * np.sin(TAU * 9 * t_(n))), n) * perc(n, 0.004, 0.12)
    return thud + 0.6 * body + 0.25 * squish + 0.3 * _rattle(rng, n, 160, 0.06) + 0.1 * spring


for _i, _p in ((1, 1.0), (2, 0.85)):
    sound("car", name=f"landing_thump_{_i}")(partial(_landing, pitch=_p))


def _impact_soft(rng, pitch):
    """Bump into foam, cushions or a car at low speed: dull pitch-dropping thud, low-passed noise
    body and a muffled 450-850 Hz plastic knock; three pitches/seeds."""
    n = n_(0.45)
    thud = boom(n, 120 * pitch, 70 * pitch, 0.08, 0.06, 0.001)
    body = eq(noise(n, rng), lambda f: lp(f, 900, 2)) * perc(n, 0.001, 0.04)
    knock = ring(n, rng, rng.uniform(450, 850, 2) * pitch, [1.0, 0.8], [0.025, 0.018],
                 burst(n_(0.004), rng, 0.001, 200, 3000))
    return thud + 0.5 * body + 0.5 * knock + 0.12 * _rattle(rng, n, 60, 0.04)


for _i, _p in ((1, 1.0), (2, 0.88), (3, 1.12)):
    sound("car", gain=-2.5, name=f"impact_soft_{_i}")(partial(_impact_soft, pitch=_p))


def _impact_hard(rng):
    """Crash into a wall or prop: sharp broadband crack, plastic body clack (seven random
    0.4-3.5 kHz modes, higher ones shorter), low thump, tinkling debris, saturation, short room."""
    n = n_(0.9)
    crack = burst(n_(0.006), rng, 0.0015, 1000, 12000)
    fr = np.sort(rng.uniform(400, 3500, 7))
    clack = ring(n, rng, fr, rng.uniform(0.5, 1.0, 7), 0.15 * (400 / fr) ** 0.6,
                 burst(n_(0.003), rng, 0.00075, 300, 8000))
    thump = boom(n, 95, 50, 0.1, 0.08, 0.001)
    x = mix(0.8 * crack, clack / (np.max(np.abs(clack)) + 1e-9), 0.9 * thump,
            0.35 * _rattle(rng, n, 90, 0.12, 2000, 6000))
    x = drive(x / np.max(np.abs(x)), 2.5)  # crunch; evens out the variants' crest factors
    return reverb(x, rng, t60=0.35, wet=0.12, predelay=0.005)


for _i in (1, 2, 3):
    sound("car", gain=1.0, name=f"impact_hard_{_i}")(_impact_hard)


# --------------------------------------------------------------------------- boost & drift
@sound("car")
def boost_whoosh(rng):
    """Boost / Turbo kick: electric FM zap onset, band-pass noise whoosh sweeping 300→3500→1100 Hz,
    sub whump, a rising filtered-saw charge tone and a decaying crackle; short reverb."""
    n = n_(1.1)
    t = t_(n)
    zap = fm(expo([(0, 900), (0.06, 2500)], n), n, ratio=1.5, index=line([(0, 6), (0.08, 1)], n))
    zap *= perc(n, 0.001, 0.03)
    whoosh = svf(noise(n, rng, -3), expo([(0, 300), (0.25, 3500), (1.1, 1100)], n), q=1.4, mode="bpn")
    whoosh *= expo([(0, 0.01), (0.06, 1.0), (0.4, 0.5), (1.1, 0.01)], n)
    sub = boom(n, 70, 42, 0.2, 0.12)
    rise = eq(saw(expo([(0, 220), (0.3, 660)], n), n), lambda f: lp(f, 2500, 2))
    rise *= line([(0, 0), (0.05, 1), (0.35, 0.6), (0.7, 0)], n)
    crackle = eq(dust(n, rng, 400 * np.exp(-t / 0.15)), lambda f: bp(f, 4500, 0.8))
    x = 0.5 * zap + rms1(whoosh) * 0.35 + 0.7 * sub + 0.15 * rise + 0.3 * crackle
    return reverb(x, rng, t60=0.6, wet=0.12)


def _arc(rng):
    m = n_(rng.uniform(0.005, 0.03))
    g = noise(m, rng) * (rng.uniform(size=m) < 0.3)  # broken-up, crackly arc
    return eq(g, lambda f: bp(f, rng.uniform(2500, 6000), 1.0)) * np.hanning(m)


@sound("loop", loop=True)
def boost_crackle_loop(rng):
    """Held boost: random electric arcs (crackly band-passed bursts), a raspy 120 Hz-gated buzz with
    no clear pitch (won't clash with engines) and an airy rush."""
    n = n_(2.5)
    arcs = _grains(n, rng, 28, _arc, spread_db=4)
    gate = (np.sin(TAU * phase(grid(120, n), n)) > 0.3).astype(float)
    buzz = eq(noise(n, rng), lambda f: bp(f, 3500, 0.8), circular=True) * gate
    buzz *= np.clip(0.4 + 0.6 * smooth_rand(n, 22.0, rng, circular=True), 0, None)
    buzz = eq(buzz, lambda f: lp(f, 9000, 2), circular=True)
    air = eq(noise(n, rng, -3), lambda f: bp(f, 1800, 0.5), circular=True)
    out = rms1(arcs) + 0.35 * rms1(buzz) + 0.5 * rms1(air)
    return eq(out, lambda f: hp(f, 150, 2) * lp(f, 12000, 4), circular=True)  # leave lows to the engine


@sound("loop", loop=True, pitch=True)
def drift_sizzle_loop(rng):
    """Drift sparks: dense sparkler crackle, a resonant 2.6/5.2 kHz sizzle band that reads as a pitch
    (playback rate follows the charge rate) and a high hiss; < 12 kHz for rate headroom."""
    n = n_(2.5)
    crackle = eq(_grains(n, rng, 2500, lambda r: noise(n_(0.0006), r), spread_db=6),
                 lambda f: bp(f, 6000, 0.6) * lp(f, 12000, 4), circular=True)
    sizzle = eq(noise(n, rng), lambda f: bp(f, 2600, 6) + 0.5 * bp(f, 5200, 6), circular=True)
    sizzle *= np.clip(1 + 0.35 * smooth_rand(n, 25.0, rng, circular=True), 0, None)
    hiss = eq(noise(n, rng), lambda f: bp(f, 7000, 1.0) * lp(f, 12000, 4), circular=True)
    return rms1(crackle) + 0.7 * rms1(sizzle) + 0.3 * rms1(hiss)


def _plink(rng):
    """Water drop: a bubble resonance chirping upward (Minnaert frequency rises as it surfaces)."""
    dec = rng.uniform(0.006, 0.014)
    m = n_(7 * dec)  # ≥ 7 decay constants: grains must not end with a click
    f0 = rng.uniform(900, 2600)
    return sine(expo([(0, f0), (0.015, 1.7 * f0)], m), m) * perc(m, 0.0005, dec)


@sound("loop", loop=True, pitch=True)
def rumble_water_loop(rng):
    """Driving through puddles / the wash bay (extra): sloshy fast-modulated band-passed noise, a
    stream of upward-chirping droplet plinks and a low slosh."""
    n = n_(LOOP_SEC)
    slosh = eq(noise(n, rng, -3), lambda f: bp(f, 1400, 0.6), circular=True)
    slosh *= np.clip(0.5 + 0.6 * smooth_rand(n, 30.0, rng, circular=True), 0.05, None)
    drops = _grains(n, rng, 70, _plink, spread_db=4)
    low = eq(noise(n, rng, -6), lambda f: lp(f, 300, 2), circular=True)
    low *= 1 + 0.5 * smooth_rand(n, 5.0, rng, circular=True)
    out = rms1(slosh) + 0.6 * rms1(drops) + 0.4 * rms1(low)
    return eq(out, lambda f: lp(f, 11000, 4) * hp(f, 40, 2), circular=True)
