"""race_1 "Volt Funk" -- funky electro / nu-disco. E dorian, 120 BPM, 64 bars.

intro 8 | verse 16 (broken electro beat, i9-IV13 vamp) | chorus 16 (four-on-the-floor disco) |
break 8 (iv-v-VI-VII climb) | chorus2 16 -> B7#9 resolves to Em9 at the loop seam.
Final-lap variant: open filters, shaker + conga toms + arp, claps on every backbeat, kick in break.
"""
import drums as D
import instruments as I
from song import Song

HOOK = ("-:2 B4:1 D5:1 E5:3! -:1 D5:1 E5:1 G5:2~ E5:2 -:2 | D5:2 B4:2 -:1 A4:3 -:2 G4:1 A4:1 C#5:2 B4:2 |"
        "-:2 B4:1 D5:1 E5:3! -:1 G5:1 A5:1 B5:2~ A5:2 G5:2 | E5:3 D5:3 B4:2 A4:4 -:4")
CHORUS = ("G5:4 E5:2 G5:2 A5:4 B5:4 | A5:6 F#5:2 E5:4 D5:4 | F#5:4 D5:2 F#5:2 A5:4 F#5:4 | G5:8 F#5:4 E5:4 |"
          "G5:4 E5:2 G5:2 B5:4 D6:4 | B5:6 A5:2 F#5:4 A5:4 | F#5:4 A5:2 B5:2 D6:4 B5:4 | A5:4 F#5:4 D#5:8")
VERSE_BASS = ("R:2! -:1 R:1? -:2 R+12:1! R+10:1 -:2 R+7:2 -:1 R+3:1 R+5:1~ R+7:1 |"
              "R:2! -:1 R:1? -:2 R+12:1! R+10:1 -:2 R+7:2 R+9:1 R+10:1 R+12:1 N-1:1")
DISCO_BASS = "R:2! R+12:2 R:2 R+12:2 R:2 R+12:2 R+10:1 R+12:1 R+7:2"
TOM_FILL = "-:8 B2:1! B2:1 G2:2 E2:1! E2:1 B1:2"


