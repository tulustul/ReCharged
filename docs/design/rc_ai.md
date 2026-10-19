# rc_ai — AI drivers, autopilot, steering assist (design note, T11, T19a)

Seven rivals who race the rc_vehicle car model cleanly and fast at Elite, beatable at Rookie, use
items, and never stay stuck; the same driver runs autopilot humans. Code: `crates/rc_ai/src/`
(`driver.rs` per-car pipeline, `pace.rs` speed & mistakes, `profile.rs` speed plan, `steering.rs`
pure pursuit & pedals, `traffic.rs` lanes, `scene.rs` what drivers see, `charge.rs` drift & boost,
`items.rs` item decisions, `recovery.rs` stuck handling, `shortcut.rs`, `catchup.rs`, `assist.rs`,
`brain.rs`, `data.rs`). Data: `assets/data/ai.ron` (hot-reloaded in debug builds, ≤ 1 s).

## Pipeline (FixedUpdate)
| Set | Systems (chained) |
|---|---|
| `SimSet::Drivers` | `observe_racers` (track coordinates of every racer and how long it has crawled since GO: `TrackView`) → `ensure_brains` (an `AiBrain` + `CatchUp` for every new `AiDriver`; re-plans on a tuning hot reload) → `update_catch_up` → `drive_ai` (`CarControl` of every `AiDriver`, autopilot humans included) |
| `SimSet::Assist` | `steering_assist`: `steer`, and lowers `throttle` / raises `brake` (auto-brake), `LocalPlayer` without `AiDriver`, `Settings.accessibility.steering_assist` on, Racing only |

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
  the car already is on, else the other side, else followed (the follow brake uses that car's own
  speed and distance). Cars alongside (±3 u along the track, beside the car — one nose to tail is
  ahead or behind, not alongside) bound the lateral range (never squeeze into a car). Never drive
  into a slower moving car that overlaps the *current* lateral: close to 3 u + 0.25 s × v behind
  it; within the ±3 u alongside window only when the bodies would touch (two cars side by side
  held each other up). Well off its line with a free lane of its own, a car does not follow a car
  that only blocks the planned line: it holds its lane. Lane changes move at 4 u/s.
- **Stopped cars** (GDD §8.1, T22 R2): a racer slower than 2 u/s along the track for 0.5 s of the
  race (counted from GO), and not driven by the AI, is stopped — a player who missed GO or
  stopped. AI cars never count: their crawl is a queue and a crash recovers by itself within
  seconds (dodging recovering AI cars at speed raised item-race respawns by half on wrench_works).
  A stopped car is seen 2 s ahead and steered around like a hazard — at once, on the free side,
  else squeezed past on the roomier side, never followed; cars still behind it do not close a side
  (they are queued or passing too), and around it a car follows only a car whose body is in its
  way (two cars beside the stopped one must not wait for each other). It brakes as late as the
  brakes allow (28 u/s²): to stop 7 u behind the stopped car while the way round is closed (room
  to pull out at full lock), to 2.5 u when its heading already carries it clear, creeping on at
  2 u/s while it turns out from a standstill; once beside it, it drives on. A slow dodge aims 2 u
  ahead (full lock); boxed in closer than 6 u, slower than 1 u/s, with the way round open and room
  behind, a car backs off 0.6 s (once until it makes progress again; not a stuck try). Before, the
  follow brake held a car at the parked car's speed — 0 — while the cars beside vetoed every side,
  so the field drained one car at a time (T21: 5 of 7 rivals held 3–8 s, the last ≈ 20 s).
