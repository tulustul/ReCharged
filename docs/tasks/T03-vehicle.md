# T03 — RC car physics (`rc_vehicle`)

**Goal:** the driving model — toy physics that feel great (GDD Pillar 2) — for all 8 cars,
measurable by tests and drivable in a test arena.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §5.2; GDD §4, §5 (all), §6.2 effects, §12.1;
`docs/design/car-dimensions.md`; digests: rapier (all — note `Velocity { linear, angular }`,
`RapierConfiguration` as a component, collision messages, `app.finish()` in tests, the built-in
vehicle controller notes), bevy §0, §2 (fixed time), §17.

## Owned paths
`crates/rc_vehicle/**`, `assets/data/vehicles/*.ron`, `docs/design/rc_vehicle.md`.
**rc_core grant:** additive edits in `crates/rc_core/src/vehicle.rs` only.

## Deliverables
1. **`VehicleParams`** RON format + 8 files tuned to GDD §4.1/§5.1 (top speed by Speed stars,
   0→top by Accel stars, handling, weight trade-offs: knock-back resistance vs air control and
   landing loss). Geometry from `car-dimensions.md`.
2. **Spawn**: on `Added<Racer>` build the rapier body (dynamic, CCD on, collision layers, mass
   properties, simplified collider inside the envelope) from the car's params; ghosting
   (`StatusEffects::ghosted`) switches car–car collision off.
3. **Model** (`SimSet::Vehicle`): 4 ray/shape-cast wheels with spring-damper suspension; tyre
   model (longitudinal drive/brake, slip-based lateral grip × `SurfaceKind::props()` × weather
   (`WeatherState` wetness/snow on outdoor surfaces) × `StatusEffects`); speed-sensitive steering;
   drift (handbrake lowers rear lateral grip); air control (weight-scaled); downforce; charge
   gain/spend (GDD §5.3 incl. the anti-farming rules); boost resolution (never stack; Turbo
   freezes charge drain); hard speed cap 1.5×; auto-right; `CatchUp` and AI-independent behaviour.
4. **Execution of requests**: `RespawnCar { FlipInPlace | ToTrack }` (teleport, zero velocities,
   send `Teleported`; the ghost effect is applied by rc_race via rc_items), `ApplyImpulse` (scaled by
   (ref_mass/mass)^0.5), `DrainCharge` / `GrantCharge`.
5. **Outputs** (`SimSet::PostPhysics`): `VehicleTelemetry` (incl. `wheels` for visuals, `up_dot`,
   `clean_landing`), `Charge`, `CarImpact` (from contact forces, debounced), `CarLanded`,
   `CarRespawned`.
6. **Rapier configuration owner**: gravity `rc_core::GRAVITY`; `physics_pipeline_active = false`
   while `PauseState::Paused`.
7. **Dev hot reload** of `vehicles/*.ron` (mtime poll ≤ 1 Hz, debug builds only).
8. **Example `arena`** (`cargo run -p rc_vehicle --example arena -- --car <id>`): flat ground,
   a ramp, a jump, a 0.5 u wall, boxes, a slope, a simple follow camera, keyboard driving (the
   example may read `ButtonInput` and write `CarControl` directly), live telemetry text. For the
   orchestrator/user to feel each car. Take screenshots for the report.
9. **`docs/design/rc_vehicle.md`**: model description, tuning knobs and their feel, the measured
   metrics table for all 8 cars (0→top, top speed, braking distance from top speed, turning radius
   at 50 % speed, drift behaviour notes).

## Tests (headless, rc_testkit in `tests/`)
Per car: 0→top time within ±5 % of its Accel-star target; top speed within ±5 % of its Speed-star
target (flat asphalt); braking distance sane; turning circle; drift starts and recovers; flips
right themselves; 3 m drop settles without bouncing forever; no tunnelling through a 0.5 u wall at
the speed cap; surface grip ordering (ice < tile < asphalt); effects (power loss, grip, boost cap);
determinism (same inputs → identical poses after 10 s); wheel layout matches
`car-dimensions.md` (and each `assets/models/cars/<car>.wheels.ron` when present).

## Acceptance
All tests pass; clippy/fmt clean; arena screenshots; metrics table in the design note. Report every
`rc_core` addition.
