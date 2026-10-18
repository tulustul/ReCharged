"""race_4 "Spark Plug" -- bright electro-pop. D major, 128 BPM, 64 bars.

intro 8 | verse 16 (I V/3 vi IV, bell riff on an A pedal) | chorus 16 (IV I/3 ii V, pluck hook) |
break 8 (vi IV I V) | chorus2 16 -> A resolves to D at the loop seam.
Final-lap variant: saw arp, extra claps and 16th hats, crash every 4 bars, open filters, lead double.
"""
import drums as D
import instruments as I
from song import Song

RIFF = ("D5:2 A4:2 F#5:2 A4:2 E5:2 A4:2 D5:2 A4:2 | C#5:2 A4:2 E5:2 A4:2 A5:2 A4:2 E5:2 A4:2 |"
        "D5:2 A4:2 F#5:2 A4:2 B5:2 A4:2 F#5:2 A4:2 | D5:2 A4:2 G5:2 A4:2 B5:2 A4:2 A5:2 G5:2")
HOOK = ("F#5:3 E5:3 D5:2 E5:4 B4:4 | A4:3 B4:3 D5:2 -:2 D5:2 E5:2 F#5:2 | G5:3 F#5:3 E5:2 F#5:4 B4:4 |"
        "C#5:3 D5:3 E5:10 | F#5:3 E5:3 D5:2 E5:4 B4:4 | A4:3 B4:3 D5:2 -:2 A5:2 G5:2 F#5:2 |"
        "G5:3 F#5:3 E5:2 D5:4 B4:2 C#5:2 | D5:4 C#5:4 A4:8")
VERSE_LEAD = "F#5:8 E5:4 D5:4 | E5:8 C#5:4 A4:4 | D5:8 C#5:4 B4:4 | D5:12 -:4"
BREAK_BELLS = ("F#5:4 D5:4 B4:8 | G5:4 D5:4 B4:8 | A5:4 F#5:4 D5:8 | E5:4 C#5:4 A4:8 |"
               "F#5:4 A5:4 B5:8 | B5:4 A5:4 G5:8 | G5:4 F#5:4 E5:8 | C#5:4 E5:4 A5:8")
CHORUS_CH = "G D/F# Em7 A G D/F# Em7 A7sus4:0.5 A:0.5 "