- **Hazards and props**: rc_items' Oil Slick / Spring Pad discs and Shock Orbs (reach from
  `ItemRadius`, + 0.6 u) are obstacles, seen 0.8 s ahead at the car's speed (at least the 16 u
  traffic look-ahead: at 30 u/s 16 u left a dodge 0.5 s and cars ran over a slick 3–4 times per
  drop), and so are static, non-drivable props in the legal
  corridor whose colliders reach into the car's height band (0.15–1.3 u above the surface; higher
  parts are driven under): flower-pot chicanes, chair legs, tyre stacks, garden edging. Each
  collider becomes footprint circles — boxes a row of small circles along their long side, so a
  wall flanking a tunnel never closes it — computed once per track. A dodge is applied at once (a
  rate-limited one could not clear a pad lying 6 u past a respawn point: the car was launched over
  the wall on every respawn). A car well off its line (> 1.5 u) also clears obstacles on its own
  path back (backyard_bash's garden edging across the lawn margin trapped cars for 7 s a lap).
  The side to pass on follows last tick's target, not the car's own lateral (a car nearly in line
  with a pad flipped sides every tick and hit it), and a side is free only when no hazard anywhere
  ahead is on it (passing a car must not lead onto the Spring Pad it hid). Cars carrying a Goo
  Bomb get no extra berth (`goo_clearance_u` is unused): the carrier needs the bump. Dynamic props
  are ignored (the chassis knocks them away); drivable ones (ramps, box tops) are driven.
- **Shortcuts** (`shortcut.rs`, B16): each brain plans `Track::shortcuts` once for its car — the
  path (a Catmull-Rom through the points with the authored end directions, joining the racing
  line 12 u before and after) driven with the car's acceleration and braking from the same entry
  speed to the same exit speed, against the racing line over the same span. A shortcut that saves
  ≥ 3 %, is ≥ 5 u wide and has no knockable prop within 0.5 u of its width is planned. 40 u before
  its entry a driver decides per lap (keyed RNG) with chance [0, 0.3, 0.5, 0.6] (Rookie … Elite+)
  × (0.5 + aggression); on it the planned line, speed plan, surface and the legal corridor
  (± its half width − 1 u, barriers and ribbon checks off) come from the plan, no drifting; 3 u
  outside the corridor drops it, a respawn never starts one. Shipped tracks (measured): no
  shortcut is planned — test_oval's, maple_street's, aisle_rush's and sofa_circuit's are slower or
  too narrow for every car, toy_town's U-turn cut is blocked by its knockable block gates (forced,
  it gained nothing on average).
- **Jumps** (`Track::jumps`, 25 u before the lip to 10 u after the landing): traffic lane changes
  ≤ 0.8 u and hazard dodges ≤ 3 u (ramps and gaps have no barriers), no traffic speed limits
  (too little speed drops the car into the gap: on test_oval one slow car under the landing ramp
  made the whole field queue and fall in), no hazards dropped there, and on the run-up (lip within
  45 u) the throttle never aims below 0.9× the jump's reference takeoff speed, whatever the
  difficulty scale (rc_track guarantees landings from 0.8×).
- **Mistakes** (GDD §8.2): every 0.5 s, chance `mistakes_per_min` × (1.5 − consistency):
  late braking (plan × 1.08–1.16) or a wide line (1–2.2 u outward, corners only), 1–2 s.

## Charge (GDD §5.3)
- **Drift zones** (per brain): racing-line radius < 32 u, planned speed ≥ 50 % top, bank ≤ 10° (on
  toy_town's raised 20° turns an over-rotating drift slid into the lip and stopped the car), ≥ 1.2 s
  long, ending 0.5 s before the corner does (the catch, ≈ 0.5 s, happens inside it). A zone is
  drifted with the difficulty's `drift` chance, from stable grip driving only (4 wheels down, slip <
  8°, within 1.5 u of its line, battery < 70 %). The drift ends when it does not charge after 0.7 s
  (`VehicleTelemetry::charge_rate`), the pursuit asks for > 0.35 counter-steer (too tight), the car
  spins (> 40°), slows below the zone speed or gets within 0.5 u of its normal line's bound (the
  drift's line keeps 1 u more room: a car turned 30° reaches the barrier with its corner; room
  measured to that tighter bound ran out at every apex, and the median drift ended after 0.15 s,
  before charging). Three in four drifts still end within 0.6 s (median 0.43 s, 8 AI on 5 tracks):
  the car tightens past its drift line toward the apex. After a drift the throttle is eased (0.4)
  until the slide is caught. Measured (solo Hornet, toy_town): drifting costs ≈ 0.5 s per lap (4WD
  cars scrub speed in a drift) that the charge partly earns back on the next straight; before these
  entry/exit rules a wall-scraping helix drift cost 2–5 s per lap.
