# Music (T10) — procedural soundtrack

All music is original and rendered offline from Python + numpy scripts in `tools/audio/music/`
(no samples, no external sources). Output: stereo OGG Vorbis, 44.1 kHz, in `assets/audio/music/`
(loops libvorbis q8 ≈ 256 kbit/s, stingers q6).

## Regenerate

```sh
python3 tools/audio/music/render.py                 # all cues, ~4 min (6 processes)
python3 tools/audio/music/render.py race_2 stingers # selected groups
python3 tools/audio/music/test_music.py             # self-checks, ~10 s
```

`render.py` writes the OGGs and, per cue, a preview PNG (waveform, log spectrogram, short-term
loudness, section markers), a loop-seam PNG and `report.json` into `--png DIR` (default
`target/music-preview/`), plus `ltas.png` (long-term average spectra of all cues). After encoding
it decodes every OGG and fails if a file breaks the spec: true peak > −1 dBTP (ffmpeg ebur128),
loudness off target by > 1 LU, decoded length ≠ rendered length, a loop seam jump above the
99.9th percentile of the file's own sample-to-sample steps, or codec error at the seam > 1.5 dB
above the error on the same audio mid-stream (each loop is also encoded rotated by half a loop
for this A/B). Output is byte-identical between runs: all randomness comes from `dsp.rng(key…)`
and ffmpeg runs with `-fflags +bitexact` (otherwise the Ogg serial number is random).

## Cues

| File | Key | BPM | Length | Structure (bars) | Character / instrumentation |
|---|---|---|---|---|---|
| `menu_theme` | D minor | 100 | 48 bars, 115.2 s loop | intro 8 · A 8 · A2 8 · B 8 · break 8 · B2 8 | relaxed synthwave: warm supersaw pad, octave bass, 16th pulse arp, gated 80s snare, soft saw lead theme, FM bell counter-melody |
| `race_1` / `_final` | E dorian | 120 | 64 bars, 128.0 s loop | intro 8 · verse 16 · chorus 16 · break 8 · chorus2 16 | funky electro / nu-disco: broken electro beat (verse) → four-on-the-floor disco (chorus), slap-style pulse bass with slides, talk-box lead, clav "chank" stabs, synth brass, FM e-piano |
| `race_2` / `_final` | A minor | 126 | 64 bars, 121.9 s loop | intro 8 · verse 16 · chorus 16 · break 8 · chorus2 16 | driving synthwave / outrun: rolling 16th octave bass, gated snare, tom fills, pulse arp, big pumping supersaw pad, heroic saw lead, FM bells in the break |
| `race_3` / `_final` | G minor | 135 | 64 bars, 113.8 s loop | intro 8 · verse 16 · drop 16 · break 8 · drop2 16 | breakbeat: syncopated break with ghost notes, wobbling reese bass, rave stabs with pitch bend, 303-style acid line (accents + slides), square-wave lead |
| `race_4` / `_final` | D major | 128 | 64 bars, 120.0 s loop | intro 8 · verse 16 · chorus 16 · break 8 · chorus2 16 | bright electro-pop: four-on-the-floor with off-beat open hats and claps, off-beat pumping bass, FM e-piano riff on an A pedal, supersaw pluck hook |
| `results_jingle` | C major | 120 | 5.5 s one-shot | 2 bars + tail | arpeggio flourish, brass I–IV–V into a ringing Cmaj9 |
| `podium_fanfare` | B♭ major | 110 | 8.7 s one-shot | 3 bars + tail | brass fanfare call (short-short-long), tom/snare roll, Bbadd9 + crash |
| `cup_victory` | D major | 120 | 14.2 s one-shot | 6 bars + tail | the menu theme turned major, full band, climbing to a ringing D |

Progressions and hooks are in each cue module (`tools/audio/music/cues/*.py`), written in the text
pattern format described in `song.py`. Every loop ends on a dominant (V or V7♯9) that resolves
to the tonic on bar 1, so the seam is also a musical cadence.

