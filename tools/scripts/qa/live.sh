#!/usr/bin/env bash
# QA runner of the final pass (T21): the binary copied from the tested build (target/qa2/bin/,
# commit in target/qa2/COMMIT) with the tree's ui/dist and assets — nobody edits the tree during the
# pass, so no pinning (pinned.sh is the T18 runner). QA_BIN picks another binary (e.g. the release
# copy). Windowed runs wait while the user plays (a recharged without a target/ user-data dir).
#   tools/scripts/qa/live.sh --user-data-dir target/qa2/data/x --window 1600x900 ... --exit-after 60
# The QA scripts use it with QA_RUN=tools/scripts/qa/live.sh QA_OUT=target/qa2.
set -euo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
case " $* " in
  *" --headless-race "*|*" --calibrate-medals "*) ;;
  *) while pgrep -a recharged | grep -vqE -- '--user-data-dir [^ ]*target/'; do
       echo "live.sh: the user is playing ($(pgrep -a recharged | grep -vE -- '--user-data-dir [^ ]*target/' | head -1)); waiting" >&2
       sleep 30
     done ;;
esac
exec "${QA_BIN:-$root/target/qa2/bin/recharged}" "$@"
