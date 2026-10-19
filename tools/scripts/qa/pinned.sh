#!/usr/bin/env bash
# QA runner: the QA build (target/qa/bin/recharged, commit in target/qa/bin/COMMIT) with the UI
# bundle and assets pinned to that commit (target/qa/pin/), while other agents edit the tree.
# A private mount namespace binds the snapshots over ui/dist and assets for this process only.
#   tools/scripts/qa/pinned.sh --user-data-dir target/qa/data/x --window 1600x900 ... --exit-after 60
# Without the pinning the same run is: cargo run -p recharged -- <args>
# Windowed runs wait while the user plays (a recharged without a target/ user-data dir).
set -euo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
case " $* " in
  *" --headless-race "*|*" --calibrate-medals "*) ;;
  *) while pgrep -a recharged | grep -vqE -- '--user-data-dir [^ ]*target/'; do
       echo "pinned.sh: the user is playing ($(pgrep -a recharged | grep -vE -- '--user-data-dir [^ ]*target/' | head -1)); waiting" >&2
       sleep 30
     done ;;
esac
exec unshare -rm sh -c '
  mount --bind "$0/target/qa/pin/ui-dist" "$0/ui/dist" &&
  mount --bind "$0/target/qa/pin/assets" "$0/assets" &&
  cd "$0" && exec "$0/target/qa/bin/recharged" "$@"' "$root" "$@"
