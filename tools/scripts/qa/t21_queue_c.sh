#!/usr/bin/env bash
# T21 extra functional runs found during the pass (window at C_XY, default 1067,600; at most one
# other game window beside it): an idle human at the start (do rivals get past a parked car?),
# the Quick Race post-race "Change track" options (Q12), the missed-checkpoint notice on Maple Street (B15), wrong way, the podium_fanfare stinger
# (a one-race cup ending on silver/bronze).
#   tools/scripts/qa/t21_queue_c.sh [first step number]
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export QA_RUN=tools/scripts/qa/live.sh QA_OUT=target/qa2 DISPLAY=${DISPLAY:-:0}
export RUST_LOG=${RUST_LOG:-info,rc_audio=debug,rc_race=debug,rc_items=debug,bevy_react_core::js=debug,bevy_gilrs=error}
q=tools/scripts/qa
r() { $q/run2.sh "$1" ${C_XY:-1067 600} "${@:2}"; }
from=${1:-1} n=0
step() { n=$((n + 1)); [ $n -ge "$from" ] && { echo "== C$n $(date +%T)"; "$@"; }; }
step r spot_idle --window 1280x720 --input-script $q/spot_idle_start.script --exit-after 120
step r qr_change --unlock-all --autopilot --window 1280x720 --input-script $q/qr_change_track.script --exit-after 240
step r spot_b15_maple --unlock-all --window 1280x720 --input-script $q/spot_b15_maple.script --exit-after 150
step r spot_wrongway --window 1280x720 --input-script $q/spot_wrongway.script --exit-after 120
assist() { mkdir -p "target/qa2/data/$1" && echo '(version: 1, data: (accessibility: (steering_assist: true)))' > "target/qa2/data/$1/settings.ron"; }
for v in "" 5; do
  step assist "podium$v"
  step env KEEP_DATA=1 RUST_LOG=info,rc_audio=debug,bevy_gilrs=error $q/run2.sh "podium$v" ${C_XY:-1067 600} --window 1280x720 \
    --quick-race aisle_rush --mode cup:rookie_cup --input-script "$q/spot_podium$v.script" --exit-after 240
done
echo "queue C done $(date +%T)"
