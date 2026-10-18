"""Ambience beds (stereo loops) and thunder one-shots.

Loops are seamless by construction: periodic noise, circular filters and reverb, events placed
circularly, periodic modulation, tones on the loop grid.  Channels are decorrelated (independent
noise per channel, panned events, one reverb IR per channel).
"""
import numpy as np

from synth import (SR, TAU, amp, bell, bp, convolve, dust, eq, expo, fades, grid, harmonics, hp,
                   line, lp, mix, n_, noise, pan, perc, phase, place, reverb, reverb_ir, smooth_rand,
                   sound, stft_eq, t_)
from synth import rms1 as _norm


# --------------------------------------------------------------------------- helpers
def _bed(n, rng, slope, mag):
    """Decorrelated stereo noise shaped by the magnitude response `mag` (circular)."""
    return eq(np.stack([noise(n, rng, slope), noise(n, rng, slope)], 1), mag, circular=True)


def _wet(x, rng, t60, predelay=0.02, damp=1.0):
    """Wet-only stereo reverb of a loop (one IR per channel; the tail wraps)."""
    chans = x.T if x.ndim == 2 else (x, x)
    return np.stack([convolve(c, reverb_ir(t60, rng, predelay, damp), circular=True) for c in chans], 1)


def _poisson(n, rng, rate, density):
    pos = rng.integers(0, n, rng.poisson(rate * n / SR))
    return pos if density is None else pos[rng.uniform(0, 1, len(pos)) < density[pos]]


def _levels(rng, k, spread_db, width):
    lvl = amp(1) ** np.clip(rng.normal(0, spread_db, k), -2 * spread_db, 2 * spread_db)
    ang = (rng.uniform(-width, width, k) + 1) * np.pi / 4
    return lvl * np.cos(ang), lvl * np.sin(ang)


def _drops(n, rng, rate, f, tau, glide=0.0, spread_db=5.0, width=1.0, density=None):
    """Stereo loop of Poisson drops (`rate`/s, thinned by a per-sample `density` 0..1): damped
    sinusoids with log-uniform frequency in `f` (Hz), decay in `tau` (s), optional upward glide
    (bubble plinks; else a random start phase = impact click), bounded random level and pan."""
    pos = _poisson(n, rng, rate, density)
    k = len(pos)
    fr, ta = np.exp(rng.uniform(*np.log(f), k)), rng.uniform(*tau, k)
    ph0 = np.zeros(k) if glide else rng.uniform(0, TAU, k)
    gains = _levels(rng, k, spread_db, width)
    t = t_(n_(6 * tau[1]))[None, :]
    out = np.zeros((n, 2))
    for s in range(0, k, 512):  # chunks bound memory
        sl = slice(s, s + 512)
        ph = TAU * fr[sl, None] * (t + glide * (t - 0.008 * (1 - np.exp(-t / 0.008)))) + ph0[sl, None]
        ev = np.sin(ph) * np.exp(-t / ta[sl, None]) * np.minimum(1, t / 0.00015)
        idx = ((pos[sl, None] + np.arange(t.shape[1])) % n).ravel()
        for c, g in enumerate(gains):
            out[:, c] += np.bincount(idx, (ev * g[sl, None]).ravel(), minlength=n)
    return out


def _scatter(n, rng, rate, irs, spread_db=5.0, width=1.0):
    """Stereo loop of Poisson events, each a random one of the mono `irs` (noise bursts), with
    bounded random level and pan (FFT convolution, circular)."""
    pos = _poisson(n, rng, rate, None)
    which = rng.integers(0, len(irs), len(pos))
    gains = _levels(rng, len(pos), spread_db, width)
    out = np.zeros((n, 2))
    for j, ir in enumerate(irs):
        sel = which == j
        for c, g in enumerate(gains):
            out[:, c] += convolve(np.bincount(pos[sel], g[sel], minlength=n), ir, circular=True)
    return out


def _burst(rng, dur, mag, decay=None):
    """Shaped noise burst (percussive envelope) filtered by `mag`, peak-normalised."""
    m = n_(dur)
    x = eq(np.pad(noise(m, rng) * perc(m, 0.0005, decay or dur / 4), (0, n_(0.02))), mag)
    return x / np.max(np.abs(x))


