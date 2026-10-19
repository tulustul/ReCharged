# rc_vehicle — RC car physics (design note)

Toy physics that feel great (GDD Pillar 2): a custom ray-cast car on a rapier rigid body, tuned by
measurement. Code: `crates/rc_vehicle/src/` (`model.rs` physical model, `assists.rs` arcade
assists, `drive.rs` per-tick system, `charge.rs`, `outputs.rs`, `spawn.rs`, `requests.rs`,
`params.rs`). Data: `assets/data/vehicles/<car>.ron`.

## Pipeline (FixedUpdate, 60 Hz)
| Set | Systems (chained) |
|---|---|
| `SimSet::Vehicle` | `spawn_bodies` (`Added<Racer>`) → `rebuild_bodies` (hot reload) → `execute_respawns` → `apply_impulses` → `sync_ghosting` → `drive_cars` |
| rapier | SyncBackend → StepSimulation → Writeback (`PhysicsPlugin`: gravity 25, one step per tick, frozen while paused) |
| `SimSet::PostPhysics` | `motion` (1.5× speed cap, air/flight timers, landings, drift state) → `update_charge` → `write_telemetry` → `report_impacts` |

`drive_cars` per car: 3 rays per wheel (centre and ±0.7 r fore/aft approximate the round tyre:
smooth over ramp lips and curbs; they look ahead by the current fall) → landing absorber →
`Command` (control × effects × boost × surface × phase) → `model::step` (suspension, tyres) →
assists → `ExternalForce`. Cars are **held** (no drive, brakes on) in `RacePhase::Intro|Countdown`;
rpm then follows the throttle (audio revs).

## Physical model
- **Drive:** a(v) = A·(1 − v/V)^0.5 toward the effective top speed V; A is derived so the car
  reaches 99 % of `top_speed` in `accel_time` (closed form, `launch_accel`). Same acceleration
  for every mass (stars define it); forces split by drivetrain (4WD 40/60, locked axles), each
  wheel limited by `traction × load` in a friction ellipse with the lateral force.
- **Brakes / reverse / coast:** `brake_decel`, reverse at 60 % of A up to `reverse_speed`, 5 u/s²
  coast with no pedal, a parking hold below 0.5 u/s, 7 u/s² above the effective top speed
  (boost ending, grass, downhill).
- **Suspension:** spring rate from `frequency` (sprung quarter mass), preload so the car rests at
  the dimension-table ride height, damping ratio `damping`; rebound 2.5× compression (no pogo);
  digressive compression above 2 u/s (lips and curbs compress instead of kicking); progressive
  bump stop. **Landing absorber:** at touchdown the closing speed beyond √(2·110·bump) is removed
  (a thud, never a launch); the chassis collider sits `bump + 0.06` above the ground.
- **Tyres:** lateral force removes the contact patch's side velocity within 0.05 s, capped by
  `grip × load × surface × weather × effects`, with progressive breakaway (full grip to 8° slip,
  78 % from 20°). Rear uses `rear_grip` (slightly above `grip` = stable understeer at the limit).
  Forces act 70 % of the way up to the centre of mass: lively body roll, no cornering rollovers.
- **Steering:** full `lock_deg` at low speed; at speed the angle at which the front tyres ask for
  1.15 × their grip (kinematic radius), so full lock never ploughs. **Counter-steer assist:** the
  front wheels follow the rear axle's slide beyond 4° (× `counter_steer`).
- **Downforce:** `downforce` × weight at top speed (∝ v²), on the ground only.

## Arcade assists (`assists.rs`)
- **Drift:** handbrake + steering above 6 u/s engages a drift in the steering direction; rear grip
  ramps to `drift_grip` (8/s down, 3/s up); a yaw servo holds the body slip angle at
  `drift_angle_deg` (× 1.45 steering in, × 0.55 counter-steering) and an extra lateral force
  (22 % of grip, ± by steering) keeps the drift line as tight as grip driving. Releasing the
  button targets 0° — the catch. Strength × surface grip: ice/oil still spin you.
- **Stability:** without the handbrake, forward slides beyond 6° are caught (× `stability` × grip).
- **Air control (weight-scaled `air_control`):** attitude targets — throttle tips the nose 15°
  down, brake lifts it 15°, handbrake + steer rolls ±30°, idle axes level out; steering yaws at
  3 rad/s. Holding the throttle in the air (everyone does) never flips the car.
- **Auto-right:** roof/side (`up_dot` < 0.3), < 2 u/s for 1 s → 6 u/s hop + righting servo until
  upright and calm. A body scraping along (no wheels down) decelerates at 12 u/s² so it can stop.
- **Wind:** airborne cars get `0.15/s × WeatherState.wind`.

