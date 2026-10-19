#!/usr/bin/env python3
"""T22c: live heap of jemalloc heap profiles (memsoak.sh JEMALLOC=...), and what grew between two.

    heap_diff.py <dump> [<later dump>] [--top N] [--depth D]
    heap_diff.py --match <regex> <dumps...>     per dump: live MB and objects of the stacks with a
                                                frame matching <regex> (a timeline of one site)

Reads jemalloc's heap_v2 dumps directly (jeprof mangles Rust names), un-samples the counts as
jeprof does, symbolizes with llvm-symbolizer (cache in <dump dir>/symbols.json) and prints the
top allocation sites — the first D frames outside allocator/std plumbing — and the top functions
by inclusive bytes. With two dumps every number is the growth from the first to the second.
"""
import bisect
import json
import math
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict

PLUMBING = re.compile(
    r"^(<?(alloc|core|std|hashbrown|smallvec|__rust|_rjem|je_|rust_|__libc|malloc|calloc|realloc"
    r"|posix_memalign|aligned_alloc|operator new|imp_|prof_|imalloc|ialloc|irealloc|iralloc|do_rallocx"
    r"|bevy_platform::|bevy_utils::)"
    r"|\?\?$|.*::(allocate|alloc|reserve|grow|with_capacity|push|extend|clone|from_iter|collect"
    r"|to_vec|to_owned|insert|resize|new)$)"
)


def parse(path):
    stacks, maps, period = {}, [], 1
    with open(path) as f:
        period = int(f.readline().split("/")[1])
        stack = None
        in_maps = False
        for line in f:
            if in_maps:
                parts = line.split()
                if len(parts) >= 6 and parts[5].startswith("/"):
                    lo, hi = (int(x, 16) for x in parts[0].split("-"))
                    maps.append((lo, hi, int(parts[2], 16), parts[5]))
            elif line.startswith("@ "):
                stack = tuple(int(x, 16) for x in line[2:].split())
            elif line.startswith("  t*:") and stack is not None:
                m = re.match(r"\s*t\*: (\d+): (\d+)", line)
                n, s = int(m[1]), int(m[2])
                if n:
                    scale = 1 / (1 - math.exp(-(s / n) / period))
                    old = stacks.get(stack, (0, 0))
                    stacks[stack] = (old[0] + s * scale, old[1] + n * scale)
                stack = None
            elif line.startswith("MAPPED_LIBRARIES"):
                in_maps = True
    maps.sort()
    return stacks, maps


def symbolize(addrs, maps, cache_path):
    cache = {}
    if os.path.exists(cache_path):
        with open(cache_path) as f:
            cache = json.load(f)
    starts = [m[0] for m in maps]
    bias = {}
    for lo, _, off, path in maps:
        bias.setdefault(path, lo - off)
    todo = defaultdict(list)
    for a in addrs:
        if str(a) in cache:
            continue
        i = bisect.bisect_right(starts, a) - 1
        if i < 0 or a >= maps[i][1]:
            cache[str(a)] = ["??"]
            continue
        path = maps[i][3]
        todo[path].append(a)
    for path, items in todo.items():
        rel = [a - bias[path] - 1 for a in items]
        out = subprocess.run(
            ["llvm-symbolizer", f"--obj={path}", "--demangle", "--inlines"],
            input="\n".join(hex(r) for r in rel) + "\n", capture_output=True, text=True,
        ).stdout
        blocks = out.strip("\n").split("\n\n")
        name = os.path.basename(path)
        for a, block in zip(items, blocks):
            lines = block.split("\n")
            funcs = [lines[k] for k in range(0, len(lines), 2)]
            cache[str(a)] = [f if f != "??" else f"?? ({name})" for f in funcs]
    with open(cache_path, "w") as f:
        json.dump(cache, f)
    return cache


def short(name):
    name = re.sub(r"::h[0-9a-f]{16}$", "", name)
    name = re.sub(r"<[^<>]*>", "<>", name)
    name = re.sub(r"<[^<>]*>", "<>", name)
    return name[:110]


def frames(stack, syms):
    out = []
    for a in stack:
        out.extend(syms.get(str(a), ["??"]))
    return out


def site(fr, depth):
    keep = []
    for f in (short(f) for f in fr):
        if not PLUMBING.match(f) and (not keep or keep[-1] != f):
            keep.append(f)  # unsymbolized driver frames repeat: keep one of a run
    return " < ".join(keep[:depth]) or " < ".join(short(f) for f in fr[:depth])


def main(argv):
    top = int(argv[argv.index("--top") + 1]) if "--top" in argv else 25
    depth = int(argv[argv.index("--depth") + 1]) if "--depth" in argv else 4
    dumps = [a for i, a in enumerate(argv) if not a.startswith("--")
             and (i == 0 or not argv[i - 1].startswith("--"))]
    parsed = [parse(p) for p in dumps]
    addrs = {a for stacks, _ in parsed for s in stacks for a in s}
    cache = os.path.join(os.path.dirname(os.path.abspath(dumps[-1])), "symbols.json")
    syms = symbolize(addrs, parsed[-1][1], cache)
    if "--match" in argv:
        pattern = re.compile(argv[argv.index("--match") + 1])
        for path, (stacks, _) in zip(dumps, parsed):
            hit = [v for st, v in stacks.items() if any(pattern.search(f) for f in frames(st, syms))]
            total = sum(v[0] for v in stacks.values())
            print(f"{os.path.basename(path)}  {sum(v[0] for v in hit) / 2**20:8.1f} MB"
                  f" {sum(v[1] for v in hit):9.0f} objs  (heap {total / 2**20:.1f} MB)")
        return
    totals = [Counter(), Counter()]
    counts = [Counter(), Counter()]
    cum = [Counter(), Counter()]
    for k, (stacks, _) in enumerate(parsed[-2:] if len(parsed) > 1 else parsed):
        for stack, (size, n) in stacks.items():
            fr = frames(stack, syms)
            totals[k][site(fr, depth)] += size
            counts[k][site(fr, depth)] += n
            for f in {short(f) for f in fr}:
                cum[k][f] += size
    two = len(parsed) > 1
    sums = [sum(v[0] for v in stacks.values()) for stacks, _ in parsed]
    print("live heap MB: " + " -> ".join(f"{s / 2**20:.1f}" for s in sums))
    for title, table in (("sites", totals), ("functions (inclusive)", cum)):
        print(f"\n== {title}{' (growth)' if two else ''}")
        keys = set(table[0]) | set(table[1])
        delta = {k: (table[1][k] - table[0][k]) if two else table[0][k] for k in keys}
        for k in sorted(keys, key=lambda k: -abs(delta[k]))[:top]:
            objs = (f" [{counts[0][k]:.0f}{f' -> {counts[1][k]:.0f}' if two else ''} objs]"
                    if table is totals else "")
            print(f"{delta[k] / 2**20:+9.1f} MB {table[0][k] / 2**20:8.1f}"
                  f"{f' -> {table[1][k] / 2**20:8.1f}' if two else ''}{objs}  {k}")


if __name__ == "__main__":
    main(sys.argv[1:])
