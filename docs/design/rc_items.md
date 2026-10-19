# rc_items — pickups, items & status effects (design note)

The ten items of GDD §6 and the single authority over `StatusEffects` (GDD §5.5). Code:
`crates/rc_items/src/` (`effects.rs` stacking & timers, `hits.rs` shields & shared geometry,
`pickups.rs` cubes/roulette/use, `odds.rs`, `data.rs`, `rocket.rs`, `orb.rs`, `leech.rs`,
`area.rs` EMP & Storm, `hazards.rs` oil/spring/goo/shield push). Data: `assets/data/items.ron`.

## Pipeline (FixedUpdate)
| Set | Systems (chained) |
|---|---|
| `SimSet::Effects` | `update_effects`: `ItemClock` +1 → timers count down → shields that blocked a hit last tick pop → new `ApplyEffect`s stack |
| `SimSet::Items` | `roulette` → `take_pickups`, `use_items` (Racing only) → items already out: `search_leeches`, `drain_leeches`, `pulse_emps`, `run_storms`, `home_rockets`, `fly_rockets`, `roll_orbs`, `zones`, `goo_bombs`, `shield_push` → this tick's uses: `self_items`, `launch_rockets`, `launch_orbs`, `start_leeches`, `start_area_items`, `drop_hazards`, `attach_goo` |

`OnEnter(RaceAttempt)`: one `Pickup` per item-row slot (if `RaceConfig.items`). `OnEnter(Setup)`:
`ItemClock` = 0 (all item entities are attempt-scoped). `OnEnter(Loading)` / `OnEnter(Unloading)`:
`ItemTuning` in / out (`LoadingDone("items")`, or `LoadingFailed` naming file and field). Debug
builds poll `items.ron` once a second and hot-reload it (a bad edit keeps the old tuning).

## Decisions
- **Hit detection runs in `SimSet::Items`**, not PostPhysics (ARCHITECTURE §3.2 lists it there):
  it reads the poses of the latest physics step, its impulses reach rc_vehicle in the same tick,
  and no rc_items system shares a set with rc_vehicle's PostPhysics systems (`DrainCharge` and
  `Transform` access would be schedule ambiguities — nondeterministic order).
- **One hit path** (`Hits::land`): a victim whose shield is up gets `HitEvent { blocked: true }` and
  nothing else; the effects system removes the shield of every blocked victim next tick, so every
  hit landing in the tick it pops is blocked as well, and a Shield Bubble used that same tick
  survives. Race effects (ghost, perfect start, stall) are not hits and never meet the shield.
- **Stacking**: one effect per kind; same kind keeps the magnitude farthest from 1 (power 0 beats
  0.5; tow 1.08 beats 1.05) and the longer duration; boosts keep the higher top speed (then force)
  with the longer duration; kinds coexist (the vehicle multiplies them).
