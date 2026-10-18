"""race_2 "Neon Highway" -- driving synthwave / outrun. A minor, 126 BPM, 64 bars.

intro 8 | verse 16 | chorus 16 | break 8 | chorus2 16  (chorus ends on E: V -> i at the loop seam)
Final-lap variant: open filters, ride + shaker, octave arp and lead doubles, kick through the break.
"""
import drums as D
import instruments as I
from song import Song

VERSE_HOOK = ("A4:3 C5:3 E5:2 D5:4 C5:4 | A4:3 C5:3 F5:2 E5:8 | G4:3 C5:3 E5:2 D5:4 E5:4 | D5:3 B4:3 G4:2 -:8 |"
              "A4:3 C5:3 E5:2 D5:4 C5:4 | A4:3 C5:3 F5:2 E5:8 | G4:3 C5:3 E5:2 D5:4 E5:4 | D5:3 E5:3 G5:2 A5:8")
CHORUS = ("A4:4 C5:4 F5:6 E5:2 | G5:6 F5:2 E5:4 D5:4 | E5:12 C5:2 D5:2~ | E5:4 A5:8 G5:4 |"
          "A5:6 G5:2 F5:4 E5:4 | D5:6 E5:2 F5:4 G5:4 | G#5:8 E5:4 B4:4 | G#4:4 B4:4 E5:8 |"
          "A4:4 C5:4 F5:6 E5:2 | G5:6 F5:2 E5:4 D5:4 | E5:12 C5:2 D5:2~ | E5:4 A5:8 G5:4 |"
          "A5:6 G5:2 F5:4 E5:4 | D5:6 E5:2 F5:4 G5:4 | B5:8 G#5:4 E5:4 | D5:4 B4:4 G#4:8")
BREAK_BELLS = ("F5:4 A5:4 D6:8 | C6:4 B5:4 A5:8 | A5:4 C6:4 F6:4 E6:4 | D6:8 B5:8 |"
               "F5:4 A5:4 D6:8 | E6:4 D6:4 C6:8 | A5:4 C6:4 F6:4 E6:4 | G#5:8 B5:4 E6:4")
TOM_FILL = "-:8 E3:2! C3:2 A2:1 A2:1 F2:1! F2:1"


