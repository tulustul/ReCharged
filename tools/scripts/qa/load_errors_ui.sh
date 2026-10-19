#!/usr/bin/env bash
# QA ROB-1: a data load failure in a normal (non-automated) run must show the error and return to
# the menu. Automated runs exit with code 1 on a load failure by design (rc_game exits_on_error), so
# this run uses no automation flag: rc_ui's RECHARGED_UI_SCRIPT drives the menus, `timeout` ends
# the process, ImageMagick `import` captures the window. Assets: target/qa/broken-assets (Toy Town
# track truncated, items.ron with a bad field).
#   tools/scripts/qa/load_errors_ui.sh <fresh name>   → target/qa/shots/<name>_NN.png
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export DISPLAY=${DISPLAY:-:0}
name=$1
xdotool mousemove 5 1435
RECHARGED_UI_SCRIPT="3.5 nav confirm; 5 nav down; 5.4 nav confirm; 7.4 nav confirm; 9.4 nav confirm; 16 nav confirm" \
  timeout 40 tools/scripts/qa/pinned.sh --user-data-dir "target/qa/data/$name" \
  --assets-dir target/qa/broken-assets --window 1280x720 >/dev/null 2>"target/qa/logs/$name.log" &
pid=$!
sleep 0.5; gpid=$(pgrep -P $pid | head -1)   # the game: timeout's child
tools/scripts/qa/place.sh "$gpid" 0 0 &
for t in 12 15 19 24; do
  sleep $(( t - ${prev:-0} )); prev=$t
  w=$(xdotool search --all --pid "$gpid" --name '^ReCharged$' 2>/dev/null | head -1)
  [ -n "$w" ] && import -window "$w" "target/qa/shots/${name}_$t.png" 2>/dev/null
done
wait $pid; echo "exit=$? (124 = ended by timeout, expected)"
