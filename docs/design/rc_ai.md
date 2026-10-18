# rc_ai — AI drivers, autopilot, steering assist (design note, T11)

Seven rivals who race the rc_vehicle car model cleanly and fast at Elite, beatable at Rookie, use
items, and never stay stuck; the same driver runs autopilot humans. Code: `crates/rc_ai/src/`
(`driver.rs` per-car pipeline, `pace.rs` speed & mistakes, `profile.rs` speed plan, `steering.rs`
pure pursuit & pedals, `traffic.rs` lanes, `scene.rs` what drivers see, `charge.rs` drift & boost,
`items.rs` item decisions, `recovery.rs` stuck handling, `catchup.rs`, `assist.rs`, `brain.rs`,
`data.rs`). Data: `assets/data/ai.ron` (hot-reloaded in debug builds, ≤ 1 s).

## Pipeline (FixedUpdate)
| Set | Systems (chained) |
|---|---|
| `SimSet::Drivers` | `observe_racers` (track coordinates of every racer: `TrackView`) → `ensure_brains` (an `AiBrain` + `CatchUp` for every new `AiDriver`; re-plans on a tuning hot reload) → `update_catch_up` → `drive_ai` (`CarControl` of every `AiDriver`, autopilot humans included) |
| `SimSet::Assist` | `steering_assist`: `steer` only, `LocalPlayer` without `AiDriver`, `Settings.accessibility.steering_assist` on, Racing only |

`AiTuning` loads on `OnEnter(Loading)` (`LoadingDone("ai")`, or `LoadingFailed` naming file and
field) and goes on `OnEnter(Unloading)`. `AiClock` (RNG tick) resets on `OnEnter(Setup)`. All
randomness is `rc_core::rng` keyed by (seed, tick, stream, racer index × 16 + purpose): line
offset and perfect start (`AiLineOffset`, `AiPerfectStart`), mistakes and drift choices
(`AiMistakes`), item reactions (`AiReaction`). A seeded race replays bit for bit (test).

