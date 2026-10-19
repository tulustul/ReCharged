#!/usr/bin/env bash
# QA: samples the RSS (MB) of a process every N seconds until it exits.
#   tools/scripts/qa/rss.sh <pid> <out file> [secs=10]
pid=$1 out=$2 every=${3:-10}
start=$(date +%s)
while kill -0 "$pid" 2>/dev/null; do
  echo "$(( $(date +%s) - start )) $(( $(ps -o rss= -p "$pid" 2>/dev/null || echo 0) / 1024 ))" >> "$out"
  sleep "$every"
done
