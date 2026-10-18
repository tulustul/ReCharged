"""Item & weapon sounds: pickups, roulette, rockets, leech coil, shock orb, EMP, goo bomb, oil,
spring pad, shield and storm strike.

Mono, 48 kHz. One-shots are layered (transient + body + tail) and glued with short reverbs; the
loops (rocket flight, leech hum, orb hum) are seamless by construction (grid frequencies, periodic
modulation, circular filtering). FM indexes are kept so sidebands stay under ~15 kHz (no aliasing).
"""
import numpy as np

from synth import (TAU, bp, drive, eq, expo, fades, fm, grid, harmonics, hp, line, lp, n_, noise, perc,
                   phase, place, pulse, reverb, saw, sine, smooth_rand, sound, stft_eq, svf, t_)
from synth import band as _band, boom as _boom, burst as _burst, crackle as _crackle, norm as _n, ring as _ring


def _drops(n, rng, count, t0, t1, lo, hi, decay=0.02):
    """Droplets: short sines gliding up an octave, scattered in [t0, t1], later ones quieter."""
    out, m = np.zeros(n), n_(0.08)
    for _ in range(count):
        at, f = rng.uniform(t0, t1), rng.uniform(lo, hi)
        d = sine(expo([(0, f), (0.03, 2 * f)], m), m) * perc(m, 0.001, decay * rng.uniform(0.6, 1.4))
        place(out, d, n_(at), gain=rng.uniform(0.3, 1.0) * (1 - (at - t0) / (t1 - t0)) ** 1.5)
    return out


# --------------------------------------------------------------------------- pickups & roulette
@sound("item")
def pickup_grab(rng):
    """FM 'bling' gliding 600→2400 Hz in 80 ms with an octave sparkle, three high glints and a short zap crackle."""
    n = n_(0.45)
    f = expo([(0, 600), (0.08, 2400), (0.35, 2500)], n)
    bling = fm(f, n, ratio=2.0, index=line([(0, 2.0), (0.1, 0.5)], n)) * perc(n, 0.002, 0.09)
    octave = sine(2 * f, n) * perc(n, 0.004, 0.06)
    glints, m = np.zeros(n), n_(0.15)
    for at, g, a in ((0.05, 4800, 1.0), (0.075, 6400, 0.8), (0.1, 7400, 0.6)):
        place(glints, a * sine(g, m) * perc(m, 0.001, 0.045), n_(at))
    zap = _crackle(n, rng, line([(0, 700), (0.06, 0)], n), 3000, 9000)
    return _n(bling) + 0.3 * octave + 0.18 * _n(glints) + 0.25 * _n(zap)


@sound("item", gain=-6)
def roulette_tick(rng):
    """Crisp tick: 1.5 ms band-passed click plus a tiny 2.5 kHz blip (12 ms decay)."""
    n = n_(0.07)
    return 0.7 * _n(_burst(n, rng, 0.0015, 1500, 9000)) + 0.6 * sine(2500, n) * perc(n, 0.0005, 0.012)


@sound("item")
def roulette_lock(rng):
    """Lock-in stab: click + bright C6+G6 chord (bell-ish partials, 2 % pitch snap, 0.1 s decay) with a C7 glint."""
    n = n_(0.5)
    snap = line([(0, 1.02), (0.012, 1.0)], n)

    def tone(f):
        return harmonics(f * snap, n, [1, 0.45, 0.25, 0.12])

    chord = (tone(1046.5) + 0.8 * tone(1568.0)) * perc(n, 0.002, 0.1)
    glint = sine(2093, n) * perc(n, 0.002, 0.05)
    return 0.5 * _n(_burst(n, rng, 0.002, 2000, 10000)) + _n(chord) + 0.2 * glint


