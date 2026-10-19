#!/usr/bin/env bash
# QA batch A (3 windows in parallel, 1280x720 logical, placed so none is fully covered): DoD-1 keyboard-only and gamepad-only mode scripts, DoD-3
# Rookie Cup + Time Trial (autopilot) with audio/item/race debug logs for DoD-6.
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
# A pointer resting over a game window steals menu focus (see docs/qa/report.md): park it.
DISPLAY=${DISPLAY:-:0} xdotool mousemove 5 1435 2>/dev/null || true
q=tools/scripts/qa
$q/pinned.sh --user-data-dir target/qa/data/kb_modes --window 1280x720 \
  --input-script $q/kb_modes.script --exit-after 900 >/dev/null 2>target/qa/logs/kb_modes.log &
p1=$!
$q/place.sh $p1 0 0 &
sleep 2
$q/pinned.sh --user-data-dir target/qa/data/pad_modes --window 1280x720 \
  --input-script $q/pad_modes.script --exit-after 900 >/dev/null 2>target/qa/logs/pad_modes.log &
p2=$!
$q/place.sh $p2 1067 0 &
sleep 2
RUST_LOG=info,rc_audio=debug,rc_items=debug,rc_race=debug,bevy_gilrs=error \
  $q/pinned.sh --user-data-dir target/qa/data/dod3 --window 1280x720 --autopilot \
  --input-script $q/dod3_cup.script --exit-after 1300 >/dev/null 2>target/qa/logs/dod3_cup.log &
p3=$!
$q/place.sh $p3 533 600 &
$q/rss.sh $p1 target/qa/logs/kb_modes_rss.log 10 &
wait $p1; echo "kb_modes exit=$?"
wait $p2; echo "pad_modes exit=$?"
wait $p3; echo "dod3_cup exit=$?"
