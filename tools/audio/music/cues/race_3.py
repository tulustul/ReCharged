"""race_3 "Short Circuit" -- breakbeat with reese bass, rave stabs and a 303 acid line.
G minor, 135 BPM, 64 bars.

intro 8 | verse 16 (i i VI VII, reese + stabs, square-lead hook) | drop 16 (iv VI i V7, acid) |
break 8 (pads, bells, filtered beat) | drop2 16 -> D7 resolves to Gm at the loop seam.
Final-lap variant: busier hats and ghosts, opener acid, stabs on every bar, extra toms, kick in break.
"""
import drums as D
import instruments as I
from song import Song

HOOK = ("D5:3 Bb4:3 G4:2 A4:2 Bb4:2 C5:4 | D5:3 F5:3 D5:2 C5:4 Bb4:4 |"
        "G5:3 F5:3 Eb5:2 D5:2 C5:2 Bb4:4 | C5:3 D5:3 F5:2 A5:4 G5:4")
DROP2_LEAD = ("G5:4 Eb5:4 C5:4 Eb5:4 | Bb5:6 G5:2 F5:4 Eb5:4 | D5:4 G5:4 Bb5:4 A5:4 | F#5:8 A5:4 C6:4 |"
              "G5:4 Eb5:4 C5:4 G5:4 | Bb5:6 C6:2 Bb5:4 G5:4 | A5:4 Bb5:4 D6:4 C6:4 | A5:8 F#5:8")
ACID = "R:1! R:1 R+12:1~ R+10:1 R:1 -:1 R+3:1! R:1 R+7:1~ R+5:1 R:1 -:1 R+10:1 R:1! -:1 R+12:1~"
BELLS = "D6:6 C6:2 Bb5:4 G5:4 | A5:6 Bb5:2 A5:4 F5:4 | G5:6 F5:2 Eb5:4 D5:4 | F#5:8 A5:8"
BREAK_K = "X.........X.....|..X.......X..x.."
BREAK_S = "....X..o.o..X..o|....X..o.o..X.o."


