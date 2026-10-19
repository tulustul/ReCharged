#!/usr/bin/env bash
# T21 functional queue A (window at 0,0; runs beside queue B = at most 2 game windows): DoD-1 menu
# scripts, Q1/Q3 repros, spot checks, DoD-3 cup + restart, DoD-4 split-screen (+ colour-blind
# markers), robustness, feel races. One run at a time; summary lines "<run> exit=<code>".
#   tools/scripts/qa/t21_queue_a.sh [first step number]   → target/qa2/logs/, target/qa2/data/<run>/
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export QA_RUN=tools/scripts/qa/live.sh QA_OUT=target/qa2 DISPLAY=${DISPLAY:-:0}
export RUST_LOG=${RUST_LOG:-info,rc_audio=debug,rc_race=debug,rc_items=debug,bevy_react_core::js=debug,bevy_gilrs=error}
q=tools/scripts/qa
r() { $q/run2.sh "$1" 0 0 "${@:2}"; }
cb_settings() { mkdir -p "target/qa2/data/$1" && echo '(version: 1, data: (accessibility: (colorblind_markers: true)))' > "target/qa2/data/$1/settings.ron"; }
from=${1:-1} n=0
step() { n=$((n + 1)); [ $n -ge "$from" ] && { echo "== A$n $(date +%T)"; "$@"; }; }
step r smoke1 --window 1600x900 --input-script tools/scripts/smoke.script --exit-after 150
step r smoke2 --window 1600x900 --input-script tools/scripts/smoke.script --exit-after 150
step r smoke3 --window 1600x900 --input-script tools/scripts/smoke.script --exit-after 150
step r smoke4 --window 1600x900 --input-script tools/scripts/smoke.script --exit-after 150
step r kb_shipped --window 1280x720 --input-script tools/scripts/keyboard_only.script --exit-after 600
step r pad_shipped --window 1280x720 --input-script tools/scripts/gamepad_only.script --exit-after 600
step r kb_modes --window 1280x720 --input-script $q/kb_modes.script --exit-after 1000
step r tsr --unlock-all --window 1280x720 --input-script $q/trackselect_reach.script --exit-after 90
step r rebind1 --window 1280x720 --input-script $q/rebind_conflict.script --exit-after 60
step r q17_bench --bench maple_street_1p --window 1280x720 --exit-after 150
step env RECHARGED_UI_SCRIPT="2 nav confirm; 3 go settings" $q/run2.sh b27_ui_script 0 0 --window 1280x720 --exit-after 12
step r spot_b23 --window 1280x720 --input-script $q/spot_b23.script --exit-after 120
step r spot_b23_deck --window 1280x720 --input-script $q/spot_b23_deck.script --exit-after 150
step r spot_b15 --window 1280x720 --input-script $q/spot_b15.script --exit-after 150
step r dod3 --window 1280x720 --autopilot --input-script $q/dod3_cup.script --exit-after 1500
step env DATA=dod3 KEEP_DATA=1 $q/run2.sh dod3_restart 0 0 --window 1280x720 --autopilot \
  --input-script $q/dod3_restart.script --exit-after 400
step r vs2v --window 1280x720 --input-script $q/vs2_vertical.script --exit-after 800
step r vs3 --unlock-all --window 1280x720 --input-script $q/vs3.script --exit-after 800
step r vs4 --window 1280x720 --input-script $q/vs4.script --exit-after 800
step cb_settings vs3_cb
step env KEEP_DATA=1 $q/run2.sh vs3_cb 0 0 --unlock-all --window 1280x720 --input-script $q/vs3.script --exit-after 800
corrupt() { rm -rf target/qa2/data/corrupt1; $q/corrupt.sh corrupt1; }
step corrupt
save_fail() {
  local d=target/qa2/data/save_fail
  chmod -R u+w $d 2>/dev/null; rm -rf $d; mkdir -p $d/logs $d/screenshots && chmod 555 $d
  KEEP_DATA=1 $q/run2.sh save_fail 0 0 --window 1280x720 --input-script $q/save_fail.script --exit-after 90
  chmod 755 $d; ls -la $d
}
step save_fail
step r load_errors --window 1280x720 --assets-dir target/qa2/broken-assets --input-script $q/load_errors.script --exit-after 120
step env ALSA_CONFIG_PATH=/nonexistent $q/run2.sh no_audio 0 0 --window 1280x720 --input-script tools/scripts/smoke.script --exit-after 150
mkdir -p target/qa2/shots/feel
step r vs4_720p --unlock-all --window 1280x720 --quick-race toy_town --mode versus --players 4 --weather storm \
  --time night --autopilot --seed 9 --screenshot target/qa2/shots/vs4_720p.png --screenshot-at 14 --screenshot-every 5 --exit-after 32
step cb_settings cb1p
step env KEEP_DATA=1 $q/run2.sh cb1p 0 0 --unlock-all --window 1600x900 --quick-race sofa_circuit --autopilot --seed 3 \
  --screenshot target/qa2/shots/feel/cb1p.png --screenshot-at 10 --screenshot-every 3 --exit-after 32
for t in toy_town sofa_circuit backyard_bash aisle_rush wrench_works maple_street; do
  step r "feel_$t" --unlock-all --window 1600x900 --quick-race $t --autopilot --ai 7 --seed 11 \
    --screenshot "target/qa2/shots/feel/$t.png" --screenshot-at 12 --screenshot-every 10 --exit-after 175
done
echo "queue A done $(date +%T)"
