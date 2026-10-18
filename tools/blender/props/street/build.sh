#!/usr/bin/env bash
# Regenerate every Maple Street asset: the prop models (Blender), the prop catalog, the track
# layout and the decor (plan.py), then validate. Usage:
#   tools/blender/props/street/build.sh [--preview DIR]
set -euo pipefail
cd "$(dirname "$0")/../../../.."
here=tools/blender/props/street
args=("$@")
python3 "$here/plan.py"
for script in kerbs parked_suv yard_ramps storm_drain street_lamp house maple kit_flat kit_small; do
    blender -b -P "$here/$script.py" -- "${args[@]}" 2>&1 \
        | grep -E "^\[[a-z0-9_]+\] (triangles|preview)|Error|Traceback|reaches" || true
done
python3 "$here/check.py"