# --------------------------------------------------------------------------- spark rocket
@sound("item")
def rocket_launch(rng):
    """Ignition pop (120→50 Hz thump + crack), gritty 900 Hz roar (20 ms rise, ~0.6 s fade) and a whistle
    rising 800→1600 Hz as the rocket leaves."""
    n = n_(0.9)
    pop = _boom(n, 120, 50, 0.06, 0.05, 0.001) + 0.5 * _n(_burst(n, rng, 0.004, 300, 8000))
    roar = eq(noise(n, rng, -3) * perc(n, 0.02, 0.2), lambda f: bp(f, 900, 0.7) * lp(f, 5000))
    roar = drive(_n(roar), 1.8)
    f = expo([(0, 800), (0.6, 1600), (0.9, 1700)], n)
    whistle = (sine(f, n) + 0.2 * sine(2 * f, n)) * line([(0, 0), (0.08, 1), (0.5, 0.6), (0.9, 0)], n)
    return _n(pop) + 0.6 * roar + 0.2 * whistle


@sound("loop", loop=True, pitch=True)
def rocket_flight_loop(rng):
    """Jet roar: pink noise peaked at 1.5 kHz with turbulent 60/13 Hz amplitude jitter, low rumble and a 1.2 kHz
    whistle with 6 Hz vibrato; 2 s, band-limited to 9 kHz for playback-rate shifting."""
    n = n_(2.0)
    roar = eq(noise(n, rng, -3), lambda f: bp(f, 1500, 0.6) * lp(f, 9000, 6) * hp(f, 150), circular=True)
    roar *= 1 + 0.25 * smooth_rand(n, 60, rng, True) + 0.12 * smooth_rand(n, 13, rng, True)
    rumble = eq(noise(n, rng, -6), lambda f: lp(f, 300) * hp(f, 40), circular=True)
    fv = grid(1200, n) * (1 + 0.004 * np.sin(TAU * phase(grid(6, n), n)))  # zero-mean vibrato
    whistle = sine(fv, n) + 0.2 * sine(2 * fv, n)
    return roar / np.std(roar) + 0.3 * rumble / np.std(rumble) + 0.55 * whistle


@sound("item", gain=2)
def rocket_explosion(rng):
    """Explosion: 10 ms broadband crack, noise body through a low-pass falling 3 kHz→300 Hz (≈1 s), 130→70 Hz punch
    over a 55→30 Hz sub boom, saturation, debris crackle thinning over 1.2 s, 0.8 s reverb. Also the generic one."""
    n = n_(2.0)
    crack = _burst(n, rng, 0.004, 400, 12000)
    body = svf(noise(n, rng) * perc(n, 0.003, 0.28), expo([(0, 3000), (1.0, 300), (2.0, 200)], n), 0.8)
    boom = _n(_boom(n, 55, 30, 0.5, 0.25, 0.003)) + 0.7 * _n(_boom(n, 130, 70, 0.12, 0.08, 0.002))
    debris = _crackle(n, rng, expo([(0, 220), (1.2, 2), (2.0, 1)], n), 1500, 6000) * line([(0, 1), (1.4, 0)], n)
    x = drive(_n(0.7 * _n(crack) + 1.2 * _n(body) + 0.5 * _n(boom)), 1.6) + 0.25 * _n(debris)
    return reverb(x, rng, t60=0.8, wet=0.25, predelay=0.012)


@sound("item")
def lockon_beep(rng):
    """Urgent lock-on beep: two band-limited pulse voices at 2200/2270 Hz (70 Hz roughness), 80 ms, 7 kHz low-pass."""
    n = n_(0.09)
    tone = pulse(2200, n, 0.35) + pulse(2270, n, 0.35, ph0=0.25)
    env = line([(0, 0), (0.002, 1), (0.07, 1), (0.085, 0)], n)
    return eq(tone * env, lambda f: lp(f, 7000, 4) * hp(f, 400))