def build(final=False):
    s = Song("race_1", bpm=120, key="E dorian", seed=1, swing=0.56)
    vamp = "Em9 A13 " * 4
    s.section("intro", 8, vamp)
    s.section("verse", 16, vamp * 2)
    s.section("chorus", 16, "Cmaj9 D69 Bm7 Em9 Cmaj9 D69 B7#9 B7#9 " * 2)
    s.section("break", 8, "Am7 Bm7 Cmaj9 D69 " * 2)
    s.section("chorus2", 16, "Cmaj9 D69 Bm7 Em9 Cmaj9 D69 B7#9 B7#9 " * 2)
    s.verb = dict(rt=1.6, damp=7000.0, predelay=0.015)
    s.delay = dict(beats=0.75, fb=0.3, hp=400.0, lp=5000.0)
    ch = "chorus chorus2"
    fills = [23, 39, 63]

    kick = s.track("kick", D.kick, level=-5, sidechain=True, tune=41.2, decay=0.34, punch=4.0)
    s.beat(kick, "X......X..X.....|X......X..X..x..", "intro verse")
    s.beat(kick, "X...X...X...X...", ch)
    s.beat(kick, "X...X...X...X...", "break" if final else (46, 2))

    snare = s.track("snare", D.snare, level=-9, verb=-18, tone=210.0, decay=0.14, human=0.002)
    s.beat(snare, "....X.......X...", "verse " + ch, skip=fills)
    s.beat(snare, "....X.......X...", (4, 4))
    s.beat(snare, "x.x.x.x.x.x.x.x.|xxxxxxxxXXXXXXXX", (46, 2))
    ghost = s.track("ghost", D.snare, level=-19, tone=230.0, decay=0.08, snap=0.3, human=0.004,
                    vel_human=0.15)
    s.beat(ghost, "..o......o.o...o", "verse" + (" " + ch if final else ""), skip=fills)
    for b in fills:
        s.beat(snare, "....X...........", (b, 1))

    clap = s.track("clap", D.clap, level=-11, verb=-14)
    s.beat(clap, "....X.......X...", "verse " + ch if final else ch, skip=fills)
    s.beat(clap, "....X.......X...", "break")

    chh = s.track("hat", D.hat, level=-17, pan=0.3, human=0.003, vel_human=0.1)
    s.beat(chh, "xoxoxoxoxoxoxoxo", "intro verse break")
    s.beat(chh, "xo.oxo.oxo.oxo.o", ch)
    ohh = s.track("ohat", D.hat, level=-18, pan=-0.25, choke="hat", open_=True)
    s.beat(ohh, "................|..............x.", "intro verse", gate=2.0)
    s.beat(ohh, "..x...x...x...x.", ch, gate=2.0)
    bell = s.track("cowbell", D.cowbell, level=-21, pan=0.45)
    s.beat(bell, "..x.....x.x.....|..x.....x...x...", "verse")

    toms = s.track("toms", D.tom, level=-11, verb=-14, pan=0.15, decay=0.28)
    for b in fills:
        s.notes(toms, TOM_FILL, (b, 1))
    crash = s.track("crash", D.crash, level=-17, verb=-18, width=1.3)
    s.at(crash, "verse chorus break chorus2")
    for b in ([32, 56] + ([28, 36, 52, 60] if final else [])):
        s.hit(crash, b)
    fx = s.track("riser", D.riser, level=-17, verb=-12, width=1.4)
    s.hit(fx, 22, dur=8.0, v=0.9)
    s.hit(fx, 44, dur=16.0, p=52)
    boom = s.track("impact", D.impact, level=-14, verb=-16)
    s.at(boom, ch)

    bass = s.track("bass", I.bass, level=-6, mono=True, duck=0.4, gate=0.7, wave="pulse",
                   cutoff=170.0, peak=3200.0 if final else 2000.0, fdecay=0.09, res=0.35, glide=0.035,
                   drive=2.4, sub=0.7)
    s.rel(bass, VERSE_BASS, "intro verse", floor=28)
    s.rel(bass, DISCO_BASS, ch, floor=28)
    s.rel(bass, "R:3! -:3 R:2 -:2 R+12:2 R+10:2 R+7:2", "break", floor=28)

    chank = s.track("chank", I.chank, level=-15, verb=-20, pan=-0.3, eq=[("hp", 350)],
                    cutoff=900.0 if final else 600.0, peak=6500.0 if final else 4500.0)
    s.chords(chank, "intro verse " + ch, lo=55, hi=74, nv=3, rhythm="..X..x.x..X..x..", gate=0.45)
    brass = s.track("brass", I.brass, level=-12, verb=-14, width=1.3, eq=[("hp", 250)])
    s.chords(brass, ch, lo=55, hi=79, nv=4, rhythm="X.....X...X.....|X.....X.........", gate=0.7)

    pad = s.track("pad", I.pad, level=-15, duck=0.5, verb=-10, chorus=0.4, width=1.3, eq=[("hp", 250)],
                  cutoff=1600.0 if final else 1100.0, attack=0.4, voices=5, detune=14.0)
    s.chords(pad, ch + " break", lo=55, hi=79, nv=4)
    keys = s.track("epiano", I.fm, level=-13, verb=-12, chorus=0.5, eq=[("hp", 200)], ratio=1.0, index=1.4,
                   idecay=0.35, tine=0.25, decay=1.4, sustain=0.3)
    s.chords(keys, "break", lo=52, hi=72, nv=4, rhythm="X.......X.x.....", gate=1.6)

    talk = s.track("talkbox", I.talkbox, level=-8, mono=True, delay=-15, verb=-14, gate=0.9,
                   eq=[("hp", 200), ("peak", 2500, 1.0, 2.0)])
    s.notes(talk, HOOK, (16, 8), transpose=-12)
    s.notes(talk, CHORUS, ch, transpose=-12, x=(("vowels", "oa"),))

    if final:
        shk = s.track("shaker", D.shaker, level=-20, pan=-0.4, human=0.003, vel_human=0.12)
        s.beat(shk, "oxoxoxoxoxoxoxox", "verse chorus break chorus2")
        conga = s.track("conga", D.tom, level=-17, pan=0.4, decay=0.16, bend=0.25, vel_human=0.1)
        s.notes(conga, "-:2 E4:1 -:1 B3:2 -:1 E4:1? E4:2 -:2 B3:1 B3:1 E4:2", "verse " + ch)
        arp = s.track("arp", I.pluck, level=-16, delay=-10, duck=0.3, eq=[("hp", 400)], voices=2,
                      wave="pulse", pw=0.3, cutoff=700.0, peak=7000.0, decay=0.16)
        s.arp(arp, ch, "0 2 4 2 1 3 5 3", lo=64)
    return s
