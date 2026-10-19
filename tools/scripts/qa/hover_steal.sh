#!/usr/bin/env bash
# QA: hover-steal repro. Runs hover_steal.script twice in a 1280x720 window placed at (0,0):
# with the pointer resting over the Settings "Hints" row position, then parked off-window.
#   tools/scripts/qa/hover_steal.sh <name suffix>   → target/qa/data/hover_<suffix>_{on,off}/screenshots
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export DISPLAY=${DISPLAY:-:0}
for mode in on off; do
  if [ $mode = on ]; then xdotool mousemove 864 591; else xdotool mousemove 5 1435; fi
  tools/scripts/qa/pinned.sh --user-data-dir "target/qa/data/hover_$1_$mode" --window 1280x720 \
    --input-script tools/scripts/qa/hover_steal.script --exit-after 60 >/dev/null 2>"target/qa/logs/hover_$1_$mode.log" &
  pid=$!
  tools/scripts/qa/place.sh $pid 0 0 &
  wait $pid; echo "$mode exit=$?"
done
xdotool mousemove 5 1435
