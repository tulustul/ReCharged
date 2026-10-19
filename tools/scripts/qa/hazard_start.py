#!/usr/bin/env python3
"""QA (feel): victims per hazard drop (Oil Slick, Spring Pad) by when it was dropped — in the first
N seconds after GO vs later. Windowed logs only (wall clock ≈ race clock), run with
RUST_LOG=rc_items=debug. A drop's victims are the `<item> by <user> hit …` lines of that user until
its next drop of the same item.
Usage: hazard_start.py [--early 10] <log>..."""
import re
import sys
from datetime import datetime

args = sys.argv[1:]
early = 10.0
if args[:1] == ["--early"]:
    early, args = float(args[1]), args[2:]
ansi = re.compile(r"\x1b\[[0-9;]*m")
stamp = re.compile(r"^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+)Z")
rows = {"early": [], "late": []}
for path in args:
    go = None
    open_drops = {}  # (user, item) -> [band, victims]
    for raw in open(path, errors="replace"):
        line = ansi.sub("", raw)
        m = stamp.match(line)
        if not m:
            continue
        t = datetime.fromisoformat(m[1][:26])
        if "RacePhase -> Racing" in line:
            go = t
        elif "RacePhase -> Setup" in line:
            for band, n in open_drops.values():
                rows[band].append(n)
            open_drops, go = {}, None
        elif go and (u := re.search(r"items: (\S+) used (OilSlick|SpringPad)", line)):
            key = (u[1], u[2])
            if key in open_drops:
                band, n = open_drops[key]
                rows[band].append(n)
            open_drops[key] = ["early" if (t - go).total_seconds() < early else "late", 0]
        elif h := re.search(r"items: (OilSlick|SpringPad) by (\S+) hit (\S+) \(blocked: false\)", line):
            if (h[2], h[1]) in open_drops:
                open_drops[(h[2], h[1])][1] += 1
    for band, n in open_drops.values():
        rows[band].append(n)
for band, ns in rows.items():
    if ns:
        print(f"{band:5} (< {early:g} s after GO)" if band == "early" else f"{band:5}", f"drops {len(ns):3}  victims {sum(ns):3}  "
              f"per drop {sum(ns) / len(ns):.2f}  max {max(ns)}  ≥3 victims: {sum(n >= 3 for n in ns)}")
