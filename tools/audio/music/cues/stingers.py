"""One-shot stingers (no loop): results_jingle (~6 s), podium_fanfare (~8 s), cup_victory (~15 s)."""
import drums as D
import instruments as I
from song import Song


def results_jingle():
    """C major, 120 BPM: arpeggio flourish, I-IV-V into a ringing Cmaj9."""
    s = Song("results_jingle", bpm=120, key="C major", seed=11, loop=False, tail=2.8)
    s.section("jingle", 2, "C:0.5 F:0.25 G:0.25 Cmaj9:1")
    s.verb = dict(rt=2.2, damp=7000.0, predelay=0.015)
    mel = "C5:1 E5:1 G5:1 C6:1 -:2 G5:1 C6:1 A5:2 C6:2 B5:2 D6:2 | E6:16"
    lead = s.track("lead", I.pluck, level=-8, verb=-10, delay=-14, voices=4, detune=12.0, cutoff=700.0,
                   peak=9000.0, decay=0.5, release=0.6, eq=[("hp", 250)])
    s.notes(lead, mel, "jingle")
    bells = s.track("bells", I.fm, level=-12, verb=-8, eq=[("hp", 500)], decay=2.0)
    s.notes(bells, mel, "jingle", transpose=12, v=0.6)
    brass = s.track("brass", I.brass, level=-10, verb=-10, width=1.3, eq=[("hp", 200)], release=0.8)
    s.chords(brass, "jingle", lo=55, hi=76, nv=4, gate=0.95)
    pad = s.track("pad", I.pad, level=-13, verb=-8, chorus=0.4, width=1.3, eq=[("hp", 200)], attack=0.05,
                  release=2.0, cutoff=2000.0)
    s.chords(pad, (1, 1), lo=52, hi=76, nv=4)
    bass = s.track("bass", I.bass, level=-7, gate=0.9, release=0.3, cutoff=200.0, peak=1500.0)
    s.rel(bass, "R:8 R:4 R:4", (0, 1), floor=33)
    s.rel(bass, "R:16", (1, 1), floor=33, x=(("release", 1.8),))
    kick = s.track("kick", D.kick, level=-6, tune=52.0)
    s.beat(kick, "X.......X...X...|X...............", "jingle")
    snare = s.track("snare", D.snare, level=-9, verb=-12, gated=0.3)
    s.beat(snare, "............x.xX", (0, 1))
    s.hit(s.track("crash", D.crash, level=-14, verb=-14, width=1.3), 1)
    s.hit(s.track("impact", D.impact, level=-14, verb=-14), 1)
    return s


def podium_fanfare():
    """Bb major, 110 BPM: brass fanfare call, tom/snare roll, big Bbadd9 with crash."""
    s = Song("podium_fanfare", bpm=110, key="Bb major", seed=12, loop=False, tail=2.2)
    s.section("fanfare", 3, "Bb Eb:0.5 F:0.5 Bbadd9")
    s.verb = dict(rt=2.1, damp=6500.0, predelay=0.02)
    mel = "F4:1 F4:1 F4:2 Bb4:4 D5:4 F5:4 | G5:6 F5:2 Eb5:4 C5:4 | D5:4 F5:4 Bb5:8"
    horn = s.track("horn", I.brass, level=-7, verb=-10, delay=-16, eq=[("hp", 200), ("peak", 2500, 1.0, 2.0)],
                   voices=7, detune=14.0, cutoff=700.0, peak=5000.0, release=0.4)
    s.notes(horn, mel, "fanfare", gate=0.9)
    s.notes(horn, mel, "fanfare", transpose=-12, gate=0.9, v=0.55)
    stab = s.track("chords", I.brass, level=-11, verb=-8, width=1.4, eq=[("hp", 220)], release=1.2)
    s.chords(stab, "fanfare", lo=53, hi=74, nv=4, gate=0.95)
    pad = s.track("pad", I.pad, level=-13, verb=-6, chorus=0.4, width=1.3, eq=[("hp", 200)], attack=0.3,
                  release=1.5, cutoff=1800.0)
    s.chords(pad, "fanfare", lo=50, hi=74, nv=4)
    bass = s.track("bass", I.bass, level=-7, gate=0.95, release=0.3, cutoff=160.0, peak=900.0)
    s.rel(bass, "R:16 R:8 R:8", (0, 2), floor=34)
    s.rel(bass, "R:16", (2, 1), floor=34, x=(("release", 1.4),))
    toms = s.track("toms", D.tom, level=-9, verb=-8, decay=0.5, bend=0.4)
    s.notes(toms, "-:16 | F2:2 F2:2 F2:2 F2:2 F2:1 F2:1 F2:1 F2:1 C3:1! C3:1 C3:1! C3:1 |", "fanfare")
    snare = s.track("snare", D.snare, level=-10, verb=-10)
    s.beat(snare, "................|........xxxxXXXX|", "fanfare")
    kick = s.track("kick", D.kick, level=-6, tune=46.0, decay=0.5)
    s.beat(kick, "X...............|................|X...............", "fanfare")
    s.hit(s.track("crash", D.crash, level=-13, verb=-12, width=1.4, decay=2.0), 2)
    s.hit(s.track("impact", D.impact, level=-13, verb=-12), 2)
    return s