# --------------------------------------------------------------------------- leech coil
@sound("loop", loop=True)
def leech_hum_loop(rng):
    """Draining tether hum: buzzy 100 Hz transformer stack through a band-pass that snaps up to 2 kHz and drains
    down to 250 Hz twice a second ('wub'), plus faint high crackle; 2 s."""
    n = n_(2.0)
    k = np.arange(1, 41)
    hum = drive(_n(harmonics(grid(100, n), n, np.where(k == 2, 1.6, 1.0) * k ** -0.8, fmax=8000)), 1.5)

    def wub(T, F):
        p = (2 * T) % 1.0  # 2 Hz cycle: fast smooth rise in 10 %, slow exponential drain
        lfo = np.where(p < 0.1, np.sin(np.pi * p / 0.2) ** 2, np.exp(-(p - 0.1) / 0.25))
        return 0.15 + bp(F, 250 * 8 ** lfo, 1.5)

    hum = stft_eq(hum, wub, circular=True)
    crackle = _crackle(n, rng, 25, 2500, 8000, circular=True)
    return _n(hum) + 0.12 * _n(crackle)


@sound("item")
def leech_zap(rng):
    """Tether attach zap: FM sweep 3 kHz→300 Hz in 150 ms (index 3→1), 3 ms noise crack and a short metallic
    ring at 1.2/1.85/2.7 kHz."""
    n = n_(0.55)
    f = expo([(0, 3000), (0.15, 300), (0.4, 250)], n)
    zap = fm(f, n, ratio=0.5, index=line([(0, 3), (0.15, 1)], n)) * perc(n, 0.001, 0.1)
    crack = _burst(n, rng, 0.003, 1000, 12000)
    ring = _ring(n, rng, [1200, 1850, 2700], [1, 0.7, 0.5], [0.12, 0.09, 0.07])
    return _n(zap) + 0.6 * _n(crack) + 0.25 * _n(ring)


# --------------------------------------------------------------------------- shock orb
@sound("loop", loop=True)
def orb_hum_loop(rng):
    """Plasma warble: FM 150 Hz carrier / 75 Hz modulator with the index wobbling at 3 Hz, a counter-phase
    octave voice, 3 Hz tremolo, light saturation and faint high crackle; 2 s."""
    n = n_(2.0)
    lfo = np.sin(TAU * phase(grid(3, n), n))
    a = fm(grid(150, n), n, fmod=grid(75, n), index=2.2 + 1.2 * lfo)
    b = fm(grid(300, n), n, fmod=grid(150, n), index=1.5 - 0.8 * lfo)
    x = drive(_n((a + 0.4 * b) * (1 + 0.25 * lfo)), 2.0)
    x = eq(x, lambda f: lp(f, 6000), circular=True)
    crackle = _crackle(n, rng, 50, 3000, 9000, circular=True)
    return _n(x) + 0.1 * _n(crackle)


@sound("item")
def orb_zap(rng):
    """Orb hit: 1-8 kHz arc noise chopped by fast random gating, FM 'pew' falling 2 kHz→200 Hz, 90→45 Hz thump,
    0.4 s reverb."""
    n = n_(0.6)
    gate = np.clip(smooth_rand(n, 180, rng), 0, 1) ** 2
    arc = eq(noise(n, rng), _band(1000, 8000)) * gate * perc(n, 0.001, 0.12)
    pew = fm(expo([(0, 2000), (0.25, 200), (0.6, 180)], n), n, ratio=0.5,
             index=line([(0, 3), (0.25, 0.5)], n)) * perc(n, 0.002, 0.12)
    thump = _boom(n, 90, 45, 0.08, 0.07)
    return reverb(_n(arc) + 0.7 * _n(pew) + 0.8 * _n(thump), rng, t60=0.4, wet=0.15)


@sound("item")
def orb_launch(rng):
    """Plasma ball fired: saturated FM 'thwum' rising 120→260 Hz then sinking to 150 Hz, a short 2-7 kHz arc
    crackle and a soft band-passed whoosh."""
    n = n_(0.7)
    f = expo([(0, 120), (0.08, 260), (0.45, 150)], n)
    blob = fm(f, n, ratio=1.0, index=line([(0, 3), (0.4, 1)], n)) * perc(n, 0.015, 0.14)
    arc = _crackle(n, rng, line([(0, 400), (0.12, 0)], n), 2000, 7000)
    whoosh = svf(noise(n, rng) * perc(n, 0.02, 0.08), expo([(0, 600), (0.2, 1500)], n), 1.2, "bpn")
    return drive(_n(blob), 1.5) + 0.3 * _n(arc) + 0.35 * _n(whoosh)


