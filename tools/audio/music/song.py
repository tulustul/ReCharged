"""Arrangement format, sequencer, mixer and mastering.

A Song is a list of sections (name, bars, chord progression) plus tracks. Parts are written as
compact text patterns and tiled over sections:

* drum grid  `s.beat(tr, "X...x...|..o.X...", "A B")`  one char per 16th: X=1.0 x=0.78 o=0.5 .=rest
* notes      `s.notes(tr, "A4:2 C5:2! [E4,G4]:4 -:4 D5:2~", ...)`  len in 16ths; ! accent, ? soft,
             ~ slide/legato into the note, `-` rest, [..] chord
* relative   `s.rel(tr, "R:2 R+12:1 5:1 N-1:2", ...)`  chord-relative: R root (slash bass if any),
             3/5/7/9 chord tones, N next chord's root, +/-n semitones; root placed in [floor, floor+12)
* chords     `s.chords(tr, where, rhythm=None)` voice-led voicings (held, or hit on a grid)
* arp        `s.arp(tr, where, "0 1 2 3 2 1")` indices into the chord tones over `span` octaves

Times are in beats (quarter notes); one bar = 4 beats; bars and sections are 0-based internally.
Rendering is deterministic: all randomness comes from dsp.rng keyed by names/indices.
"""
from __future__ import annotations

import math
import re
from dataclasses import dataclass, field
from itertools import product
from typing import Callable, NamedTuple

import numpy as np

import dsp
from dsp import SR


class Note(NamedTuple):
    t: float
    d: float
    p: float = 60.0
    v: float = 0.78
    slide: bool = False
    x: tuple = ()  # per-note instrument param overrides ((key, value), ...)


# ------------------------------------------------------------------ theory

PC = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}
QUAL = {
    "": (0, 4, 7), "m": (0, 3, 7), "dim": (0, 3, 6), "aug": (0, 4, 8), "sus2": (0, 2, 7),
    "sus4": (0, 5, 7), "5": (0, 7), "6": (0, 4, 7, 9), "m6": (0, 3, 7, 9), "7": (0, 4, 7, 10),
    "maj7": (0, 4, 7, 11), "m7": (0, 3, 7, 10), "m7b5": (0, 3, 6, 10), "7sus4": (0, 5, 7, 10),
    "add9": (0, 4, 7, 14), "madd9": (0, 3, 7, 14), "9": (0, 4, 7, 10, 14), "maj9": (0, 4, 7, 11, 14),
    "m9": (0, 3, 7, 10, 14), "69": (0, 4, 7, 9, 14), "m11": (0, 3, 7, 10, 14, 17),
    "13": (0, 4, 10, 14, 21), "7#9": (0, 4, 7, 10, 15), "7b9": (0, 4, 7, 10, 13),
}


def pc(name):
    return (PC[name[0].upper()] + name[1:].count("#") - name[1:].count("b")) % 12


def midi(name):
    """'A4' -> 69, 'C#5' -> 73, 'Bb-1' ok."""
    m = re.fullmatch(r"([A-Ga-g][#b]?)(-?\d)", name)
    if not m:
        raise ValueError(f"bad note {name!r}")
    return pc(m.group(1)) + 12 * (int(m.group(2)) + 1)


class Chord(NamedTuple):
    root: int
    iv: tuple
    bass: int
    name: str


def chord(sym):
    m = re.fullmatch(r"([A-G][#b]?)([^/]*)(?:/([A-G][#b]?))?", sym)
    if not m or m.group(2) not in QUAL:
        raise ValueError(f"bad chord {sym!r}")
    r = pc(m.group(1))
    return Chord(r, QUAL[m.group(2)], pc(m.group(3)) if m.group(3) else r, sym)


def tone(ch, which):
    """Interval above the root of the chord's 3rd/5th/7th/9th (sensible fallbacks)."""
    ivs = set(ch.iv)
    pick = {"3": (4, 3, 5, 2), "5": (7, 6, 8), "7": (10, 11, 9), "9": (14, 13, 15)}[which]
    for i in pick:
        if i in ivs:
            return i
    return {"3": 4, "5": 7, "7": 12, "9": 14}[which]