def cup_victory():
    """D major, 120 BPM: the menu theme turned major, full band, climbing to a ringing D."""
    s = Song("cup_victory", bpm=120, key="D major", seed=13, loop=False, tail=3.5)
    s.section("victory", 6, "D Bm7 Gmaj7 A G:0.5 A:0.5 D")
    s.verb = dict(rt=2.6, damp=7000.0, predelay=0.02)
    s.delay = dict(beats=0.75, fb=0.35, hp=400.0, lp=5000.0)
    mel = ("-:2 D5:2 E5:2 F#5:6 E5:2 D5:2 | C#5:4 D5:8 A4:4 | -:2 B4:2 D5:2 F#5:6 G5:2 F#5:2 |"
           "E5:4 A4:4 C#5:8 | B4:2 D5:2 G5:4 A5:2 C#6:2 E6:4 | D6:16")
    lead = s.track("lead", I.lead, level=-7, mono=True, verb=-10, delay=-12, gate=0.95,
                   eq=[("hp", 200), ("peak", 2800, 1.0, 2.0)], voices=3, detune=12.0, cutoff=2400.0,
                   release=0.9)
    s.notes(lead, mel, "victory")
    bells = s.track("bells", I.fm, level=-12, verb=-8, delay=-12, eq=[("hp", 500)], decay=1.6)
    s.notes(bells, mel, "victory", transpose=12, v=0.55)
    pad = s.track("pad", I.pad, level=-11, duck=0.4, verb=-8, chorus=0.4, width=1.3, eq=[("hp", 200)],
                  attack=0.15, release=2.5, cutoff=2000.0)
    s.chords(pad, "victory", lo=52, hi=76, nv=4)
    arp = s.track("arp", I.pluck, level=-15, delay=-9, duck=0.3, eq=[("hp", 350)], voices=2, decay=0.2)
    s.arp(arp, (0, 5), "0 1 2 3 4 3 2 1", lo=62)
    bass = s.track("bass", I.bass, level=-6, duck=0.45, gate=0.8, cutoff=170.0, peak=1800.0, release=0.5)
    s.rel(bass, "R:2 R+12:2 " * 4, (0, 5), floor=33)
    s.rel(bass, "R:16", (5, 1), floor=33, gate=1.0, x=(("release", 2.0),))
    kick = s.track("kick", D.kick, level=-5, sidechain=True, tune=55.0, decay=0.32)
    s.beat(kick, "X...X...X...X...", (0, 5))
    s.beat(kick, "X...............", (5, 1))
    snare = s.track("snare", D.snare, level=-8, verb=-14, gated=0.28)
    s.beat(snare, "....X.......X...", (0, 4))
    s.beat(snare, "x.x.x.x.xxxxXXXX", (4, 1))
    clap = s.track("clap", D.clap, level=-12, verb=-12)
    s.beat(clap, "....X.......X...", (0, 4))
    hat = s.track("hat", D.hat, level=-17, pan=0.25, vel_human=0.1)
    s.beat(hat, "xoxoxoxoxoxoxoxo", (0, 5))
    toms = s.track("toms", D.tom, level=-11, verb=-10)
    s.notes(toms, "-:8 A2:2 F#2:2 D2:2 A1:2", (4, 1))
    crash = s.track("crash", D.crash, level=-15, verb=-14, width=1.4, decay=2.4)
    s.hit(crash, 0)
    s.hit(crash, 5)
    s.hit(s.track("impact", D.impact, level=-13, verb=-12), 5)
    return s