## Driving
- **Speed plan per car** (`profile.rs`): built when the brain is made, sampled every 1 u along the
  centreline. Corner limit √(a·grip / κ) on the racing line's curvature (Menger, ±3 u), a =
  `lateral_accel_by_handling[stars]` (measured full-lock cornering of rc_vehicle's cars, 25–34.5
  u/s²) × the difficulty's `corner_grip`; then a backward braking pass at `brake_decel` (28 u/s²;
  cars brake at ≈ 40) around the loop, no braking in the air. No top-speed cap in the plan: the
  car's *current* top speed (Speed stars × surface × catch-up × status effects × boost) caps the
  throttle at query time, so braking zones stay right whatever the car is doing.
  - **Banking does not raise the plan.** rc_vehicle's speed-sensitive lock asks the front tyres
    for 1.15× their flat-ground grip at full lock, independent of bank, so a banked turn cannot be
    taken tighter than a flat one (measured: crediting g·sin(bank) put cars on full lock and into
    toy_town's T3/T5 outer lip). Off-camber still costs grip.
  - Query time: × `speed_scale` (difficulty, GDD §8.2), × √(weather grip ratio) of the surface,
    × √(status `grip_mul` × `adapt`), × an inside-lane factor √(1 − κ·d) when traffic puts the
    car inside the racing line. `adapt` drops 0.4/s while the car slides (> 11°, not drifting)
    and recovers 0.02/s, floor 0.8 — the plan adapts to measured telemetry (wet, oil, ice).
- **Pedals** (`steering::pedals`): throttle below the target; **brake only above the
  brake-point speed** (the lowest √(v(d)² + 2·a·d) over the braking distance ahead, 0.1 s lead),
  coast in between — a speed cap (difficulty scale, a boost running out) is coasted down, never
  braked for. rc_vehicle has no rolling resistance under throttle, so holding a speed alternates
  throttle and coasting.
- **Pure pursuit** (`steering.rs`): target `lookahead` = clamp(0.45 s × v, 4, 16) u ahead on the
  car's line; κ = 2 sin α / d; steer = κ / κ_max(v) with κ_max = min(tan 31°/1.3 u, 1.2·a/v²)
  (rc_vehicle's speed-sensitive lock). While sliding (> 8°) or drifting the angle is taken from
  the velocity, not the nose: rc_vehicle's counter-steer and stability assists catch the slide,
  the AI steers the path (heading-referenced full opposite lock on top of the assist spun cars).
- **The line**: racing line + a personal offset (± `line_spread_u`, keyed RNG) + a lane deviation,
  clamped to the **narrowest** legal width within 16 u ahead (`half width − edge_margin_u`) — cars
  queue before the dollhouse instead of arriving three abreast.
- **Traffic** (`traffic.rs`): a slower car on the planned lateral within 16 u is passed on the side
  the car already is on, else the other side, else followed. Cars alongside (±3 u along the track)
  bound the lateral range (never squeeze into a car). Never drive into a slower car that overlaps
  the *current* lateral: close to 3 u + 0.25 s × v behind it. Lane changes move at 4 u/s.
- **Hazards and props**: rc_items' Oil Slick / Spring Pad discs and Shock Orbs (reach from
  `ItemRadius`, + 0.6 u) are obstacles, and so are static, non-drivable props in the legal
  corridor whose colliders reach into the car's height band (0.15–1.3 u above the surface; higher
  parts are driven under): flower-pot chicanes, chair legs, tyre stacks, garden edging. Each
  collider becomes footprint circles — boxes a row of small circles along their long side, so a
  wall flanking a tunnel never closes it — computed once per track. A dodge is applied at once (a
  rate-limited one could not clear a pad lying 6 u past a respawn point: the car was launched over
  the wall on every respawn). A car well off its line (> 1.5 u) also clears obstacles on its own
  path back (backyard_bash's garden edging across the lawn margin trapped cars for 7 s a lap).
  Cars carrying a Goo Bomb get 2 u more berth. Dynamic props are ignored (the chassis knocks them
  away); drivable ones (ramps, box tops) are driven.
- **Jumps** (`Track::jumps`, 25 u before the lip to 10 u after the landing): traffic lane changes
  ≤ 0.8 u and hazard dodges ≤ 3 u (ramps and gaps have no barriers), no traffic speed limits
  (too little speed drops the car into the gap: on test_oval one slow car under the landing ramp
  made the whole field queue and fall in), no hazards dropped there, and on the run-up (lip within
  45 u) the throttle never aims below 0.9× the jump's reference takeoff speed, whatever the
  difficulty scale (rc_track guarantees landings from 0.8×).
- **Mistakes** (GDD §8.2): every 0.5 s, chance `mistakes_per_min` × (1.5 − consistency):
  late braking (plan × 1.08–1.16) or a wide line (1–2.2 u outward, corners only), 1–2 s.

## Charge (GDD §5.3)
- **Drift zones** (per brain): racing-line radius < 32 u, planned speed ≥ 50 % top, ≥ 1.2 s long,
  ending 0.5 s before the corner does (the catch, ≈ 0.5 s, happens inside it). A zone is drifted
  with the difficulty's `drift` chance, from stable grip driving only (4 wheels down, slip < 8°,
  within 1.5 u of its line, battery < 70 %). The drift ends when it does not charge after 0.7 s
  (`VehicleTelemetry::charge_rate`), the pursuit asks for > 0.35 counter-steer (too tight), the
  car spins (> 40°), slows below the zone speed or gets within 0.5 u of the edge (its line keeps
  1 u more room: a car turned 30° reaches the barrier with its corner). After a drift the throttle
  is eased (0.4) until the slide is caught. Measured (solo Hornet, toy_town): drifting costs
  ≈ 0.5 s per lap (4WD cars scrub speed in a drift) that the charge partly earns back on the next
  straight; before these entry/exit rules a wall-scraping helix drift cost 2–5 s per lap.
- **Boost**: from `BOOST_MIN_CHARGE` + (1 − eagerness) × 0.2 when the plan stays above the boosted
  top speed for `boost_straight_secs` × (1.5 − eagerness) (integrated with the car's Accel-star
  acceleration), all wheels down, not drifting, > 50 % top; held until braking is due.

## Items (GDD §8.1, `items.rs`)
| Item | Used when |
|---|---|
| Spark Rocket | a car in the 30° cone ahead within 70 u (forward), else one behind within 40 u (backward) |
| Leech Coil | the car directly ahead in `Standings` within 38 u |
| Shock Orb | a car ahead along the track within 45 u |
| EMP Pulse | a car within 10 u |
| Goo Bomb | a car within 25 u; then the carrier aims at the nearest unshielded car ahead (no avoidance, no follow limit) |
| Oil Slick / Spring Pad | a car within 14 u behind on the line (never on a jump approach); leaders keep one to defend, others drop it after `idle` |
| Shield Bubble | a rocket is locked on (a `Projectile` whose `ItemTarget` is this car), or after `idle` |
| Turbo Cell | ≥ 1.5 s × (1 + hoard) of straight ahead, not boosting |
| Storm Strike | not leading |

`idle` = 4 s + hoard × 20 s. Ranges × (0.75 … 1.2 by aggression) × (1 − 0.35 × hoard): Mayhem
fires early, Nova waits for surer shots and keeps defensive items. A wanted use starts the
difficulty's reaction delay (Rookie 0.8–1.5 s … Elite+ 0.15–0.3 s, keyed RNG); the item goes when
it ends and the reason still holds (`use_item` for one tick, `look_back` for backward).