# --------------------------------------------------------------------------- EMP pulse
@sound("item")
def emp_charge(rng):
    """0.4 s wind-up peaking right at the blast: FM tone rising 200→1600 Hz (index 1→3), tremolo accelerating
    8→40 Hz, band-passed noise rising 500 Hz→6 kHz; 40 ms release after 0.4 s."""
    n = n_(0.45)
    f = expo([(0, 200), (0.4, 1600)], n)
    tone = fm(f, n, ratio=2.0, index=line([(0, 1), (0.4, 3)], n))
    trem = 0.6 + 0.4 * np.sin(TAU * phase(expo([(0, 8), (0.4, 40)], n), n))
    hiss = svf(noise(n, rng), expo([(0, 500), (0.4, 6000)], n), 2.0, "bpn")
    env = expo([(0, 0.05), (0.4, 1.0)], n) * line([(0, 1), (0.4, 1), (0.44, 0)], n)
    return (tone * trem + 0.35 * _n(hiss)) * env


@sound("item", gain=2)
def emp_blast(rng):
    """EMP pulse: 90→30 Hz whomp (+ octave), noise burst through a band-pass diving 6 kHz→200 Hz in 0.35 s,
    metallic inharmonic ring, electric crackle thinning over 1.2 s, 1 s reverb."""
    n = n_(1.5)
    whomp = _boom(n, 90, 30, 0.6, 0.3, 0.003) + 0.3 * _boom(n, 180, 60, 0.6, 0.2, 0.003)
    burst = svf(noise(n, rng) * perc(n, 0.001, 0.25), expo([(0, 6000), (0.35, 200), (1.5, 150)], n), 1.5, "bpn")
    ring = _ring(n, rng, [523, 811, 1197, 1660, 2390, 3150], [1, .8, .7, .5, .35, .25], [.8, .6, .5, .4, .3, .25])
    crackle = _crackle(n, rng, expo([(0, 150), (1.2, 1)], n), 2000, 8000) * line([(0, 1), (1.3, 0)], n)
    x = _n(whomp) + 0.8 * _n(burst) + 0.2 * _n(ring) + 0.25 * _n(crackle)
    return reverb(fades(x, 0, 0.4), rng, t60=1.0, wet=0.2)  # the 0.8 s ring must not be cut off


# --------------------------------------------------------------------------- goo bomb
@sound("item", gain=-3)
def goo_beep(rng):
    """Fuse beep: 1.4 kHz sine + 25 % third harmonic, 2 ms attack, 45 ms hold, 18 ms release."""
    n = n_(0.07)
    return (sine(1400, n) + 0.25 * sine(4200, n)) * line([(0, 0), (0.002, 1), (0.047, 1), (0.065, 0)], n)


@sound("item", gain=1)
def goo_explosion(rng):
    """Goo bomb: rounder boom (70→35 Hz + short low-passed noise body), a wet splat of noise through a resonant
    band-pass wobbling 300-1600 Hz, and a spray of droplets; 0.6 s reverb."""
    n = n_(1.5)
    body = svf(noise(n, rng) * perc(n, 0.004, 0.15), expo([(0, 1500), (0.5, 200), (1.5, 150)], n), 0.7)
    boom = _n(_boom(n, 70, 35, 0.3, 0.2, 0.003)) + 0.6 * _n(body)
    splat = svf(noise(n, rng) * perc(n, 0.002, 0.12), 700 * 2 ** (1.2 * smooth_rand(n, 40, rng)), 3.0, "bpn")
    drops = _drops(n, rng, 28, 0.05, 0.9, 900, 2600)
    return reverb(0.6 * _n(boom) + _n(splat) + 0.35 * _n(drops), rng, t60=0.6, wet=0.15)


