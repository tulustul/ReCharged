#!/usr/bin/env bash
# T21 headless/CPU checks (no window; run beside the functional queues, never beside timing runs):
# DoD-2 every_layout + plots/lints of every layout, DoD-8 determinism (both tests + 3 CLI runs),
# Q6 item statistics from headless 8-car item races. Test binaries must be built beforehand
# (cargo test -p rc_ai --test race --no-run; cargo test -p recharged --no-run; plot example).
#   tools/scripts/qa/t21_headless.sh   → target/qa2/logs/{every_layout,determinism,plots,items_*}.log
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
L=target/qa2/logs
echo "== every_layout $(date +%T)"
( time nice cargo test -p rc_ai --test race -- --ignored --nocapture every_layout ) > $L/every_layout.log 2>&1
echo "every_layout exit=$? $(grep -E 'test result' $L/every_layout.log)"
echo "== determinism $(date +%T)"
( time nice cargo test -p recharged -- --include-ignored repeats_exactly ) > $L/determinism.log 2>&1
echo "determinism tests exit=$? $(grep -E 'test result' $L/determinism.log)"
for i in 1 2 3; do
  tools/scripts/qa/live.sh --user-data-dir target/qa2/data/det_cli$i --headless-race --quick-race backyard_bash \
    --weather storm --ai 7 --seed 7 > $L/det_cli$i.json 2> $L/det_cli$i.log
  echo "det_cli$i exit=$? md5 $(md5sum < $L/det_cli$i.json)"
done
echo "== plots $(date +%T)"
mkdir -p target/qa2/plots
: > $L/plots.log
for t in toy_town sofa_circuit backyard_bash aisle_rush wrench_works maple_street test_oval; do
  for v in "" "--rev" "--mirror" "--rev --mirror"; do
    # shellcheck disable=SC2086
    echo "## $t $v" >> $L/plots.log
    target/debug/examples/plot $t $v --out target/qa2/plots >> $L/plots.log 2>&1 || echo "plot $t $v exit=$?"
  done
done
for f in target/qa2/plots/*.svg; do magick "$f" "${f%.svg}.png" 2>/dev/null; done
echo "plots: $(ls target/qa2/plots/*.svg | wc -l) svg; lint lines: $(grep -ci lint $L/plots.log)"
echo "== item races $(date +%T)"
for t in toy_town backyard_bash maple_street sofa_circuit aisle_rush wrench_works; do
  for seed in 5 6 7; do
    RUST_LOG=warn,rc_items=debug,rc_race=debug tools/scripts/qa/live.sh --user-data-dir target/qa2/data/hli/${t}_$seed \
      --unlock-all --headless-race --quick-race $t --ai 7 --seed $seed > $L/items_${t}_$seed.json 2> $L/items_${t}_$seed.log
    echo "items $t $seed exit=$?"
  done
done
python3 tools/scripts/qa/item_stats.py $L/items_*.log > $L/item_stats.txt
cat $L/item_stats.txt
echo "== assist probe (Q7/B35: throttle-only + steering assist, 3 laps) $(date +%T)"
for t in toy_town sofa_circuit backyard_bash aisle_rush wrench_works maple_street; do
  AI_TRACK=$t AI_LAPS=3 nice cargo test -q -p rc_ai --test behaviour -- --ignored --nocapture assist_probe \
    > $L/assist_$t.log 2>&1
  echo "assist $t exit=$? $(grep -iE 'ratio|lap|respawn|finished' $L/assist_$t.log | tail -3 | tr '\n' ' ')"
done
echo "== Time Trial spread (B25: Elite autopilot Phantom on Toy Town, 8 seeds) $(date +%T)"
for seed in 1 2 3 4 5 6 7 8; do
  tools/scripts/qa/live.sh --user-data-dir target/qa2/data/tt_spread_$seed --unlock-all --headless-race \
    --quick-race toy_town --mode tt --seed $seed > $L/tt_spread_$seed.json 2> $L/tt_spread_$seed.log
  python3 -c "import json,sys; e=json.load(open(sys.argv[1]))['entries'][0]; print('tt seed', sys.argv[2], e['time_ticks']/60, 'best lap', e['best_lap_ticks']/60)" $L/tt_spread_$seed.json $seed
done
echo "headless done $(date +%T)"