- **Boost**: from `BOOST_MIN_CHARGE` + (1 − eagerness) × 0.2 when the plan stays above the boosted
  top speed for `boost_straight_secs` × (1.5 − eagerness) (integrated with the car's Accel-star
  acceleration), all wheels down, not drifting, > 50 % top; held until braking is due.

## Items (GDD §8.1, `items.rs`)
| Item | Used when |
|---|---|
| Spark Rocket | a car in the 30° cone ahead within 70 u (forward), else one behind within 40 u (backward), in sight from the muzzle |
| Leech Coil | the car directly ahead in `Standings` within 38 u, in sight |
| Shock Orb | a car ahead along the track within 45 u |
| EMP Pulse | a car within 10 u |
| Goo Bomb | an unshielded car in bump reach (alongside to 15 u ahead, within 4 u to the side); the carrier then aims at the nearest one in reach (else the nearest ahead within `goo_seek_u`) at 1.1× its plan, no avoidance, no follow limit |
| Oil Slick / Spring Pad | a car within 14 u behind on the line (never on a jump approach); leaders keep one to defend, others drop it after `idle` |
| Shield Bubble | a rocket is locked on (a `Projectile` whose `ItemTarget` is this car), or after `idle` |
| Turbo Cell | ≥ 1.5 s × (1 + hoard) of straight ahead, not boosting |
| Storm Strike | not leading |