## Start, recovery, catch-up, assist
- **Countdown**: the throttle goes down at countdown tick `COUNTDOWN_TICKS − 4…14` with the
  difficulty's perfect-start chance (5/20/40/60 %), else 5–21 ticks after GO; never early enough to
  stall (unit test over 4000 seeds; headless: no stalls).
- **Recovery** (`recovery.rs`, after a 1 s grace): slower than 1.5 u/s with throttle for 0.8 s,
  or < 3 u progress in 2.5 s → reverse 1.2 s (steering the nose toward the line); the third try in
  a row holds reset 0.6 s (rc_race respawns at 0.4 s). Off the course (beyond the legal margin,
  > 3 u below the surface, behind a barrier on a walled side, under a solid ribbon) for 1.5 s →
  hold reset. On the roof and slow for 0.5 s → reset tap (flip). < 12 u progress in 7 s → hold
  reset whatever the tries say (trapped).
- **Catch-up** (`catchup.rs`): rivals (`AiDriver` without `LocalPlayer`) get −sign(gap) × 6 % ×
  clamp((|gap| − 10 u) / 140 u) of the leading human's race distance, eased at 2 %/s; 0 in Time
  Trial, with `ai_catch_up` off, without humans, after finishing.
- **Steering assist** (`assist.rs`): steer = player + w × (line steer − player), w = 0.4 × (1 −
  0.6 |player|); 0.8 for 1.5 s after a respawn or while pointing > 46° away from the track.

## Data (`assets/data/ai.ron`, every struct `deny_unknown_fields`, ranges validated)
`difficulty.{rookie,pro,elite,elite_plus}` (speed_scale, corner_grip, mistakes_per_min,
reaction_secs, perfect_start, drift, line_spread_u, hoard), `neutral` + `personalities` by rival
id (hoard, boost eagerness; aggression and consistency come from `catalog.ron`), `driving`,
`traffic`, `charge`, `recovery`, `catch_up`, `items`, `assist`.

## Lap times
8 AI (the seven catalog rivals + one neutral driver) in Pro-class cars (Hornet / Torque),
3 laps from the grid (countdown start included), 3 seeds per row, items off. *Expected* = 3 ×
the track's reference lap (rc_track's reference car on its speed profile, flying). Respawns
are to-track respawns summed over the 3 races (72 racer-laps). test_oval and toy_town are the
tested layouts; the four environment tracks were mid-edit (2026-10-18 evening).

| Track | Difficulty | Reference lap (s) | Flying lap, median (s) | Best lap (s) | Race ÷ expected, mean | worst | Respawns | DNF |
|---|---|---|---|---|---|---|---|---|
| test_oval | Rookie | 10.8 | 12.30 | 11.62 | 1.211 | 1.265 | 0 | 0 |
| test_oval | Pro | 10.8 | 11.35 | 10.90 | 1.124 | 1.322 | 0 | 0 |
| test_oval | Elite | 10.8 | 10.72 | 10.40 | 1.060 | 1.109 | 0 | 0 |
| test_oval | Elite+ | 10.8 | 10.68 | 10.18 | 1.045 | 1.091 | 0 | 0 |
| toy_town | Rookie | 24.5 | 29.60 | 28.20 | 1.250 | 1.288 | 0 | 0 |
| toy_town | Pro | 24.5 | 27.32 | 25.98 | 1.164 | 1.244 | 0 | 0 |
| toy_town | Elite | 24.5 | 26.37 | 24.65 | 1.138 | 1.252 | 0 | 0 |
| toy_town | Elite+ | 24.5 | 26.40 | 24.35 | 1.131 | 1.275 | 1 | 0 |
| sofa_circuit | Rookie | 20.1 | 21.90 | 20.42 | 1.140 | 1.204 | 0 | 0 |
| sofa_circuit | Pro | 20.1 | 19.63 | 18.87 | 1.042 | 1.109 | 0 | 0 |
| sofa_circuit | Elite | 20.1 | 19.15 | 17.98 | 0.997 | 1.051 | 0 | 0 |
| sofa_circuit | Elite+ | 20.1 | 18.65 | 17.77 | 0.981 | 1.029 | 0 | 0 |
| backyard_bash | Rookie | 21.4 | 24.52 | 23.23 | 1.186 | 1.241 | 0 | 0 |
| backyard_bash | Pro | 21.4 | 22.17 | 21.20 | 1.081 | 1.191 | 0 | 0 |
| backyard_bash | Elite | 21.4 | 20.92 | 20.07 | 1.037 | 1.110 | 0 | 0 |
| backyard_bash | Elite+ | 21.4 | 20.68 | 19.87 | 1.008 | 1.089 | 0 | 0 |
| maple_street | Rookie | 34.9 | 39.77 | 37.80 | 1.180 | 1.230 | 4 | 0 |
| maple_street | Pro | 34.9 | 36.53 | 34.90 | 1.077 | 1.120 | 2 | 0 |
| maple_street | Elite | 34.9 | 34.52 | 32.73 | 1.044 | 1.149 | 8 | 0 |
| maple_street | Elite+ | 34.9 | 34.33 | 32.75 | 1.029 | 1.135 | 4 | 0 |
| wrench_works | Rookie | 28.0 | 31.20 | 30.15 | 1.164 | 1.304 | 2 | 0 |
| wrench_works | Pro | 28.0 | 28.68 | 27.82 | 1.079 | 1.274 | 2 | 0 |
| wrench_works | Elite | 28.0 | 27.15 | 26.30 | 1.015 | 1.180 | 2 | 0 |
| wrench_works | Elite+ | 28.0 | 26.85 | 26.07 | 1.009 | 1.150 | 3 | 0 |

