#!/usr/bin/env bash
# T21 functional queue B (window at 1067,600; runs beside queue A = at most 2 game windows):
# pad_modes, the 30-min soak with RSS sampling (seed of the T18 soak), the DoD-5 weather/time
# matrix and the DoD-2 tours of the four tracks whose data changed, one window at a time.
#   tools/scripts/qa/t21_queue_b.sh [first step number]
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export QA_RUN=tools/scripts/qa/live.sh QA_OUT=target/qa2 QA_XY="1067 600" DISPLAY=${DISPLAY:-:0}
q=tools/scripts/qa
from=${1:-1} n=0
step() { n=$((n + 1)); [ $n -ge "$from" ] && { echo "== B$n $(date +%T)"; "$@"; }; }
step env RUST_LOG=info,rc_audio=debug,bevy_react_core::js=debug,bevy_gilrs=error \
  $q/run2.sh pad_modes 1067 600 --window 1280x720 --input-script $q/pad_modes.script --exit-after 1000
step env RSS_EVERY=10 RUST_LOG=info,rc_audio=debug,bevy_gilrs=error \
  $q/run2.sh soak30 1067 600 --soak 30 --soak-seed 1792365836540779723
cells=()
for t in backyard_bash maple_street; do for w in clear cloudy rain storm fog snow; do cells+=("$t:$w:noon"); done; done
for t in toy_town sofa_circuit backyard_bash wrench_works maple_street; do
  for tod in morning sunset night; do cells+=("$t:clear:$tod"); done
done
cells+=("toy_town:clear:noon" "sofa_circuit:clear:noon" "wrench_works:clear:noon")
cells+=("aisle_rush:clear:sunset" "aisle_rush:clear:night")
cells+=("toy_town:storm:night" "sofa_circuit:rain:sunset" "wrench_works:storm:noon")
for c in "${cells[@]}"; do step env RUST_LOG=info,rc_render=debug,rc_audio=debug,bevy_gilrs=error $q/weather_time.sh "$c"; done
for c in toy_town:fwd toy_town:rev wrench_works:fwd wrench_works:rev backyard_bash:fwd backyard_bash:rev \
  maple_street:fwd maple_street:rev; do step $q/tours.sh "$c"; done
echo "queue B done $(date +%T)"
