# Sound effects & ambience (T09 phase 1)

All SFX and ambience are **procedurally synthesised** by `tools/audio/sfx/` (Python 3 + numpy,
ffmpeg/libvorbis) — original content, no external samples, nothing to credit.
Output: `assets/audio/sfx/*.ogg` (mono) and `assets/audio/ambience/*.ogg` (stereo).
Music is T10's (`assets/audio/music/`).

## Regenerate & verify
```
python3 tools/audio/sfx/build.py                     # all sounds (~1 min)
python3 tools/audio/sfx/build.py rocket_ ui_         # names or name prefixes
python3 tools/audio/sfx/build.py --previews DIR      # PNGs (default target/audio-previews/sfx)
python3 tools/audio/sfx/synth.py                     # DSP kit self-check
```
Deterministic: each recipe's RNG is seeded from its name; re-running gives byte-identical OGGs.
The build prints the cue table below and exits 1 if any check fails.

| File | Role |
|---|---|
| `synth.py` | DSP kit: band-limited oscillators (polyBLEP saw/pulse, harmonic stacks, FM), noise colours, Poisson grains, hit building blocks (noise burst, pitch-drop boom, modal ring, crackle), FFT filters (min-phase for one-shots, circular zero-phase for loops), time-varying SVF and STFT EQ, envelopes, saturation, convolution reverb (synthetic IRs), panning, kira-exact Hermite resampling; the `@sound` registry |
| `recipes_car.py` | engines, tyre/surface loops, impacts, landings, boost, drift |
| `recipes_items.py` | pickups, roulette, weapons, hazards, shield |
| `recipes_race_ui.py` | race events, jingles, UI cues |
| `recipes_ambience.py` | ambience beds, rain, thunder |
| `build.py` | master → encode → decode → verify → previews → table |

## Mastering & checks (per sound)
1. 22 Hz DC/sub high-pass; one-shots: trailing silence trimmed (−66 dB), 10 ms fade-out, padded
   to ≥ 0.15 s.
2. Loudness-normalised to its **category target** (+ a per-cue offset), allowing ≤ 4 dB of
   look-ahead brick-wall limiting (5 ms attack, 20 ms hold; circular for loops).
3. OGG Vorbis 48 kHz: one-shots q6, SFX loops q8 (end-of-stream coding error = mid-file error, so
   seams stay clean), ambience q5; 50 ms OGG pages (ffmpeg mis-trims single-page Vorbis streams by
   up to 128 samples; symphonia handles them, but multi-page files decode identically everywhere).
4. Decoded and verified: sample-exact length; **true peak ≤ −1 dBTP** (4× oversampled — kira's
   Hermite resampler interpolates between samples; gain is lowered and re-encoded if needed);
   loudness within ±1.5 dB of target; loops: seam jump (|x[0]−x[−1]| vs the 99.9th-percentile
   sample step) and seam spectral flux (vs the 99th percentile elsewhere) both ≤ 1.5.
5. Preview PNG: waveform + log-frequency spectrogram; loops add a ±0.4 s seam panel; loops meant
   to be pitch-shifted add playback-rate 0.5 / 1 / 2 panels (alias check).

**Loudness targets** (loops: integrated LUFS, BS.1770 gated; one-shots: loudest 100 ms K-weighted
window "L100"; mono measured as played, i.e. dual-mono):

| Category | Target | Cues |
|---|---|---|
| engine | −17 LUFS-I | engine loops |
| loop | −19 LUFS-I | tyre, surface, boost, drift, projectile and hazard loops |
| car | −11 L100 | impacts (soft −13.5, hard −10), landings, boost whoosh |
| item | −11 L100 | weapon and item one-shots (per-cue offsets, e.g. explosions +2, ticks −6) |
| race | −12 L100 | countdown, lap, fanfare, horn, jingles |
| ui | −16 L100 | menu cues |
| amb | −25 LUFS-I | ambience beds (heavy rain +3/+4) |
| thunder | −13 L100 | thunder (medium −2, distant −5) |

Files therefore carry their intended relative level: rc_audio can start every cue at 0 dB and mix
with the bus volumes.

## Loops are seamless by construction
No crossfading: every loop is one exact period. Tonal partials sit on the loop's frequency grid
(`grid()`: whole cycles per loop), modulation is periodic (zero-mean, so total phase stays on the
grid), noise is FFT-generated (periodic), filters/reverb are circular, events wrap around the end.
The decoded seam check confirms it after encoding.

