#!/usr/bin/env bash
# QA DoD-6: is sound really produced and mixed by the Settings buses? Records the default sink's
# monitor while one game runs alone (nothing else may play audio) and prints the RMS level (dBFS)
# per second. Runs: menu with default volumes (mute_unfocused off), menu with music 0, menu with
# mute_unfocused on (the automated window is usually unfocused), an autopilot race (engines + SFX).
#   tools/scripts/qa/audio_check.sh   → target/qa/audio/<run>.wav + RMS table
set -uo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
mkdir -p target/qa/audio
sink=$(pactl get-default-sink)
others=$(pactl list short sink-inputs | wc -l)
[ "$others" -gt 0 ] && echo "warning: $others other audio streams are playing"
settings() { # music mute_unfocused
  printf '(version: 1, data: (audio: (master: 1.0, music: %s, sfx: 0.9, engines: 0.8, ui: 0.8, ambience: 0.7, mute_unfocused: %s)))\n' "$1" "$2"
}
run() { # name music mute args...
  local name=$1 music=$2 mute=$3; shift 3
  local dir="target/qa/data/audio_$name"
  mkdir -p "$dir" && settings "$music" "$mute" > "$dir/settings.ron"
  parecord --device="$sink.monitor" --file-format=wav --channels=2 --rate=48000 "target/qa/audio/$name.wav" &
  local rec=$!
  tools/scripts/qa/pinned.sh --user-data-dir "$dir" --window 1280x720 "$@" >/dev/null 2>"target/qa/logs/audio_$name.log"
  kill $rec; wait $rec 2>/dev/null
  python3 - "target/qa/audio/$name.wav" "$name" <<'PY'
import sys, wave, numpy as np
w = wave.open(sys.argv[1]); n = w.getnframes(); rate = w.getframerate()
a = np.frombuffer(w.readframes(n), dtype=np.int16).astype(np.float32) / 32768.0
a = a.reshape(-1, w.getnchannels()).mean(axis=1)
sec = [a[i:i + rate] for i in range(0, len(a) - rate + 1, rate)]
db = [20 * np.log10(max(np.sqrt(np.mean(s * s)), 1e-6)) for s in sec]
print(f"{sys.argv[2]:>14}: " + " ".join(f"{d:6.1f}" for d in db))
PY
}
run menu 0.7 false --exit-after 12
run menu_music0 0.0 false --exit-after 12
run menu_muted_unfocused 0.7 true --exit-after 12
run race 0.7 false --quick-race toy_town --autopilot --seed 3 --exit-after 25