def voicing(ch, prev, lo, hi, nv):
    """Voice-led chord voicing: guide tones first, close to the previous voicing, no mud."""
    order = [tone(ch, "3") % 12, tone(ch, "7") % 12] + [i % 12 for i in ch.iv if i > 12]
    order += [7, 0]
    pcs = []
    for i in order:
        if i in [j % 12 for j in ch.iv] and i not in pcs:
            pcs.append(i)
    while len(pcs) < nv:
        pcs.append(pcs[len(pcs) % max(1, len(pcs))])
    pcs = pcs[:nv]
    opts = [[p for p in range(lo, hi + 1) if p % 12 == (ch.root + i) % 12] for i in pcs]
    best, best_cost = None, 1e9
    centre = (lo + hi) / 2
    for c in product(*opts):
        c = sorted(c)
        if len(set(c)) < len(c):
            continue
        cost = 0.25 * abs(np.mean(c) - centre) + 0.5 * max(0, c[-1] - c[0] - 19)
        cost += sum(3.0 for a, b in zip(c, c[1:]) if b - a < 3 and a < 57)
        if prev:
            cost += sum(abs(a - b) for a, b in zip(c, prev)) if len(prev) == len(c) else 0
        if cost < best_cost:
            best, best_cost = c, cost
    return best


# ------------------------------------------------------------------ text patterns

TOK = re.compile(r"(\[[^\]]+\]|[A-Ga-g][#b]?-?\d|-|R|N|[3579])([+-]\d+)?(?::([\d.]+))?([!?~]*)$")


def _mods(m, v):
    return (1.0 if "!" in m else 0.55 if "?" in m else v), "~" in m


def parse(text, v=0.78):
    """Tokens -> [(step, len, pitch-or-token, offset, vel, slide)], length in steps."""
    out, step = [], 0.0
    for tok in text.replace("|", " ").split():
        m = TOK.match(tok)
        if not m:
            raise ValueError(f"bad token {tok!r}")
        head, off, ln, mods = m.group(1), int(m.group(2) or 0), float(m.group(3) or 1), m.group(4)
        vel, slide = _mods(mods, v)
        if head != "-":
            heads = head[1:-1].split(",") if head.startswith("[") else [head]
            for h in heads:
                out.append((step, ln, h.strip(), off, vel, slide))
        step += ln
    return out, step


def grid(pattern):
    """Drum grid -> [(step, vel)], length in steps."""
    vel = {"X": 1.0, "x": 0.78, "o": 0.5}
    cells = [c for c in pattern if c not in "| "]
    return [(i, vel[c]) for i, c in enumerate(cells) if c in vel], len(cells)


# ------------------------------------------------------------------ song

@dataclass
class Track:
    name: str
    inst: Callable
    level: float = -12.0            # stem loudness, LU relative to REF_LUFS
    mono: bool = False              # render legato phrases instead of single notes
    params: dict = field(default_factory=dict)
    eq: list = field(default_factory=list)
    auto: list = field(default_factory=list)   # [(mode, [(beat, hz), ...], q)] filter sweeps
    pan: float = 0.0
    width: float = 1.0
    chorus: float = 0.0
    verb: float | None = None       # send level dB
    delay: float | None = None      # send level dB
    duck: float = 0.0               # sidechain depth 0..1 (from sidechain tracks)
    duck_release: float = 0.18
    sidechain: bool = False         # this track's onsets drive the ducking
    swing: float | None = None      # off-16th delay ratio (0.5 straight); None = song swing
    human: float = 0.0              # timing jitter, seconds (sd)
    vel_human: float = 0.0
    gate: float = 1.0               # note-length factor for text patterns
    choke: str | None = None        # notes cut at this other track's next onset
    notes: list = field(default_factory=list)


REF_LUFS = -20.0


