#!/usr/bin/env bash
# T21: one run of the tested build (live.sh) with a fresh data dir target/qa2/data/<name>
# (DATA=<dir name> uses another one, KEEP_DATA=1 keeps its contents), stdout/stderr in target/qa2/logs/<name>.{out,log}, the window placed at
# x,y; RSS_EVERY=<s> samples the RSS into target/qa2/logs/<name>.rss. Prints "<name> exit=<code>".
#   tools/scripts/qa/run2.sh <name> <x> <y> <game args...>
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
name=${1:?name} x=$2 y=$3
shift 3
data=target/qa2/data/${DATA:-$name}
mkdir -p "$(dirname "target/qa2/logs/$name")"
[ -n "${KEEP_DATA:-}" ] || rm -rf "$data"
tools/scripts/qa/live.sh --user-data-dir "$data" "$@" >"target/qa2/logs/$name.out" 2>"target/qa2/logs/$name.log" &
pid=$!
tools/scripts/qa/place.sh $pid "$x" "$y" &
if [ -n "${RSS_EVERY:-}" ]; then rm -f "target/qa2/logs/$name.rss"; tools/scripts/qa/rss.sh $pid "target/qa2/logs/$name.rss" "$RSS_EVERY" & fi
wait $pid
code=$?
echo "$name exit=$code"