**Line of sight** (B16): holding a Spark Rocket, a cone target counts only when a ray from the
rocket's muzzle (1.4 u ahead of / behind the car's centre) to the target's centre meets no track
or prop collider (rc_items' lock filter); holding a Leech Coil, the car ahead counts only in
sight from the car's centre. Rays are cast only for candidates nearer than the best so far.

`idle` = 4 s + hoard × 20 s. Ranges × (0.75 … 1.2 by aggression) × (1 − 0.35 × hoard): Mayhem
fires early, Nova waits for surer shots and keeps defensive items. A wanted use starts the
difficulty's reaction delay (Rookie 0.8–1.5 s … Elite+ 0.15–0.3 s, keyed RNG); the item goes when
it ends and the reason still holds (`use_item` for one tick, `look_back` for backward).

## Start, recovery, catch-up, assist
- **Countdown**: the throttle goes down at countdown tick `COUNTDOWN_TICKS − 4…14` with the
  difficulty's perfect-start chance (5/20/40/60 %), else 5–21 ticks after GO; never early enough to
  stall (unit test over 4000 seeds; headless: no stalls).
- **Recovery** (`recovery.rs`, after a 1 s grace): slower than 1.5 u/s with throttle for 0.8 s,
  or < 3 u progress in 2.5 s → reverse 1.2 s (steering the nose toward the line); the second try
  in a row holds reset 0.6 s (rc_race respawns at 0.4 s; with a third try, reverse-and-retry
  loops cost Elite autopilot runs 5–10 s). Off the course (beyond the legal margin, > 3 u below
  the surface, behind a barrier on a walled side, under a solid ribbon, or the next gate to
  validate > 6 u behind: a gate missed after a slide or a shove) for 1.5 s → hold reset — rc_race
  puts the car back before the gate instead of the car driving a whole extra lap. On the roof and
  slow for 0.5 s → reset tap (flip). < 12 u progress in 7 s → hold reset whatever the tries say
  (trapped). Without drive power (`power_mul` < 0.5: EMP, Storm Strike) the car is never stuck and
  its progress windows start over.
- **Catch-up** (`catchup.rs`): rivals (`AiDriver` without `LocalPlayer`) get −sign(gap) × 6 % ×
  clamp((|gap| − 10 u) / 140 u) of the leading human's race distance, eased at 2 %/s; 0 in Time
  Trial, with `ai_catch_up` off, without humans, after finishing.
- **Steering assist** (`assist.rs`): steer = player + w × (line steer − player), w = max(0.4,
  0.9 × (1 − |player| / 0.25)) × (1 − 0.6 |player|): hands off it steers the line almost fully (a
  throttle-only player understeered off every corner at 0.4), and hands control back as the input
  grows; 0.8 for 1.5 s after a respawn or while pointing > 46° away from the track.
  **Throttle-only pedals** (Q7, T19a round 2): after 2 s without steering (|steer| > 0.1) or
  braking (> 0.05) — from GO for a player who never touches them — the assist also drives the
  pedals; any steering or braking hands them back for the next 2 s, so a player who steers and
  brakes is never slowed. A speed plan per assisted car (built on its first assisted tick: the
  car's stars, 0.85 of its cornering grip, braking at 14 u/s² — the 0.4 brake of a 40 u/s² car,
  less a margin) gives the brake-point speed ahead at **0.73× pace** (weather included; on a jump
  run-up never below 0.9× the takeoff speed), capped at 0.73× the car's top speed on the surface
  (not while boosting). Over it the throttle lifts over 1 u/s and the brake ramps to at most 0.4
  over the next 2 u/s; speed only ever goes down. At full pace throttle-only play earned Silver
  medals; GDD medals start at 1.15× the Elite reference (Bronze). Measured (`assist_probe`,
  throttle only = auto-accelerate, 3 laps, against `medals.ron`):

  | Layout | Reference (s) | Hornet | Phantom |
  |---|---|---|---|
  | sofa_circuit | 53.78 | 69.06 s (1.28×) | 67.69 s (1.26×) |
  | sofa_circuit rev | 54.32 | 69.22 s (1.27×) | 67.76 s (1.25×) |
  | backyard_bash | 59.87 | 74.36 s (1.24×) | 72.49 s (1.21×) |
  | backyard_bash rev | 61.03 | 74.80 s (1.23×) | 74.04 s (1.21×) |
  | toy_town | 72.37 | 93.55 s (1.29×) | 91.37 s (1.26×) |
  | toy_town rev | 72.28 | 93.15 s (1.29×) | 91.15 s (1.26×) |

  0 respawns and no "Hold Reset" in every run; also 0 respawns for the Hornet on the other four
  tracks both ways and for Sparky, Zip, Dumpster, Torque and Volt GT on these three (at 0.74×).
  Before (full pace, round 1): sofa_circuit 3 laps 55.94 s in a Hornet (1.04×, Silver).

## Data (`assets/data/ai.ron`, every struct `deny_unknown_fields`, ranges validated)
`difficulty.{rookie,pro,elite,elite_plus}` (speed_scale, corner_grip, mistakes_per_min,
reaction_secs, perfect_start, drift, line_spread_u, hoard), `neutral` + `personalities` by rival id
(hoard, boost eagerness; aggression and consistency come from `catalog.ron`), `driving`, `traffic`,
`charge`, `recovery`, `catch_up`, `items`, `assist`. T19a changed values only (the format stays
readable by older builds): `recovery.reset_after_tries` 2 → 1; `traffic.goo_clearance_u` is unused.
The shortcut chances are a code table (`ponytail:` in `shortcut.rs`).

## Lap times
8 AI (the seven catalog rivals + one neutral driver) in Pro-class cars (Hornet / Torque),
3 laps from the grid (countdown start included), 3 seeds per row, items off. *Expected* = 3 ×
the track's reference lap (rc_track's reference car on its speed profile, flying). Respawns
are to-track respawns summed over the 3 races (72 racer-laps). Snapshot 2026-10-19 (T19a), all
seven tracks; toy_town and backyard_bash re-measured after their round-2 data changes.