**Engines** (4 s, mono): pure harmonic stacks + a little commutator/inverter hiss, steady, so
playback rate is the only pitch control. Rate 1.0 = mid revs (buggy f0 185 Hz, truck 92 Hz,
sports 245 Hz). Content ≤ 9 kHz → alias-free up to rate ≈ 2.4 (kira resamples without an
anti-alias filter). Verified with a simulated 0.55 → 2.0 → 0.7 rate ramp across loop wraps.

## Playback
How each file is triggered, mixed, spatialised and loaded is documented with the code in
`docs/design/rc_audio.md` (engine rate mapping, car layers, message → cue table, ambience mix,
music director, memory). Engine timbre per car comes from `catalog.ron` (`CarInfo.engine`).

## Cue table
Output of `build.py` (decoded files). RMS: whole file for loops, loudest 100 ms for one-shots.
Loudness: LUFS-I for loops, L100 for one-shots (see targets). Seam: jump ratio / flux ratio /
level step across the wrap (the level step is informational: events may legitimately sit there).
59 SFX (1.5 MB OGG, ≈ 29 MB decoded) + 14 ambience (5.1 MB OGG, ≈ 110 MB decoded if all loaded).

| cue | file | dur s | loop | ch | peak dBFS | true peak | RMS dBFS | loudness | target | seam j/f/step |
|---|---|---|---|---|---|---|---|---|---|---|
| engine_buggy | sfx/engine_buggy.ogg | 4.00 | yes | 1 | -9.7 | -9.6 | -20.2 | -17.0 LUFS-I | -17 | 0.92/0.98/+0.5dB |
| engine_truck | sfx/engine_truck.ogg | 4.00 | yes | 1 | -8.2 | -8.1 | -18.9 | -17.0 LUFS-I | -17 | 1.04/1.00/+0.7dB |
| engine_sports | sfx/engine_sports.ogg | 4.00 | yes | 1 | -9.5 | -9.5 | -20.8 | -17.0 LUFS-I | -17 | 0.57/0.93/+0.1dB |
| skid_loop | sfx/skid_loop.ogg | 3.00 | yes | 1 | -11.6 | -11.6 | -23.9 | -19.0 LUFS-I | -19 | 0.92/0.94/+0.7dB |
| scrub_loop | sfx/scrub_loop.ogg | 3.00 | yes | 1 | -9.4 | -9.4 | -23.4 | -19.2 LUFS-I | -19 | 0.14/0.93/-1.7dB |
| rumble_grass_loop | sfx/rumble_grass_loop.ogg | 3.00 | yes | 1 | -5.7 | -5.6 | -21.7 | -19.1 LUFS-I | -19 | 0.04/0.72/-4.7dB |
| rumble_dirt_loop | sfx/rumble_dirt_loop.ogg | 3.00 | yes | 1 | -6.4 | -6.3 | -24.5 | -19.2 LUFS-I | -19 | 0.19/0.88/+2.4dB |
| landing_thump_1 | sfx/landing_thump_1.ogg | 0.60 | no | 1 | -2.0 | -2.0 | -12.2 | -11.0 L100 | -11 |  |
| landing_thump_2 | sfx/landing_thump_2.ogg | 0.60 | no | 1 | -1.3 | -1.3 | -14.1 | -11.3 L100 | -11 |  |
| impact_soft_1 | sfx/impact_soft_1.ogg | 0.39 | no | 1 | -1.3 | -1.3 | -16.4 | -14.7 L100 | -14 |  |
| impact_soft_2 | sfx/impact_soft_2.ogg | 0.39 | no | 1 | -1.8 | -1.8 | -15.5 | -13.5 L100 | -14 |  |
| impact_soft_3 | sfx/impact_soft_3.ogg | 0.39 | no | 1 | -1.2 | -1.2 | -15.7 | -13.6 L100 | -14 |  |
| impact_hard_1 | sfx/impact_hard_1.ogg | 0.57 | no | 1 | -3.1 | -3.1 | -12.4 | -10.0 L100 | -10 |  |
| impact_hard_2 | sfx/impact_hard_2.ogg | 0.61 | no | 1 | -4.8 | -4.8 | -12.2 | -10.0 L100 | -10 |  |
| impact_hard_3 | sfx/impact_hard_3.ogg | 0.58 | no | 1 | -4.2 | -4.1 | -14.0 | -10.1 L100 | -10 |  |
| boost_whoosh | sfx/boost_whoosh.ogg | 1.14 | no | 1 | -3.7 | -3.7 | -14.8 | -11.3 L100 | -11 |  |
| boost_crackle_loop | sfx/boost_crackle_loop.ogg | 2.50 | yes | 1 | -1.4 | -1.0 | -25.3 | -19.2 LUFS-I | -19 | 0.11/0.72/+2.8dB |
| drift_sizzle_loop | sfx/drift_sizzle_loop.ogg | 2.50 | yes | 1 | -2.6 | -2.4 | -25.6 | -19.3 LUFS-I | -19 | 0.03/0.99/-0.4dB |
| rumble_water_loop | sfx/rumble_water_loop.ogg | 3.00 | yes | 1 | -7.8 | -7.7 | -23.5 | -19.2 LUFS-I | -19 | 0.03/0.95/+3.9dB |
| pickup_grab | sfx/pickup_grab.ogg | 0.45 | no | 1 | -8.3 | -8.3 | -15.5 | -11.1 L100 | -11 |  |
| roulette_tick | sfx/roulette_tick.ogg | 0.15 | no | 1 | -2.6 | -2.6 | -23.1 | -17.2 L100 | -17 |  |
| roulette_lock | sfx/roulette_lock.ogg | 0.50 | no | 1 | -3.8 | -3.8 | -15.5 | -11.0 L100 | -11 |  |
| rocket_launch | sfx/rocket_launch.ogg | 0.90 | no | 1 | -3.9 | -3.9 | -13.1 | -11.1 L100 | -11 |  |
| rocket_flight_loop | sfx/rocket_flight_loop.ogg | 2.00 | yes | 1 | -10.2 | -10.1 | -23.5 | -19.3 LUFS-I | -19 | 0.70/0.94/-0.2dB |
| rocket_explosion | sfx/rocket_explosion.ogg | 1.80 | no | 1 | -4.0 | -3.9 | -13.3 | -9.2 L100 | -9 |  |
| lockon_beep | sfx/lockon_beep.ogg | 0.15 | no | 1 | -7.1 | -7.0 | -16.7 | -10.9 L100 | -11 |  |
| leech_hum_loop | sfx/leech_hum_loop.ogg | 2.00 | yes | 1 | -12.7 | -12.7 | -21.1 | -19.0 LUFS-I | -19 | 0.58/0.68/-3.0dB |
| leech_zap | sfx/leech_zap.ogg | 0.55 | no | 1 | -6.3 | -6.0 | -16.9 | -11.4 L100 | -11 |  |
| orb_hum_loop | sfx/orb_hum_loop.ogg | 2.00 | yes | 1 | -16.4 | -16.4 | -20.9 | -19.0 LUFS-I | -19 | 1.05/0.96/+1.7dB |
| orb_zap | sfx/orb_zap.ogg | 0.63 | no | 1 | -1.5 | -1.2 | -15.0 | -11.6 L100 | -11 |  |
| orb_launch | sfx/orb_launch.ogg | 0.70 | no | 1 | -7.5 | -7.5 | -13.3 | -11.0 L100 | -11 |  |
| emp_charge | sfx/emp_charge.ogg | 0.45 | no | 1 | -7.3 | -6.9 | -17.5 | -11.2 L100 | -11 |  |
| emp_blast | sfx/emp_blast.ogg | 1.67 | no | 1 | -1.8 | -1.7 | -10.7 | -9.1 L100 | -9 |  |
| goo_beep | sfx/goo_beep.ogg | 0.15 | no | 1 | -13.2 | -13.2 | -18.2 | -13.9 L100 | -14 |  |
| goo_explosion | sfx/goo_explosion.ogg | 1.26 | no | 1 | -1.6 | -1.6 | -12.1 | -10.2 L100 | -10 |  |
| oil_splat | sfx/oil_splat.ogg | 0.50 | no | 1 | -1.4 | -1.4 | -12.8 | -11.2 L100 | -11 |  |
| oil_slip | sfx/oil_slip.ogg | 1.20 | no | 1 | -4.2 | -4.2 | -13.6 | -11.0 L100 | -11 |  |
| spring_boing | sfx/spring_boing.ogg | 1.00 | no | 1 | -4.4 | -4.4 | -13.2 | -11.0 L100 | -11 |  |
| item_drop | sfx/item_drop.ogg | 0.30 | no | 1 | -1.3 | -1.3 | -12.8 | -11.1 L100 | -11 |  |
| shield_up | sfx/shield_up.ogg | 0.80 | no | 1 | -8.2 | -8.2 | -13.4 | -10.9 L100 | -11 |  |
| shield_hit | sfx/shield_hit.ogg | 0.60 | no | 1 | -4.7 | -4.6 | -16.0 | -11.0 L100 | -11 |  |
| shield_pop | sfx/shield_pop.ogg | 0.57 | no | 1 | -2.2 | -2.1 | -14.4 | -11.0 L100 | -11 |  |
| storm_strike | sfx/storm_strike.ogg | 1.49 | no | 1 | -1.5 | -1.2 | -13.7 | -9.9 L100 | -9 |  |
| countdown_beep | sfx/countdown_beep.ogg | 0.43 | no | 1 | -12.4 | -12.4 | -14.5 | -11.9 L100 | -12 |  |
| countdown_go | sfx/countdown_go.ogg | 0.96 | no | 1 | -6.9 | -6.9 | -14.8 | -10.9 L100 | -11 |  |
| lap_chime | sfx/lap_chime.ogg | 1.78 | no | 1 | -6.4 | -6.3 | -15.6 | -12.0 L100 | -12 |  |
| final_lap_fanfare | sfx/final_lap_fanfare.ogg | 2.64 | no | 1 | -4.8 | -4.8 | -14.5 | -12.0 L100 | -12 |  |
| finish_horn | sfx/finish_horn.ogg | 1.82 | no | 1 | -4.7 | -4.7 | -14.5 | -12.0 L100 | -12 |  |
| wrong_way_buzzer | sfx/wrong_way_buzzer.ogg | 0.69 | no | 1 | -9.5 | -9.5 | -13.6 | -12.0 L100 | -12 |  |
| respawn_materialise | sfx/respawn_materialise.ogg | 1.51 | no | 1 | -7.2 | -7.2 | -16.3 | -12.0 L100 | -12 |  |
| golden_battery_jingle | sfx/golden_battery_jingle.ogg | 2.23 | no | 1 | -6.1 | -6.1 | -16.3 | -12.0 L100 | -12 |  |
| charge_ready | sfx/charge_ready.ogg | 1.01 | no | 1 | -11.9 | -11.9 | -19.3 | -14.0 L100 | -14 |  |
| ui_hover | sfx/ui_hover.ogg | 0.15 | no | 1 | -10.4 | -10.4 | -25.0 | -20.2 L100 | -20 |  |
| ui_select | sfx/ui_select.ogg | 0.24 | no | 1 | -10.6 | -10.6 | -19.5 | -16.0 L100 | -16 |  |
| ui_back | sfx/ui_back.ogg | 0.22 | no | 1 | -12.4 | -12.4 | -20.9 | -18.0 L100 | -18 |  |
| ui_toggle | sfx/ui_toggle.ogg | 0.15 | no | 1 | -7.0 | -7.0 | -20.0 | -16.1 L100 | -16 |  |
| ui_slider_tick | sfx/ui_slider_tick.ogg | 0.15 | no | 1 | -10.1 | -10.0 | -28.1 | -22.1 L100 | -22 |  |
| ui_error | sfx/ui_error.ogg | 0.34 | no | 1 | -8.1 | -8.1 | -18.1 | -15.9 L100 | -16 |  |
| ui_unlock | sfx/ui_unlock.ogg | 1.74 | no | 1 | -10.1 | -10.0 | -19.6 | -14.0 L100 | -14 |  |
| room_tone | ambience/room_tone.ogg | 20.00 | yes | 2 | -13.9 | -13.9 | -31.0 | -25.3 LUFS-I | -25 | 0.48/0.95/-1.2dB |
| birds | ambience/birds.ogg | 30.00 | yes | 2 | -11.7 | -11.6 | -34.2 | -25.3 LUFS-I | -25 | 0.03/0.57/-1.5dB |
| crickets | ambience/crickets.ogg | 24.00 | yes | 2 | -12.8 | -12.5 | -32.0 | -25.1 LUFS-I | -25 | 0.38/0.99/+14.7dB |
| wind | ambience/wind.ogg | 30.00 | yes | 2 | -13.3 | -13.3 | -30.9 | -25.2 LUFS-I | -25 | 0.14/0.80/-0.1dB |
| rain_light | ambience/rain_light.ogg | 20.00 | yes | 2 | -10.9 | -10.6 | -33.5 | -25.4 LUFS-I | -25 | 0.15/1.05/+0.6dB |
| rain_heavy | ambience/rain_heavy.ogg | 20.00 | yes | 2 | -8.9 | -8.7 | -28.2 | -21.4 LUFS-I | -21 | 0.26/1.04/-0.7dB |
| rain_light_indoor | ambience/rain_light_indoor.ogg | 20.00 | yes | 2 | -10.5 | -10.5 | -30.3 | -25.2 LUFS-I | -25 | 0.18/0.82/+1.8dB |
| rain_heavy_indoor | ambience/rain_heavy_indoor.ogg | 20.00 | yes | 2 | -7.9 | -7.9 | -25.4 | -22.0 LUFS-I | -22 | 0.07/0.82/+0.5dB |
| thunder_1 | ambience/thunder_1.ogg | 5.20 | no | 2 | -1.4 | -1.4 | -13.8 | -13.0 L100 | -13 |  |
| thunder_2 | ambience/thunder_2.ogg | 6.26 | no | 2 | -4.3 | -4.3 | -16.5 | -15.1 L100 | -15 |  |
| thunder_3 | ambience/thunder_3.ogg | 6.92 | no | 2 | -6.5 | -6.5 | -17.7 | -18.0 L100 | -18 |  |
| supermarket_hum | ambience/supermarket_hum.ogg | 24.00 | yes | 2 | -15.3 | -15.3 | -31.8 | -25.3 LUFS-I | -25 | 0.28/0.94/+0.3dB |
| workshop_hum | ambience/workshop_hum.ogg | 24.00 | yes | 2 | -14.5 | -14.5 | -31.4 | -25.3 LUFS-I | -25 | 0.62/1.00/-0.4dB |
| workshop_tools | ambience/workshop_tools.ogg | 36.00 | yes | 2 | -12.5 | -12.5 | -40.9 | -27.1 LUFS-I | -27 | 0.00/0.15/-0.4dB |

