#!/usr/bin/env python3
"""T22c: RSS of a /proc/<pid>/smaps copy split by mapping (memsoak.sh writes the copies).

    smaps_split.py <smaps> [<later smaps>]     one copy: top mappings; two: what grew between them

Groups: file mappings by basename, shmem (memfd, /dev/shm, SysV, shared anonymous = /dev/zero),
devices (/dev/nvidia*, /dev/dri), [heap], [stack], and anonymous mappings split by address:
`anon<7e` (below 0x7e00_0000_0000: V8's randomly placed heap pages and code range; glibc,
jemalloc and the driver map near the top of the address space) and `anon:<size class>` above it.
"""
import re
import sys
from collections import Counter

HEADER = re.compile(r"^([0-9a-f]+)-([0-9a-f]+) \S+ \S+ \S+ \d+\s*(.*)$")


def group(start, end, name):
    if name.startswith("/memfd:") or name.startswith("/dev/shm") or name.startswith("/SYSV") \
            or name.startswith("/dev/zero"):
        return "shmem:" + re.sub(r"[0-9]+", "#", name.split("/")[-1])[:40]
    if name.startswith("/dev/"):
        return "dev:" + re.sub(r"[0-9]+", "#", name)
    if name.startswith("/"):
        return "file:" + name.split("/")[-1].replace(" (deleted)", "")
    if name:
        return name  # [heap], [stack], [vdso], ...
    if start < 0x7E00_0000_0000:
        return "anon<7e"
    size = end - start
    return "anon:" + ("<1M" if size < 1 << 20 else "<64M" if size < 64 << 20 else ">=64M")


def load(path):
    rss, cur = Counter(), None
    with open(path) as f:
        for line in f:
            m = HEADER.match(line)
            if m:
                cur = group(int(m[1], 16), int(m[2], 16), m[3].strip())
            elif line.startswith("Rss:") and cur:
                rss[cur] += int(line.split()[1])
    return rss


def mb(kb):
    return f"{kb / 1024:8.1f}"


def main(paths):
    a = load(paths[0])
    if len(paths) == 1:
        print(f"{mb(sum(a.values()))} MB total")
        for k, v in a.most_common(25):
            print(f"{mb(v)} MB {k}")
        return
    b = load(paths[1])
    print(f"{mb(sum(a.values()))} -> {mb(sum(b.values()))} MB total")
    delta = Counter({k: b[k] - a[k] for k in set(a) | set(b)})
    for k, v in sorted(delta.items(), key=lambda kv: -abs(kv[1]))[:25]:
        print(f"{mb(a[k])} -> {mb(b[k])} ({v / 1024:+8.1f}) MB {k}")


if __name__ == "__main__":
    main(sys.argv[1:])
