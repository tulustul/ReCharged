#!/usr/bin/env bash
# QA: contact sheets of a weather_time.sh matrix (frame N of each cell, default 001 = 19 s):
# weather per outdoor track, time of day per track, the indoor-weather/misc cells, and — when an
# older matrix is given — old vs new time-of-day sheets (Q8).
#   tools/scripts/qa/wt_sheets.sh <shots dir> <out dir> [old shots dir] [frame=001]
set -uo pipefail
dir=$1 out=$2 old=${3:-} f=${4:-001}
mkdir -p "$out"
sheet() { # name tile files...
  local name=$1 tile=$2; shift 2
  local files=() x
  for x in "$@"; do [ -f "$x" ] && files+=("$x"); done
  [ ${#files[@]} -gt 0 ] && magick montage -label '%t' "${files[@]}" -tile "$tile" -geometry 900x506+6+6 -pointsize 22 "$out/$name.png"
}
for t in backyard_bash maple_street; do
  sheet "wt_${t}_weather" 2x3 $(for w in clear cloudy rain storm fog snow; do echo "$dir/${t}_${w}_noon_$f.png"; done)
done
for t in toy_town sofa_circuit backyard_bash wrench_works maple_street; do
  sheet "wt_tod_$t" 2x2 $(for tod in morning noon sunset night; do echo "$dir/${t}_clear_${tod}_$f.png"; done)
  [ -n "$old" ] && sheet "wt_tod_${t}_old_vs_new" 2x4 \
    $(for tod in morning noon sunset night; do echo "$old/${t}_clear_${tod}_$f.png" "$dir/${t}_clear_${tod}_$f.png"; done)
done
sheet wt_misc 2x3 "$dir/aisle_rush_clear_sunset_$f.png" "$dir/aisle_rush_clear_night_$f.png" \
  "$dir/toy_town_storm_night_$f.png" "$dir/sofa_circuit_rain_sunset_$f.png" "$dir/wrench_works_storm_noon_$f.png"
ls "$out"
