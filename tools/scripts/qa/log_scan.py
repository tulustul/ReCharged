#!/usr/bin/env python3
"""QA: scan game logs (stderr captures or session logs) for panics, ERROR/WARN lines (deduplicated,
entity ids normalised), load times (Loading → InRace) and restart times (Setup → Countdown/Intro).
Usage: log_scan.py <log>...   (prints one block per file, then a total of distinct messages)"""
import re
import sys
from collections import Counter
from datetime import datetime

ansi = re.compile(r"\x1b\[[0-9;]*m")
stamp = re.compile(r"^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+)Z\s+(\w+)\s+(\S+?):?\s(.*)$")
total = Counter()


def t(s):
    return datetime.fromisoformat(s[:26])


for path in sys.argv[1:]:
    issues, loads, restarts = Counter(), [], []
    load_at = setup_at = None
    panics = 0
    for raw in open(path, errors="replace"):
        line = ansi.sub("", raw.rstrip("\n"))
        if "panicked" in line:
            panics += 1
            issues["PANIC " + line[:200]] += 1
        m = stamp.match(line)
        if not m:
            continue
        when, level, target, msg = m.groups()
        if level in ("WARN", "ERROR"):
            key = f"{level} {target}: " + re.sub(r"\d+v\d+", "<e>", re.sub(r"[0-9]+\.[0-9]+", "<n>", msg))[:180]
            issues[key] += 1
        if "AppState Some(Menu) -> Some(Loading)" in msg or "AppState Some(Unloading) -> Some(Loading)" in msg:
            load_at = t(when)
        elif "-> Some(InRace)" in msg and load_at:
            loads.append((t(when) - load_at).total_seconds())
            load_at = None
        elif "RacePhase -> Setup" in msg:
            setup_at = t(when)
        elif ("RacePhase -> Countdown" in msg or "RacePhase -> Intro" in msg) and setup_at:
            restarts.append((t(when) - setup_at).total_seconds())
            setup_at = None
    print(f"== {path}: panics {panics}, loads {['%.2f' % x for x in loads]}, setup→countdown/intro {['%.2f' % x for x in restarts]}")
    for k, v in issues.most_common():
        print(f"   {v:4} × {k}")
        total[k] += v
print("== distinct WARN/ERROR/PANIC across files:", len(total))
