#!/usr/bin/env bash
# QA batch B: the shipped DoD-1 scripts (keyboard_only, gamepad_only) and the DoD-4 split-screen
# scripts (vs2_vertical, vs3, vs4), 3 windows at a time.
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
# A pointer resting over a game window steals menu focus (see docs/qa/report.md): park it.
DISPLAY=${DISPLAY:-:0} xdotool mousemove 5 1435 2>/dev/null || true
q=tools/scripts/qa
run() { # name x y script extra...
  local name=$1 x=$2 y=$3 script=$4; shift 4
  $q/pinned.sh --user-data-dir "target/qa/data/$name" --window 1280x720 --input-script "$script" \
    --exit-after 800 "$@" >/dev/null 2>"target/qa/logs/$name.log" &
  local pid=$!
  $q/place.sh $pid "$x" "$y" &
  wait $pid
  echo "$name exit=$?"
}
run kb_shipped 0 0 tools/scripts/keyboard_only.script &
sleep 2; run pad_shipped 1067 0 tools/scripts/gamepad_only.script &
sleep 2; run vs2v 533 600 $q/vs2_vertical.script &
wait
run vs3 0 0 $q/vs3.script --unlock-all &
sleep 2; run vs4 1067 600 $q/vs4.script &
wait
