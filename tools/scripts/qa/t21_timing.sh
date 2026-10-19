#!/usr/bin/env bash
# T21 timing block — run with nothing else on the machine (no other recharged, no cargo build):
#  1. bench.sh on the tested dev build: the T18 table's 12 scenarios (docs/qa/report.md §3.8)
#  2. bench.sh on the release copy: 1P/4P Toy Town and Backyard Bash (fps + cold release loads)
#  3. menu-flow loads after the menu warm-up with T20c's measurement build (menu_loads*.script)
#  4. human-mode runs of the tested build (human.sh): menu warm-up log, a Quick Race started from the
#     menu with real keys after the warm-up (+ focus-loss pause), window close 1 s after start (B2)
# Results: target/qa2/logs/ (bench-*.json, t_*.log), summary in target/qa2/logs/t21_timing.out.
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export QA_RUN=tools/scripts/qa/live.sh QA_OUT=target/qa2 DISPLAY=${DISPLAY:-:0}
q=tools/scripts/qa
xdotool mousemove 5 1435
$q/bench.sh toy_town_1p toy_town_2p toy_town_3p toy_town_4p backyard_bash_1p backyard_bash_2p \
  backyard_bash_4p maple_street_1p maple_street_2p maple_street_4p
BENCH_ARGS="--weather storm --time night" BENCH_TAG=_storm_night $q/bench.sh maple_street_1p maple_street_4p
QA_BIN=$root/target/qa2/bin/recharged-release BENCH_TAG=_rel $q/bench.sh toy_town_1p toy_town_4p \
  backyard_bash_1p backyard_bash_4p
idle() { while pgrep -a recharged >/dev/null; do sleep 5; done; }
M=$root/target/agent-data/T20c/bin/recharged-measure-dev9
idle; QA_BIN=$M RUST_LOG=info,rc_render=debug $q/run2.sh t_menu_loads 0 0 --window 1280x720 \
  --input-script $q/menu_loads.script --exit-after 300
idle; QA_BIN=$M RUST_LOG=info,rc_render=debug $q/run2.sh t_menu_loads_vs4 0 0 --window 1280x720 \
  --input-script $q/menu_loads_vs4.script --exit-after 200
idle; $q/human.sh idle t_human_idle 45
idle; $q/human.sh race t_human_race
idle; $q/human.sh close t_human_close 1
idle; $q/human.sh close t_human_close3 3
echo "timing block done"
