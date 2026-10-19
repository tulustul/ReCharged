#!/usr/bin/env bash
# QA batch C (short robustness/persistence runs, 2-3 windows at a time):
#   dod3_restart (app restart on the dod3 profile), corrupt saves, data load errors, read-only
#   data dir (save failure), no audio device, 4P at 720p (indoor storm, night) via --quick-race.
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
# A pointer resting over a game window steals menu focus (see docs/qa/report.md): park it.
DISPLAY=${DISPLAY:-:0} xdotool mousemove 5 1435 2>/dev/null || true
q=tools/scripts/qa
placed() { # x y name -- args...   (pinned run, placed window, log to target/qa/logs/<name>.log)
  local x=$1 y=$2 name=$3; shift 4
  $q/pinned.sh "$@" >/dev/null 2>"target/qa/logs/$name.log" &
  local pid=$!
  $q/place.sh $pid "$x" "$y" &
  wait $pid; echo "$name exit=$?"
}
placed 0 0 dod3_restart -- --user-data-dir target/qa/data/dod3 --window 1280x720 --autopilot \
  --input-script $q/dod3_restart.script --exit-after 400 &
sleep 2
( $q/corrupt.sh corrupt1 > target/qa/logs/corrupt1.out 2>&1; echo "corrupt exit=$?" ) &
sleep 2
placed 533 600 load_errors -- --user-data-dir target/qa/data/load_errors --window 1280x720 \
  --assets-dir target/qa/broken-assets --input-script $q/load_errors.script --exit-after 120 &
wait
# Read-only data dir: outside the pinned namespace (root-in-userns would bypass the permissions).
mkdir -p target/qa/data/save_fail/logs target/qa/data/save_fail/screenshots && chmod 555 target/qa/data/save_fail
target/qa/bin/recharged --user-data-dir target/qa/data/save_fail --window 1280x720 \
  --input-script $q/save_fail.script --exit-after 90 >/dev/null 2>target/qa/logs/save_fail.log
echo "save_fail exit=$?"; chmod 755 target/qa/data/save_fail; ls -la target/qa/data/save_fail
# No audio device: ALSA without a configuration (only this process).
( export ALSA_CONFIG_PATH=/nonexistent; placed 0 0 no_audio -- --user-data-dir target/qa/data/no_audio \
  --window 1280x720 --input-script tools/scripts/smoke.script --exit-after 120 ) &
sleep 2
placed 1067 600 vs4_720p -- --user-data-dir target/qa/data/vs4_720p --unlock-all --window 1280x720 \
  --quick-race toy_town --mode versus --players 4 --weather storm --time night --autopilot --seed 9 \
  --screenshot target/qa/shots/vs4_720p.png --screenshot-at 14 --screenshot-every 5 --exit-after 30 &
wait