| Track | Difficulty | Reference lap (s) | Flying lap, median (s) | Best lap (s) | Race ÷ expected, mean | worst | Respawns | DNF |
|---|---|---|---|---|---|---|---|---|
| test_oval | Rookie | 11.0 | 12.27 | 11.60 | 1.180 | 1.235 | 0 | 0 |
| test_oval | Pro | 11.0 | 11.20 | 10.72 | 1.080 | 1.132 | 0 | 0 |
| test_oval | Elite | 11.0 | 10.50 | 10.10 | 1.016 | 1.068 | 0 | 0 |
| test_oval | Elite+ | 11.0 | 10.35 | 9.95 | 1.002 | 1.110 | 0 | 0 |
| toy_town | Rookie | 24.8 | 29.12 | 27.77 | 1.215 | 1.255 | 0 | 0 |
| toy_town | Pro | 24.8 | 26.58 | 25.72 | 1.108 | 1.146 | 0 | 0 |
| toy_town | Elite | 24.8 | 25.13 | 24.15 | 1.050 | 1.089 | 0 | 0 |
| toy_town | Elite+ | 24.8 | 24.72 | 23.57 | 1.054 | 1.148 | 2 | 0 |
| sofa_circuit | Rookie | 20.1 | 22.20 | 20.42 | 1.143 | 1.205 | 0 | 0 |
| sofa_circuit | Pro | 20.1 | 19.78 | 19.05 | 1.045 | 1.108 | 0 | 0 |
| sofa_circuit | Elite | 20.1 | 19.10 | 17.98 | 0.996 | 1.052 | 0 | 0 |
| sofa_circuit | Elite+ | 20.1 | 18.80 | 18.32 | 0.991 | 1.050 | 0 | 0 |
| backyard_bash | Rookie | 21.3 | 24.15 | 23.05 | 1.186 | 1.240 | 0 | 0 |
| backyard_bash | Pro | 21.3 | 22.02 | 21.07 | 1.079 | 1.130 | 0 | 0 |
| backyard_bash | Elite | 21.3 | 20.58 | 19.90 | 1.030 | 1.095 | 0 | 0 |
| backyard_bash | Elite+ | 21.3 | 20.72 | 19.70 | 1.006 | 1.071 | 0 | 0 |
| maple_street | Rookie | 34.7 | 39.32 | 37.28 | 1.156 | 1.222 | 0 | 0 |
| maple_street | Pro | 34.7 | 36.35 | 34.38 | 1.072 | 1.121 | 1 | 0 |
| maple_street | Elite | 34.7 | 34.32 | 32.75 | 1.027 | 1.100 | 1 | 0 |
| maple_street | Elite+ | 34.7 | 34.17 | 32.28 | 1.009 | 1.079 | 1 | 0 |
| wrench_works | Rookie | 28.0 | 31.30 | 30.12 | 1.155 | 1.236 | 0 | 0 |
| wrench_works | Pro | 28.0 | 28.62 | 27.72 | 1.073 | 1.139 | 1 | 0 |
| wrench_works | Elite | 28.0 | 27.15 | 26.30 | 1.006 | 1.045 | 0 | 0 |
| wrench_works | Elite+ | 28.0 | 26.83 | 26.07 | 0.999 | 1.072 | 1 | 0 |
| aisle_rush | Rookie | 30.9 | 35.22 | 34.33 | 1.175 | 1.211 | 0 | 0 |
| aisle_rush | Pro | 30.9 | 32.28 | 31.82 | 1.085 | 1.120 | 0 | 0 |
| aisle_rush | Elite | 30.9 | 30.75 | 30.13 | 1.028 | 1.056 | 0 | 0 |
| aisle_rush | Elite+ | 30.9 | 30.25 | 29.82 | 1.021 | 1.057 | 0 | 0 |

