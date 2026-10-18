#!/bin/bash
# Rebuild every Backyard Bash prop (assets/models/props/garden/*.glb), the layout and the decor,
# then validate. Previews: PREVIEW=<dir> tools/blender/props/garden/build.sh
set -e
cd "$(dirname "$0")/../../../.."
for s in bench sandbox kicker deck deck_ramp edging pots pot_cluster hose_reel hose_spiral shed \
         bbq knockables lights plants fence house trees round_bed veg_bed puddle_rims; do
  out=$(blender -b -P tools/blender/props/garden/$s.py -- ${PREVIEW:+--preview $PREVIEW} 2>&1)
  echo "$out" | grep -E "^\[.*\] triangles|rror|reaches" || { echo "$out" | tail -5; exit 1; }
done
python3 tools/blender/props/garden/plan.py
python3 tools/blender/props/garden/decor.py
python3 tools/blender/props/garden/check.py
