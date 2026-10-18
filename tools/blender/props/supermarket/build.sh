#!/bin/sh
# Rebuild everything for Aisle Rush: textures, every prop glb, the prop catalog, the track and
# decor layout. Pass --preview DIR to also render previews. Run from anywhere.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../../.." && pwd)
cd "$ROOT"
python3 "$HERE/textures.py"
for p in gondola endcap freezer cooler checkout lane_gate service_desk railing box_jump \
         water_pallet water_ramp can pallet_stack pallet crate_stack produce_table produce_bin \
         produce_rack trolley basket wet_floor_sign mop_bucket boxes exit_sign \
         hanging_sign ceiling_light wall_sign wall_band topstock paper_bag \
         entrance_doors; do
  out=$(blender -b -P "$HERE/$p.py" -- "$@" 2>&1)
  echo "$out" | grep -E "triangles|reaches" || true
  # a Python error or a propkit budget / ground-contact refusal leaves a stale glb: stop
  if echo "$out" | grep -qE "Traceback|> budget|below the origin"; then
    echo "$out" | tail -20
    exit 1
  fi
done
python3 "$HERE/catalog.py"
python3 "$HERE/plan.py"
python3 "$HERE/check.py"