# --------------------------------------------------------------------------- oil slick
@sound("item")
def oil_splat(rng):
    """Oil dropped: pink noise through a resonant band-pass sweeping 900→250 Hz in 120 ms (wet 'splotch'), a soft
    80 Hz thud and a few low droplets."""
    n = n_(0.5)
    splotch = svf(noise(n, rng, -3) * perc(n, 0.003, 0.08), expo([(0, 900), (0.12, 250), (0.5, 200)], n), 4.0, "bpn")
    thud = _boom(n, 80, 50, 0.05, 0.05)
    drops = _drops(n, rng, 8, 0.06, 0.4, 400, 900, 0.012)
    return _n(splotch) + 0.5 * _n(thud) + 0.25 * _n(drops)


@sound("item")
def oil_slip(rng):
    """Slipping through oil: pink noise in a resonant band-pass wobbling around 500 Hz with gurgling AM, plus a
    fading 1.8 kHz tyre squeal; 1.2 s."""
    n = n_(1.2)
    fc = 500 * 2 ** (0.8 * smooth_rand(n, 18, rng))
    gurgle = svf(noise(n, rng, -3), fc, 5.0, "bpn") * (0.6 + 0.4 * np.abs(smooth_rand(n, 25, rng)))
    fs = 1800 * (1 + 0.05 * smooth_rand(n, 8, rng) + 0.01 * smooth_rand(n, 35, rng))
    squeal = (sine(fs, n) + 0.3 * sine(2 * fs, n)) * line([(0, 0), (0.1, 1), (1.1, 0)], n) ** 2
    env = line([(0, 0), (0.03, 1), (0.8, 0.7), (1.2, 0)], n)
    return (_n(gurgle) + 0.15 * squeal) * env


# --------------------------------------------------------------------------- spring pad
@sound("item")
def spring_boing(rng):
    """Cartoon boing: saw + sine rising 220→440 Hz in 0.25 s with 14 Hz vibrato (15 %, slowly decaying), a jaw-harp
    formant (band-pass at 3× pitch) and metallic twang partials."""
    n = n_(1.0)
    t = t_(n)
    f = expo([(0, 220), (0.25, 440), (0.8, 420)], n) * (1 + 0.15 * np.exp(-t / 0.4) * np.sin(TAU * 14 * t))
    buzz = svf(saw(f, n), 3 * f, 3.0, "bpn") + 0.6 * sine(f, n)
    twang = _ring(n, rng, [1310, 2050, 2870, 3700], [1, .6, .4, .3], [.25, .2, .15, .12])
    return (_n(buzz) * perc(n, 0.003, 0.25) + 0.15 * _n(twang)) * line([(0, 1), (0.7, 1), (1.0, 0)], n)


@sound("item")
def item_drop(rng):
    """Plastic thunk: low-Q modal ring (180/410/730/1150 Hz) struck by a soft exciter, 110→70 Hz thud, tiny click."""
    n = n_(0.3)
    body = _ring(n, rng, [180, 410, 730, 1150], [1, .7, .5, .3], [.06, .04, .025, .015])
    thud = _boom(n, 110, 70, 0.04, 0.05)
    click = _burst(n, rng, 0.0008, 3000, 12000)
    return _n(body) + 0.6 * _n(thud) + 0.3 * _n(click)


# --------------------------------------------------------------------------- shield bubble
@sound("item")
def shield_up(rng):
    """Bubble forming: a sine gliding 300→900 Hz with two quieter, unevenly detuned twins (shimmer, no pumping) under
    a swell, FM sparkle with 12 Hz shimmer, a noise whoosh opening 300 Hz→6 kHz, settling into a 225 Hz hum."""
    n = n_(0.8)
    f = expo([(0, 300), (0.5, 900), (0.8, 900)], n)
    tones = sum(a * sine(f * d, n, ph) for d, a, ph in ((1, 1.0, 0), (1.006, 0.3, 0.3), (0.9965, 0.3, 0.6)))
    env = line([(0, 0), (0.25, 1), (0.55, 0.8), (0.8, 0)], n)
    sparkle = fm(4 * f, n, ratio=1.5, index=1.0) * (0.5 + 0.5 * np.sin(TAU * 12 * t_(n)))
    sparkle *= line([(0, 0), (0.3, 1), (0.6, 0)], n)
    whoosh = svf(noise(n, rng) * line([(0, 0), (0.3, 1), (0.6, 0)], n), expo([(0, 300), (0.4, 6000)], n), 0.7)
    hum = sine(225, n) * line([(0, 0), (0.45, 0), (0.55, 1), (0.8, 0)], n)
    return _n(tones) * env + 0.15 * _n(sparkle) + 0.3 * _n(whoosh) + 0.4 * hum


