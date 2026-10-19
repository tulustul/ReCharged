#!/usr/bin/env bash
# DoD-7: --bench scenarios on the pinned QA build, one at a time, nothing else running.
# Any other recharged process seen before or during a scenario makes it re-run (up to 3 tries).
#   tools/scripts/qa/bench.sh toy_town_1p toy_town_2p ...   → ${QA_OUT:-target/qa}/logs/bench-<s>.json
# Extra game flags: BENCH_ARGS="--weather rain" BENCH_TAG=_rain tools/scripts/qa/bench.sh maple_street_1p
# Another build/output root: QA_RUN=tools/scripts/qa/live.sh QA_OUT=target/qa2 (T21; same for the other scripts)
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
for s in "$@"; do
  for try in 1 2 3; do
    while pgrep -a recharged >/dev/null; do echo "waiting for: $(pgrep -a recharged | head -1)"; sleep 15; done
    # shellcheck disable=SC2086
    ${QA_RUN:-tools/scripts/qa/pinned.sh} --user-data-dir "${QA_OUT:-target/qa}/data/bench-$s${BENCH_TAG:-}" --unlock-all --bench "$s" \
      --window 1920x1080 --exit-after 280 ${BENCH_ARGS:-} \
      > "${QA_OUT:-target/qa}/logs/bench-$s${BENCH_TAG:-}.json" 2> "${QA_OUT:-target/qa}/logs/bench-$s${BENCH_TAG:-}.log" &
    pid=$! other=""
    while kill -0 "$pid" 2>/dev/null; do
      other=${other:-$(pgrep recharged | grep -vx "$pid" | head -1)}
      sleep 0.5
    done
    wait "$pid"; code=$?
    if [ -n "$other" ]; then echo "$s: another recharged ($other) ran meanwhile, retry"; continue; fi
    echo "$s exit=$code $(cat "${QA_OUT:-target/qa}/logs/bench-$s${BENCH_TAG:-}.json")"
    break
  done
done