def _put(buf, send, ev, rng, start, dist, cutoff):
    """Place a mono event at distance 0 (near) .. 1 (far): quieter, low-passed, wetter, random pan."""
    ev = eq(np.pad(ev, (0, n_(0.02))), lambda f: lp(f, cutoff, 2))
    st = pan(ev, rng.uniform(-0.85, 0.85))
    place(buf, st, start, circular=True, gain=amp(-15 * dist))
    place(send, st, start, circular=True, gain=amp(-15 * dist) * (0.3 + 0.9 * dist))


def _stratified(n, rng, count):
    """`count` random start samples spread evenly over the loop (no clumps)."""
    return [int((i + rng.uniform(0.1, 0.9)) * n / count) for i in rng.permutation(count)]


# --------------------------------------------------------------------------- indoor beds
@sound("amb", loop=True, folder="ambience")
def room_tone(rng):
    """Quiet room: soft pink air LP 6 kHz + distant HVAC rumble, faint 50 Hz-family fridge hum,
    slow air movement, small-room reverb."""
    n = n_(20.0)
    air = _norm(_bed(n, rng, -3.0, lambda f: lp(f, 6000, 2) * hp(f, 40, 2)))
    air += 0.5 * _norm(_bed(n, rng, -6.0, lambda f: lp(f, 180, 2) * hp(f, 25, 2)))
    air *= (1 + 0.12 * smooth_rand(n, 0.25, rng, circular=True))[:, None]
    hum = harmonics(grid(50.0, n), n, [0.25, 1.0, 0.4, 0.3, 0.0, 0.12])
    hum = _norm(hum) * (1 + 0.3 * smooth_rand(n, 0.15, rng, circular=True))
    dry = air + 0.2 * hum[:, None] * np.array([0.8, 0.6])
    return dry + 0.15 * _wet(dry, rng, 0.6)


# --------------------------------------------------------------------------- outdoor life
def _note(fpts, apts, dur, h2=0.08):
    m = n_(dur)
    ph = phase(expo(fpts, m), m)
    return line(apts, m) * (np.sin(TAU * ph) + h2 * np.sin(2 * TAU * ph))


def _whistler(rng):
    """Song phrase: 3-6 swept or steady whistles, 2-4.8 kHz."""
    out = []
    for _ in range(rng.integers(3, 7)):
        d = rng.uniform(0.07, 0.2)
        f1, f2, f3 = rng.uniform(2000, 4800, 3)
        fpts = ([(0, f1), (d, f2)], [(0, f1), (d / 2, f3), (d, f2)], [(0, f1), (d, f1 * 1.03)])[rng.integers(3)]
        out += [_note(fpts, [(0, 0), (0.012, 1), (d - 0.02, 0.8), (d, 0)], d),
                np.zeros(n_(rng.uniform(0.04, 0.14)))]
    return np.concatenate(out)


def _chirper(rng):
    """Sparrow-like series: 3-8 fast down-chirps 5 -> 3 kHz at ~8/s."""
    out, f = [], rng.uniform(4200, 5400)
    for _ in range(rng.integers(3, 9)):
        d = rng.uniform(0.03, 0.06)
        out += [_note([(0, f), (d, f * rng.uniform(0.55, 0.7))], [(0, 0), (0.004, 1), (d, 0)], d, 0.15),
                np.zeros(n_(rng.uniform(0.06, 0.12)))]
    return np.concatenate(out)


def _trill(rng):
    """Finch-like trill: a 25 ms up-down note repeated at 16-22 Hz for ~1 s, swelling."""
    rate, f, cnt = rng.uniform(16, 22), rng.uniform(3500, 4800), rng.integers(14, 26)
    unit = _note([(0, f * 0.85), (0.012, f * 1.1), (0.025, f * 0.9)], [(0, 0), (0.004, 1), (0.025, 0)], 0.025)
    gap = np.zeros(n_(1 / rate) - len(unit))
    sw = np.sin(np.linspace(0.2, np.pi - 0.2, cnt))
    return np.concatenate([np.concatenate([unit * a, gap]) for a in sw])


def _dove(rng):
    """Collared-dove coo phrase: soft 500-650 Hz 'hoo-HOO-hoo' with pitch rise and fall."""
    f, out = rng.uniform(500, 650), []
    for d, a, bend in ((0.35, 0.7, 1.05), (0.55, 1.0, 1.12), (0.4, 0.8, 1.04)):
        out += [a * _note([(0, f), (d * 0.4, f * bend), (d, f * 0.95)], [(0, 0), (0.08, 1), (d - 0.1, 0.8), (d, 0)],
                          d, 0.12), np.zeros(n_(0.15))]
    return np.concatenate(out)


