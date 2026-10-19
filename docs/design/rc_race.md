# rc_race — race rules, the referee (design note, T06, T19a)

`RacePlugin` spawns each attempt's racers, drives the phases, keeps everyone's progress, decides
resets and respawns, ends the race and writes `RaceResults`. It never moves a car: respawns are
`RespawnCar` requests to rc_vehicle, effects are `ApplyEffect` requests to rc_items. Pure rules sit
next to thin systems: `phases.rs`, `progress.rs`, `standings.rs`, `recovery.rs`, `finish.rs`,
`ghost.rs`, `hints.rs`, `spawn.rs`.

## Lifecycle
| When | What |
|---|---|
| `OnEnter(RacePhase::Setup)` | fresh `RaceStatus` (`attempt` + 1), empty `Standings`, `RaceResults` removed, ghost recorder cleared |
| `OnEnter(RaceAttempt)` | `RaceConfig` re-read: racers on `Track::grid()` slots (grid order = `RaceConfig.racers`), `GoldenBattery` (if `golden_battery` and the layout has one), `GhostCar` (Time Trial with a ghost); all `DespawnOnExit(RaceAttempt)`, `Teleported` sent |
| `OnEnter(RacePhase::Racing)` | `RaceEvent::Go`, perfect starts and stalls |
| `FixedUpdate`, `SimSet::Race` (chained) | phase driver → countdown throttle → progress → standings → respawns → battery → hints → ghost record/play → referee |
| `Update`, `FlowSet` | `PlayerDropped` → `AiDriver` on that slot's racer (runs while paused) |
| `OnEnter(AppState::Unloading)` | `RaceStatus`, `Standings`, `RaceResults` and private resources removed |

A racer gets `Racer` (its required components: `CarControl`, `RaceProgress`, `Interpolated`, …),
`LocalPlayer(slot)` for humans, `AiDriver` for AI and for autopilot humans (`rival: "autopilot"`,
Elite), plus private `Tracker`, `Recovery`, `Launch`. A human who finishes gets the same
`AiDriver` in the finishing tick (Re-Volt style: the car drives its cool-down lap, at rc_ai's
finished pace, while the player waits for the others); `LocalPlayer` stays (camera, HUD).

## Phases (fixed ticks; a pause freezes them)
| Phase | Length | Leaves when |
|---|---|---|
| Setup | 1 tick | → Intro on the session's first attempt, else → Countdown |
| Intro | 180 ticks (3 s) | timeout, or any `UiNav` Confirm |
| Countdown | `rc_core::COUNTDOWN_TICKS` (180) | `CountdownTick(3/2/1)` at ticks 1, 61, 121 (`RaceStatus.countdown`), then → Racing; GO applies before the next tick (race tick 1 = countdown tick 181) |
| Racing | — | the referee (below) inserts `RaceResults` and requests Finished |
| Finished | — | rc_game (results screen, restart, next race) |

rc_game only sets `Setup` (start, restart); state requests from rc_game's `Update` override the
driver's `FixedUpdate` request of the same frame, so a restart or quit always wins. A transition is
applied once per frame: with several ticks in a frame the extra ticks stay in the old phase.

**Start timing:** the countdown tick at which each racer's throttle (> 0.5) press began is kept;
at GO (`OnEnter(Racing)`, `go_at` = countdown ticks run) a press ≤ 18 ticks (0.3 s) before GO →
`PerfectStart` + `Boost 1.30/1.50, 1 s`; held since > 60 ticks (1 s) before GO → `Stall` +
`PowerLoss 0, 0.6 s`. AI follow the same rule (rc_ai times its throttle). rc_input applies
auto-accelerate only in Racing, so it never stalls.

## Progress (Intro, Countdown, Racing)
- **Projection:** `Track::project(pos, hint)` with the previous tick's `s`; global (`None`) on the
  first tick and after a `Teleported` (respawn), when crossings and speed are skipped.
- **Lap 0:** the grid is behind the line; a racer is on lap 0 (`next_checkpoint` 0,
  `last_valid_checkpoint` = last) until it first crosses the start line, which starts lap 1.