## How to verify
- `cargo test -p rc_ai` — unit (43): data file, validation messages and debug hot reload; speed
  plan (curvature, corner speed, braking pass, airborne, brake point, star caps & acceleration);
  pure pursuit geometry, speed-sensitive steering, pedals; lane choice (clear road, faster car,
  sides, edge, blocked, hazards, alongside, rear-end); recovery state machine (stuck → reverse →
  reset, tries forgiven, flip tap, off course, trapped); item decision table, aggression and
  hoarding, reaction delay, cancelled use; catch-up target; assist blend; perfect-start
  statistics; drift zones across the start line; prop footprints (walls as circle rows) and no
  prop obstacle on toy_town's racing line.
  Headless (`tests/`, `rc_testkit::SimPlugins`, ≈ 15 s): 8 Rookie and 8 Elite AI finish 3 laps of
  test_oval within 1.5× with ≤ 2 respawns per lap and Elite laps < 0.92× Rookie's; 8 Pro AI on
  toy_town likewise; a seeded race (items on) replays bit for bit; perfect starts at Elite+
  without stalls; items are used and an item race finishes; the autopilot finishes a lap and ends
  the race; the assist changes only `steer`; catch-up slows rivals ahead and is 0 in Time Trial;
  a car put nose-first against a wall gets back into the race.
- `cargo test -p rc_ai --test race -- --ignored --nocapture every_layout` (QA, ≈ 2–4 min): every
  catalog track with a data file × forward/reverse/mirror, 8 Elite AI in Elite cars, 3 laps,
  DoD-2 limits. Snapshot 2026-10-18 23:30: all 24 variants of the 6 tracks with files pass (no
  DNF, worst racer 1.41×, at most 22 to-track respawns per 24 racer-laps on maple_street reverse
  mirror); aisle_rush has no file yet.
- `cargo test -p rc_ai --test race -- --ignored --nocapture lap_table` prints the table above;
  `probe` runs one race with `AI_TRACK`, `AI_DIFF`, `AI_CLASS`, `AI_N`, `AI_LAPS`, `AI_SEED`,
  `AI_REV=1`, `AI_MIRROR=1`, `AI_ITEMS=1`, `AI_TRACE=<racer>` (per-tick trace),
  `AI_TRACE_HAZARDS=1`.
- In the game: `cargo run -p recharged -- --user-data-dir target/agent-data/T11 --quick-race
  toy_town --ai 7 --autopilot --exit-after 90`.

## Known limitations
- Shortcuts are never taken (`ponytail:` in `scene.rs`; `Track::shortcuts` has the paths).
- Obstacles are steered around one at a time within 16 u: a dense prop maze would need real path
  planning. Prop footprints use the prop's horizontal extent (pitched/rolled colliders are treated
  as upright).
- The environment tracks (sofa_circuit, backyard_bash, maple_street, wrench_works) were being edited
  while these numbers were taken; rerun `every_layout` when they settle.
- One steering geometry for all cars (rc_ai cannot read vehicle params); pure pursuit absorbs it.
- No line-of-sight test for rockets / Leech Coil; incoming rockets are met with a shield only.
- With items on, races stay complete (0 DNF over 18 item races) but rougher: 0.2–0.33 to-track
  respawns per racer-lap on the short test_oval, 0.14–0.28 on toy_town, worst racers up to 1.4–1.7×
  the expected time — item hits and Spring Pad launches over walls (GDD §6 by design).
- Traffic costs Elite fields ≈ 1.5–2 s per toy_town lap against a solo car (conservative headway).
- The countdown press is counted in ticks against `COUNTDOWN_TICKS`; extra fixed ticks in a frame
  at GO shift it by a tick or two (still inside the window).
