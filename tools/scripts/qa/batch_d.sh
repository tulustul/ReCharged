#!/usr/bin/env bash
# QA batch D: DoD-5 weather/time matrix, then DoD-2 track tours (2 windows at a time each).
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
tools/scripts/qa/weather_time.sh > target/qa/logs/weather_time.out 2>&1
echo "weather_time done: $(grep -c 'exit=0' target/qa/logs/weather_time.out) ok, $(grep -vc 'exit=0' target/qa/logs/weather_time.out) other"
tools/scripts/qa/tours.sh > target/qa/logs/tours.out 2>&1
echo "tours done: $(grep -c 'exit=0' target/qa/logs/tours.out) ok, $(grep -vc 'exit=0' target/qa/logs/tours.out) other"