## Data (`VehicleParams`, `deny_unknown_fields`; errors name file and field)
| Field | Feel |
|---|---|
| `top_speed`, `accel_time` | Speed / Accel stars (GDD §5.1, `rc_core::top_speed_for_stars` …); tests assert both |
| `reverse_speed`, `brake_decel` | reversing, braking distance |
| `body`, `wheelbase`, `front`/`rear` (`track`, `radius`, `width`) | binding geometry of `car-dimensions.md` (test) |
| `mass` | Weight stars: 2.0 / 2.3 / 2.6 / 3.0 / 3.4 — pushing, knock-back |
| `com_height` | lower = flatter |
| `suspension` (`bump`, `droop`, `frequency`, `damping`) | travel (visible bounce), stiffness, settle |
| `tyres` (`grip`, `rear_grip`, `traction`, `drift_grip`) | Handling stars: grip 1.15 / 1.25 / 1.35; traction (launch, brakes); drift looseness |
| `steering` (`lock_deg`, `counter_steer`, `drift_angle_deg`, `stability`) | low-speed agility, drift catch, drift angle, slide catch |
| `air_control` | Weight stars: 1.0 / 0.9 / 0.8 / 0.68 / 0.55 |
| `landing_loss` | Weight stars: 0.04 / 0.06 / 0.09 / 0.12 / 0.16 of horizontal speed at strength 1 |
| `downforce` | planted feel at speed (wing cars 0.35–0.4, monster truck 0.05) |

Loaded for every car on `OnEnter(Loading)` (`LoadingDone("vehicles")`, or `LoadingFailed` for a
bad file or a racer car without params); removed on `OnEnter(Unloading)`. Debug builds poll the
files' mtimes once a second and rebuild live bodies whose params changed.

## Charge & boost (GDD §5.3, §5.5)
Drift 22 %/s × min(1, yaw rate / 1.8) only above 40 % top speed, after 0.4 s of continuous drift,
at 10–45° slip (a spin past 45° resets the timer); air 15 %/s after 0.3 s of *free flight* (resting
on the body > 0.2 s is not flight); clean landing +10 % (flight > 0.8 s, all four wheels within
0.15 s, tilt < 20°); drafting 10 %/s (≤ 6 u behind, ±15° cone around the leader's travel,
≥ 50 % own top speed). Charge boost ×1.30 / ×1.50, needs 15 % to start, drains 35 %/s. Boosts
never stack: the strongest of charge and status boost applies; charge drains only while the charge
boost is the one in effect and no freezing (Turbo) boost runs. Effective top speed =
base × surface × effects × boost × (1 + catch-up), capped at 1.5× base; the velocity itself is
also capped at 1.5× base after every step. `DrainCharge` multiplies, `GrantCharge` adds (≤ 1).

## Outputs
- `VehicleTelemetry` (PostPhysics): wheel `spin` (rad, + = rolling forward), `steer` (rad,
  + = right), `compression` (0 extended … 1 bottomed), **`offset`** (u from the rest position along
  car up — add to the model's wheel node), `slip` (sliding speed / 6 u/s); `slip_angle` (+ = moving
  right of the heading), `drifting` (10° in / 7° out, < 90°), `airborne_time` (no wheel down),
  `rpm` (driven-wheel speed / effective top), **`charge_rate`** (%/s of the latest tick).
  Per wheel also **`contact`** / **`normal`** (world space): the tyre's lowest point in the
  post-step pose moved onto the plane its ray hit at the start of the tick (`ground_contact`),
  so effects sit on ramps and curbs; airborne wheels report the tyre bottom and the car's up.
- `CarLanded` after ≥ 0.25 s of air: at touchdown, or after the clean-landing judgement (≤ 0.15 s)
  for flights over 0.8 s; strength = pre-absorber closing speed / 15 u/s.
- `CarImpact` from rapier contact forces: strength = Δv / 15 u/s (≥ 0.12), near-vertical normals
  skipped (landings), a 0.2 s debounce unless 1.5× stronger; `point` is where the contact normal
  leaves the chassis box.
- `RespawnCar` → teleport, zero velocity, `Teleported` + `CarRespawned` (same tick).
  `ApplyImpulse` × (`REFERENCE_CAR_MASS` / mass)^0.5.

Collision: chassis = one rounded box inside the body envelope (friction 0.25 `Min`, restitution
0.3 `Max`: bouncy, never sticky), CCD, sleeping off, `CAR` membership; ghosted cars add
`GHOST_CAR` and drop `CAR`/`GHOST_CAR` from their filter (still hit pickups, items, track). Wheel
rays hit fixed `TRACK | PROP` colliders whose surface is within 60° of the car's up axis, never
dynamic props or cars.