@sound("amb", loop=True, folder="ambience")
def birds(rng):
    """Clear day: sparse calls of 4 species (whistled phrases, sparrow chirps, finch trills, a
    distant dove) at random distances/pans with damped outdoor reflections, over a soft air bed
    (breathing pink air + leaf rustle) so the scene never goes dead between calls."""
    n = n_(30.0)
    dry, send = np.zeros((n, 2)), np.zeros((n, 2))
    for make, count, dmin in ((_whistler, 7, 0.0), (_chirper, 9, 0.0), (_trill, 4, 0.2), (_dove, 2, 0.6)):
        for start in _stratified(n, rng, count):
            d = rng.uniform(dmin, 1.0)
            _put(dry, send, make(rng), rng, start, d, 12000 - 8000 * d)
    calls = _norm(dry + 0.5 * _wet(send, rng, 1.2, 0.03, 1.5))
    air = _bed(n, rng, -3.0, lambda f: lp(f, 2500, 2) * hp(f, 80, 2))
    air *= (1 + 0.3 * smooth_rand(n, 0.15, rng, True))[:, None]
    leaves = _bed(n, rng, 0.0, lambda f: bp(f, 5000, 0.8)) * (0.5 + 0.5 * smooth_rand(n, 0.2, rng, True))[:, None]
    return calls + 0.35 * _norm(air) + 0.11 * _norm(leaves)


