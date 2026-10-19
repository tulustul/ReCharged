#!/usr/bin/env bash
# T21 solo stage (nothing else running — window focus, sink capture and RSS comparisons need it):
#  1. corrupt saves in a human-mode run: the "Save file restored" toast (corrupt_ui.sh)
#  2. a data load error in a human-mode run: error dialog → back in the menu (human.sh error)
#  3. audio sink capture per settings case (audio_check.sh → target/qa2/audio)
#  4. a 10-min soak at 1280×720 with RSS every 5 s, the conditions of T20c's 2.97 GB figure
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export QA_RUN=tools/scripts/qa/live.sh QA_OUT=target/qa2 DISPLAY=${DISPLAY:-:0}
q=tools/scripts/qa
from=${1:-1} n=0
step() { n=$((n + 1)); [ $n -ge "$from" ] && { echo "== F$n $(date +%T)"; "$@"; }; }
idle() { while pgrep -a recharged >/dev/null; do sleep 5; done; }
corrupt_ui() { rm -rf target/qa2/data/corrupt_ui1; $q/corrupt_ui.sh corrupt_ui1; }
step idle
step corrupt_ui
step idle
step $q/human.sh error t_load_error --assets-dir target/qa2/broken-assets
step idle
step $q/audio_check.sh
step idle
step env RSS_EVERY=5 $q/run2.sh soak10_720p 0 0 --window 1280x720 --soak 10 --soak-seed 1792365836540779723
echo "final stage done $(date +%T)"
