"""menu_theme "Charging Up" -- relaxed synthwave. D minor, 100 BPM, 48 bars.

intro 8 | A 8 (theme) | A2 8 (theme + bells) | B 8 (iv VI i V) | break 8 (bells carry the theme) |
B2 8 -> A7 resolves to Dm9 at the loop seam. The cup_victory stinger quotes this theme in D major.
"""
import drums as D
import instruments as I
from song import Song

THEME_A = ("-:2 D5:2 E5:2 F5:6 E5:2 D5:2 | C5:4 D5:8 A4:4 | -:2 C5:2 D5:2 E5:6 F5:2 E5:2 | D5:4 G4:4 E5:8 |"
           "-:2 D5:2 E5:2 F5:4 A5:4 G5:2 | F5:6 E5:2 D5:8 | C5:2 D5:2 E5:4 C5:4 A4:4 | G4:4 A4:2 C5:2 D5:8")
THEME_B = ("D5:4 F5:4 A5:6 G5:2 | F5:8 D5:4 C5:2 Bb4:2 | A4:4 D5:4 E5:4 F5:4 | G5:4 F5:2 E5:2 C#5:8 |"
           "D5:4 F5:4 A5:6 Bb5:2 | A5:8 F5:4 D5:4 | E5:4 F5:4 A5:4 D6:4 | D6:4 A5:4 C#5:8")
COUNTER = "-:8 A5:2 G5:2 E5:4 | -:16 | -:8 G5:2 E5:2 C5:4 | -:16"
A_CH = "Dm9 Bbmaj9 Fmaj7 Cadd9 " * 2
B_CH = "Gm9 Bbmaj7 Dm9 A7sus4:0.5 A7:0.5 " * 2


def build():
    s = Song("menu_theme", bpm=100, key="D minor", seed=5, swing=0.54)
    s.section("intro", 8, A_CH)
    s.section("A", 8, A_CH)
    s.section("A2", 8, A_CH)
    s.section("B", 8, B_CH)
    s.section("break", 8, A_CH)
    s.section("B2", 8, B_CH)
    s.verb = dict(rt=2.8, damp=6000.0, predelay=0.025)
    s.delay = dict(beats=0.75, fb=0.42, hp=350.0, lp=4000.0)
    full = "A A2 B B2"
    fills = [23, 31, 47]

    kick = s.track("kick", D.kick, level=-6, sidechain=True, tune=46.0, decay=0.4, punch=3.0, drive=1.8)
    s.beat(kick, "X.......X.......", "A A2", skip=[15])
    s.beat(kick, "X.......X.......", (4, 4))
    s.beat(kick, "X...X...X...X...", "B B2")
    s.beat(kick, "X.......X..X....", (15, 1))

    snare = s.track("snare", D.snare, level=-8, verb=-14, gated=0.34, tone=180.0, decay=0.2, human=0.003)
    s.beat(snare, "....X.......X...", full, skip=fills)
    s.beat(snare, "....X...x.x.xxXX", (39, 1))
    for b in fills:
        s.beat(snare, "....X...........", (b, 1))

    chh = s.track("hat", D.hat, level=-18, pan=0.3, human=0.004, vel_human=0.12, decay=0.05)
    s.beat(chh, "x.x.x.x.x.x.x.x.", "A A2 break")
    s.beat(chh, "x.x.x.x.x.x.x.x.", (4, 4))
    s.beat(chh, "xoxoxoxoxoxoxoxo", "B B2")
    ohh = s.track("ohat", D.hat, level=-19, pan=-0.25, choke="hat", open_=True)
    s.beat(ohh, "................|..............x.", "A A2", gate=2.0)
    s.beat(ohh, "..x...x...x...x.", "B B2", gate=2.0)
    shk = s.track("shaker", D.shaker, level=-22, pan=-0.4, human=0.004, vel_human=0.15)
    s.beat(shk, "oxoxoxoxoxoxoxox", "break B2")

    toms = s.track("toms", D.tom, level=-10, verb=-10, pan=-0.1, decay=0.45, bend=0.8)
    for b in fills:
        s.notes(toms, "-:8 A2:2! F2:2 D2:2 A1:2", (b, 1))
    crash = s.track("crash", D.crash, level=-17, verb=-16, width=1.3)
    s.at(crash, "A B break B2")
    fx = s.track("riser", D.riser, level=-18, verb=-10, width=1.4)
    s.hit(fx, 36, dur=16.0, p=57, v=0.8)

    bass = s.track("bass", I.bass, level=-7, duck=0.35, gate=0.85, cutoff=150.0, peak=1100.0, fdecay=0.14,
                   sub=0.85, drive=1.4, release=0.08)
    s.rel(bass, "R:2 R+12:2 " * 4, (4, 4), floor=28, v=0.6)
    s.rel(bass, "R:2 R+12:2 " * 3 + "R:2 R+12:1 R+7:1", full, floor=28)
    s.rel(bass, "R:6 -:2 R:4 R+7:4", "break", floor=28, gate=0.9, v=0.3)

    arp = s.track("arp", I.pluck, level=-15, delay=-8, verb=-14, duck=0.25, eq=[("hp", 300)], wave="pulse",
                  pw=0.4, voices=2, detune=7.0, cutoff=400.0, peak=4000.0, fdecay=0.12, decay=0.3,
                  auto=[("lp", [(0, 900), (16, 900), (32, 9000), (192, 9000)], 0.7)])
    s.arp(arp, "intro A A2 B break B2", "0 1 2 3 4 3 2 1", lo=62)

    pad = s.track("pad", I.pad, level=-10, duck=0.3, verb=-8, chorus=0.45, width=1.35,
                  eq=[("hp", 180), ("peak", 3000, 1.0, -2.0)], cutoff=1200.0, attack=0.6, release=1.8,
                  detune=16.0)
    s.chords(pad, "intro A A2 B break B2", lo=50, hi=74, nv=4)

    lead = s.track("lead", I.lead, level=-8, mono=True, delay=-10, verb=-10, gate=0.95,
                   eq=[("hp", 200), ("peak", 2500, 1.0, 1.5)], voices=2, detune=8.0, cutoff=1500.0,
                   fenv=1.8, vib=0.2, vib_rate=5.0, glide=0.07)
    s.notes(lead, THEME_A, "A A2")
    s.notes(lead, THEME_B, "B B2")

    bells = s.track("bells", I.fm, level=-12, delay=-8, verb=-6, eq=[("hp", 450)], index=1.8, decay=1.6)
    s.notes(bells, COUNTER, "A2")
    s.notes(bells, THEME_A, "break", transpose=12, v=0.6)
    s.notes(bells, THEME_B, "B2", transpose=12, v=0.45)
    return s
