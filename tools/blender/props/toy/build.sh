#!/usr/bin/env bash
# Regenerate every toy-store prop and decor model (assets/models/props/toy/<prop>.glb), then validate them.
# Usage: tools/blender/props/toy/build.sh [--preview DIR] [prop ...]
# The support pillars of the track are placed separately: python3 tools/blender/props/toy/supports.py
set -euo pipefail
cd "$(dirname "$0")/../../../.."
preview=()
if [[ "${1:-}" == "--preview" ]]; then preview=(--preview "$2"); shift 2; fi
props=("$@")
if [[ ${#props[@]} -eq 0 ]]; then
    props=(block_cube block_plank block_pillar block_roof block_arch block_pile track_support
           track_ramp dollhouse castle play_table shelf_a shelf_b cubby start_arch bear ball_pit
           rocking_horse toy_box robot train stacking_rings duck crayons foam_tiles bunting sign
           tunnel light_arch fairy_lights balloons wall wall_gate wall_curve)
fi
for p in "${props[@]}"; do
    blender -b -P "tools/blender/props/toy/$p.py" -- "${preview[@]}" 2>&1 \
        | grep -E "^\[$p\] (triangles|preview)|reaches|Error|Traceback" || true
done
python3 tools/blender/props/toy/check.py
