#!/usr/bin/env bash
# QA DoD-3 / ROB-1: corrupt saves → recovery. Writes garbage profile/settings and a truncated
# ghost into a fresh data dir, runs corrupt.script, lists the data dir afterwards.
#   tools/scripts/qa/corrupt.sh <fresh data dir name under target/qa/data/>
set -euo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
dir="target/qa/data/$1"
[ -e "$dir" ] && { echo "$dir exists; pass a fresh name"; exit 2; }
mkdir -p "$dir/ghosts"
printf '(version: 1, data: (unlocked: (cars: ["sparky", ' > "$dir/profile.ron"      # truncated
printf 'this is not ron {{{\n' > "$dir/settings.ron"
printf '(version: 1, data: (samples: [' > "$dir/ghosts/toy_town.ghost"
tools/scripts/qa/pinned.sh --user-data-dir "$dir" --window 1600x900 \
  --input-script tools/scripts/qa/corrupt.script --exit-after 60 >/dev/null 2>"target/qa/logs/$1.log"
echo "exit=$?"
ls -la "$dir" "$dir/ghosts"