## Measured metrics (flat asphalt, `cargo test -p rc_vehicle --test metrics -- --ignored --nocapture`)
| Car | Top speed u/s (target) | 0→99 % s (target) | Braking from top, u | Turn radius @50 %, u | Lateral accel @50 %, u/s² | Yaw response, s | Drift entry / hold, ° | Drift lateral accel | Drift recovery, s | Drift speed kept |
|---|---|---|---|---|---|---|---|---|---|---|
| sparky | 27.0 (27) | 2.98 (3.00) | 9.1 | 5.6 | 32.7 | 0.13 | 35 / 30–35 | 32.7 | 0.40 | 81 % |
| dumpster | 27.0 (27) | 3.23 (3.25) | 9.2 | 6.9 | 26.8 | 0.15 | 32 / 28–32 | 27.9 | 0.38 | 88 % |
| zip | 27.0 (27) | 2.73 (2.75) | 9.1 | 6.1 | 30.7 | 0.12 | 36 / 32–36 | 31.7 | 0.50 | 86 % |
| torque | 33.0 (33) | 2.98 (3.00) | 13.6 | 9.8 | 28.9 | 0.10 | 35 / 31–35 | 30.2 | 0.53 | 101 % |
| hornet | 30.0 (30) | 2.73 (2.75) | 11.2 | 7.1 | 32.6 | 0.10 | 35 / 30–35 | 33.8 | 0.42 | 103 % |
| phantom | 33.0 (33) | 2.73 (2.75) | 13.6 | 8.3 | 33.8 | 0.10 | 37 / 32–37 | 35.9 | 0.53 | 94 % |
| volt_gt | 36.0 (36) | 2.73 (2.75) | 16.1 | 10.5 | 32.2 | 0.10 | 35 / 30–35 | 34.0 | 0.50 | 109 % |
| bolt | 36.0 (36) | 2.48 (2.50) | 16.1 | 10.4 | 32.4 | 0.10 | 36 / 32–36 | 35.4 | 0.52 | 109 % |

Turn: full lock at a held 50 % of top speed (steady state). Yaw response: time to 90 % of the
steady yaw rate. Drift: from 70 % top speed at full throttle, 0.8 s handbrake + full lock, 1.2 s
handbrake + half lock (hold range), then release and centre (recovery to < 5°). Drift notes:
slip passes 10° after 0.17–0.18 s, never 45° (the charge window), lateral grip in the drift ≈
grip driving (+5–10 %), so drifting is a cornering tool, not a detour; 4WD cars (Sparky,
Dumpster) scrub speed in a drift while light RWD cars keep accelerating; Dumpster and Torque
(Handling 2★) drift widest.

| Car | Kicker (14°, full throttle held): take-off u/s | air s | landing strength | clean | 3 u drop: deepest, u | settled (±0.02) after, s | roof → upright, s | knock-back Δv (vs 3★) |
|---|---|---|---|---|---|---|---|---|
| sparky | 22.4 | 0.82 | 0.91 | yes | −0.17 | 0.52 | 1.67 | 1.20 |
| dumpster | 23.6 | 0.63 | 0.81 | no | −0.29 | 0.62 | 1.62 | 0.67 |
| zip | 22.1 | 0.88 | 0.93 | yes | −0.06 | 0.35 | 1.72 | 1.48 |
| torque | 27.2 | 0.90 | 0.95 | yes | −0.09 | 0.38 | 1.72 | 0.81 |
| hornet | 25.3 | 0.83 | 0.92 | yes | −0.14 | 0.53 | 1.72 | 1.00 |
| phantom | 25.9 | 0.72 | 0.86 | no | −0.07 | 0.42 | 1.80 | 1.00 |
| volt_gt | 28.7 | 0.75 | 0.84 | no | −0.07 | 0.43 | 1.78 | 1.20 |
| bolt | 31.3 | 0.67 | 0.85 | no | −0.06 | 0.52 | 1.80 | 1.48 |

Drop settle times count from the release (touchdown at 0.49 s). "No" clean landings are jumps
under the 0.8 s flight the rule needs. Knock-back = (2.6 / mass)^1.5 for the same `ApplyImpulse`.

## How to verify
- `cargo test -p rc_vehicle` — unit (model maths, charge rules, params) + integration
  (`tests/driving.rs` star targets, braking, turning & handling order, drift, jumps, flips, drops,
  tunnelling, trimesh ground, surfaces, weather, effects & cap, countdown hold, determinism;
  `charge.rs`; `requests.rs` respawn, impulses, ghosting, impacts, walls, pause; `data.rs` files,
  catalog, dimension table, model `.wheels.ron`, loading gate, hot reload).
- Tables and robustness probes: `cargo test -p rc_vehicle --test metrics -- --ignored --nocapture`.
- Feel it: `cargo run -p rc_vehicle --example arena -- --car <id>` (keys in the window; 1–8
  switch cars). Automation: `--auto --screenshot <png> --screenshot-at 2.6,3.3,6.0 --exit-after 12`.

## Known limitations
- Wheel rays ignore dynamic props and other cars (the chassis handles them): a car never drives
  on top of another car's roof.
- Rapier's speculative contacts already stop the tunnelling cases tested at the speed cap; CCD is
  the backstop for thinner geometry (the test cannot tell them apart).
- `CarImpact.point` is approximated on the chassis box (no manifold query).
- Sharp ramp lips (a flat-to-14° kink) still pitch the car up on entry; tracks should model
  transitions. Long-flight landings report `CarLanded` up to 0.15 s after touchdown.
- Drafting uses a straight cone behind the leader (no track curvature).
