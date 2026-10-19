#!/usr/bin/env bash
# T22c: one game run with memory sampling, pinned to a binary copy and snapshots of ui/dist and
# assets in BIN_DIR (default target/agent-data/T22c/bin: recharged, ui-dist/, assets/; `ui` ->
# ui-dist for release binaries). A private mount namespace binds ui-dist over ui/dist (a dev
# build loads the workspace bundle and hot-reloads it), so other agents' edits never reach the run.
#   tools/scripts/qa/memsoak.sh <name> [game args...]      e.g. soak60 --soak 60 --soak-seed 1
# Writes OUT/<name>.{log,out} (OUT default target/agent-data/T22c/logs) and, until the game exits:
#   <name>.mem     every EVERY s (60): secs VmRSS RssAnon RssFile RssShmem VmSize (MB, the kernel's
#                  counters in /proc/<pid>/status, what ps shows) threads vram (MB, this process,
#                  nvidia-smi) smaps_rollup Rss Pss (MB, a page-table walk: VmRSS exceeds it by
#                  RssShmem with the NVIDIA driver)
#   <name>.smaps/  every SMAPS_EVERY s (300): copies of /proc/<pid>/smaps (smaps_split.py reads them)
# JEMALLOC=<lg> preloads jemalloc with heap profiling (1 sample per 2^LG_SAMPLE bytes, default 19)
# and a dump every 2^<lg> allocated bytes into <name>.heap/ (heap_diff.py compares two dumps).
# Data dir target/agent-data/T22c/data/<name> (fresh). BIN=<file> picks another binary; X/Y place
# the window. Waits while the user plays; holds the shared GPU lock while the game runs.
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
name=${1:?name}; shift
bin_dir=$(realpath "${BIN_DIR:-target/agent-data/T22c/bin}")
bin=$(realpath "${BIN:-$bin_dir/recharged}")
out=${OUT:-target/agent-data/T22c/logs}/$name
data=target/agent-data/T22c/data/$name
every=${EVERY:-60} smaps_every=${SMAPS_EVERY:-300}
mkdir -p "$(dirname "$out")" && rm -rf "$data" "$out.smaps" "$out.heap" "$out.mem" && mkdir -p "$out.smaps"
je_conf=""
if [ -n "${JEMALLOC:-}" ]; then
  mkdir -p "$out.heap"
  je_conf="prof:true,prof_final:true,lg_prof_sample:${LG_SAMPLE:-19},lg_prof_interval:$JEMALLOC,prof_prefix:$(realpath "$out.heap")/h"
fi
while pgrep -a recharged | grep -vqE -- '--user-data-dir [^ ]*target/'; do
  echo "memsoak.sh: the user is playing; waiting" >&2; sleep 30
done
exec 9>>target/agent-data/gpu.lock
flock -s 9
JE_CONF=$je_conf unshare -rm sh -c '
  mount --bind "$1/ui-dist" "$2/ui/dist" && cd "$2" || exit 3
  [ -n "$JE_CONF" ] && export LD_PRELOAD=/usr/lib/libjemalloc.so.2 MALLOC_CONF="$JE_CONF"
  bin=$0; shift 2; exec "$bin" "$@"' "$bin" "$bin_dir" "$root" \
  --user-data-dir "$data" --assets-dir "$bin_dir/assets" "$@" >"$out.out" 2>"$out.log" &
pid=$!
tools/scripts/qa/place.sh $pid "${X:-0}" "${Y:-0}" &
start=$(date +%s) next_smaps=0
while kill -0 $pid 2>/dev/null; do
  t=$(( $(date +%s) - start ))
  vram=$(nvidia-smi --query-compute-apps=pid,used_memory --format=csv,noheader,nounits 2>/dev/null |
    awk -F', *' -v p=$pid '$1 == p { print $2 }')
  roll=$(awk '/^Rss:/ { r = $2 } /^Pss:/ { p = $2 } END { printf "%d %d", r / 1024, p / 1024 }' /proc/$pid/smaps_rollup 2>/dev/null)
  awk -v t=$t -v vram="${vram:-0}" -v roll="$roll" '/^(VmRSS|RssAnon|RssFile|RssShmem|VmSize|Threads):/ { v[$1] = $2 }
    END { if (v["VmRSS:"]) printf "%d %d %d %d %d %d %d %d %s\n", t, v["VmRSS:"]/1024, v["RssAnon:"]/1024,
      v["RssFile:"]/1024, v["RssShmem:"]/1024, v["VmSize:"]/1024, v["Threads:"], vram, roll }' \
    /proc/$pid/status >>"$out.mem" 2>/dev/null
  if [ $t -ge $next_smaps ]; then
    cp /proc/$pid/smaps "$out.smaps/$(printf %05d $t)" 2>/dev/null
    next_smaps=$(( next_smaps + smaps_every ))
  fi
  sleep "$every"
done
wait $pid
echo "$name exit=$?"
