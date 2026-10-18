# T06 — Race rules (`rc_race`)

**Goal:** racers on the grid, phases, laps, positions, finish, respawns, ghosts — the referee.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §3.1 (lifecycle — attempt entities spawn in
`OnEnter(RaceAttempt)`), §5.5; GDD §3.2–3.3, §5.4, §7.3, §9 (records), §13 (hints);
`docs/design/rc_track.md` (Track API); `docs/design/rc_vehicle.md`.

## Owned paths
`crates/rc_race/**`, `docs/design/rc_race.md`. **rc_core grant:** additive edits in
`crates/rc_core/src/race.rs` only.

## Deliverables
1. `OnEnter(RacePhase::Setup)`: reset race resources. `OnEnter(RaceAttempt)`: spawn racers on the
   grid from `RaceConfig` (`Racer`, `LocalPlayer`/`AiDriver` — autopilot adds both, `CarControl`,
   `RaceProgress`, `Interpolated`, scoped to the attempt, `Teleported`), the `GoldenBattery` when
   `RaceConfig.golden_battery`, the `GhostCar` in Time Trial.
2. Phases Intro (≈3 s; Confirm via `UiNav` skips; skipped on restart attempts) → Countdown
   (`RaceEvent::CountdownTick`/`Go`; perfect start / stall from throttle timing → `ApplyEffect`) →
   Racing → Finished.
3. Progress (`SimSet::Race`): `total_distance` ≤ 0 for cars still behind the start line (rc_weather's
   Dynamic trigger relies on it); projection with continuity hints, ordered checkpoint validation,
   lap timing in ticks, `LapCompleted`/`FinalLap`/`Finished`, standings (finished by time, then
   `total_distance`), wrong way (> 5 u/s against the track for 2 s), soft-lock guard (GDD §3.2) with
   `RaceEndsIn`, projected times; `RaceStatus`, `Standings`; `RaceResults` (+ ghost, geometry
   hash) when the race ends.
4. **Sole owner of reset & respawn decisions**: reset tap (flip in place if `up_dot` < 0.3) / hold
   0.4 s (respawn, timed here from `reset_held`), kill height, off-track (> 3 s; 2 s with steering
   assist), stuck (4 s) → `RespawnCar { kind }` + `ApplyEffect(Ghost 1.5 s)`; 2 s cooldown; respawn
   point never ahead of the last validated checkpoint.
5. Golden Battery pickup → `RaceEvent::GoldenBattery`. Ghost recording at 20 Hz (Time Trial) and
   `GhostCar` playback.
6. Onboarding hints (`RaceEvent::Hint`) per GDD §13 triggers — only if
   `Settings.gameplay.hints` is on and the kind is not in `rc_core::HintsSeen` (missing resource =
   empty); remember within the session what was already emitted.
7. **Integration with rc_game (T07, done)**: insert `RaceResults` no later than entering `Finished`;
   remove it in `OnEnter(Setup)` and `OnEnter(Unloading)`. On `PlayerDropped { slot }` add `AiDriver`
   to that slot's racer. Read `RaceConfig` per attempt (rc_game may update it during Setup). Set
   `GhostData.geometry_hash` from `TrackSummary`. **Take over phase driving** from rc_game's
   placeholder `crates/rc_game/src/phases.rs`: delete that file and its one registration line in
   `crates/rc_game/src/lib.rs` (granted to you for exactly this), and replace the orchestrator's
   placeholder grid spawner in `crates/rc_race/src/spawn.rs` with track grid slots.

## Tests
Unit: checkpoint/lap rules incl. start-line wrap and reverse layouts; standings ordering with ties
and finished racers; wrong-way hysteresis; soft-lock timings for 1 vs 2+ humans; respawn decision
table (tap vs hold, cooldown, off-track, stuck); perfect-start/stall windows; projected times.
Headless (rc_testkit, `tests/`): an 8-car race on `test_oval` with scripted/AI-free control runs
to the finish; restart mid-race recreates the grid; leaving during Setup leaks nothing; pause
freezes race time.

## Acceptance
Tests, clippy, fmt clean; `docs/design/rc_race.md` (rules & events).