@sound("amb", loop=True, folder="ambience")
def crickets(rng):
    """Clear night: 6 crickets (4.2-5.2 kHz pulse chirps, 3-4 pulses at ~30 Hz, periods dividing the
    loop, jittered) at own pans/distances + a distant pulsing chorus and faint night air."""
    n = n_(24.0)
    out = np.zeros((n, 2))
    for _ in range(6):
        fc, pulses = rng.uniform(4200, 5200), rng.integers(3, 5)
        plen, pgap = rng.uniform(0.012, 0.018), rng.uniform(0.014, 0.02)
        m = n_(plen)
        tt = t_(m)
        env = np.minimum(1, np.minimum(tt / 0.002, (plen - tt) / 0.004).clip(0))
        ph = phase(fc * (1 - 0.02 * tt / plen), m)
        p = env * (np.sin(TAU * ph) + 0.06 * np.sin(2 * TAU * ph))
        chirp = np.concatenate([np.concatenate([p * (0.7 if i == 0 else 1.0), np.zeros(n_(pgap))])
                                for i in range(pulses)])
        k = rng.integers(24, 48)  # chirps per loop -> period 24/k s (0.5-1.0 s)
        imp = np.zeros(n)
        times = (np.arange(k) * n // k + rng.integers(-n_(0.004), n_(0.004), k)) % n
        imp[times] = amp(1) ** rng.normal(0, 1.5, k)
        dist = rng.uniform(0, 1)
        track = eq(convolve(imp, chirp, circular=True), lambda f: lp(f, 11000 - 5000 * dist, 2), circular=True)
        out += amp(-14 * dist) * pan(track, rng.uniform(-0.9, 0.9))
    chorus = _bed(n, rng, 0.0, lambda f: bp(f, 4600, 5))
    pulse = sum((0.5 + 0.5 * np.sin(TAU * phase(grid(r, n), n) + rng.uniform(0, TAU))) ** 3 for r in (28, 30.5, 33))
    chorus *= (pulse * (0.7 + 0.3 * smooth_rand(n, 0.2, rng, True)))[:, None]
    air = _bed(n, rng, -3.0, lambda f: lp(f, 1500, 2) * hp(f, 60, 2))
    out = _norm(out + 0.3 * _wet(out, rng, 0.9, 0.02, 1.2))
    return out + 0.18 * _norm(chorus) + 0.08 * _norm(air)


@sound("amb", loop=True, folder="ambience")
def wind(rng):
    """Cloudy / snow: brown-pink noise through a gust-driven band-pass (gusts every ~3-8 s, ~8 dB
    swing, brighter when stronger; channels gust slightly apart) + two airy whistles 500-950 Hz
    that sing up in the strong gusts."""
    n = n_(30.0)
    T, t = n / SR, t_(n)
    g = 0.6 + 0.3 * smooth_rand(n, 0.2, rng, True) + 0.15 * smooth_rand(n, 0.6, rng, True)

    def gust(x):
        return np.interp(x, t, g, period=T)

    wf = [600 + 120 * smooth_rand(n, 0.1, rng, True), 850 + 100 * smooth_rand(n, 0.1, rng, True)]
    out = np.zeros((n, 2))
    for c, lag in enumerate((0.0, 0.4)):
        body = stft_eq(noise(n, rng, -4.5), lambda T_, F: gust(T_ - lag) ** 1.3
                       * bp(F, 180 + 900 * gust(T_ - lag), 0.7) * hp(F, 40, 2), frame=2048, circular=True)
        whistle = sum(stft_eq(noise(n, rng), lambda T_, F, w=w, e=e: gust(T_ - lag) ** e
                              * bp(F, np.interp(T_, t, w, period=T), 25), frame=4096, circular=True)
                      for w, e in zip(wf, (3.0, 4.0)))
        out[:, c] = _norm(body) + 0.4 * _norm(whistle)
    return out


# --------------------------------------------------------------------------- rain
def _rain_bed(n, rng, heavy):
    hiss = _bed(n, rng, -1.5, lambda f: hp(f, 200 if heavy else 400, 2) * lp(f, 12000, 2)
                * bell(f, 2500 if heavy else 4000, 0.6, 6))
    if heavy:
        return _norm(hiss) + 0.8 * _norm(_bed(n, rng, -6.0, lambda f: lp(f, 700, 2) * hp(f, 60, 2)))
    return _norm(hiss)


@sound("amb", loop=True, folder="ambience")
def rain_light(rng):
    """Fine rain: hiss bed (HP 400 Hz, +6 dB at 4 kHz) + ~150 drops/s (impact ticks 1.2-8 kHz with
    0.3-2.5 ms decay, ~20 % bubble plinks 1.5-5 kHz), wide pan, gently breathing intensity."""
    n = n_(20.0)
    dens = 0.85 + 0.15 * smooth_rand(n, 0.2, rng, True)
    bed = _rain_bed(n, rng, False) * dens[:, None]
    drops = _drops(n, rng, 120, (1200, 8000), (0.0003, 0.0025), density=dens)
    drops += 0.6 * _drops(n, rng, 30, (1500, 5000), (0.003, 0.008), glide=0.15, density=dens)
    return bed + 0.55 * _norm(drops)


@sound("amb", loop=True, folder="ambience", gain=4.0)
def rain_heavy(rng):
    """Storm rain: hiss bed + low-mid roar, ~400 heavier drops/s (600-5000 Hz ticks, ~15 % plinks),
    ~1.5 noisy splashes/s, gusting intensity."""
    n = n_(20.0)
    dens = 0.8 + 0.2 * smooth_rand(n, 0.12, rng, True)
    bed = _rain_bed(n, rng, True) * dens[:, None]
    drops = _drops(n, rng, 340, (600, 5000), (0.0005, 0.004), density=dens)
    drops += 0.6 * _drops(n, rng, 60, (1000, 4000), (0.003, 0.009), glide=0.15, density=dens)
    splash = [_burst(rng, d, lambda f: hp(f, 300, 2) * lp(f, 3000, 2)) for d in rng.uniform(0.04, 0.08, 8)]
    return _norm(bed) + 0.5 * _norm(drops) + 1.5 * _scatter(n, rng, 1.5, splash, spread_db=4)


def _rain_indoor(rng, heavy):
    n = n_(20.0)
    bed = eq(_rain_bed(n, rng, heavy), lambda f: lp(f, 700 if heavy else 900, 3), circular=True)
    thuds = [_burst(rng, d, lambda f: bp(f, fc, 1.0) * lp(f, 1200, 3))
             for d, fc in zip(rng.uniform(0.006, 0.015, 16), rng.uniform(150, 500, 16))]
    roof = _scatter(n, rng, 150 if heavy else 60, thuds)
    taps = _drops(n, rng, 10 if heavy else 4, (2000, 4500), (0.0015, 0.003), spread_db=4, width=0.5)
    taps += 0.25 * _wet(taps, rng, 0.5)
    out = _norm(bed) + 0.5 * _norm(roof) + (0.4 if heavy else 0.5) * taps  # sparse: scaled by tap peak
    if heavy:
        out += 0.8 * _norm(_bed(n, rng, -6.0, lambda f: lp(f, 300, 2) * hp(f, 30, 2)))
        drip = _drops(n, rng, 0.8, (650, 800), (0.02, 0.035), glide=0.6, spread_db=2, width=0.0)
        drip = eq(drip.sum(1), lambda f: lp(f, 3000, 2), circular=True)
        drips = pan(drip, 0.6)
        out += 0.6 * (drips + 0.3 * _wet(drips, rng, 0.6))
    return out


@sound("amb", loop=True, folder="ambience")
def rain_light_indoor(rng):
    """Light rain heard indoors: rain bed LP 900 Hz (3rd order) + soft roof patter (dull 150-500 Hz
    thuds LP 1.2 kHz, ~60/s) + ~4 crisp glass ticks/s (2-4.5 kHz) in a small room."""
    return _rain_indoor(rng, False)


@sound("amb", loop=True, folder="ambience", gain=3.0)
def rain_heavy_indoor(rng):
    """Heavy rain heard indoors: heavy bed LP 700 Hz + low rumble, dense roof drumming, ~10 glass
    ticks/s, occasional gutter drips (rising 700 Hz plinks) off to the right."""
    return _rain_indoor(rng, True)


# --------------------------------------------------------------------------- thunder
def _roll(n, rng, t0, a, atk, dec, fc):
    """One thunder roll: brown noise with a fluttering swell/decay, its low-pass closing as it dies."""
    t = t_(n)
    tt = np.maximum(t - t0, 0)
    env = (t > t0) * (1 - np.exp(-tt / atk)) * np.exp(-tt / dec)
    env *= np.clip(1 + 0.6 * smooth_rand(n, 3.0, rng), 0.15, None)
    x = noise(n, rng, -6) * env * a
    x = stft_eq(x, lambda T, F: lp(F, fc * (0.35 + 0.65 * np.exp(-np.maximum(T - t0, 0) / (1.5 * dec))), 2)
                * hp(F, 35, 2), frame=2048)
    return pan(x, rng.uniform(-0.6, 0.6))


def _thunder(rng, dur, rolls, crack, t60, wet, damp, total):
    """Sum of rolls (+ optional crack), faded so nothing is cut, reverberated, cropped to `total` s."""
    out = sum(_roll(n_(dur), rng, *r) for r in rolls)
    if crack is not None:
        out[: len(crack)] += 0.9 * np.max(np.abs(out)) / np.max(np.abs(crack)) * pan(crack, 0.15)
    out = reverb(fades(out, 0.0, 0.25 * dur), rng, t60=t60, wet=wet, predelay=0.03, damp=damp)
    return fades(out[: n_(total)], 0.0, 0.3 * total)


@sound("thunder", folder="ambience")
def thunder_1(rng):
    """Close strike: 0.3 s ripping crack (dense decaying crackle 0.8-9 kHz + transient), immediate
    450 Hz boom and three later rolls, 2.4 s outdoor reverb."""
    m = n_(0.35)
    rip = dust(m, rng, line([(0, 3000), (0.3, 0)], m), 8) * perc(m, 0.001, 0.09)
    rip += noise(m, rng) * perc(m, 0.0003, 0.012) * 0.6
    rip = eq(rip, lambda f: hp(f, 800, 2) * lp(f, 9000, 2))
    rolls = [(0.05, 1.0, 0.03, 1.2, 450), (0.45, 0.7, 0.2, 1.5, 320), (1.3, 0.55, 0.3, 1.4, 260),
             (2.3, 0.4, 0.4, 1.6, 200)]
    return _thunder(rng, 5.0, rolls, rip, 2.4, 0.45, 1.5, 6.0)


@sound("thunder", folder="ambience", gain=-2.0)
def thunder_2(rng):
    """Medium distance: no crack; a 0.4 s swell into two big rolls (0.6 s, 2.6 s) with smaller
    rumbles between, LP ~350 Hz, 2.6 s reverb."""
    rolls = [(0.0, 0.5, 0.35, 0.9, 380), (0.6, 1.0, 0.25, 1.4, 350), (1.6, 0.4, 0.4, 1.0, 260),
             (2.6, 0.9, 0.3, 1.7, 300), (3.8, 0.35, 0.5, 1.5, 200)]
    return _thunder(rng, 6.0, rolls, None, 2.6, 0.55, 2.0, 7.0)


@sound("thunder", folder="ambience", gain=-5.0)
def thunder_3(rng):
    """Distant: soft, long low rumble — slow 1 s swell, LP ~170 Hz, weak secondary rolls, 3 s tail."""
    rolls = [(0.0, 1.0, 1.0, 2.5, 170), (1.8, 0.5, 0.6, 1.8, 150), (3.5, 0.35, 0.8, 2.0, 130)]
    return _thunder(rng, 6.5, rolls, None, 3.0, 0.6, 2.5, 8.0)


# --------------------------------------------------------------------------- big rooms
def _buzz(n, rng):
    """Fluorescent lamp: 100 Hz buzz with harmonics to ~2 kHz + a little gated sizzle."""
    f = grid(100.0, n)
    k = np.arange(1, 21)
    tone = harmonics(f, n, k ** -1.3 * np.where(k % 2 == 0, 1.4, 1.0))
    sizzle = eq(noise(n, rng), lambda fr: bp(fr, 3500, 1.0), circular=True)
    sizzle *= (0.5 + 0.5 * np.sin(TAU * phase(f, n))) ** 4
    return (_norm(tone) + 0.3 * _norm(sizzle)) * (1 + 0.1 * smooth_rand(n, 3.0, rng, True))


def _wobble(n, f0, depth, rate=0.25):
    """Zero-mean periodic frequency wobble around a grid frequency (keeps the loop phase whole)."""
    return grid(f0, n) * (1 + depth * np.sin(TAU * phase(grid(rate, n), n)))


@sound("amb", loop=True, folder="ambience")
def supermarket_hum(rng):
    """Closed supermarket: two refrigeration compressors (50 Hz-family stacks beating at 0.33 Hz),
    freezer-fan whoosh with a 117 Hz blade-pass tone, faint fluorescent buzz, 2.5 s hall reverb."""
    n = n_(24.0)
    comp = harmonics(grid(50.0, n), n, [0.3, 1.0, 0.35, 0.25, 0.15, 0.1])
    comp += harmonics(grid(50.15, n), n, [0.2, 0.7, 0.3, 0.2])
    fan = _bed(n, rng, -2.0, lambda f: bp(f, 900, 0.5) * hp(f, 120, 2) * lp(f, 6000, 2))
    blade = harmonics(_wobble(n, 117.0, 0.003), n, [1.0, 0.4, 0.15])
    mono = 0.3 * _norm(comp) + 0.1 * _norm(blade) + 0.08 * _norm(_buzz(n, rng))
    dry = _norm(fan) + mono[:, None] * np.array([0.65, 0.5])
    return 0.5 * dry + 0.8 * _norm(_wet(dry, rng, 2.5, 0.03, 1.0))


@sound("amb", loop=True, folder="ambience")
def workshop_hum(rng):
    """Auto-repair workshop: fluorescent buzz, idling compressor throb (24 Hz thump pulses, 2-cylinder
    accent) with 50 Hz motor hum, ventilation fan + 85 Hz blade tone, bright 1.8 s garage reverb."""
    n = n_(24.0)
    per = n_(1 / 24)
    cnt = n // per
    imp = np.zeros(n)
    imp[np.arange(cnt) * per] = (1 + 0.3 * (np.arange(cnt) % 2)) * (1 + 0.15 * rng.standard_normal(cnt))
    m = n_(0.06)
    thump = np.sin(TAU * 70 * t_(m)) * np.exp(-t_(m) / 0.012)
    throb = eq(convolve(imp, thump, circular=True), lambda f: lp(f, 400, 2), circular=True)
    motor = harmonics(grid(50.0, n), n, [0.5, 1.0, 0.3, 0.2])
    fan = _bed(n, rng, -2.5, lambda f: bp(f, 700, 0.6) * hp(f, 80, 2))
    blade = harmonics(_wobble(n, 85.0, 0.004), n, [1.0, 0.5, 0.2])
    mono = 0.4 * _norm(throb) + 0.12 * _norm(motor) + 0.15 * _norm(_buzz(n, rng)) + 0.1 * _norm(blade)
    dry = _norm(fan) + mono[:, None] * np.array([0.5, 0.65])
    return 0.6 * dry + 0.7 * _norm(_wet(dry, rng, 1.8, 0.02, 0.6))


def _wrench(rng):
    """Impact wrench: air-motor whine spinning up + 15-20 Hz hammer clicks + exhaust hiss."""
    d = rng.uniform(0.5, 1.5)
    m = n_(d)
    t = t_(m)
    f = 300 + 350 * (1 - np.exp(-t / 0.08))
    whine = harmonics(f * (1 + 0.01 * np.sin(TAU * 7 * t)), m, [1.0, 0.5, 0.3, 0.15])
    imp = np.zeros(m)
    imp[(np.arange(0.05, d - 0.03, 1 / rng.uniform(15, 20)) * SR).astype(int)] = 1.0
    ring = np.sin(TAU * 4100 * t_(n_(0.03))) * np.exp(-t_(n_(0.03)) / 0.006)
    hammer = convolve(imp, mix(_burst(rng, 0.004, lambda fr: bp(fr, 2600, 1.5)), 0.4 * ring))[:m]
    hiss = eq(noise(m, rng), lambda fr: hp(fr, 3000, 2))
    env = np.minimum(1, t / 0.02) * np.minimum(1, (d - t) / 0.05).clip(0)
    return env * (0.25 * _norm(whine) + _norm(hammer) + 0.2 * _norm(hiss))


def _hammer(rng):
    """2-4 hammer taps on a steel part: free-bar modes (1, 2.76, 5.40, 8.93) + click."""
    f1, out = rng.uniform(900, 1600), []
    for _ in range(rng.integers(2, 5)):
        m = n_(0.5)
        tap = np.zeros(m)
        for r, a, dcy in ((1.0, 1.0, 0.35), (2.756, 0.6, 0.2), (5.404, 0.4, 0.12), (8.933, 0.25, 0.07)):
            tap += a * np.sin(TAU * f1 * r * t_(m)) * np.exp(-t_(m) / dcy)
        tap = mix(tap, 0.8 * _burst(rng, 0.003, lambda fr: bp(fr, 4000, 1.0)))
        out += [tap * rng.uniform(0.7, 1.0), np.zeros(n_(rng.uniform(0.0, 0.2)))]
    return np.concatenate(out)


def _ratchet(rng):
    """Ratchet wrench: 6-12 clicks at 12-18 Hz with a small 3.2/5.1 kHz ring."""
    rate, cnt = rng.uniform(12, 18), rng.integers(6, 13)
    m = n_(0.03)
    tt = t_(m)
    ring = np.sin(TAU * 3200 * tt) * np.exp(-tt / 0.008) + 0.5 * np.sin(TAU * 5100 * tt) * np.exp(-tt / 0.005)
    click = mix(0.6 * ring, _burst(rng, 0.003, lambda fr: bp(fr, 3500, 2.0)))
    gap = np.zeros(max(0, n_(1 / rate) - len(click)))
    return np.concatenate([np.concatenate([click * rng.uniform(0.7, 1.0), gap]) for _ in range(cnt)])


def _airhose(rng):
    """Air-hose blast 'pssht': HP 2.5 kHz noise, 15 ms attack, 0.3-0.6 s hold, 0.15 s release."""
    hold = rng.uniform(0.3, 0.6)
    m = n_(hold + 0.6)
    t = t_(m)
    x = eq(noise(m, rng), lambda f: hp(f, 2500, 2) * lp(f, 9000, 2))
    return x * np.minimum(1, t / 0.015) * np.where(t < hold, 1.0, np.exp(-(t - hold) / 0.15))


@sound("amb", loop=True, folder="ambience", gain=-2.0)
def workshop_tools(rng):
    """Distant tools, sparse (6 per 36 s): impact-wrench bursts, hammer taps, ratchet clicks and an
    air-hose blast, dulled (LP 2.5-4 kHz), reverb-dominated (1.8 s garage, DRR ≈ -15 dB) over a faint room bed."""
    n = n_(36.0)
    dry, send = np.zeros((n, 2)), np.zeros((n, 2))
    kinds = [_wrench, _hammer, _ratchet, _airhose, _wrench, _hammer]
    for make, start in zip(kinds, _stratified(n, rng, len(kinds))):
        d = rng.uniform(0.45, 0.8)
        _put(dry, send, make(rng), rng, start, d, 5500 - 3500 * d)
    room = _bed(n, rng, -4.0, lambda f: lp(f, 2000, 2) * hp(f, 50, 2))
    return 0.6 * dry + 2.5 * _wet(send, rng, 1.8, 0.04, 0.8) + 0.004 * _norm(room)  # DRR ≈ -15 dB