def build(final=False):
    s = Song("race_3", bpm=135, key="G minor", seed=3, swing=0.54)
    s.section("intro", 8, "Gm Gm Eb F " * 2)
    s.section("verse", 16, "Gm Gm Eb F " * 4)
    s.section("drop", 16, "Cm Eb Gm D7 " * 4)
    s.section("break", 8, "Ebmaj7 Dm7 Cm7 D7 " * 2)
    s.section("drop2", 16, "Cm Eb Gm D7 " * 4)
    s.verb = dict(rt=1.9, damp=6000.0, predelay=0.012)
    s.delay = dict(beats=0.75, fb=0.4, hp=500.0, lp=4000.0)
    beat = "intro verse drop drop2"
    fills = [23, 39, 63]

    kick = s.track("kick", D.kick, level=-5, sidechain=True, tune=49.0, decay=0.3, punch=4.0, drive=3.0)
    s.beat(kick, BREAK_K, beat, skip=fills)
    s.beat(kick, "X...X...X...X..." if final else BREAK_K, "break" if final else (46, 2))
    for b in fills:
        s.beat(kick, "X.........X.....", (b, 1))

    snare = s.track("snare", D.snare, level=-8.5, verb=-16, tone=185.0, decay=0.15, snap=0.6, human=0.002,
                    vel_human=0.06, eq=[("high", 4500, 0.7, -5.0)])
    s.beat(snare, BREAK_S, beat, skip=fills)
    s.beat(snare, "x...x...x...x...|x.x.x.x.xxxxXXXX", (46, 2))
    for b in fills:
        s.beat(snare, "....X...x.xxXXXX", (b, 1))
    if final:
        ghost = s.track("ghost", D.snare, level=-20, tone=220.0, decay=0.07, snap=0.3, human=0.004,
                        vel_human=0.2)
        s.beat(ghost, ".o.....o.o...o..|.o..o..........o", "verse drop drop2", skip=fills)

    chh = s.track("hat", D.hat, level=-19, pan=0.3, human=0.003, vel_human=0.12, decay=0.04)
    s.beat(chh, "x.x.x.x.x.x.x.x." if not final else "xoxoxoxoxoxoxoxo", "intro verse")
    s.beat(chh, "xoxoxoxoxoxoxoxo", "drop drop2 break")
    ohh = s.track("ohat", D.hat, level=-18, pan=-0.25, choke="hat", open_=True)
    s.beat(ohh, "..x...x...x...x.", "drop drop2", gate=2.0)
    ride = s.track("ride", D.ride, level=-21, pan=-0.4, vel_human=0.1)
    s.beat(ride, "x.x.x.x.x.x.x.x.", "drop2" + (" drop" if final else ""))

    toms = s.track("toms", D.tom, level=-12, verb=-12, pan=0.2, decay=0.3, bend=0.9)
    s.notes(toms, "-:12 D3:1 Bb2:1 G2:1 D2:1", (7, 1))
    if final:
        s.notes(toms, "-:16 -:12 D3:1 Bb2:1 G2:1 D2:1", "verse drop2", skip=fills)
    else:
        s.notes(toms, "-:12 D3:1 Bb2:1 G2:1 D2:1", (31, 1))
    crash = s.track("crash", D.crash, level=-16, verb=-18, width=1.3)
    s.at(crash, "verse drop break drop2")
    for b in ([28, 32, 36, 52, 56, 60] if final else [32, 56]):
        s.hit(crash, b)
    fx = s.track("riser", D.riser, level=-16, verb=-12, width=1.4)
    s.hit(fx, 20, dur=16.0, v=0.9)
    s.hit(fx, 44, dur=16.0, p=55)
    down = s.track("down", D.downlifter, level=-18, verb=-12)
    s.hit(down, 40, dur=8.0)
    boom = s.track("impact", D.impact, level=-12, verb=-14)
    s.at(boom, "drop drop2")

    reese = s.track("reese", I.reese, level=-6, mono=True, duck=0.6, gate=0.9, cutoff=420.0, detune=18.0,
                    wobble=1.5 if final else 1.0, rate=135 / 60 / 2)
    s.rel(reese, "R:6! R:2? -:2 R:2 R+10:2 R+12:2", "intro verse", floor=31)
    s.rel(reese, "R:6! -:4 R:6 | -:2 R:8 R:3 R:3", "drop drop2", floor=31,        # locked to the kicks
          x=(("cutoff", 260.0), ("wobble", 0.0)))
    s.rel(reese, "R:16", "break", floor=31, gate=1.0, v=0.45, x=(("cutoff", 300.0), ("wobble", 0.0)))

    acid = s.track("acid", I.acid, level=-10, mono=True, duck=0.3, delay=-14, gate=0.55, eq=[("hp", 120)],
                   cutoff=320.0 if final else 230.0, octaves=3.8 if final else 3.2, res=0.85 if final else 0.78)
    s.rel(acid, ACID, "drop drop2", floor=40)

    stab = s.track("stab", I.stab, level=-12, verb=-12, delay=-18, width=1.3, eq=[("hp", 280)], bend=-2.0)
    s.chords(stab, "verse drop2" if not final else "verse drop drop2", lo=55, hi=72, nv=3,
             rhythm="X.....X.....X..." if not final else "X.....X.....X...|..X.....X...X...", gate=0.6)

    pad = s.track("pad", I.pad, level=-14, duck=0.45, verb=-8, chorus=0.4, width=1.3, eq=[("hp", 220)],
                  cutoff=800.0 if not final else 1300.0, attack=0.8, release=1.6, voices=7, detune=22.0)
    s.chords(pad, "intro break drop", lo=50, hi=74, nv=4)

    lead = s.track("lead", I.lead, level=-9, mono=True, delay=-12, verb=-14, gate=0.92,
                   eq=[("hp", 200), ("peak", 2800, 1.0, 2.0)], wave="pulse", pw=0.42, voices=2, detune=8.0,
                   cutoff=1800.0, glide=0.04, vib=0.12)
    s.notes(lead, HOOK, (16, 8))
    s.notes(lead, DROP2_LEAD, "drop2")
    bells = s.track("bells", I.fm, level=-13, delay=-9, verb=-8, eq=[("hp", 450)], ratio=3.5, index=2.2)
    s.notes(bells, BELLS, "break")
    if final:
        perc = s.track("perc", D.tom, level=-18, pan=-0.4, decay=0.12, bend=0.3, vel_human=0.15)
        s.notes(perc, "-:3 G4:1 -:2 D4:1 -:3 G4:1 -:1 D4:1 G4:1? -:2", "verse drop drop2")
    return s
