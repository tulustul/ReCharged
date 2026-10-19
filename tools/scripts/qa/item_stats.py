#!/usr/bin/env python3
"""QA: item statistics from game logs run with RUST_LOG=rc_items=debug,rc_race=debug.
Usage: item_stats.py <log>...   (prints grants per item/band, uses, hits, blocked, respawns)"""
import re
import sys
from collections import Counter

ansi = re.compile(r"\x1b\[[0-9;]*m")
got, used, hits, blocked, selfhit, respawn = Counter(), Counter(), Counter(), Counter(), Counter(), Counter()
bands = Counter()
for path in sys.argv[1:]:
    for line in open(path, errors="replace"):
        line = ansi.sub("", line)
        if m := re.search(r"items: (.+?) got (\w+) \((\w+)\)", line):
            got[m[2]] += 1
            bands[(m[3], m[2])] += 1
        elif m := re.search(r"items: (\S+) used (\w+)", line):
            used[m[2]] += 1
        elif m := re.search(r"items: (\w+) by (\S+) hit (\S+) \(blocked: (\w+)\)", line):
            (blocked if m[4] == "true" else hits)[m[1]] += 1
            if m[2] == m[3]:
                selfhit[m[1]] += 1
        elif m := re.search(r"respawn (\S+) \((\w+)\)", line):
            respawn[m[2]] += 1
print(f"{'item':14} {'got':>4} {'used':>5} {'hits':>5} {'blocked':>8} {'self':>5}")
for item in sorted(set(got) | set(used) | set(hits)):
    print(f"{item:14} {got[item]:4} {used[item]:5} {hits[item]:5} {blocked[item]:8} {selfhit[item]:5}")
print("respawns:", dict(respawn))
print("grants by band:", {b: sum(v for (bb, _), v in bands.items() if bb == b) for b in {b for b, _ in bands}})
