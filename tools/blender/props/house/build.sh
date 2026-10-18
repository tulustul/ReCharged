#!/usr/bin/env bash
# Regenerate the family-house props (assets/models/props/house/*.glb), then validate them
# against assets/data/props/house.ron.
# Usage: tools/blender/props/house/build.sh [--preview DIR] [prop ...]
set -euo pipefail
cd "$(dirname "$0")/../../../.."
preview=()
if [[ "${1:-}" == "--preview" ]]; then preview=(--preview "$2"); shift 2; fi
props=("$@")
if [[ ${#props[@]} -eq 0 ]]; then
    for f in tools/blender/props/house/*.py; do
        n=$(basename "$f" .py)
        [[ "$n" == housekit || "$n" == check || "$n" == layout ]] || props+=("$n")
    done
fi
for p in "${props[@]}"; do
    blender -b -P "tools/blender/props/house/$p.py" -- "${preview[@]}" 2>&1 \
        | grep -E "^\[[a-z_]+\] triangles|Error|Traceback|SystemExit" || true
done
python3 tools/blender/props/house/check.py
