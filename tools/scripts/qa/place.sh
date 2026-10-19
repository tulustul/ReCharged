#!/usr/bin/env bash
# QA: moves the game window of <pid> to <x> <y> (physical px) once it appears, so parallel runs
# do not cover each other completely (fully covered windows may be throttled by the driver).
#   tools/scripts/qa/place.sh <pid> <x> <y>
export DISPLAY=${DISPLAY:-:0}
pid=$1 x=$2 y=$3
for _ in $(seq 120); do
  w=$(xdotool search --all --pid "$pid" --name '^ReCharged$' 2>/dev/null | head -1)
  if [ -n "$w" ]; then
    # i3 re-centres the window when the game reconfigures it after creation: move it again later.
    for delay in 1 3 6; do
      sleep $delay
      i3-msg -q "[id=$w] floating enable, move position $x px $y px" 2>/dev/null || xdotool windowmove "$w" "$x" "$y"
    done
    exit 0
  fi
  sleep 0.5
done
