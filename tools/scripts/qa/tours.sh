#!/usr/bin/env bash
# DoD-2: --track-tour of every track forward and reverse (2 in parallel), then contact sheets of
# 12 frames each: ${QA_OUT:-target/qa}/shots/tours/<track>_<fwd|rev>_sheetN.png (frame numbers = 25 u steps).
#   tools/scripts/qa/tours.sh [track[:rev]] ...   (QA_XY="x y": position of the first window)
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
# A pointer resting over a game window steals menu focus (see docs/qa/report.md): park it.
DISPLAY=${DISPLAY:-:0} xdotool mousemove 5 1435 2>/dev/null || true
mkdir -p ${QA_OUT:-target/qa}/shots/tours ${QA_OUT:-target/qa}/logs/tours
cells=("$@")
[ ${#cells[@]} -eq 0 ] && for t in toy_town sofa_circuit backyard_bash aisle_rush wrench_works maple_street; do cells+=("$t:fwd" "$t:rev"); done
run() { # cell x y
  IFS=: read -r t dir <<<"$1"
  local flag=() name="${t}_${dir}"
  [ "$dir" = rev ] && flag=(--reverse)
  ${QA_RUN:-tools/scripts/qa/pinned.sh} --user-data-dir "${QA_OUT:-target/qa}/data/tour/$name" --unlock-all --window 1280x720 \
    --track-tour "$t" "${flag[@]}" --exit-after 300 >/dev/null 2>"${QA_OUT:-target/qa}/logs/tours/$name.log" &
  local pid=$!
  tools/scripts/qa/place.sh $pid "$2" "$3" &
  wait $pid
  echo "$name exit=$?"
  local dir_out
  dir_out=$(ls -d "${QA_OUT:-target/qa}/data/tour/$name/captures/"tour_* 2>/dev/null | head -1)
  [ -n "$dir_out" ] || return
  local frames=("$dir_out"/*.png) i=0 n=0
  while [ $i -lt ${#frames[@]} ]; do
    magick montage -label '%t' "${frames[@]:i:12}" -tile 4x3 -geometry 480x270+4+4 \
      -title "$name" "${QA_OUT:-target/qa}/shots/tours/${name}_sheet$n.png" 2>/dev/null
    i=$((i + 12)); n=$((n + 1))
  done
}
for ((i = 0; i < ${#cells[@]}; i += 2)); do
  run "${cells[i]}" ${QA_XY:-0 0} &
  [ -n "${cells[i+1]:-}" ] && run "${cells[i+1]}" 1067 600 &
  wait
done
