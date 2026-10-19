# rc_audio — sound (design note, T09)

bevy_kira_audio 0.27 (kira 0.12). One plugin, `GameAudioPlugin` (adds kira's `AudioPlugin`).
Everything runs in `Update` / `AudioSet` in a fixed internal order
(`Probe → Load → Volumes → Listen → Music → Ambience → Cars → Shots → Items`), plus a few
`OnEnter`/`OnExit` systems. The sound files and their recipes: `docs/design/sfx.md` (SFX,
ambience), `docs/design/music.md` (music).

| Module | Role |
|---|---|
| `output` | device probe → `OutputState` (pub) |
| `buses` | the five buses, settings → volumes, mute when unfocused, pause/duck |
| `bank` | resident SFX + stingers; per-session music & ambience; the `"audio"` loading key |
| `spatial` | listeners (`GameCamera`s), distance rolloff, panning |
| `music` | the director |
| `ambience` | beds, weather loops, fog muffling, thunder |
| `cars` | engines and car layers |
| `shots` / `items` | one-shots (player, limiter, schedule), race/UI cues; item sounds and loops |

## No audio device
Without a device bevy_kira_audio never drains channel commands, so they would pile up forever.
At startup a 10 ms silent sound is played on the UI bus; if its instance appears within 5 frames
the output is `Available`, otherwise `Unavailable` (warning "no audio device: the game runs
muted"). Every system that queues audio runs `.run_if(audio_on)`; nothing is loaded and the
`"audio"` key completes at once. Tests force `OutputState::Unavailable` by inserting it first.

## Buses
| Bus | Content | Paused with the race | Extra |
|---|---|---|---|
| Music | menu theme, race loops, stingers | no — ducked −10 dB | |
| Sfx | one-shots, car layers, projectile loops | yes | |
| Engines | one engine loop per racer | yes | |
| Ui | menu cues (`UiSound`) | never | |
| Ambience | beds, weather loops, thunder | yes | low-pass (fog) |

Each bus is a kira sub-track with a `VolumeControl` effect (never `AudioChannel::set_volume`:
it overwrites every instance's own volume). Level = `gain_db(master × bus)` from
`Settings.audio` (−60 dB floor), all silent while the window is unfocused if
`mute_unfocused`. Changes fade over 150 ms; pause/resume over 120 ms (new sounds on a paused bus
start paused). Default bus volumes are rc_core's (`AudioSettings::default`).

## Loading & memory
| What | When | Size (decoded) |
|---|---|---|
| 59 SFX + 3 stingers | once, when the device is confirmed | ≈ 29 MB + 10 MB |
| menu theme | when the director needs it; dropped on `OnEnter(Loading)` | ≈ 40 MB |
| race cue + its `_final` | session (`Loading`) | ≈ 90 MB |
| ambience loops (+ thunder) | session, for the track's environment and its possible weathers | 8–45 MB |

The number of loaded sources (`Assets<AudioSource>`) is published as the Bevy diagnostic
`assets/audio_sources`; rc_game's soak census checks it returns to its baseline in the menu.

`OnEnter(Loading)` builds the session from `RaceConfig` + `Catalog` (cue, environment, indoor,
time of day; weathers = the fixed preset, else the track's allowed presets). The `"audio"` key
completes when the race loop and the ambience loops are loaded; the final-lap variant and thunder
keep loading in the background. A file that fails to load only warns (audio never blocks a race).
`OnEnter(Unloading)` drops the session handles.

## Spatialisation (`spatial`)
Listeners are all `GameCamera`s. A sound is attenuated by its distance to the **nearest** one:
0 dB within `near`, −6 dB per doubling beyond, an amplitude fade to silence over the last 30 % of
`far`. Panning (±0.8 × the side component) only with a single listener; centred in split-screen.
Menus have no listeners: sounds play centred and unattenuated.

| Reach | near / far (u) | Used for |
|---|---|---|
| car | 8 / 90 | engines, tyres, impacts, item use, projectile loops |
| loud | 12 / 170 | explosions, EMP |
| small | 6 / 55 | others' pickups, goo beeps |

## Engines & car layers (`cars`)
- One engine loop per racer on Engines (timbre `CarInfo.engine`), reconciled with the live racers
  every frame (restart and despawn need no special cases).
- Playback rate = `0.55 + 1.3·rpm + 0.08·load (+0.1 boosting)`, clamped 0.5–2.2, × per-car pitch
  `1.12 − 0.05·(weight★ − 1)` (Zip/Bolt 1.12 … Dumpster 0.92). Level = −9 dB coasting → 0 dB at
  full load. Local players hear their own engine centred and unattenuated, −6 dB each in
  split-screen; other racers are attenuated/panned (car reach).
- Local players' cars loop seven layers on Sfx, silent until needed: skid (hard floors) / scrub
  (carpet, grass, dirt, water, ice) by the largest grounded wheel slip (0.15 → −20 dB … 0.75 →
  0 dB; rate 0.9–1.3 by speed); rumble grass/dirt/water above 1.5 u/s (−22 → 0 dB, rate 0.6–1.6);
  drift sizzle while drifting and gaining charge (rate 0.8–1.6 and level by the measured charge
  gain rate, full at 22 %/s); boost crackle while boosting.
- One-shots: `boost_whoosh` on every boost start (charge boost, Turbo Cell, perfect start — from
  the telemetry edge, so nothing doubles), `charge_ready` when a local player's charge crosses 15 %.

## One-shots (`shots`, `items`)
`Shots` plays SFX positionally or centred; a cue may start at most 4 times per 250 ms; sounds
quieter than −50 dB after attenuation are skipped; world sounds rotate through pitches
1.0/0.96/1.04/0.98/1.02 and through their variants (round-robin); centred HUD/musical cues stay
in tune. A pending queue plays delayed ones (roulette ticks, shield pop). Stopped instances are
pruned from `Assets<AudioInstance>` every 2 s.

**Cue log** (`shots::CueLog`, QA Q19): every started one-shot (Sfx, Ui), stinger (Music) and
thunder (Ambience) logs `audio cue <stem>: <bus> bus, <dB> dB` at debug level, at most one line per
cue per second; a line that stands for several starts says `(×N since the last line)`. Shows from
a log which cues a run triggered: `RUST_LOG=rc_audio=debug`.

| Message | Cue |
|---|---|
| `CarImpact` | strength < 0.05 ignored; < 0.45 `impact_soft_1..3` (−12 → 0 dB); else `impact_hard_1..3` (−4 → 0 dB) at the contact point |
| `CarLanded` | `landing_thump_1..2`, −14 → 0 dB by strength |
| `CarRespawned` | `ToTrack` → `respawn_materialise`; `FlipInPlace` → landing thump −8 dB |
| `ItemEvent::Picked` | local: `pickup_grab` + 11 `roulette_tick`s slowing down over 1 s; others: `pickup_grab` −4 dB (small reach) |
| `ItemEvent::Ready` | local: `roulette_lock` |
| `ItemEvent::Used` | rocket → `rocket_launch`, leech → `leech_zap`, orb → `orb_launch`, EMP → `emp_charge`, goo/spring pad → `item_drop`, oil → `oil_splat`, shield → `shield_up` (Turbo: boost edge; Storm: per hit) |
| `ItemEvent::LockOn/LockLost` | `lockon_beep` every 0.22 s while a rocket is locked on a local player (lock ends on `LockLost`, after 6 s, or 0.3 s after the shooter has no rocket in flight) |
| `HitEvent` | blocked → `shield_hit` + `shield_pop` 80 ms later; orb → `orb_zap`, oil → `oil_slip`, spring → `spring_boing`, leech → `leech_zap`, storm → `storm_strike` (rocket/goo/EMP: their `Explosion`) |
| `Explosion` | goo → `goo_explosion`, EMP → `emp_blast`, else `rocket_explosion` (loud reach) |
| `Projectile` entities | rocket → `rocket_flight_loop`, orb → `orb_hum_loop`, leech → `leech_hum_loop`, following the projectile |
| `Hazard` goo | `goo_beep` from 1 s to 0.12 s apart over the 6 s fuse |
| `RaceEvent` | `CountdownTick` → `countdown_beep`, `Go` → `countdown_go`, `RaceEndsIn` → `countdown_beep` −6 dB; for local players (or `racer: None`): `LapCompleted` → `lap_chime`, `FinalLap` → `final_lap_fanfare`, `Finished` → `finish_horn` (a finish beats the fanfare, which beats the lap chime), `WrongWay(true)` → `wrong_way_buzzer`, `GoldenBattery` → `golden_battery_jingle` |
| `UiSound(cue)` | `cue.stem()` on the UI bus |
| `WeatherEvent::Lightning` | thunder (see Ambience) |
| `MusicStinger(cue)` | the stinger (see Music) |

## Music director (`music`)
Reconciles every frame: target = f(app state, race phase, session cue, final-lap flag, stinger).
| State | Target |
|---|---|
| Menu | `menu_theme` (fade in 1.5 s) |
| Boot, Loading, Unloading, `RacePhase::Setup` | silence (fade out 1 s) |
| `Intro`, `Countdown`, `Racing` | the track's cue from 0 (fade in 0.8 s) |
| … after the first local (or global) `FinalLap` | `race_N_final` started at the base loop's playback position, 1.5 s crossfade (phase-aligned, music.md) |
| `Finished` | silence + `results_jingle` |
| `MusicStinger(cue)` | the stinger once; the current music cuts in 0.4 s and silence holds for the stinger's length (`podium_fanfare` / `cup_victory` from the Cup-end screen) |
Restart (`Setup`) fades the loop out and restarts it from 0; the final-lap flag resets.

## Ambience (`ambience`)
| Environment | Bed |
|---|---|
| `toy_store`, `house` (and other indoor) | `room_tone` |
| `supermarket` | `supermarket_hum` |
| `workshop` | `workshop_hum` + `workshop_tools` |
| outdoor (`garden`, `street`, `test`) | `birds`, or `crickets` at Night |

Weather loops preloaded per possible preset — outdoor: Cloudy/Fog/Snow → `wind`; Rain →
`rain_light`; Storm → `rain_heavy` + `wind` (+ thunder); indoor tracks (`outside_only`): Rain →
`rain_light_indoor`, Storm → `rain_heavy_indoor` (+ thunder). All loops play from session start
(through restarts) and are mixed every frame (1.5 s fades) from `WeatherState`'s **blended**
columns only — `kind` jumps to the target at the start of a 20 s blend, so it is never used:

| Loop | Gain |
|---|---|
| rain light | `min(p/0.6, 1) × (1 − h)` with rain `p` = precipitation unless `snowing`, `h` = 0 → 1 over p 0.6 → 0.9 |
| rain heavy | `h` (Rain 0.6 → pure light, Storm 1.0 → pure heavy) |
| wind | `max((|wind| − 1.5)/12, snow hush)`: Cloudy ≈ 0.3, Storm ≈ 1 swelling with gusts; snow hush 0.35 × `min(p/0.5, 1)` while `snowing`; 0 indoors (no wind) |
| birds / crickets | `(1 − 0.6 × cloudiness) × (1 − min(precipitation/0.3, 1))` |
| room beds | 1 |
Fog: the ambience bus low-pass (1.2 kHz) is mixed in by `WeatherState::fog`.
Thunder on `WeatherEvent::Lightning { intensity }`: ≥ 0.7 `thunder_1` (close), ≥ 0.35 `thunder_2`,
else `thunder_3`; delay 2.6 s → 0.2 s and −6 → 0 dB with intensity; through a 700 Hz low-pass
indoors.

## Tests
`cargo test -p rc_audio` — unit (22): cue log rate limit, bus composition (master × bus, mute, pause duck, clamping),
attenuation/pan maths (rolloff, fade, monotonic, nearest listener, split-screen centring), engine
rate/level/pitch mapping, car layer mix (slip, surface, airborne, rumble, drift rate, boost), music
state machine (menu, loading, race phases, final lap, finished, stinger hold), ambience selection
(environment, time, indoor variants, thunder) and mix (every preset, Rain→Storm and Snow→Cloudy
blends), cue mappings (every item and race event maps to an existing file), roulette and goo
timing. Headless (`tests/headless.rs`, 2): the whole lifecycle with no device (loading key,
Intro → Racing, messages, pause, resume, restart, Finished, next race, quit) and the real probe
resolving within 10 frames with a race still loading either way (master volume 0).

## Known limitations
- Car layers (tyres, rumble, drift, boost) only on local players' cars; AI cars carry engines and
  one-shots. Upgrade: layers for the nearest few AI cars.
- One engine loop per timbre: beyond ≈ 0.5–2.2× it gets muddy/chipmunky (sfx.md).
- Positions use the simulation `Transform` (≤ one tick off the interpolated proxy): inaudible.
- No reverb zones (tunnels) and no doppler.
- `Cargo.toml` still lists `serde`/`ron` (unused; left untouched to avoid changing `Cargo.lock`).