def build(final=False):
    s = Song("race_2", bpm=126, key="A minor", seed=2)
    s.section("intro", 8, "Am F C G Am F C G")
    s.section("verse", 16, "Am F C G " * 4)
    s.section("chorus", 16, "F G Am Am F G E E " * 2)
    s.section("break", 8, "Dm Am F G Dm Am F E")
    s.section("chorus2", 16, "F G Am Am F G E E " * 2)
    s.verb = dict(rt=2.4, damp=6500.0, predelay=0.02)
    s.delay = dict(beats=0.75, fb=0.38, hp=350.0, lp=4500.0)
    groove = "verse chorus chorus2"
    fills = [23, 39, 63]

    kick = s.track("kick", D.kick, level=-5, sidechain=True, tune=55.0, decay=0.32)
    s.beat(kick, "X...X...X...X...", "intro " + groove)
    s.beat(kick, "X...X...X...X...", "break" if final else (46, 2))

    snare = s.track("snare", D.snare, level=-8, verb=-16, gated=0.28, tone=200.0, human=0.002)
    s.beat(snare, "....X.......X...", groove, skip=fills)
    s.beat(snare, "....X.......X...", (4, 3))
    s.beat(snare, "....X...xxxxXXXX", (7, 1))
    s.beat(snare, "x...x...x...x...|x.x.x.x.xxxxXXXX", (46, 2))
    for b in fills:
        s.beat(snare, "....X...........", (b, 1))

    clap = s.track("clap", D.clap, level=-13, verb=-14)
    s.beat(clap, "....X.......X...", groove if final else "chorus chorus2", skip=fills)

    chh = s.track("hat", D.hat, level=-17, pan=0.25, human=0.003, vel_human=0.08)
    s.beat(chh, "x.x.x.x.x.x.x.x.", (4, 4))
    s.beat(chh, "xoxoxoxoxoxoxoxo", "verse")
    s.beat(chh, "xo.oxo.oxo.oxo.o", "chorus chorus2")
    s.beat(chh, "x.x.x.x.x.x.x.x.", "break" if final else (44, 4))
    ohh = s.track("ohat", D.hat, level=-18, pan=-0.2, choke="hat", open_=True)
    s.beat(ohh, "..x...x...x...x.", "chorus chorus2" + (" verse" if final else ""), gate=2.0)

    toms = s.track("toms", D.tom, level=-10, verb=-12, pan=-0.15)
    for b in fills:
        s.notes(toms, TOM_FILL, (b, 1))

    crash = s.track("crash", D.crash, level=-16, verb=-18, width=1.3)
    s.at(crash, "verse chorus chorus2")
    for b in ([28, 32, 36, 52, 56, 60] if final else [32, 56]):
        s.hit(crash, b)
    fx = s.track("riser", D.riser, level=-16, verb=-12, width=1.4)
    s.hit(fx, 22, dur=8.0, v=0.9)
    s.hit(fx, 44, dur=16.0, p=57)
    rev = s.track("revcrash", D.revcrash, level=-18, verb=-14)
    s.hit(rev, 23, dur=4.0)
    s.hit(rev, 47, dur=4.0)
    boom = s.track("impact", D.impact, level=-13, verb=-16)
    s.at(boom, "chorus chorus2")

    bass = s.track("bass", I.bass, level=-6, duck=0.55, gate=0.8, cutoff=140.0, fdecay=0.07,
                   peak=2600.0 if final else 1500.0, drive=2.2 if final else 1.7, sub=0.8)
    s.rel(bass, "R:2 R+12:2 " * 4, (4, 4), floor=28)
    s.rel(bass, "R:1! R:1 R+12:1 R:1 " * 4, groove, floor=28)
    if final:
        s.rel(bass, "R:2 R+12:2 " * 4, "break", floor=28)
    else:
        s.rel(bass, "R:2 R+12:2 " * 4, (44, 4), floor=28, v=0.6)

    arp = s.track("arp", I.pluck, level=-14, delay=-9, verb=-16, duck=0.3, eq=[("hp", 300)],
                  wave="pulse", pw=0.35, voices=2, detune=6.0, cutoff=500.0, fdecay=0.09, decay=0.22,
                  peak=8000.0 if final else 5000.0,
                  auto=[("lp", [(0, 700), (32, 12000), (160, 12000), (164, 1500), (184, 12000), (256, 12000)], 0.8)])
    s.arp(arp, "intro verse chorus break chorus2", "0 2 1 3 2 4 3 5", lo=57)

    pad = s.track("pad", I.pad, level=-11, duck=0.6, verb=-10, chorus=0.3, width=1.2,
                  eq=[("hp", 200), ("peak", 2500, 1.0, -2.0)], cutoff=2400.0 if final else 1400.0,
                  attack=0.25, release=1.0)
    s.chords(pad, "intro verse chorus break chorus2", lo=52, hi=76, nv=4)

    lead = s.track("lead", I.lead, level=-8, mono=True, delay=-12, verb=-12,
                   eq=[("hp", 180), ("peak", 3000, 1.0, 2.0)], voices=3, detune=12.0, cutoff=2200.0)
    s.notes(lead, VERSE_HOOK, (16, 8), x=(("wave", "pulse"), ("cutoff", 1500.0), ("voices", 1)))
    s.notes(lead, CHORUS, "chorus chorus2")

    bells = s.track("bells", I.fm, level=-12, delay=-10, verb=-8, eq=[("hp", 400)], index=2.0, decay=1.2)
    s.notes(bells, BREAK_BELLS, "break")

    if final:
        ride = s.track("ride", D.ride, level=-19, pan=0.35, vel_human=0.1)
        s.beat(ride, "x.o.x.o.x.o.x.o.", groove)
        shk = s.track("shaker", D.shaker, level=-20, pan=-0.35, human=0.003, vel_human=0.1)
        s.beat(shk, "oxoxoxoxoxoxoxox", "verse chorus break chorus2")
        arp2 = s.track("arp2", I.pluck, level=-16, delay=-10, duck=0.3, eq=[("hp", 500)], voices=3,
                       detune=12.0, cutoff=900.0, peak=9000.0, decay=0.18)
        s.arp(arp2, "chorus chorus2", "5 4 3 2 1 0 1 2", lo=69)
        lead2 = s.track("lead2", I.lead, level=-15, mono=True, delay=-10, verb=-10, eq=[("hp", 400)],
                        voices=5, detune=18.0, cutoff=3000.0)
        s.notes(lead2, CHORUS, "chorus chorus2", transpose=12)
    return s
