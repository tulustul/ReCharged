#!/usr/bin/env bash
# QA ROB-1: the "Save file restored" toast in a normal (non-automated) run. In automated runs the
# first frames stall ~5 s on synchronous pipeline compilation while React's wall-clock timers run,
# so the 4.5 s toast (and the 2.6 s splash) expire unseen. No automation flag here; `timeout` ends
# the run, ImageMagick `import` captures the window at 1.5 / 3 / 4.5 s.
#   tools/scripts/qa/corrupt_ui.sh <fresh name>   → ${QA_OUT:-target/qa}/shots/<name>_<t>.png
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export DISPLAY=${DISPLAY:-:0}
name=$1 dir="${QA_OUT:-target/qa}/data/$1"
[ -e "$dir" ] && { echo "$dir exists; pass a fresh name"; exit 2; }
mkdir -p "$dir"
printf '(version: 1, data: (unlocked: (cars: ["sparky", ' > "$dir/profile.ron"
printf 'this is not ron {{{\n' > "$dir/settings.ron"
xdotool mousemove 5 1435
timeout 15 ${QA_RUN:-tools/scripts/qa/pinned.sh} --user-data-dir "$dir" --window 1280x720 >/dev/null 2>"${QA_OUT:-target/qa}/logs/$name.log" &
pid=$!
sleep 0.5; gpid=$(pgrep -P $pid | head -1)   # the game: timeout's child
tools/scripts/qa/place.sh "$gpid" 0 0 &
prev=0
for t in 2 3 4 5 6 8; do
  sleep $(( t - prev )); prev=$t
  w=$(xdotool search --all --pid "$gpid" --name '^ReCharged$' 2>/dev/null | head -1)
  [ -n "$w" ] && import -window "$w" "${QA_OUT:-target/qa}/shots/${name}_$t.png" 2>/dev/null
done
wait $pid; echo "exit=$? (124 = ended by timeout, expected)"; ls "$dir"