- **Ticks, not clocks**: timers count down `SIM_DT` per tick and end below half a tick, so an
  effect of `secs` is in force `secs_to_ticks(secs)` ticks; item timers start the tick after use
  (systems for items already out run before this tick's uses): the Goo fuse is exactly 360 ticks,
  the EMP wind-up 24, the roulette 60.
- **Owner immunity** (1 s from `Born`) applies to rockets and dropped hazards; the orb never hits
  its owner; EMP and Storm never target the user; the Goo explosion hits its carrier.
- **Rockets** lock at launch (nearest car in the 35° cone and range, in sight, never the owner)
  and announce it (`LockOn`); the lock is kept only while the target stays in the cone (sharp
  turns break it), in range and in sight (`LockLost`, also when a locked rocket detonates). A
  rocket without a lock looks for one every tick by the same rules and announces it, so a lock
  lost behind a corner is re-acquired. They skim surfaces whose normal is within 45° of up (ramps,
  banks: the direction turns parallel) and detonate on anything steeper, on a car within 1.3 u of
  their path, or after 4 s; the blast (1.8 u) tumbles along the flight direction.
- **Line of sight for area hits** (`hits::exposed`): the rocket blast, the Goo explosion and the
  EMP pulse reach a car only when a ray from the blast to the car's centre meets no track or prop
  collider (no hits through walls, ribbons or props). A Storm Strike bolt comes down from 40 u
  above the car: a car under cover (an overpass, a table, the dollhouse) is spared and the next
  car ahead is struck in its place.
- **Shock Orb** lives in track coordinates (`s`, lateral): it runs along `Track::frame_at` at
  `speed × cos(launch angle ≤ 30°)` and bounces off the barriers: a ray across the track at the
  orb's height finds the track's wall collider within the racing width + 1.5 u on each side (an
  Asphalt curb puts the barrier 0.9 u beyond the width); `edge_margin` inside it, or inside the
  racing width's edge on a side without a barrier. It never leaves the track, glides over gaps and
  through props. Each car is knocked once, sideways across the track and up.
- **Leech Coil** "ahead" = ahead in `Standings`. The transfer moves `share × charge at attach`
  evenly over 3 s (180 equal `DrainCharge`/`GrantCharge` pairs, capped by what the target has);
  the tow is a `TopSpeed 1.08` refreshed every tick for 0.1 s. A target raising a shield breaks it.
- **Goo Bomb** passes on rapier `CollisionEvent::Started` between two racers (a car's collider is
  its racer entity): real bumps only, once per contact; ghosted cars cannot take it (no contacts).
  `Hazard.owner` becomes the passer (the explosion's attacker).
- **Dropped hazards** lie on the ground found by a ray below the drop point (nothing below — over
  a gap — nothing is dropped), rotated to its normal. A car triggers one by *entering*: its origin
  over the disc and within −1…+1.5 u of the surface; it re-arms when the car leaves the disc (cars
  jumping over do not trigger it; a launched car landing back on the pad does not either).
- **Storm Strike** hits the unfinished cars ahead in `Standings`, leader first, the first bolt one
  interval after the use (the `Used` event is the warning).
- **Odds**: band from `Standings`; a racer not in it (no ranking yet) draws from Middle. The roll is
  `rng::weighted(seed, ItemClock, ItemRoll, racer index)` when the roulette ends.
- **Not decorated here**: `Racer` already requires `ItemSlot` and `StatusEffects` (rc_core), so
  there is no `On<Add<Racer>>` observer.
- Entities for VFX/HUD/AI/audio: `Pickup` (+ `ItemTimer` = respawn, 0 = available); `Projectile`
  for rocket (`ItemTarget` = lock), orb, leech tether (at its owner, `ItemTarget` = target) and
  EMP wind-up (at its owner, `ItemTimer` = wind-up left); `Hazard` for oil, spring pad and goo
  (`ItemTarget` = carrier, `ItemTimer` = fuse). `ItemRadius` carries the reach to draw (hazard
  disc, orb knock, EMP pulse, goo and rocket blast) — rc_vfx cannot read `items.ron`, and what is
  drawn must be what triggers. All `Interpolated` + `Teleported` at spawn except the static cubes.

## Tuning (`assets/data/items.ron`)
Impulses are Δv of the 2.6-mass reference car (× `REFERENCE_CAR_MASS`); rc_vehicle scales them by
(2.6 / mass)^0.5, so the Δv a car feels is (2.6 / mass)^1.5 of it (Zip/Bolt 1.48×, Dumpster 0.67×).
`spin` is an angular impulse (reference inertia ≈ pitch 0.96, yaw 1.18, roll 0.33).

| Item / rule | Values | Why |
|---|---|---|
| Volt Cube | radius 1.5 u (car centre), respawn 2 s, roulette 1 s | GDD §6.1; covers the 2.25 u cube spacing |
| Owner immunity | 1.0 s | GDD §5.5 |
| Odds | GDD §6.3 rows, each 100 | validated at load |
| Turbo Cell | ×1.45 top / ×1.80 force, 2.5 s, freezes charge | GDD §5.5 |
| Spark Rocket | 55 u/s, cone 35°, lock 80 u (back 60 u), 120 °/s, body 0.3 u, hit 1.3 u, blast 1.8 u, life 4 s; tumble up 8 / push 6 u/s, spin 1.5 | GDD §6.2; up 8 = 0.64 s air, ≈ 1.5 s lost with recovery |
| Leech Coil | ahead 40 u, near 20 u, wait 3 s, tether 3 s, share 30 %, tow ×1.08, breaks > 45 u | GDD §6.2 (10 %/s) |
| Shock Orb | 24 u/s, radius 3 u, 10 s / 3 hits, 0.7 u high, edge margin 0.5 u (off a barrier or the width's edge), launch ≤ 30°; knock up 6 / sideways 7 u/s, spin 1 | GDD §6.2 |
| EMP Pulse | wind-up 0.4 s, 12 u, power 0 for 2 s, −50 % charge | GDD §6.2 |
| Goo Bomb | fuse 6 s, no pass-back 0.5 s, 7 u; tumble up 7 / push 5 u/s, spin 1.5 | GDD §6.2; apex ≈ 1 u, under the 1.2 u barriers (up 9 threw cars over them) |
| Oil Slick | radius 2.5 u, 15 s, grip 0.2 for 1.2 s, yaw spin 2.5 (≈ 2 rad/s) | GDD §6.2; 15 s (was 25): a slick caught ≈ 3 cars (QA) |
| Spring Pad | radius 1.6 u, 15 s, up 8 / forward 2 u/s, random spin ≤ 0.3 per axis | GDD §6.2; up 8 = 0.64 s air: up 12 (0.96 s) launched cars over the barriers, the largest source of to-track respawns in item races |
| Shield Bubble | 10 s, bump push 6 u/s (+1.5 up) | GDD §6.2 |
| Storm Strike | every 0.25 s, power 0 for 1.5 s, hop 4 u/s | GDD §6.2 |

## How to verify
- `cargo test -p rc_items` — unit: odds rows & bands for N = 2…8 (integer maths vs the float
  formula), keyed roll, data validation messages; stacking, boosts, shield/ghost durations, tick
  counting; immunity window, tumble direction; homing turn limit (90° in 0.75 s), lock target
  (cone, range, sight, backward), segment distance; orb bounce; leech target choice; storm
  targets; goo pass rules; odds band fallback. Headless (`tests/items.rs`, test_oval, box cars;
  `TestVehicle` stands in for rc_vehicle's impulse/charge handling): rocket locks, hits a parked
  car and tumbles it; backward rocket; unlocked rocket explodes on the wall (its blast spares a car
  behind the wall); a rocket that lost its lock re-locks onto a car that comes into its cone
  (`LockOn` again); shield blocks and pops; two EMPs in one tick both blocked, EMP power loss +
  drain, 0.4 s wind-up, no EMP hit through a wall; goo passes on a bump and explodes (carrier and
  passer hit, exactly 6 s); oil slip + spin, owner spared; spring launch; leech moves 30 % and
  tows, breaks out of range; orb follows the track round turn 1 and bounces off the barrier (and
  the open edge); orb knocks sideways, never its owner; storm hits only cars ahead, 0.25 s apart,
  and spares a car under cover; seeded roulette
  is deterministic (1 s, band-legal items, held items never replaced, cubes back after 2 s);
  restart clears every item entity and respawns the cubes; rc_race's effects (perfect start
  60 ticks, stall 36, respawn ghost 90) stack by the rules, a Turbo Cell supersedes the perfect
  start, and a shield never meets them.
- In the game: `RUST_LOG=info,rc_items=debug cargo run -p recharged -- --user-data-dir
  target/agent-data/T12 --quick-race test_oval --exit-after 45` logs the cube count, every item
  granted (with its odds band), used and every hit. Item visuals are rc_vfx's (T14).

## Item race numbers (QA setup, T19a)
Six headless toy_town races (`--headless-race --quick-race toy_town --ai 7 --seed 1…6`, logs
through `tools/scripts/qa/item_stats.py`): used → hits (self-hits), and to-track respawns.

| | Goo Bomb | Oil Slick | Spring Pad | Storm Strike | Spark Rocket | To-track respawns |
|---|---|---|---|---|---|---|
| QA, before T19a | 15 used, 16 self | 18 → 62 | 12 → 33 | 7 → 40 | 54 → 27 | — |
| T19a, before the Q6 tuning | 18 → 22 (18 self) | 21 → 58 | 22 → 61 | 10 → 45 | 43 → 26 | 36 |
| T19a final | 12 → 20 (10 self) | 18 → 38 | 15 → 24 | 7 → 30 | 60 → 38 | 26 |

## Known limitations
- The orb passes through props. Line-of-sight rays go from centre to centre: a car whose centre is
  hidden behind a low wall is spared even when its roof shows.
- Hit tests use the car centre (origin + 0.4 u along its up) as a point, not its box.
- Goo Bombs still mostly go off on their carrier: the AI hunts a bump only within 15 u ahead, and
  fields spread out over a lap.
- Before rc_race's `Standings` exist every racer rolls from the Middle row, the Leech Coil only uses
  its 20 u any-direction rule and Storm Strike hits nobody.
