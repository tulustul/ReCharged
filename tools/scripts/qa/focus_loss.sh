#!/usr/bin/env bash
# QA GDD §12.3: window focus loss during a race auto-pauses (skipped in automated runs by design,
# so this run uses no automation flag: RECHARGED_UI_SCRIPT drives the menus into a Quick Race,
# a zenity dialog takes the focus at ~24 s, ImageMagick `import` captures the window).
#   tools/scripts/qa/focus_loss.sh <fresh name>   → target/qa/shots/<name>_<t>.png
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export DISPLAY=${DISPLAY:-:0}
name=$1
xdotool mousemove 5 1435
RECHARGED_UI_SCRIPT="3.5 nav confirm; 5 nav down; 5.4 nav confirm; 7.4 nav confirm; 9.4 nav confirm" \
  timeout 40 tools/scripts/qa/pinned.sh --user-data-dir "target/qa/data/$name" --window 1280x720 \
  >/dev/null 2>"target/qa/logs/$name.log" &
pid=$!
sleep 0.5; gpid=$(pgrep -P $pid | head -1)
tools/scripts/qa/place.sh "$gpid" 0 0 &
prev=0
for t in 21 24 27 31 35; do
  sleep $(( t - prev )); prev=$t
  w=$(xdotool search --all --pid "$gpid" --name '^ReCharged$' 2>/dev/null | head -1)
  [ $t = 24 ] && (zenity --info --text "QA focus test (closes itself)" --timeout 5 >/dev/null 2>&1 &) && sleep 1
  [ -n "$w" ] && import -window "$w" "target/qa/shots/${name}_$t.png" 2>/dev/null
done
wait $pid; echo "exit=$? (124 = ended by timeout, expected)"
