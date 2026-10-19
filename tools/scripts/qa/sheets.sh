#!/usr/bin/env bash
# QA: contact sheets (2 columns × 3 rows, labelled with the file name) of a run's screenshots.
#   tools/scripts/qa/sheets.sh <screenshot dir> <out prefix>   → <out prefix>_sheetN.png
set -euo pipefail
dir=$1 out=$2
mapfile -t files < <(ls "$dir"/*.png | sort)
n=0
for ((i = 0; i < ${#files[@]}; i += 6)); do
  magick montage -label '%t' "${files[@]:i:6}" -tile 2x3 -geometry 900x506+6+6 -pointsize 22 \
    "${out}_sheet$n.png"
  n=$((n + 1))
done
echo "$n sheets"
