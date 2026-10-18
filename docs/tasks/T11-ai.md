# T11 — AI drivers (`rc_ai`)

**Goal:** seven rivals who race cleanly and fast at Elite, beatable at Rookie, use items smartly,
never get stuck — plus autopilot and the steering assist.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §3.2 (Drivers/Assist sets), §5.6; GDD §5 (driving
model), §6 (items & counterplay), §8 (AI), §15 (assist); `docs/design/rc_track.md`,
`docs/design/rc_vehicle.md`, `docs/design/rc_race.md`, `docs/design/rc_items.md` (when present).

## Owned paths
`crates/rc_ai/**`, `assets/data/ai.ron`, `docs/design/rc_ai.md`. **rc_core grant:** additive edits
in `crates/rc_core/src/ai.rs` (create it) only.

## Deliverables
1. **Driving** (`SimSet::Drivers`, `AiDriver` racers): pure-pursuit steering on a personal lateral
   offset of the racing line with speed-dependent lookahead; throttle/brake from
   `Track::target_speed_at` scaled by difficulty and by the car's capability (stars → speeds from
   rc_core constants; adapt to measured telemetry); drift in long corners for charge; boost on
   straights; overtaking lanes when blocked; hazard avoidance (`Hazard`, `Projectile`); ignores
   small dynamic props; difficulty mistakes (late braking, wide lines) via the keyed RNG.
2. **Items**: decisions per GDD §8.1 with reaction delays by difficulty and personality (Nova hoards,
   Mayhem attacks, …).
3. **Countdown**: perfect-start chance by difficulty (5/20/40/60 %).
4. **Recovery**: detect being stuck → reverse & steer out → hold reset.
5. **Catch-up** (`CatchUp` component, ±6 %) when enabled; never in Time Trial.
6. **Autopilot**: works for `LocalPlayer` racers that also have `AiDriver`.
7. **Steering assist** (`SimSet::Assist`): blend only `steer` toward the racing line for human
   racers with assist on; turn back toward the track after a crash/respawn.
8. `ai.ron` (difficulty profiles, personalities) with dev hot reload.

## Tests
Unit: pure-pursuit geometry, speed control, lane choice, item decision tables, reaction timers.
Headless: 8 AI finish 3 laps on `test_oval` and `toy_town` (and later every layout) within 1.5×
the expected time with ≤ 2 respawns per lap (`#[ignore]` for the long all-tracks variant, with run
instructions); Elite beats Rookie by a clear margin; determinism with a fixed seed.

## Acceptance
Tests, clippy, fmt clean; lap-time table per difficulty per track in `docs/design/rc_ai.md`.
