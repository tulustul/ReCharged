# T10 — Music (procedural composition & rendering)

**Goal:** an original, energetic synthwave / electro-funk soundtrack rendered offline from
scripts: menu theme, race loops with final-lap variants, and stingers.

**Read first:** PRACTICES (§7 audio); GDD §1 (tone), §16.1.

## Owned paths
`tools/audio/music/**`, `assets/audio/music/**`, `docs/design/music.md`.

## Deliverables
1. **Synth & sequencer** (`tools/audio/music/`, Python + numpy, ffmpeg for OGG): subtractive voices
   (saw/square/pulse with filter envelopes, unison detune), FM bells/plucks, sub bass, synthesised
   drum kit (kick, snare, clap, hats, toms, crash), effects (stereo delay, chorus, plate/FDN reverb,
   sidechain ducking, bus compression, soft clip/limiter), a pattern/arrangement format (tempo, key,
   chord progressions, sections), deterministic rendering (seeded).
2. **Cues** (stereo OGG, 44.1/48 kHz, ≈ −14 LUFS, peaks ≤ −1 dBFS):
   - `menu_theme` (~90–120 s loop, relaxed synthwave ~100 BPM),
   - `race_1` … `race_4` (each 90–150 s loop, 118–140 BPM, distinct moods — e.g. funky electro,
     driving synthwave, breakbeat, bright electro-pop) **and** `race_N_final` variants with the
     same length/tempo/structure but higher intensity (extra drums/arp, opened filters) so the game
     can crossfade at the same playback position,
   - `results_jingle` (~6 s), `podium_fanfare` (~8 s), `cup_victory` (~15 s).
   Loops must be seamless (render with tail wrap-around; verify the seam).
3. **Musicality** (you cannot listen — rely on theory and production rules): strong chord
   progressions and memorable motifs per cue, proper voice leading, tight drum programming with
   swing/humanisation, arrangement with intro/A/B/break/build, frequency-aware mixing (high-pass
   non-bass parts, carve space for the kick). Verify with spectrogram & waveform PNGs, loudness and
   clipping analysis.
4. `docs/design/music.md`: per cue — key, tempo, structure, instrumentation; how to regenerate.

## Acceptance
All cue files present with matching final-lap variants; spectrograms inspected; loudness within
spec; seams verified; regeneration documented. Report the files and durations.
