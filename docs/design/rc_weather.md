# rc_weather — weather state (design note, T15)

A deterministic state machine that every other system reads through `WeatherState` (sole writer:
rc_weather). Pure logic in `plan.rs` (what a race's weather does) and `sim.rs` (one fixed tick);
`lib.rs` holds three thin systems. No rendering, no sound, no randomness outside `rc_core::rng`.

## Lifecycle
| When | What |
|---|---|
| build | `WeatherState::default()` (Clear, Noon, all zeros): what menus and `Loading` see |
| `OnEnter(RacePhase::Setup)` | plan from `RaceConfig` + `Catalog`, reset `WeatherState` to the settled start, reset the private `WeatherSim` |
| `FixedUpdate`, `SimSet::Weather` | one `WeatherSim::step` per tick, written with `set_if_neq` (settled weather does not trip change detection) |
| `OnEnter(AppState::Unloading)` | remove `WeatherState` and `WeatherSim` |
| `OnExit(AppState::Unloading)` | default `WeatherState` again, for the menu or the next `Loading` |

`WeatherState` is absent only in the one `Unloading` frame. The sim ticks in every race phase while
`PauseState::Running` (the intro fly-by shows live weather); pausing freezes it. Restarts replan
with the same seed, so every attempt of a session gets identical weather.

## Plan (`plan.rs`)
- **Fixed(k)**: `k` all race. **Random**: uniform pick from the track's `allowed_weather`
  (`WeatherPlan` stream, key 0). **Dynamic { start }**: one change to a uniformly picked neighbour
  that the track allows (key 2), starting when the leader has covered a uniform 30–70 % of the race
  distance (key 1). Neighbours (GDD §10.2): Clear↔Cloudy↔Rain↔Storm, Cloudy↔Fog, Cloudy↔Snow. No
  allowed neighbour → no change.
- **Indoor** (`TrackInfo.indoor`) → `outside_only`: wetness, snow cover and wind stay 0;
  precipitation, cloudiness, fog and lightning are still computed for the windows. rc_game only
  builds Dynamic for outdoor tracks; an indoor one would change the outside weather only.
- Track not in the catalog (or no `Catalog`, e.g. tests) → outdoor, every preset allowed.
- Wind direction: a seeded horizontal unit vector per race (key 3).
- Race distance = `TrackSummary.length × RaceConfig.laps`; leader = max
  `RaceProgress.total_distance`. No `TrackSummary` → the change never starts.

## Parameter table (steady state per preset, `sim::preset`)
| Kind | Precipitation | Cloudiness | Fog | Wind u/s | Gust peak u/s | Lightning |
|---|---|---|---|---|---|---|
| Clear | 0 | 0.10 | 0 | 2 | 0 | 0 |
| Cloudy | 0 | 0.80 | 0.05 | 5 | 0 | 0 |
| Rain | 0.6 (rain) | 0.90 | 0.15 | 6 | 0 | 0 |
| Storm | 1.0 (rain) | 1.00 | 0.25 | 14 | +16 | 1 |
| Fog | 0 | 0.50 | 0.85 | 1 | 0 | 0 |
| Snow | 0.5 (snow) | 0.85 | 0.30 | 3 | 0 | 0 |

Wind is at world scale (1 u = 22.5 cm; real rain falls ≈ 40 u/s, snow ≈ 4 u/s): Rain falls nearly
straight (≈ 9°), Storm slants ≈ 20° and ≈ 37° at gust peaks, snow drifts visibly. Lightning
(0–1) scales strike intensity and is lerped like every other column, so strikes fade in and out
with a Rain↔Storm blend.

| Timing | Value |
|---|---|
| Dynamic blend | 20 s, smoothstep weight on every column; `kind` = the target from the first blend tick |
| Wetness | +1 per 20 s while rain falls, −1 per 60 s once it stops (linear) |
| Snow cover | +1 per 20 s while snow falls, −1 per 90 s once it stops (linear) |
| Lightning slots | every 5–14 s; strike intensity = lightning × U(0.5, 1); the flash (`lightning`) jumps to it and fades linearly (1 → 0 in 0.3 s) |
| Gusts | 2.5 s sin² pulse, peak = gust peak × U(0.5, 1), 3–8 s calm between gusts; direction unchanged |

Rain/snow "falls" while `precipitation > 0`, so the ground stays wet through a fading drizzle and
starts drying when the blend ends.

## Decisions
- **Races start settled**: a Rain race starts on wet ground, a Snow race on full cover (it has been
  raining before the race). Only a Dynamic change ramps wetness/snow — a fresh 20 s ramp at GO would
  read as a bug.
- **`WeatherState.snowing`** (additive, rc_core): during Snow → Cloudy `kind` is already Cloudy
  while snow still fades out, so `kind` alone cannot tell rain from snow. Rain↔Snow are never
  neighbours, so one flag covers every blend.
- **Indoor wind is zero**, so gameplay (gusts pushing airborne cars) needs no indoor special case.
- Lightning and gust schedules draw from `RngStream::Lightning` / `WindGust` keyed by the sim's own
  tick (ticks since Setup), so they depend only on the seed. Trig uses `bevy::math::ops`.
- Flash brightness is gameplay-neutral data; *Reduce flashing* (GDD §18) is applied by renderers.

## Tests
`cargo test -p rc_weather` (11 unit + 1 headless, ≈ 1 s):
- `plan.rs`: determinism per seed; Random covers exactly the allowed presets; the neighbour graph
  is the GDD's; Dynamic picks neighbours and uses the whole 30–70 % window; the track's allowed
  presets restrict Dynamic; indoor → `outside_only`.
- `sim.rs`: Fixed starts settled and holds still (non-Storm); the change fires once at its distance
  and blends smoothly and monotonically over 20 s, settling exactly on the target's start state;
  wetness and snow ramp in 20 s and dry/melt in 60/90 s; indoor never wet, white or windy; Storm
  strike count, spacing, intensity, flash fade, gust range.
- `tests/race.rs` (headless, rc_testkit + a stand-in for rc_race's progress): a 3-lap Dynamic race
  changes exactly once inside 30–70 %, a restart replays the identical change, `WeatherState` is
  removed while unloading and the menu default returns.

## Known limitations
- The trigger trusts rc_race's `total_distance`: a grid behind the start line must read ≤ 0 (not
  "almost a lap"), or a 30 % change could fire at GO.
- A pause during a flash freezes `lightning` until resume; renderers may ignore it while paused.