### Final-lap variants (`race_N_final`)
Same tempo, length, structure and note timing as the base loop (built by the same function with
`final=True`), so the music director can crossfade at the **same playback position**. Shared parts
are rendered identically — same notes, same humanisation (seeded by note time, not index), same
oscillator phases — so a crossfade is phase-coherent; the variant adds layers and energy: opened filters on
bass/pads/plucks, extra percussion (ride, shaker, conga/perc, ghost snares), an extra arp and/or
lead octave double, more crashes, claps on every backbeat, and the kick keeps running through the
break. Mastered 0.5 LU hotter (−13.5 LUFS) for lift.

## Using the cues (music director, T09)
- Loops: the whole file is the loop — play with `.looped()` from 0, no loop points needed.
- Final lap: start `race_N_final` at the base cue's current playback position (`start_from(pos)`)
  with `.looped()` and crossfade (1–2 s); the two files are phase-aligned, so any fade shape works.
- Stingers are one-shots: `results_jingle` (results screen), `podium_fanfare` (podium),
  `cup_victory` (cup end). They start on the downbeat at sample 0.
- Levels are matched: loops −14 LUFS, finals −13.5 LUFS (intended lift), stingers −14 LUFS — no
  per-cue gain needed.

## Loudness & format
Loops are mastered to −14.0 LUFS integrated (final variants −13.5), limited to −2.0 dBTP before
encoding so the decoded file stays ≤ −1 dBTP (Vorbis adds up to ~0.5 dB; `render.py` re-masters
lower if it ever overshoots). Stingers −14 LUFS integrated, trimmed when the tail falls below
−50 dBFS, 300 ms fade. Loops use q8 because at q5/q6 the last ~40 ms of a stream carry
+1.7/+3.4 dB more coding error than the same audio mid-stream (measured); at q8 the difference
is ≤ 0.6 dB. Mix balance follows a −4.5 dB/oct long-term spectrum from 200 Hz to 10 kHz
(`ltas.png`); the master has a −3 dB low shelf at 100 Hz to leave room for engines and impacts.

## How it works (short)
- `dsp.py` — PolyBLEP oscillators, unison stacks, ADSR, TPT state-variable filter with per-sample
  cutoff (filter envelopes), RBJ biquads applied as circular FFT filters, closed-form ping-pong
  delay, synthetic plate IR (decaying decorrelated noise with octave-band RT60) + FFT convolution,
  chorus, sidechain pump from kick onsets, bus compressor, oversampled soft clip, look-ahead
  true-peak limiter, BS.1770-4 loudness meter.
- `instruments.py` / `drums.py` — synth voices (supersaw pad, pluck, FM bell/e-piano, brass, chank,
  stab, mono bass/acid/reese/lead/talk-box with glide) and a synthesised kit (kick, snare with
  optional gated plate, clap, 808-style hats, toms, crash, ride, shaker, cowbell, riser,
  downlifter, impact, reverse crash).
- `song.py` — sections + chord progressions, text patterns (drum grids, notes, chord-relative
  lines, voice-led chords, arps), swing/humanisation, note cache, mixer (stems normalised to a
  loudness target per track, high-passed non-bass parts, sends, kick sidechain) and master chain
  (mono lows < 110 Hz, glue compressor, soft clip, limiter iterated to the target).
- **Seamless loops by construction:** notes that ring past the loop end are folded onto its start
  (tail wrap-around); EQ, delay and reverb are circular FFT operations on the loop buffer, i.e. the
  exact steady state of an endlessly repeated loop; stateful processes (filter sweeps, compressor,
  limiter) run over a wrap-around warm-up; LFOs use whole cycles per loop.

## Known limitations
- Every cue is held fully in memory by kira (static sound data): a 128 s stereo cue decodes to
  ≈ 45 MB; a race keeps base + final loaded (≈ 90 MB).
- Vorbis gapless trimming relies on the decoder honouring the end granule position (ffmpeg does;
  verified decoded length = rendered length).
- No stems: the final-lap variant is a full alternative mix, not an additive layer.
- The plate is a synthetic impulse response convolved by FFT, not a recursive FDN: identical for a
  static reverb, cheaper in numpy and exactly periodic on a loop.
- Verified objectively only (no listening): spectrograms, long-term spectra, loudness, seams.
  race_3 (breakbeat) is the crispest/bassiest cue by design (≈ 3 dB more 40–100 Hz and 5–12 kHz
  than the others relative to the mids). Loops are steady by design (LRA 1.5–3.5 LU; breaks dip
  2–3 LU).