## How to verify
- `cargo test -p rc_ai` — unit (54): data file, validation messages and debug hot reload; speed
  plan (curvature, corner speed, braking pass, airborne, brake point, star caps & acceleration);
  pure pursuit geometry, speed-sensitive steering, pedals; lane choice (clear road, faster car,
  sides, edge, blocked, hazards, alongside, rear-end); recovery state machine (stuck → reverse →
  reset, tries forgiven, flip tap, off course, trapped); item decision table, aggression and
  hoarding, reaction delay, cancelled use, Goo Bomb only with a bump in reach; catch-up target;
  assist blend, auto-brake pedals and the hands-off window; perfect-start statistics; drift zones across the start line;
  prop footprints (walls as circle rows) and no prop obstacle on toy_town's racing line; a hazard
  beyond a car decides the passing side; a stopped car is steered around, never followed (a car
  behind it does not close a side, one beside it does: squeeze), braked for as late as the brakes
  allow and never pushed, and a car side by side holds nobody up; a car boxed in behind a stopped
  car backs off once, then the stuck detectors take over; a powerless car is not stuck; shortcuts (traversal time;
  test_oval's and toy_town's not planned, a synthetic bulge's chord planned and followed, a narrow
  one never; the chance table; one decision per approach, kept through the cut, dropped after it
  or when pushed off its corridor).
  Headless (`tests/`, `rc_testkit::SimPlugins`, ≈ 15 s): 8 Rookie and 8 Elite AI finish 3 laps of
  test_oval within 1.5× with ≤ 2 respawns per lap and Elite laps < 0.92× Rookie's; 8 Pro AI on
  toy_town likewise; a seeded race (items on) replays bit for bit; perfect starts at Elite+
  without stalls; items are used and an item race finishes; the autopilot finishes a lap and ends
  the race; the assist steers toward the line and keeps the pedals on a straight, lifts and brakes
  gently (≤ 0.4) into test_oval's turn 1 for throttle-only play, leaves the pedals to a player who
  steers, and changes nothing when off; an Elite AI alone on test_oval earns charge from its
  drifts; catch-up slows rivals ahead
  and is 0 in Time Trial; a car put nose-first against a wall gets back into the race; rockets are
  fired only at cars in sight (not through a wall); a car that missed a gate holds reset and is put
  back before it; every one of 7 Pro AI passes a car parked on the racing line on pole within 3 s
  of reaching it, on toy_town and test_oval (`tests/race.rs`).
