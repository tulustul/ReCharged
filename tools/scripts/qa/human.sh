#!/usr/bin/env bash
# T21: human-mode runs of the tested build (no automation flag: vsync, async pipelines, the menu
# warm-up, deferred exit, focus-loss pause). Log: target/qa2/logs/<name>.log (RUST_LOG defaults to
# info,rc_render=debug), captures: target/qa2/shots/<name>_*.png (ImageMagick import).
#   human.sh idle <name> [secs=45]   sit in the title while the menu warm-up runs, then SIGTERM
#   human.sh close <name> [secs=1]   close the window (i3 kill = WM_DELETE) <secs> after it appears (B2)
#   human.sh error <name> --assets-dir <dir>   Quick Race into a load error: dialog, dismiss, menu
#   human.sh race <name>             after "menu warm-up done": real key presses (xdotool, only while
#                                    the game window has the focus) start a Quick Race on the focused
#                                    track → load time from the menu; ~8 s after GO a zenity dialog
#                                    takes the focus → the race must auto-pause; then SIGTERM
# Extra game args after the mode arguments are passed through (e.g. --assets-dir for a load error).
# A background job ignores SIGINT, so the game is stopped with SIGTERM (docs/tasks/T21-final-qa.md).
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
export DISPLAY=${DISPLAY:-:0}
mode=$1 name=$2 arg=
shift 2
case ${1:-} in --*|'') ;; *) arg=$1; shift ;; esac
log=target/qa2/logs/$name.log shots=target/qa2/shots
mkdir -p "$shots"
xdotool mousemove 5 1435
rm -rf "target/qa2/data/$name"
RUST_LOG=${RUST_LOG:-info,rc_render=debug} tools/scripts/qa/live.sh --user-data-dir "target/qa2/data/$name" \
  --window 1280x720 "$@" >/dev/null 2>"$log" &
pid=$!
win() { xdotool search --all --pid "$pid" --name '^ReCharged$' 2>/dev/null | head -1; }
shot() { local w; w=$(win); [ -n "$w" ] && import -window "$w" "$shots/${name}_$1.png" 2>/dev/null; }
until_log() { # pattern timeout_s
  local t=0
  until grep -q "$1" "$log" 2>/dev/null; do
    sleep 0.25; t=$((t + 1))
    kill -0 $pid 2>/dev/null || return 1
    [ $t -ge $(($2 * 4)) ] && return 1
  done
  return 0
}
key() { # only into our own focused window
  local w; w=$(win)
  if [ -z "$w" ] || [ "$(xdotool getactivewindow 2>/dev/null)" != "$w" ]; then
    echo "$name: the game window does not have the focus; key $1 not sent" >&2; return 1
  fi
  xdotool key --clearmodifiers "$1"
}
stop() { kill -TERM $pid 2>/dev/null; for _ in $(seq 40); do kill -0 $pid 2>/dev/null || break; sleep 0.25; done; kill -9 $pid 2>/dev/null; }
for _ in $(seq 120); do [ -n "$(win)" ] && break; sleep 0.25; done
w=$(win)
case $mode in
  idle)
    sleep "${arg:-45}"; shot idle; stop ;;
  close)
    sleep "${arg:-1}"
    i3-msg -q "[id=$w] kill"
    for _ in $(seq 40); do kill -0 $pid 2>/dev/null || break; sleep 0.25; done ;;
  race)
    xdotool windowactivate --sync "$w" 2>/dev/null
    until_log 'menu warm-up done' 90 || echo "$name: no 'menu warm-up done'" >&2
    sleep 1; shot 01_title
    key space; sleep 1.2          # Title
    key Down; sleep 0.5           # Quick Race
    key Return; sleep 1.5         # → Track select
    key Return; sleep 0.8         # the focused track: select → Choose car
    key Return; sleep 1.5         # → Car select
    shot 02_car_select
    key Return                    # race
    until_log 'Some(InRace)' 30 || echo "$name: no InRace" >&2
    until_log 'race start:' 40 || echo "$name: no GO" >&2
    sleep 8; shot 03_racing
    (zenity --info --text "QA focus test (closes itself)" --timeout 4 >/dev/null 2>&1 &)
    sleep 1.5; shot 04_focus_lost
    sleep 4; shot 05_after_dialog
    stop ;;
  error) # with --assets-dir <broken copy>: the error dialog, then back in the menu (ROB-1)
    xdotool windowactivate --sync "$w" 2>/dev/null
    until_log 'UI screen: title' 60; sleep 2
    key space; sleep 1.2; key Down; sleep 0.5; key Return; sleep 1.5   # Title → Quick Race
    key Return; sleep 0.8; key Return; sleep 1.5                       # Toy Town → Car select
    key Return                                                         # race → load error
    sleep 6; shot 01_error
    key Return; sleep 2; shot 02_after_dismiss
    stop ;;
esac
wait $pid 2>/dev/null; code=$?
echo "$name exit=$code"
xdotool mousemove 5 1435
