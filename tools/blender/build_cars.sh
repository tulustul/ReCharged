#!/usr/bin/env bash
# Regenerate every car: assets/models/cars/<id>.glb + <id>.wheels.ron + assets/ui/cars/<id>.png,
# then validate them. Usage: tools/blender/build_cars.sh [--preview DIR] [car ...]
set -euo pipefail
cd "$(dirname "$0")/../.."
preview=()
if [[ "${1:-}" == "--preview" ]]; then preview=(--preview "$2"); shift 2; fi
cars=("$@")
[[ ${#cars[@]} -eq 0 ]] && cars=(sparky dumpster zip torque hornet phantom volt_gt bolt)
for car in "${cars[@]}"; do
    blender -b -P "tools/blender/cars/$car.py" -- "${preview[@]}" 2>&1 \
        | grep -E "^\[$car\]|^(decal|strip|shell):|Error|Traceback" || true
done
python3 tools/blender/check_cars.py
