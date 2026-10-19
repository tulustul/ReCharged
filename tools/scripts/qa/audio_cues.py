#!/usr/bin/env python3
"""QA (Q19 / DoD-6): which audio cues did runs trigger? Scans logs made with RUST_LOG=rc_audio=debug
for `audio cue <id>: <bus> bus, <dB> dB` (one-shots, stingers, thunder) and `music: A → B` (the
director's loop targets), and compares them with the files in assets/audio/.
Usage: audio_cues.py <log>...   (loops — engines, skids, rumbles, item hums, ambience beds — are
continuous voices and not in the cue log; they are listed separately)"""
import re
import sys
from collections import Counter
from pathlib import Path

ansi = re.compile(r"\x1b\[[0-9;]*m")
cue_re = re.compile(r"audio cue (\w+): (\w+) bus, (-?[\d.]+) dB(?: \(×(\d+))?")
music_re = re.compile(r"music: (.+?) → (.+)$")
cues, buses, music, warns = Counter(), {}, Counter(), Counter()
for path in sys.argv[1:]:
    for line in open(path, errors="replace"):
        line = ansi.sub("", line.rstrip("\n"))
        if m := cue_re.search(line):
            cues[m[1]] += int(m[4] or 1)
            buses[m[1]] = m[2]
        elif m := music_re.search(line):
            music[m[2].strip()] += 1
        if "rc_audio" in line and (" WARN " in line or " ERROR " in line):
            warns[line[line.find("rc_audio"):][:160]] += 1

root = Path(__file__).resolve().parents[3] / "assets/audio"
stems = lambda d: sorted(p.stem for p in (root / d).glob("*.ogg"))
loops = [s for s in stems("sfx") if s.endswith("_loop") or s.startswith("engine_")]
oneshots = [s for s in stems("sfx") if s not in loops]
stingers = ["results_jingle", "podium_fanfare", "cup_victory"]
thunder = [s for s in stems("ambience") if s.startswith("thunder")]
for title, names in [("SFX/UI one-shots", oneshots), ("stingers", stingers), ("thunder", thunder)]:
    seen = [n for n in names if cues[n]]
    print(f"{title}: {len(seen)}/{len(names)} triggered")
    for n in names:
        print(f"   {'✓' if cues[n] else '✗'} {n:24} {cues[n]:6}  {buses.get(n, '')}")
print("music director targets:", dict(music))
print("not in the cue log by design (continuous voices):", ", ".join(loops))
print("other cues:", {k: v for k, v in cues.items() if k not in oneshots + stingers + thunder})
print("rc_audio WARN/ERROR:", dict(warns) or "none")
