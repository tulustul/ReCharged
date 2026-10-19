#!/usr/bin/env bash
# DoD-5: screenshots of every weather (outdoor tracks) and every time of day (every track) in an
# autopilot quick race, 2 runs in parallel. Output: target/qa/shots/wt/<track>_<weather>_<time>_NNN.png
# (chase camera at 13 s and 19 s after start; first GO is ≈ 7 s after loading). Logs: target/qa/logs/wt/.
#   tools/scripts/qa/weather_time.sh            # the whole matrix
#   tools/scripts/qa/weather_time.sh maple_street:snow:night ...   # chosen cells
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
# A pointer resting over a game window steals menu focus (see docs/qa/report.md): park it.
DISPLAY=${DISPLAY:-:0} xdotool mousemove 5 1435 2>/dev/null || true
mkdir -p target/qa/shots/wt target/qa/logs/wt
cells=("$@")
if [ ${#cells[@]} -eq 0 ]; then
  for t in backyard_bash maple_street; do
    for w in clear cloudy rain storm fog snow; do cells+=("$t:$w:noon"); done
  done
  for t in toy_town sofa_circuit backyard_bash wrench_works maple_street; do
    for tod in morning sunset night; do cells+=("$t:clear:$tod"); done
  done
  cells+=("toy_town:clear:noon" "sofa_circuit:clear:noon" "wrench_works:clear:noon")
  cells+=("aisle_rush:clear:sunset" "aisle_rush:clear:night")
  cells+=("toy_town:storm:night" "sofa_circuit:rain:sunset" "wrench_works:storm:noon")
fi
run() { # cell x y
  IFS=: read -r t w tod <<<"$1"
  local name="${t}_${w}_${tod}"
  tools/scripts/qa/pinned.sh --user-data-dir "target/qa/data/wt/$name" --unlock-all --window 1600x900 \
    --quick-race "$t" --weather "$w" --time "$tod" --autopilot --seed 4 \
    --screenshot "target/qa/shots/wt/$name.png" --screenshot-at 13 --screenshot-every 6 --exit-after 21 \
    >/dev/null 2>"target/qa/logs/wt/$name.log" &
  local pid=$!
  tools/scripts/qa/place.sh $pid "$2" "$3" &
  wait $pid
  echo "$name exit=$?"
}
for ((i = 0; i < ${#cells[@]}; i += 2)); do
  run "${cells[i]}" 0 0 &
  [ -n "${cells[i+1]:-}" ] && run "${cells[i+1]}" 693 390 &
  wait
done
