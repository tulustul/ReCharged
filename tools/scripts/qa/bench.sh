#!/usr/bin/env bash
# DoD-7: --bench scenarios on the pinned QA build, one at a time, nothing else running.
# Any other recharged process seen before or during a scenario makes it re-run (up to 3 tries).
#   tools/scripts/qa/bench.sh toy_town_1p toy_town_2p ...   → target/qa/logs/bench-<s>.json
# Extra game flags: BENCH_ARGS="--weather rain" tools/scripts/qa/bench.sh maple_street_1p
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
for s in "$@"; do
  for try in 1 2 3; do
    while pgrep -a recharged >/dev/null; do echo "waiting for: $(pgrep -a recharged | head -1)"; sleep 15; done
    # shellcheck disable=SC2086
    tools/scripts/qa/pinned.sh --user-data-dir "target/qa/data/bench-$s" --unlock-all --bench "$s" \
      --window 1920x1080 --exit-after 280 ${BENCH_ARGS:-} \
      > "target/qa/logs/bench-$s.json" 2> "target/qa/logs/bench-$s.log" &
    pid=$! other=""
    while kill -0 "$pid" 2>/dev/null; do
      other=${other:-$(pgrep recharged | grep -vx "$pid" | head -1)}
      sleep 0.5
    done
    wait "$pid"; code=$?
    if [ -n "$other" ]; then echo "$s: another recharged ($other) ran meanwhile, retry"; continue; fi
    echo "$s exit=$code $(cat "target/qa/logs/bench-$s.json")"
    break
  done
done
