# T12 — Items, pickups & status effects (`rc_items`)

**Goal:** the ten items of GDD §6 with fair, readable, telegraphed behaviour, and the single
authority over status effects.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §4.5 (messages), §5.7; GDD §5.5 (effects, stacking,
shield, immunity, speed cap), §6 (items & odds); `docs/design/rc_track.md`,
`docs/design/rc_vehicle.md`.

## Owned paths
`crates/rc_items/**`, `assets/data/items.ron`, `docs/design/rc_items.md`. **rc_core grant:**
additive edits in `crates/rc_core/src/items.rs` only.

## Deliverables
1. Decorate racers (`On<Add<Racer>>`): `ItemSlot`, `StatusEffects`.
2. **Effects system** (`SimSet::Effects`, single writer of `StatusEffects`): consume `ApplyEffect`
   from items and race; stacking (same kind: strongest magnitude, max duration; kinds multiply);
   shield blocks every hit-producing effect and pops (blocking hits in the same tick); owner
   immunity 1 s; timers in ticks.
3. **Pickups**: attempt entities spawned in `OnEnter(RaceAttempt)` from the track's item rows (4–5
   cubes per row), 2 s respawn, sensor/distance overlap → 1 s roulette → `ItemEvent::Ready`; odds
   from `items.ron` by the GDD §6.3 band formula over `Standings`, drawn with the keyed RNG.
4. **Items** (use edge from `CarControl.use_item`, backward with `look_back` where allowed): Turbo
   Cell, Spark Rocket (homing, turn-rate limit, `LockOn`/`LockLost` events), Leech Coil (target
   rules, LOS ray, charge transfer via `DrainCharge`/`GrantCharge`, tow effect), Shock Orb (follows
   the track via `Track`, wall bounces, 3 hits/10 s, never the owner), EMP Pulse (wind-up, radius,
   power loss + charge drain), Goo Bomb (attach, fuse, pass on contact with no-pass-back, explode
   incl. carrier, can't pass to shielded), Oil Slick, Spring Pad (launch impulse), Shield Bubble,
   Storm Strike (sequential strikes ahead). Projectiles are kinematic with shape casts and
   `Interpolated` (+ `Teleported` at spawn).
5. Outputs: `ApplyImpulse` (tumbles; heavier cars get scaled impulses), `HitEvent`, `Explosion`,
   `ItemEvent`; everything resets on Setup.
6. `items.ron`: odds table + tuning for every item (speeds, radii, durations).

## Tests
Unit: odds rows sum to 100 and band boundaries for N = 2…8; stacking/shield/immunity rules;
homing turn limit maths; leech target selection. Headless: rocket hits a stationary target and
tumbles it; shield blocks and pops; goo passes on contact and explodes after the fuse; oil slip;
spring pad launch; leech moves charge; orb follows the track and bounces; storm strike hits only
cars ahead; deterministic roulette for a fixed seed.

## Acceptance
Tests, clippy, fmt clean; `docs/design/rc_items.md` (tuning table).