- **Checkpoints:** the next gate counts when `s` passes it forward in this tick's move *and* the car
  is inside the gate: |lateral| ≤ half width + off-road margin (the legal corridor — open sides and
  floors may be driven within the margin), ≤ 4 u from the gate plane (rejects projection jumps),
  ≤ 4 u from the surface. Backward crossings validate nothing; a missed gate must be passed again.
  Passing the next gate's `s` outside the gate sends `RaceEvent::MissedCheckpoint` once per pass:
  it re-arms only when the car is back more than the gate depth (4 u) behind the gate, so a car
  jittering on the gate line outside it reports once (rc_ui shows the message).
- **Laps:** validating checkpoint 0 with all others done completes a lap: `LapCompleted { lap,
  time, best }` (`best` = beat this race's best, so never on lap 1), `FinalLap` when the last lap
  starts, finish after the last. Lap 1 is timed from GO (it includes the run-up), so lap times sum
  to the finish time.
- **Distances:** `distance` = arc length from the start line, never past the next gate still to
  pass (no gain from a cut); lap 0 → `distance` ≤ 0. `total_distance` = (lap − 1) × length +
  distance: ≤ 0 on the grid, continuous across the line, frozen at laps × length on finishing.
- **Wrong way:** on after 120 ticks moving < −5 u/s along the track, off after 30 ticks moving
  forward (hysteresis); `RaceEvent::WrongWay(bool)`.
- **Lost** (`lost.rs`, GDD §7.3): on after 30 ticks outside the legal corridor (|lateral| > half
  width + margin, or > 3 u below the surface) or 120 ticks moving > 5 u/s with < 30 % of the motion
  along the track (speed from the position change per tick); off after 30 ticks inside the
  corridor with ≥ 60 % along it (or slower than 5 u/s). Never with the wrong-way warning (reset
  while it is on), not before GO, after a teleport or once finished. `rejoin` = the centreline
  10 u ahead of the projection, every tick until the finish (the HUD's arrow).
- **Standings:** finished racers by finish tick, then by `total_distance`; ties keep the previous
  order, then grid order (deterministic). `Finished { rank, time }` comes with the standings of
  the finishing tick. `RaceStatus.leader_lap` = the leader's lap.
- **Splits (Time Trial):** the ghost's checkpoint times are found by projecting its samples;
  `CheckpointSplit(seconds, + = slower)` at every gate the human validates (fractional ticks).

## Resets & respawns (Racing; sole decider)
| Trigger | Respawn |
|---|---|
| Reset released before 24 ticks (tap) while `up_dot` < 0.3 | `FlipInPlace`: car position + 0.5 u, upright, same heading (track forward if pointing straight down/up) |
| Reset held 24 ticks (0.4 s), once per press | `ToTrack` |
| below `Track::kill_height()` (the kill floor is 3 u lower) | `ToTrack` |
| off-track > 180 ticks (> 120 with steering assist, humans only): no wheel grounded, or \|lateral\| > half width + off-road margin, or > 3 u below the surface (fell off an upper level) | `ToTrack` |
| stuck: speed < 1 u/s with throttle ≥ 0.5 for 240 ticks | `ToTrack` |

Every respawn also sends `ApplyEffect(Ghost 1.5 s)`; an automatic one (kill height, off-track,
stuck — not a hold) also `RaceEvent::AutoRespawn(cause)` (`Fell`, `OffTrack`, `Stuck`; the player
is told why). `respawn_in` = seconds left on the off-track timer while the car is outside the
corridor (airborne alone does not show it: every jump would). Cooldown 120 ticks: triggers wait for it
(a hold or an automatic trigger fires when it ends), taps during it are dropped. `ToTrack` uses
`Track::respawn_before(s, last_valid_checkpoint)`: never past the next gate still to pass. Timers
reset on every respawn; `off_track_time` mirrors the off-track timer.

## End of the race (referee)
Humans = racers with `LocalPlayer` (autopilot and dropped players included). The race ends when
every human has finished (no humans: every racer), or when the soft-lock guard expires. The guard
starts at the winner's finish (≤ 1 human) or the first human's finish (2+ humans), at the latest
7200 ticks (120 s) after the winner, and runs 3600 ticks: `RaceStatus.ends_in` while it runs,
`RaceEndsIn(10…1)` on each of the last ten seconds. Then `RaceResults` (standings order; finishers
with their time, the rest `finished: false` with a projected time = now + remaining distance at
their average pace, at least 5 u/s — always after now, never later for more distance), the
Time Trial human's `GhostData` (`time_ticks` = finish, `geometry_hash` from `TrackSummary`), and
`RacePhase::Finished`. After that nothing in the race changes (race clock and progress stop).

## Golden Battery, ghosts, hints
- **Golden Battery:** a `LocalPlayer` racer within 1.6 u collects it: `RaceEvent::GoldenBattery`
  and the entity despawns (rc_game saves it and clears `RaceConfig.golden_battery`). AI pass
  through.
- **Ghost recording (Time Trial):** sample `i` = the human's pose after race tick `3i + 1`
  (20 Hz) until the finish. **Playback:** the `GhostCar` stands at sample 0 until GO, then
  interpolates between samples at the same race ticks (no collider; rc_render draws it).
- **Hints** (GDD §13) for `LocalPlayer` racers without `AiDriver`: `DriftCharge` (drifting and
  gaining charge), `BoostReady` (charge ≥ 15 %), `ItemReady` (item held, roulette over),
  `HoldReset` (stuck 180 ticks), `WrongWay` (warning on). Sent only with `Settings.gameplay.hints`
  on (missing `Settings` = defaults), not in `HintsSeen` (missing = empty) and once per session.

## Events produced
`CountdownTick(n)`, `Go` (racer `None`); `PerfectStart`, `Stall`, `CheckpointSplit`,
`MissedCheckpoint`, `LapCompleted`, `FinalLap`, `Finished`, `WrongWay`, `AutoRespawn`,
`GoldenBattery`, `Hint`
(racer `Some`);
`RaceEndsIn(n)` (`None`). Messages: `RespawnCar`, `ApplyEffect`, `Teleported` (spawns).

## Decisions
- **Gate width = the legal corridor** (half width + off-road margin), not the ribbon: on open sides
  and Invisible floors a racer may legally run wide, and the off-track rule already polices the
  margin. Gates still reject the cuts the track note warns about (projection jumps across infields
  are > 4 u from the gate plane or outside the corridor).
- **Lap 0 / lap 1 from GO**: keeps `total_distance` ≤ 0 on the grid (rc_weather's Dynamic trigger)
  and makes lap times add up to the race time.
- **GO in `OnEnter(Racing)`**: boosts, the race clock and the release of the cars (rc_vehicle holds
  them in Intro/Countdown) start in the same tick.
- **Races without humans** end when every racer has finished (headless/AI-only runs terminate).
- **Splits from the ghost's samples** instead of a new persisted field: old ghosts keep working.

## Tests (`cargo test -p rc_race`)
- Unit (22): lap/checkpoint rules on `test_oval` forward and reverse (grid ≤ 0, continuity across
  the line, laps only after every gate, lap times = finish, frozen after the finish, missed gate and
  its notice once per pass — not again while jittering on the gate line — backwards), crossing wrap,
  standings (finishers, distance, ties, same-tick finishes), wrong-way hysteresis, soft-lock start
  for 0/1/2+ humans, projected times, respawn table (tap vs hold, cooldown, off-track with/without
  assist, stuck, automatic causes), lost rule (corridor, sideways, hysteresis, slow cars and
  hairpins never), lost vs wrong way and the rejoin point on `test_oval`, flip pose, start windows, phase table, countdown numbers, ghost playback and
  splits, hint triggers.
- Headless (`tests/race.rs`, rc_testkit): an 8-car 2-lap race on the full sim stack
  (`SimPlugins` without `AiPlugin`; scripted pure-pursuit pilots write `CarControl`, real physics)
  through every phase to the finish — countdown events, a perfect start and a stall, laps, ranks,
  results, the clock stops. The other tests use `TrackPlugin` + `RacePlugin` and a kinematic
  stand-in for rc_vehicle (exact positions): restart mid-race recreates the grid without the
  intro; leaving during Setup leaks nothing; pause freezes race time; kill-height respawn +
  ghosting + cooldown, Golden Battery pickup, dropped player; a finished human is handed to the AI
  (`AiDriver` "autopilot", `LocalPlayer` kept) while the other human races on; Time Trial ghost
  recorded, raced back at the same pace with ~0 splits; the soft-lock guard classifies a human who
  never finishes.

## Known limitations
- A missed gate stops the racer's distance growing until it goes back (the AI holds reset to be
  put back before it, rc_ai).
- Respawns in the same spot overlap; ghosting covers 1.5 s (no spawn jitter).
- In `Finished` nothing is refereed: a car falling off during the results stays there.
- Projected times assume the average pace so far (a racer stuck at the end projects slow).