## Sound list & recipes
One line per recipe (the docstring in its module). "Rate-shifted" loops are designed for playback-
rate changes (alias-checked at rates 0.5/1/2). Extras beyond GDD §16.2: `rumble_water_loop`,
`orb_launch`, `item_drop`.
### Car (engines, tyres, surfaces, impacts, boost, drift)
- `engine_buggy` (loop, rate-shifted) — Brushed RC buggy: bright nasal whine, gear-mesh whine around the 7th/14th harmonic, 12.5 Hz gear flutter and commutator hiss.
- `engine_truck` (loop, rate-shifted) — Big monster-truck motor: low pulsy buzz with a 450-700 Hz growl formant, sub-octave weight, 23 Hz drivetrain growl and an 11th-harmonic gear grind.
- `engine_sports` (loop, rate-shifted) — Electric hypercar scream: a bright 6th-10th harmonic scream band (1.5-2.5 kHz at rate 1) over smooth low partials, a thin 13th-harmonic inverter whine; random phases keep it smooth, not buzzy.
- `skid_loop` (loop, rate-shifted) — Tyre squeal on hard floors: three wandering stick-slip squeal tones (~1.35/1.9/2.6 kHz + harmonics) that wax and wane with irregular chatter, over band-passed friction hiss and a little rubber-scrub body.
- `scrub_loop` (loop, rate-shifted) — Tyre scrub on carpet, grass and dirt: grainy band-passed friction noise, fibre-scratch crackle and a low rumbling body — no squeal.
- `rumble_grass_loop` (loop, rate-shifted) — Driving over a lawn: random low suspension thuds, swishing grass against the chassis (high-passed fluttering noise) and crisp blade flicks.
- `rumble_dirt_loop` (loop, rate-shifted) — Dirt and gravel crunch: dense band-passed sand/grit grains, small pebble knocks (short damped partials), low rumble and a sandy hiss.
- `landing_thump_1..2` (one-shot) — Tyre-and-chassis landing: pitch-dropping low sine thud, low-passed body noise, tyre squish, decaying plastic rattle and a faint suspension spring; _2 is heavier and lower.
- `impact_soft_1..3` (one-shot) — Bump into foam, cushions or a car at low speed: dull pitch-dropping thud, low-passed noise body and a muffled 450-850 Hz plastic knock; three pitches/seeds.
- `impact_hard_1..3` (one-shot) — Crash into a wall or prop: sharp broadband crack, plastic body clack (seven random 0.4-3.5 kHz modes, higher ones shorter), low thump, tinkling debris, saturation, short room.
- `boost_whoosh` (one-shot) — Boost / Turbo kick: electric FM zap onset, band-pass noise whoosh sweeping 300→3500→1100 Hz, sub whump, a rising filtered-saw charge tone and a decaying crackle; short reverb.
- `boost_crackle_loop` (loop) — Held boost: random electric arcs (crackly band-passed bursts), a raspy 120 Hz-gated buzz with no clear pitch (won't clash with engines) and an airy rush.
- `drift_sizzle_loop` (loop, rate-shifted) — Drift sparks: dense sparkler crackle, a resonant 2.6/5.2 kHz sizzle band that reads as a pitch (playback rate follows the charge rate) and a high hiss; < 12 kHz for rate headroom.
- `rumble_water_loop` (loop, rate-shifted) — Driving through puddles / the wash bay (extra): sloshy fast-modulated band-passed noise, a stream of upward-chirping droplet plinks and a low slosh.

### Items & weapons
- `pickup_grab` (one-shot) — FM 'bling' gliding 600→2400 Hz in 80 ms with an octave sparkle, three high glints and a short zap crackle.
- `roulette_tick` (one-shot) — Crisp tick: 1.5 ms band-passed click plus a tiny 2.5 kHz blip (12 ms decay).
- `roulette_lock` (one-shot) — Lock-in stab: click + bright C6+G6 chord (bell-ish partials, 2 % pitch snap, 0.1 s decay) with a C7 glint.
- `rocket_launch` (one-shot) — Ignition pop (120→50 Hz thump + crack), gritty 900 Hz roar (20 ms rise, ~0.6 s fade) and a whistle rising 800→1600 Hz as the rocket leaves.
- `rocket_flight_loop` (loop, rate-shifted) — Jet roar: pink noise peaked at 1.5 kHz with turbulent 60/13 Hz amplitude jitter, low rumble and a 1.2 kHz whistle with 6 Hz vibrato; 2 s, band-limited to 9 kHz for playback-rate shifting.
- `rocket_explosion` (one-shot) — Explosion: 10 ms broadband crack, noise body through a low-pass falling 3 kHz→300 Hz (≈1 s), 130→70 Hz punch over a 55→30 Hz sub boom, saturation, debris crackle thinning over 1.2 s, 0.8 s reverb. Also the generic one.
- `lockon_beep` (one-shot) — Urgent lock-on beep: two band-limited pulse voices at 2200/2270 Hz (70 Hz roughness), 80 ms, 7 kHz low-pass.
- `leech_hum_loop` (loop) — Draining tether hum: buzzy 100 Hz transformer stack through a band-pass that snaps up to 2 kHz and drains down to 250 Hz twice a second ('wub'), plus faint high crackle; 2 s.
- `leech_zap` (one-shot) — Tether attach zap: FM sweep 3 kHz→300 Hz in 150 ms (index 3→1), 3 ms noise crack and a short metallic ring at 1.2/1.85/2.7 kHz.
- `orb_hum_loop` (loop) — Plasma warble: FM 150 Hz carrier / 75 Hz modulator with the index wobbling at 3 Hz, a counter-phase octave voice, 3 Hz tremolo, light saturation and faint high crackle; 2 s.
- `orb_zap` (one-shot) — Orb hit: 1-8 kHz arc noise chopped by fast random gating, FM 'pew' falling 2 kHz→200 Hz, 90→45 Hz thump, 0.4 s reverb.
- `orb_launch` (one-shot) — Plasma ball fired: saturated FM 'thwum' rising 120→260 Hz then sinking to 150 Hz, a short 2-7 kHz arc crackle and a soft band-passed whoosh.
- `emp_charge` (one-shot) — 0.4 s wind-up peaking right at the blast: FM tone rising 200→1600 Hz (index 1→3), tremolo accelerating 8→40 Hz, band-passed noise rising 500 Hz→6 kHz; 40 ms release after 0.4 s.
- `emp_blast` (one-shot) — EMP pulse: 90→30 Hz whomp (+ octave), noise burst through a band-pass diving 6 kHz→200 Hz in 0.35 s, metallic inharmonic ring, electric crackle thinning over 1.2 s, 1 s reverb.
- `goo_beep` (one-shot) — Fuse beep: 1.4 kHz sine + 25 % third harmonic, 2 ms attack, 45 ms hold, 18 ms release.
- `goo_explosion` (one-shot) — Goo bomb: rounder boom (70→35 Hz + short low-passed noise body), a wet splat of noise through a resonant band-pass wobbling 300-1600 Hz, and a spray of droplets; 0.6 s reverb.
- `oil_splat` (one-shot) — Oil dropped: pink noise through a resonant band-pass sweeping 900→250 Hz in 120 ms (wet 'splotch'), a soft 80 Hz thud and a few low droplets.
- `oil_slip` (one-shot) — Slipping through oil: pink noise in a resonant band-pass wobbling around 500 Hz with gurgling AM, plus a fading 1.8 kHz tyre squeal; 1.2 s.
- `spring_boing` (one-shot) — Cartoon boing: saw + sine rising 220→440 Hz in 0.25 s with 14 Hz vibrato (15 %, slowly decaying), a jaw-harp formant (band-pass at 3× pitch) and metallic twang partials.
- `item_drop` (one-shot) — Plastic thunk: low-Q modal ring (180/410/730/1150 Hz) struck by a soft exciter, 110→70 Hz thud, tiny click.
- `shield_up` (one-shot) — Bubble forming: a sine gliding 300→900 Hz with two quieter, unevenly detuned twins (shimmer, no pumping) under a swell, FM sparkle with 12 Hz shimmer, a noise whoosh opening 300 Hz→6 kHz, settling into a 225 Hz hum.
- `shield_hit` (one-shot) — 'Dink-ziiing': inharmonic FM bell at 1.1 kHz (ratio 1.41) with fast decay, a 2.3 kHz ring with 30 Hz energy tremolo, 3 ms noise burst and a 110→60 Hz thump.
- `shield_pop` (one-shot) — Bubble pop: sine diving 1500→300 Hz in 40 ms, 1.5 ms crack, glassy two-partial tinkles (3-6 kHz) scattering and fading over 0.5 s.
- `storm_strike` (one-shot) — Lightning on a car: two broadband cracks 18 ms apart (return strokes), dense crackling sizzle thinning over 300 ms, 60→35 Hz boom with low rumble, FM zap falling 1.8 kHz→400 Hz, 1.2 s reverb.

### Race events & UI
- `countdown_beep` (one-shot) — D5 synth beep: sine + soft odd harmonics (gentle square), 5 ms attack, 0.2 s body, short room.
- `countdown_go` (one-shot) — GO: D6 + D5 body, brighter harmonics, ±6 cent chorus, 0.6 s hold, upward noise swoosh.
- `lap_chime` (one-shot) — G5-B5-D6 FM-bell arpeggio, 75 ms apart, the top note rings longest.
- `final_lap_fanfare` (one-shot) — Synth-brass fanfare in C: staccato G3+G4 triplet pickup, then a held C major chord (C3-C5) with a filter swell over a soft drum hit.
- `finish_horn` (one-shot) — Two-tone air horn F4+A4 (band-limited pulses) with a 2-semitone scoop up, saturated, then horn-bell EQ (1 kHz bump, 2 kHz low-pass, 160 Hz high-pass trims the difference-tone rumble).
- `wrong_way_buzzer` (one-shot) — 'Bzzt-bzzt': band-limited squares at 110 + 117 Hz (7 Hz beating) + saw buzz, low-passed, saturated, two 0.27 s bursts.
- `respawn_materialise` (one-shot) — Hologram: C5-G5-C6-E6-G6-C7 sine arpeggio under an accelerating tremolo, rising band-passed noise sweep, in-key sparkle pings, settling C6+G6 pad.
- `golden_battery_jingle` (one-shot) — C6-E6-G6-C7 marimba run, then a ringing C major bell chord (C5-E5-G5-C6 + C7) with pentatonic sparkle.
- `charge_ready` (one-shot) — Boost-ready: E6→B6 hollow 1:2 FM chime, each note zipping up from a semitone below, with a +8 cent chorus copy of B6 for shimmer.
- `ui_hover` (one-shot) — Soft G#6 glass blip, 3 ms attack, ~40 ms decay.
- `ui_select` (one-shot) — Confirm: A5 then E6 glass blips 50 ms apart (rising fifth).
- `ui_back` (one-shot) — Back: C#6 then F#5 glass blips 45 ms apart (falling fifth), less shimmer.
- `ui_toggle` (one-shot) — Two-click switch: band-passed noise click + E6 glass tick, softer second click 35 ms later.
- `ui_slider_tick` (one-shot) — Tiny tick: E7 sine blip (8 ms decay) + a narrow noise click.
- `ui_error` (one-shot) — Low double 'bonk': square-ish A3 + A#3 (minor-second rub) with a small pitch drop, low-passed.
- `ui_unlock` (one-shot) — Unlock: fast rising A-major-pentatonic glass glissando (A5→A7), then an A major chord (A5-C#6-E6-A6) with sparkle and a soft room.

### Ambience & thunder
- `room_tone` (loop) — Quiet room: soft pink air LP 6 kHz + distant HVAC rumble, faint 50 Hz-family fridge hum, slow air movement, small-room reverb.
- `birds` (loop) — Clear day: sparse calls of 4 species (whistled phrases, sparrow chirps, finch trills, a distant dove) at random distances/pans with damped outdoor reflections, over a soft air bed (breathing pink air + leaf rustle) so the scene never goes dead between calls.
- `crickets` (loop) — Clear night: 6 crickets (4.2-5.2 kHz pulse chirps, 3-4 pulses at ~30 Hz, periods dividing the loop, jittered) at own pans/distances + a distant pulsing chorus and faint night air.
- `wind` (loop) — Cloudy / snow: brown-pink noise through a gust-driven band-pass (gusts every ~3-8 s, ~8 dB swing, brighter when stronger; channels gust slightly apart) + two airy whistles 500-950 Hz that sing up in the strong gusts.
- `rain_light` (loop) — Fine rain: hiss bed (HP 400 Hz, +6 dB at 4 kHz) + ~150 drops/s (impact ticks 1.2-8 kHz with 0.3-2.5 ms decay, ~20 % bubble plinks 1.5-5 kHz), wide pan, gently breathing intensity.
- `rain_heavy` (loop) — Storm rain: hiss bed + low-mid roar, ~400 heavier drops/s (600-5000 Hz ticks, ~15 % plinks), ~1.5 noisy splashes/s, gusting intensity.
- `rain_light_indoor` (loop) — Light rain heard indoors: rain bed LP 900 Hz (3rd order) + soft roof patter (dull 150-500 Hz thuds LP 1.2 kHz, ~60/s) + ~4 crisp glass ticks/s (2-4.5 kHz) in a small room.
- `rain_heavy_indoor` (loop) — Heavy rain heard indoors: heavy bed LP 700 Hz + low rumble, dense roof drumming, ~10 glass ticks/s, occasional gutter drips (rising 700 Hz plinks) off to the right.
- `thunder_1` (one-shot) — Close strike: 0.3 s ripping crack (dense decaying crackle 0.8-9 kHz + transient), immediate 450 Hz boom and three later rolls, 2.4 s outdoor reverb.
- `thunder_2` (one-shot) — Medium distance: no crack; a 0.4 s swell into two big rolls (0.6 s, 2.6 s) with smaller rumbles between, LP ~350 Hz, 2.6 s reverb.
- `thunder_3` (one-shot) — Distant: soft, long low rumble — slow 1 s swell, LP ~170 Hz, weak secondary rolls, 3 s tail.
- `supermarket_hum` (loop) — Closed supermarket: two refrigeration compressors (50 Hz-family stacks beating at 0.33 Hz), freezer-fan whoosh with a 117 Hz blade-pass tone, faint fluorescent buzz, 2.5 s hall reverb.
- `workshop_hum` (loop) — Auto-repair workshop: fluorescent buzz, idling compressor throb (24 Hz thump pulses, 2-cylinder accent) with 50 Hz motor hum, ventilation fan + 85 Hz blade tone, bright 1.8 s garage reverb.
- `workshop_tools` (loop) — Distant tools, sparse (6 per 36 s): impact-wrench bursts, hammer taps, ratchet clicks and an air-hose blast, dulled (LP 2.5-4 kHz), reverb-dominated (1.8 s garage, DRR ≈ -15 dB) over a faint room bed.

## Known limitations
- Designed without listening: everything is verified objectively (spectrograms, loudness, seams,
  rate tests, decoded files) but taste needs a human listening pass. Every recipe is a short,
  parameterised function — tweak, re-run, done.
- Engines are one steady loop per timbre: beyond ≈ 0.5–2.2× they turn muddy/chipmunky. A fuller
  engine would layer 2–3 loops recorded at different revs.
- Musical cues use fixed keys (countdown D, lap chime G, fanfare/jingle/respawn C, UI A major);
  matching T10's keys is a MIDI-number change in `recipes_race_ui.py`.
- Droplet "plinks" (rain, goo, oil, water) are synthetic bubble chirps and may read slightly
  artificial up close.
- Snow, fog and indoor thunder have no files of their own (runtime filters, see playback notes).