- `cargo test -p rc_ai --test race -- --ignored --nocapture every_layout` (QA, ≈ 2–4 min): every
  catalog track with a data file × forward/reverse/mirror, 8 Elite AI in Elite cars, 3 laps, DoD-2
  limits. Snapshot 2026-10-20 (drift edge-room fix): all 28 pass (no DNF, worst racer 1.24× on
  test_oval forward, at most 2 to-track respawns per car and 3 per 24 racer-laps; 25 respawns in
  all, 17 without the fix, and Elite fields 0.4 % slower on average: the AI's drifts now last).
  2026-10-19 (T22): all 28 variants of the 7 tracks pass (no DNF, worst racer 1.20× on backyard_bash
  reverse mirror, at most 2 to-track respawns per car and 6 per 24 racer-laps, on maple_street;
  T19a: worst 1.43× on test_oval reverse mirror; before T19a QA saw wrench_works fail).
- `cargo test -p rc_ai --test race -- --ignored --nocapture lap_table` prints the table above;
  `probe` runs one race with `AI_TRACK`, `AI_DIFF`, `AI_CLASS`, `AI_N`, `AI_LAPS`, `AI_SEED`,
  `AI_REV=1`, `AI_MIRROR=1`, `AI_ITEMS=1`, `AI_TT=1` (the calibration Time Trial instead),
  `AI_TRACE=<racer>` (per-tick trace every `AI_TRACE_EVERY` ticks), `AI_TRACE_HAZARDS=1`,
  `AI_TRACE_PROPS=1` (dynamic props near the traced car), `AI_TRACE_HITS=1` (item uses, hits,
  impacts, missed gates and respawn causes); `tt_table` runs `recharged --calibrate-medals`'s Time
  Trials (`AI_TRACKS`, `AI_SEEDS`) and flags runs > 3 s over the layout's median.
  `tests/behaviour.rs assist_probe` drives a throttle-only assisted player (`AI_TRACK`, `AI_LAPS`,
  `AI_REV`, `AI_MIRROR`, `AI_CAR`, `AI_TRACE`). `parked_probe` runs the parked-car race
  (`AI_TRACK`, `AI_SEED`, `AI_PARK_AHEAD` = u past the pole slot, `AI_TRACE_ALL=1` for every AI's
  distance, lateral, speed and recovery every 0.2 s, `AI_TRACE=<racer>`) and prints each AI's
  seconds from reaching the parked car to getting past it. Measured (T22, 6 seeds per row, worst
  AI): parked on the racing line on pole — toy_town ≤ 2.3 s, test_oval ≤ 1.5 s, backyard_bash
  ≤ 2.6 s, sofa_circuit ≤ 1.1 s in 3 seeds and 4.4–8.2 s in 3; parked 30 u past the pole (the
  field arrives at speed) — toy_town ≤ 1.4 s, sofa_circuit ≤ 2.7 s, backyard_bash ≤ 2.1 s,
  test_oval ≤ 3.4 s but one 6.3 s. Before (T21 AI): the car right behind took 8.8–9.2 s on
  toy_town and test_oval; with the car on the grid slot itself several never got past in 20 s.
- In the game: `cargo run -p recharged -- --user-data-dir target/agent-data/T11 --quick-race
  toy_town --ai 7 --autopilot --exit-after 90`.

## Known limitations
- No shipped shortcut is faster for any car (see Driving): the planner is exercised by unit
  tests on a synthetic track only.
- Obstacles are steered around one at a time within 16 u: a dense prop maze would need real path
  planning. Prop footprints use the prop's horizontal extent (pitched/rolled colliders are treated
  as upright).
- One steering geometry for all cars (rc_ai cannot read vehicle params); pure pursuit absorbs it.
- Incoming rockets are met with a shield only, never a dodge. Line of sight is centre to centre.
- With items on, races stay complete (0 DNF over 48 item races: test_oval and toy_town, 4
  difficulties, 6 seeds) with 0.050 (test_oval) and 0.054 (toy_town) to-track respawns per
  racer-lap (B17, T19a; before: 0.229 and 0.260 over 3 seeds — Spring Pads throwing cars over
  barriers, dodges hiding pads behind cars and flipping sides, EMP-stopped cars counted as stuck,
  Goo tumbles over barriers); worst racers up to 1.5× the expected time.
- Elite autopilot Time Trials (`--calibrate-medals`, 28 layouts × 3 seeds): 6 runs more than 3 s
  over their layout's median, worst + 8 s (sofa_circuit, backyard_bash, wrench_works and
  maple_street mirrored or reversed layouts; before T19a 3, worst + 20 s) — slides into a barrier
  after a hop on a kerbed corner or a bump. toy_town has none since its raised turns' ends are half
  banked (8 seeds × 4 layouts all within 71.5–75.4 s; before: medians up to 81.1 s, 4 outliers).
- Traffic costs Elite fields ≈ 1.5–2 s per toy_town lap against a solo car (conservative headway).
- Stopped cars (R2): two cars in line behind a stopped car, one on each side of it, can hold each
  other for 4–8 s (the lane choice flips between them) — measured only with the stopped car on
  pole, which the T22 grid rules out in the game (the players start behind the AI field). A
  stopped AI car is followed, not dodged (its own recovery frees it within seconds).
- The countdown press is counted in ticks against `COUNTDOWN_TICKS`; extra fixed ticks in a frame
  at GO shift it by a tick or two (still inside the window).