def build(final=False):
    s = Song("race_4", bpm=128, key="D major", seed=4)
    s.section("intro", 8, "D A/C# Bm7 Gadd9 " * 2)
    s.section("verse", 16, "D A/C# Bm7 Gadd9 " * 4)
    s.section("chorus", 16, CHORUS_CH * 2)
    s.section("break", 8, "Bm7 G D A Bm7 G Em7 A")
    s.section("chorus2", 16, CHORUS_CH * 2)
    s.verb = dict(rt=2.0, damp=8000.0, predelay=0.02)
    s.delay = dict(beats=0.75, fb=0.35, hp=400.0, lp=6000.0)
    ch = "chorus chorus2"
    groove = "verse " + ch

    kick = s.track("kick", D.kick, level=-5, sidechain=True, tune=55.0, decay=0.3, punch=3.6)
    s.beat(kick, "X...X...X...X...", "intro " + groove)
    s.beat(kick, "X...X...X...X...", "break" if final else (44, 4))

    clap = s.track("clap", D.clap, level=-9, verb=-13, human=0.002)
    s.beat(clap, "....X.......X...", groove + " " + ("intro" if final else ""))
    s.beat(clap, "....X.......X...", (4, 4))
    if final:
        clap2 = s.track("clap2", D.clap, level=-15, verb=-12, pan=0.3, tone=1800.0, spread=0.012)
        s.beat(clap2, "....X.......X..x", ch)
    snare = s.track("snare", D.snare, level=-11, verb=-16, tone=230.0, decay=0.12)
    s.beat(snare, "x...x...x...x...|x.x.x.x.x.x.x.x.|xxxxxxxxxxxxxxxx|xxxxxxxxXXXXXXXX", (44, 4))
    s.beat(snare, "........xxxxXXXX", (23, 1))

    chh = s.track("hat", D.hat, level=-18, pan=0.25, human=0.003, vel_human=0.1, decay=0.035)
    s.beat(chh, "x.x.x.x.x.x.x.x." if not final else "xoxoxoxoxoxoxoxo", "intro verse")
    s.beat(chh, "xoxoxoxoxoxoxoxo", ch)
    ohh = s.track("ohat", D.hat, level=-16, pan=-0.2, choke="hat", open_=True, tone=1.1)
    s.beat(ohh, "..x...x...x...x.", groove + (" break" if final else ""), gate=2.0)
    shk = s.track("shaker", D.shaker, level=-20, pan=-0.4, human=0.004, vel_human=0.12)
    s.beat(shk, "oxoxoxoxoxoxoxox", ch + (" verse break" if final else ""))
    crash = s.track("crash", D.crash, level=-16, verb=-18, width=1.3)
    s.at(crash, "verse chorus chorus2")
    for b in ([28, 32, 36, 52, 56, 60] if final else [32, 56]):
        s.hit(crash, b)
    fx = s.track("riser", D.riser, level=-16, verb=-12, width=1.4)
    s.hit(fx, 20, dur=16.0, v=0.85)
    s.hit(fx, 44, dur=16.0, p=62)
    rev = s.track("revcrash", D.revcrash, level=-18, verb=-14)
    s.hit(rev, 39, dur=4.0)
    boom = s.track("impact", D.impact, level=-14, verb=-14)
    s.at(boom, ch)

    bass = s.track("bass", I.bass, level=-6, duck=0.5, gate=0.75, cutoff=200.0, fdecay=0.12, sub=0.75,
                   peak=2800.0 if final else 1800.0, drive=1.8)
    s.rel(bass, "-:2 R:2 -:2 R:2 -:2 R:2 -:2 R+12:2", "intro " + groove, floor=33)
    s.rel(bass, "R:16", "break", floor=33, gate=1.0, v=0.3, skip=range(44, 48))
    s.rel(bass, "-:2 R:2 -:2 R:2 -:2 R:2 -:2 R+12:2", (44, 4), floor=33)

    riff = s.track("riff", I.fm, level=-13, delay=-12, verb=-14, pan=-0.15, eq=[("hp", 300)], ratio=1.0,
                   index=1.8, idecay=0.18, tine=0.35, decay=0.6, release=0.15)
    s.notes(riff, RIFF, "intro verse")
    pluck = s.track("pluck", I.pluck, level=-8, delay=-11, verb=-14, duck=0.25, eq=[("hp", 250)],
                    voices=4, detune=14.0, cutoff=500.0, peak=9000.0 if final else 6500.0, fdecay=0.14,
                    decay=0.45, release=0.15)
    s.notes(pluck, HOOK, ch)
    lead = s.track("lead", I.lead, level=-11, mono=True, delay=-12, verb=-12, gate=0.95, eq=[("hp", 250)],
                   voices=3, detune=10.0, cutoff=2000.0, vib=0.15)
    s.notes(lead, VERSE_LEAD, (16, 8))

    pad = s.track("pad", I.pad, level=-12, duck=0.65, duck_release=0.22, verb=-10, chorus=0.35, width=1.3,
                  eq=[("hp", 220)], cutoff=2200.0 if final else 1500.0, attack=0.2)
    s.chords(pad, "verse chorus break chorus2", lo=54, hi=78, nv=4)
    bells = s.track("bells", I.fm, level=-12, delay=-9, verb=-8, eq=[("hp", 450)], index=2.0, decay=1.4)
    s.notes(bells, BREAK_BELLS, "break")
    if final:
        arp = s.track("arp", I.pluck, level=-15, delay=-10, duck=0.3, eq=[("hp", 400)], voices=3, detune=12.0,
                      cutoff=800.0, peak=8000.0, decay=0.15)
        s.arp(arp, groove, "0 1 2 4 3 2 1 2", lo=66)
        lead2 = s.track("lead2", I.pluck, level=-15, delay=-10, verb=-12, eq=[("hp", 500)], voices=3,
                        detune=20.0, cutoff=900.0, peak=10000.0, decay=0.4)
        s.notes(lead2, HOOK, ch, transpose=12)
    return s