class Song:
    def __init__(self, name, bpm, key="", seed=0, loop=True, tail=4.0, swing=0.5):
        self.name, self.bpm, self.key, self.seed, self.loop, self.tail = name, bpm, key, seed, loop, tail
        self.swing = swing
        self.sections, self.tracks = [], {}
        self.verb = dict(rt=2.2, damp=7000.0, predelay=0.02)
        self.delay = dict(beats=0.75, fb=0.35, hp=300.0, lp=5000.0)
        self.target = -14.0

    # -- structure
    def section(self, name, bars, chords):
        start = sum(s[2] for s in self.sections)
        prog, b = [], start * 4.0
        for tok in chords.split():
            sym, _, ln = tok.partition(":")
            prog.append((b, float(ln or 1) * 4.0, chord(sym)))
            b += float(ln or 1) * 4.0
        if abs(b - (start + bars) * 4.0) > 1e-9:
            raise ValueError(f"{self.name}/{name}: chords span {b / 4 - start} bars, section has {bars}")
        self.sections.append((name, start, bars, prog))

    @property
    def bars(self):
        return sum(s[2] for s in self.sections)

    def beats_to_s(self, b):
        return b * 60.0 / self.bpm

    def length(self):
        return int(round(self.beats_to_s(self.bars * 4.0) * SR))

    def chord_at(self, beat, offset=0):
        prog = [c for s in self.sections for c in s[3]]
        beat %= self.bars * 4.0
        i = max(k for k, c in enumerate(prog) if c[0] <= beat + 1e-9)
        return prog[(i + offset) % len(prog)][2]

    def segs(self, where):
        """'A B' (section names) | (bar, nbars) | list of those -> [(start_beat, n_beats)]."""
        if isinstance(where, str):
            by = {s[0]: s for s in self.sections}
            return [(by[w][1] * 4.0, by[w][2] * 4.0) for w in where.split()]
        if isinstance(where, tuple):
            return [(where[0] * 4.0, where[1] * 4.0)]
        return [s for w in where for s in self.segs(w)]

    def start(self, name):
        return next(s[1] for s in self.sections if s[0] == name)

    # -- tracks & placement
    def track(self, name, inst, **kw):
        params = {k: kw.pop(k) for k in list(kw) if k not in Track.__dataclass_fields__}
        self.tracks[name] = tr = Track(name, inst, params=params, **kw)
        return tr

    def _tile(self, tr, items, plen, where, skip, fn):
        skip = set(skip or ())
        for s0, n in self.segs(where):
            for k in range(math.ceil(n / plen - 1e-9)):
                for it in items:
                    t = s0 + k * plen + it[0]
                    if t < s0 + n - 1e-9 and int(t // 4) not in skip:
                        tr.notes.extend(fn(t, it))

    def beat(self, tr, pattern, where, p=60, step=0.25, gate=1.0, skip=None, x=()):
        hits, n = grid(pattern)
        self._tile(tr, [(i * step, v) for i, v in hits], n * step, where, skip,
                   lambda t, h: [Note(t, step * gate, p, h[1], False, x)])

    def notes(self, tr, text, where, transpose=0, step=0.25, skip=None, v=0.78, gate=None, x=()):
        evs, n = parse(text, v)
        g = tr.gate if gate is None else gate
        self._tile(tr, [(e[0] * step,) + e for e in evs], n * step, where, skip,
                   lambda t, e: [Note(t, e[2] * step * g, midi(e[3]) + e[4] + transpose, e[5], e[6], x)])

    def rel(self, tr, text, where, floor=40, step=0.25, skip=None, v=0.78, gate=None, x=()):
        evs, n = parse(text, v)
        g = tr.gate if gate is None else gate

        def mk(t, e):
            head, off = e[3], e[4]
            ch = self.chord_at(t, 1 if head == "N" else 0)
            base = ch.bass if head in "RN" else ch.root
            base = floor + (base - floor) % 12
            iv = 0 if head in "RN" else tone(ch, head)
            return [Note(t, e[2] * step * g, base + iv + off, e[5], e[6], x)]
        self._tile(tr, [(e[0] * step,) + e for e in evs], n * step, where, skip, mk)

    def chords(self, tr, where, lo=55, hi=79, nv=4, rhythm=None, step=0.25, gate=0.9, x=()):
        prev = None
        for s0, n in self.segs(where):
            for b, ln, ch in [c for s in self.sections for c in s[3]]:
                if not (s0 - 1e-9 <= b < s0 + n - 1e-9):
                    continue
                prev = voicing(ch, prev, lo, hi, nv)
                if rhythm is None:
                    tr.notes += [Note(b, ln * gate, p, 0.78, False, x) for p in prev]
                else:
                    hits, m = grid(rhythm)
                    for k in range(math.ceil(ln / (m * step))):
                        for i, vel in hits:
                            t = b + k * m * step + i * step
                            if t < b + ln - 1e-9:
                                tr.notes += [Note(t, step * gate * 2, p, vel, False, x) for p in prev]

    def arp(self, tr, where, pattern="0 1 2 3", step=0.25, lo=60, span=2, gate=0.6, accent=4, x=()):
        idx = [int(i) for i in pattern.split()]
        for s0, n in self.segs(where):
            for k in range(int(round(n / step))):
                t = s0 + k * step
                ch = self.chord_at(t)
                pcs = sorted({(ch.root + i) % 12 for i in ch.iv})
                tones = sorted(p for p in range(lo, lo + 12 * span) if p % 12 in pcs)
                vel = 0.95 if k % accent == 0 else 0.7
                tr.notes.append(Note(t, step * gate, tones[idx[k % len(idx)] % len(tones)], vel, False, x))

    def hit(self, tr, bar, beat=0.0, dur=1.0, p=60, v=1.0, x=()):
        tr.notes.append(Note(bar * 4.0 + beat, dur, p, v, False, x))

    def at(self, tr, names, offset=0.0, dur=1.0, p=60, v=1.0, x=()):
        for nm in names.split():
            self.hit(tr, self.start(nm), offset, dur, p, v, x)


# ------------------------------------------------------------------ rendering

CACHE: dict = {}


def _events(song, tr):
    """Beats -> seconds with swing, humanisation and choke; returns sorted Notes in seconds."""
    out = []
    swing = song.swing if tr.swing is None else tr.swing
    choke = sorted(n.t for n in song.tracks[tr.choke].notes) if tr.choke else []
    for nt in sorted(tr.notes):
        t, d = nt.t, nt.d
        st = t * 4.0
        if abs(st - round(st)) < 1e-6 and int(round(st)) % 2 == 1:
            t += (swing - 0.5) * 0.5
        if choke:
            j = np.searchsorted(choke, nt.t + 1e-6)
            nxt = choke[j] if j < len(choke) else choke[0] + song.bars * 4.0
            d = min(d, nxt - nt.t)
        r = dsp.rng(song.seed, tr.name, round(nt.t, 4), nt.p)  # by time, not index: shared notes of
        # a final-lap variant get the same humanisation as in the base loop (clean crossfades)
        dt = float(np.clip(r.normal(0.0, tr.human), -2 * tr.human, 2 * tr.human)) if tr.human else 0.0
        v = nt.v * (1.0 + r.normal(0.0, tr.vel_human)) if tr.vel_human else nt.v
        v = round(float(np.clip(v, 0.05, 1.0)) * 20) / 20
        out.append(Note(song.beats_to_s(t) + dt, song.beats_to_s(d), nt.p, v, nt.slide, nt.x))
    return out


def _phrases(evs, mono, gap):
    if not mono:
        return [[e] for e in evs]
    out = []
    for e in evs:
        if out and e.t - max(o.t + o.d for o in out[-1]) < gap:
            out[-1].append(e)
        else:
            out.append([e])
    return out


def render_stem(song, tr, n_out):
    segs = []
    for ph in _phrases(_events(song, tr), tr.mono, song.beats_to_s(0.75)):
        t0 = ph[0].t
        rel = tuple(Note(round(e.t - t0, 4), round(e.d, 4), e.p, e.v, e.slide) for e in ph)
        params = {**tr.params, **dict(ph[0].x)}
        key = (tr.inst.__module__, tr.inst.__name__, tuple(sorted(params.items())), rel)
        if key not in CACHE:
            CACHE[key] = tr.inst(rel, **params)
        segs.append((int(round(t0 * SR)), CACHE[key]))
    if not segs:
        return None
    lo = min(0, min(s for s, _ in segs))
    hi = max(n_out, max(s + a.shape[1] for s, a in segs))
    buf = np.zeros((2, hi - lo))
    for s, a in segs:
        buf[:, s - lo: s - lo + a.shape[1]] += a
    if not song.loop:
        return buf[:, -lo: -lo + n_out]
    out = np.zeros((2, n_out))
    j = 0
    while j < buf.shape[1]:                           # fold tails/pickups onto the loop
        pos = (j + lo) % n_out
        k = min(n_out - pos, buf.shape[1] - j)
        out[:, pos: pos + k] += buf[:, j: j + k]
        j += k
    return out


def _auto_curve(song, pts, n):
    t = np.arange(n) / SR
    bt, hz = zip(*pts)
    return np.exp(np.interp(t, [song.beats_to_s(b) for b in bt], np.log(hz)))


def mix(song, stems_out=None):
    """Render all tracks, normalise each stem to REF_LUFS + level, apply sidechain, sends,
    delay and plate returns. Returns the pre-master stereo mix."""
    n = song.length() + (0 if song.loop else int(song.tail * SR))
    spb = 60.0 / song.bpm
    keys = [int(round(song.beats_to_s(nt.t) * SR)) - int(0.002 * SR)
            for tr in song.tracks.values() if tr.sidechain for nt in tr.notes]
    out, vbus, dbus = np.zeros((2, n)), np.zeros((2, n)), np.zeros((2, n))
    for tr in song.tracks.values():
        x = render_stem(song, tr, n)
        if x is None:
            continue
        x = dsp.eq(x, tr.eq)
        for mode, pts, q in tr.auto:
            fc = _auto_curve(song, pts, n)
            w = int(0.2 * SR) if song.loop else 0     # warm-up so the filter state wraps too
            x = dsp.svf(np.concatenate([x[:, n - w:], x], axis=1), np.concatenate([fc[n - w:], fc]),
                        q, mode)[:, w:]
        if tr.chorus:
            x = dsp.chorus(x, mix=tr.chorus, loop=song.loop)
        x = dsp.width(x, tr.width)
        if tr.pan:
            x = dsp.pan(x, tr.pan)
        x *= dsp.undb(REF_LUFS + tr.level - dsp.lufs(x))
        if tr.duck:
            x *= dsp.duck_env(n, keys, tr.duck, release=tr.duck_release, loop=song.loop)
        if stems_out is not None:
            stems_out[tr.name] = x
        out += x
        if tr.verb is not None:
            vbus += x * dsp.undb(tr.verb)
        if tr.delay is not None:
            dbus += x * dsp.undb(tr.delay)
    if dbus.any():
        d = song.delay
        echo = dsp.pingpong(dbus, d["beats"] * spb, d["fb"], d["hp"], d["lp"])
        out += echo
        vbus += echo * dsp.undb(-8.0)
    if vbus.any():
        wet = dsp.convolve(vbus.mean(axis=0, keepdims=True).repeat(2, 0), dsp.reverb_ir(**song.verb))
        wet = dsp.eq(wet, [("hp", 220), ("lp", 9500)])
        out += wet * dsp.duck_env(n, keys, 0.35, release=0.25, loop=song.loop)
    return out


def master(x, loop, target=-14.0, ceiling=-2.0):
    """Clean-up EQ, mono lows, glue compression, soft clip and true-peak limiting, iterated to
    the loudness target."""
    x = dsp.eq(x, [("hp", 24, 0.7), ("low", 100, 0.7, -3.0)])
    m, s = 0.5 * (x[0] + x[1]), 0.5 * (x[0] - x[1])
    s = dsp.eq(s, [("hp", 110), ("hp", 110)])
    x = np.vstack([m + s, m - s])
    x *= dsp.undb(-18.0 - dsp.lufs(x))
    x = dsp.compress(x, thresh=-17.0, ratio=2.0, attack=0.015, release=0.2, loop=loop)
    gain, y = target - dsp.lufs(x), x
    for _ in range(5):
        y = dsp.limiter(dsp.softclip(x * dsp.undb(gain), -4.0), ceiling, loop=loop)
        err = target - dsp.lufs(y)
        if abs(err) < 0.05:
            break
        gain += err
    return y