@sound("item")
def shield_hit(rng):
    """'Dink-ziiing': inharmonic FM bell at 1.1 kHz (ratio 1.41) with fast decay, a 2.3 kHz ring with 30 Hz energy
    tremolo, 3 ms noise burst and a 110→60 Hz thump."""
    n = n_(0.6)
    dink = fm(1100, n, ratio=1.41, index=line([(0, 4), (0.1, 0.5)], n)) * perc(n, 0.0005, 0.08)
    zing = fm(2300, n, ratio=1.41, index=2.0) * perc(n, 0.002, 0.2) * (0.7 + 0.3 * np.sin(TAU * 30 * t_(n)))
    burst = _burst(n, rng, 0.003, 2000, 9000)
    thump = _boom(n, 110, 60, 0.05, 0.05)
    return _n(dink) + 0.45 * _n(zing) + 0.4 * _n(burst) + 0.5 * _n(thump)


@sound("item")
def shield_pop(rng):
    """Bubble pop: sine diving 1500→300 Hz in 40 ms, 1.5 ms crack, glassy two-partial tinkles (3-6 kHz) scattering
    and fading over 0.5 s."""
    n = n_(0.6)
    pop = sine(expo([(0, 1500), (0.04, 300), (0.6, 250)], n), n) * perc(n, 0.001, 0.03)
    crack = _burst(n, rng, 0.0015, 1000, 6000)
    tinkles, m = np.zeros(n), n_(0.12)
    for i in range(12):
        at, f = 0.03 + 0.47 * (i / 12) ** 1.3 + rng.uniform(0, 0.03), rng.uniform(3000, 6000)
        tk = (sine(f, m) + 0.3 * sine(2.32 * f, m)) * perc(m, 0.0008, rng.uniform(0.02, 0.06))
        place(tinkles, tk, n_(at), gain=(1 - at / 0.6) ** 1.5)
    return _n(pop) + 0.4 * _n(crack) + 0.3 * _n(tinkles)


# --------------------------------------------------------------------------- storm strike
@sound("item", gain=2)
def storm_strike(rng):
    """Lightning on a car: two broadband cracks 18 ms apart (return strokes), dense crackling sizzle thinning over
    300 ms, 60→35 Hz boom with low rumble, FM zap falling 1.8 kHz→400 Hz, 1.2 s reverb."""
    n = n_(1.5)
    crack = _burst(n, rng, 0.006, 800, 14000)
    place(crack, 0.7 * _burst(n_(0.05), rng, 0.004, 1200, 14000), n_(0.018))
    sizzle = _crackle(n, rng, expo([(0, 2500), (0.3, 30), (1.5, 1)], n), 2000, 9000)
    sizzle *= (0.6 + 0.4 * np.clip(smooth_rand(n, 90, rng), 0, 1)) * line([(0, 1), (0.45, 1), (0.6, 0)], n)
    boom = _n(_boom(n, 60, 35, 0.3, 0.2, 0.004)) + 0.6 * _n(svf(noise(n, rng) * perc(n, 0.01, 0.3), 400, 0.7))
    zap = fm(expo([(0, 1800), (0.2, 400), (1.5, 350)], n), n, ratio=0.5,
             index=line([(0, 4), (0.2, 1)], n)) * perc(n, 0.001, 0.15)
    x = _n(crack) + 0.7 * _n(sizzle) + 0.45 * _n(boom) + 0.45 * _n(zap)
    return reverb(x, rng, t60=1.2, wet=0.25)
