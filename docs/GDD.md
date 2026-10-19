# ReCharged — Game Design Document

> *Small cars. Big world. Full charge.*

| | |
|---|---|
| Status | **v2.1 — APPROVED** (reviews 1 & 2 in `docs/reviews/`); v2.2 post-QA tuning: Toy Town and Wrench Works default to 4 laps (§7.1); Oil Slick / Spring Pad last 15 s, softer pad launch, Goo Bomb near-miss pass (§6.2); steering assist brakes before corners, relaxed throttle-only pace (§15); v2.3 post-final-QA: local players start behind the AI field (§3.2), Oil Slick / Spring Pad are used up after 3 cars (§6.2), AI steer around stopped cars (§8.1) |
| Owner | Game designer / orchestrator |
| Engine | Bevy 0.20, bevy-react 0.8.1 (UI), bevy_rapier3d 0.37 (physics), bevy_kira_audio 0.27 (audio) |
| Companion docs | `docs/ARCHITECTURE.md` (technical design), `docs/PRACTICES.md` (engineering rules) |

Requirement IDs (`[VEH-3]`, `[UI-12]` …) are referenced by implementation tasks and by QA.

---

## 1. Vision

**ReCharged** is an arcade combat racer for electric RC cars tearing through giant everyday
worlds — a living room, a back garden, a toy store, a supermarket after closing, a suburban
street, an auto-repair workshop. It is *inspired by* Re-Volt (RC scale, household environments,
weapon pickups, reverse/mirror tracks) but is not a clone: it has its own cars, tracks and item
set built around a **battery-charge** mechanic, dynamic weather and time of day, a modern UI and
couch split-screen for up to four players.

### 1.1 Pillars
1. **Big world, small car.** Scale is the star. Everyday objects become cliffs, ramps,
   tunnels and obstacles. The camera sits low; props tower over the car.
2. **Toy physics that feel great.** Light, bouncy, responsive and forgiving. Big air, quick
   self-righting, drifts that charge your battery. Fun within five seconds of picking it up.
3. **Chaos with fairness.** Weapons create comebacks; skill still wins. Position-weighted item
   odds, telegraphed attacks (lock-on warnings), nothing truly unavoidable, no long stuns.
4. **Couch first.** One to four local players, any mix of keyboard and gamepads. Every menu
   works with a gamepad alone.
5. **Modern polish.** Fast loads, **instant restart** (< 0.5 s, no reload), a readable HUD,
   juicy feedback (particles, sound, rumble), settings that apply live, onboarding hints,
   accessibility options.

### 1.2 What we keep from Re-Volt / what we change
| Keep (in spirit) | Change / add |
|---|---|
| RC cars in household & neighbourhood environments | Own car roster and track layouts; an auto-repair workshop track Re-Volt never had |
| Pickups that grant a random weapon | Charge-themed items (Leech Coil, Spring Pad, Turbo Cell), backward fire, shield counterplay, lock-on telegraphs, position-weighted odds |
| Bouncy, flippable toy cars, "flip car" key | Auto self-righting + smart respawn (tap = flip, hold = respawn) |
| Reverse and mirror track variants | Direction-specific track features; dynamic weather & time of day; wet/snow grip |
| Championship cups that unlock content | Medals, collectibles (Golden Batteries), paints, records, ghosts, cup podium ceremony |
| Split-screen | Four players, device-join lobby, per-player HUD, session scoreboard |

### 1.3 Target
- **Platform:** desktop PC (Linux first; Windows/macOS should work). Keyboard and gamepads
  (Xbox/PlayStation/generic). Mouse optional in menus.
- **Performance:** 1080p @ 60 fps at *High* in single-player on an RTX 3060/3070-class GPU;
  4-player split-screen @ ≥ 50 fps at *Medium*.
- **Session:** one race 1.5–2.5 min; one cup ~10–15 min.
- **Audience:** everyone 8+; pick-up-and-play for families, depth (charge management, lines,
  item timing) for enthusiasts.

---

## 2. Core loop

```
 pick mode ─▶ pick car & paint ─▶ race ─▶ results ─▶ unlocks / records ─┐
     ▲                     (items, charge-boost, lines, rivals)          │
     └──────────────────────────────────────────────────────────────────┘
```
Inside a race: **drive the line → collect a pickup → hold or use the item → drift / jump / draft
to charge the battery → spend charge on straights → recover from hits fast.**

---

## 3. Game modes

| ID | Mode | Players | Summary |
|---|---|---|---|
| [MODE-1] | **Championship** | 1 | A cup of 4 races (Grand Volt Cup: 6) vs 7 AI. Points per race; a podium finish of the cup unlocks content. |
| [MODE-2] | **Quick Race** | 1 | Any unlocked track/layout, car, laps (track default, 1–9), 0–7 AI, AI difficulty (Rookie/Pro/Elite as unlocked), weather & time of day within the track's allowed presets (or Random / Dynamic), items on/off. |
| [MODE-3] | **Time Trial** | 1 | Solo, no items, 3 laps. Race your personal-best **ghost**. Bronze/Silver/Gold/Platinum medal targets per layout. |
| [MODE-4] | **Versus** | 2–4 | Local split-screen, available from the start. Lobby where each player joins with a device and picks a car. Quick Race options; AI fill optional (≤ 8 racers). Session scoreboard. |

### 3.1 Championship [MODE-1]
- Points per finishing position (8 racers): **10, 8, 6, 5, 4, 3, 2, 1**. Ties broken by the
  better result in the last race, then the race before, etc.
- Laps per race = the track's `default_laps` (§7.1).
- Standings screen (UI-6) after each race, with animated point gains.
- **Restart** a race from Pause or Results (unlimited, instant); it reverts that race's points.
- **Quit cup** asks for confirmation ("progress in this cup will be lost"). A cup in progress is
  not saved across app restarts.
- After the last race: **Cup end** ceremony (UI-21) — top-3 podium, trophy, unlocks.
  1st **Gold**, 2nd **Silver**, 3rd **Bronze**; a podium completes the cup. Best trophy per cup is
  kept.
- AI difficulty is set by the cup (§8.2). Elite+ exists only in the Grand Volt Cup.

| Cup | Difficulty | Races (track · layout · weather · time) | Unlock requirement |
|---|---|---|---|
| **Rookie Cup** | Rookie | Sofa Circuit · Fwd · Clear · Noon; Toy Town · Fwd · Clear · Morning; Backyard Bash · Fwd · Clear · Morning; Aisle Rush · Fwd · Clear · Night | start |
| **Pro Cup** | Pro | Maple Street · Fwd · Cloudy · Noon; Wrench Works · Fwd · Clear · Night; Backyard Bash · Rev · Rain · Noon; Sofa Circuit · Rev · Rain · Sunset | Rookie Cup podium |
| **Elite Cup** | Elite | Aisle Rush · Rev · Clear · Night; Maple Street · Rev · Storm · Night; Toy Town · Fwd+Mirror · Clear · Noon; Wrench Works · Rev+Mirror · Storm · Night | Pro Cup podium |
| **Grand Volt Cup** (6) | Elite+ | Toy Town · Rev+Mirror · Clear · Sunset; Backyard Bash · Fwd+Mirror · Snow · Morning; Sofa Circuit · Fwd+Mirror · Storm · Night; Aisle Rush · Fwd+Mirror · Clear · Sunset; Wrench Works · Fwd+Mirror · Cloudy · Sunset; Maple Street · Rev+Mirror · Dynamic (starts Fog) · Sunset | Elite Cup podium |

(Weather on indoor tracks is "outside weather": visible through windows and audible, no grip
effect — §10.2.)

### 3.2 Race flow (all modes) [RACE-1]
1. **Intro** (≈ 3 s, skippable with Confirm; skipped on restart): camera fly-by, track banner.
2. **Countdown** 3‑2‑1‑GO (≈ 3 s). Cars are held. **Perfect start**: throttle pressed within
   0.3 s *before* GO → 1 s boost (§5.5). Throttle held earlier than 1.0 s before GO → 0.6 s
   wheel-spin stall. Auto-accelerate (assist) never triggers the stall. AI perfect-start chance
   by difficulty: Rookie 5 %, Pro 20 %, Elite 40 %, Elite+ 60 %.
3. **Racing.** Laps count only after all checkpoints in order; "FINAL LAP" banner.
4. **Finish.** A finished human gets an orbit camera, may spectate; in Versus their viewport
   shows "Waiting for other players". The race ends when **every human has finished**.
   **Soft-lock guard:** starts when the winner finishes (one human) or when the first human
   finishes (2+ humans), and in any case 120 s after the winner finishes; it runs **60 s**, the
   last 10 s are shown as "Race ends in 10…"; on expiry every unfinished racer is classified by
   progress with a projected time.
5. **Results** (UI-13), then the mode's post-race flow (§3.3).

**Grid.** AI rivals fill the front of the grid in a seeded order and the local players start
behind them, as in most kart racers: a race is a climb through the field, and a human never
leads a bunched pack from pole. Humans line up in slot order; from the second race of a Versus
session on, in reverse order of the session points (the leader starts last).

### 3.3 Post-race flows [RACE-2]
| Mode | Results actions | Notes |
|---|---|---|
| Championship | **Continue** → Standings (UI-6) → Next race · **Restart** | after the last race: Cup end (UI-21) → Cup select |
| Quick Race | **Restart** · Change track/options · Menu | |
| Time Trial | **Retry** (instant) · Change track · Menu | shows medal earned, delta to the next medal, "Ghost saved" |
| Versus | **Rematch** · Next track (P1 picks) · Lobby · Menu | session scoreboard (championship points) updates |

---

## 4. Cars

### 4.1 Roster [CAR-1]
Seven cars in three classes plus one secret car (8 total). Stats are 1–5 stars, shown in menus;
physics implements them (§5). Every stat is a trade-off: **Weight** pushes others and resists
knock-back but reduces air control and makes landings cost more speed.

| Car | Class | Body style | Drive | Speed | Accel | Handling | Weight | Unlock |
|---|---|---|---|---|---|---|---|---|
| **Sparky** | Rookie | Off-road buggy, roll cage, big knobbly tyres | 4WD | 2 | 3 | 4 | 2 | start |
| **Dumpster** | Rookie | Monster truck, huge tyres, long travel | 4WD | 2 | 2 | 2 | 5 | start |
| **Zip** | Rookie | Tiny touring car, low and light | RWD | 2 | 4 | 3 | 1 | start |
| **Torque** | Pro | Muscle car, wide rear tyres | RWD | 4 | 3 | 2 | 4 | Rookie Cup podium |
| **Hornet** | Pro | Rally hatchback, roof scoop, mud flaps | 4WD | 3 | 4 | 4 | 3 | Rookie Cup podium |
| **Phantom** | Elite | Low sports coupe, big wing | RWD | 4 | 4 | 4 | 3 | Pro Cup podium |
| **Volt GT** | Elite | Hypercar, glowing accent strips | 4WD | 5 | 4 | 3 | 2 | Elite Cup podium |
| **Bolt** | Secret | Futuristic open-wheel "formula" with fins | RWD | 5 | 5 | 3 | 1 | all 6 Golden Batteries |

- Every car has **4 base paints**; each unlockable paint set (§9) adds one paint to every car.
- Visual identity [CAR-2]: chunky RC proportions — oversized wheels with tread, visible
  suspension arms and shocks, a whip antenna, glossy clear-coated bodies, racing number +
  stripes, emissive head/tail lights, brake lights that glow when braking.
- Wheels spin with speed, steer, and travel with the suspension [CAR-3].

### 4.2 Scale conventions [CAR-4]
- **1 world unit (u) = 22.5 cm of the real world.** Props are real size × 4.44.
- The car is ≈ **2.0 u long** and ≈ **1.1 u wide** (a 45 cm RC car).
- Displayed speed is toy-scale: **km/h = u/s × 1.8** (mph = km/h × 0.621) — deliberately not
  true scale.
- **Gravity 25 u/s²** (≈ 0.57 g at world scale) — a feel value chosen for big air; tune in 25–40.
- Rooms and gardens may be up to 1.5× their real floor area; furniture and props stay × 4.44.

---

## 5. Driving model

### 5.1 Feel targets [VEH-1]
| Aspect | Target |
|---|---|
| 0 → top speed | by Accel stars: 1★ 3.5 s · 2★ 3.25 s · 3★ 3.0 s · 4★ 2.75 s · 5★ 2.5 s (asserted by tests) |
| Top speed (u/s) by Speed stars | 1★ 24 · 2★ 27 · 3★ 30 · 4★ 33 · 5★ 36 |
| Steering | Tight and responsive at low speed; speed-sensitive (less lock at top speed); digital input ramped (0→full in ~0.12 s) so keyboard play is smooth |
| Grip | Planted on good surfaces; progressive breakaway; drift easy to start, easy to catch |
| Suspension | Visibly bouncy, absorbs bumps, recovers in < 0.5 s, never pogo-sticks |
| Air | ~0.6–1.2 s air time off big ramps; **air control** (pitch/roll/yaw with steer & throttle/brake), weaker for heavy cars |
| Weight | Heavier cars push lighter ones and resist knock-back; lighter cars have more air control and lose less speed on landing |
| Collisions | Bouncy off walls, never sticky; head-on hits cost speed, glancing hits keep most of it |
| Speed cap | Effective top speed never exceeds **1.5 × the car's base top speed** (after boosts, catch-up, AI scale and surface) |

### 5.2 Surfaces [VEH-2]
Grip multiplies tyre friction; the top-speed factor models rolling resistance.

| Surface | Grip | Top-speed factor | Used for |
|---|---|---|---|
| Asphalt | 1.00 | 1.00 | roads, concrete floors, curbs, drains |
| Stone | 1.00 | 1.00 | patio, paving |
| Plastic toy track | 1.00 | 1.02 | toy track ribbons |
| Wood | 0.95 | 1.00 | floors, deck boards, stairs, plywood ramps, shelves |
| Tile | 0.82 | 1.00 | tiles, marble, vinyl, polished floors |
| Carpet | 1.05 | 0.92 | rugs, play-mats, rubber mats |
| Cardboard | 0.90 | 0.95 | boxes, cardboard ramps |
| Grass | 0.75 | 0.85 | lawns (bumpy suspension noise, debris) |
| Dirt | 0.80 | 0.92 | paths, sandbox (dust) |
| Ice | 0.45 | 1.00 | freezer aisle |
| Oil | 0.50 | 1.00 | workshop oil stains |
| Water | 0.65 | 0.85 | puddles, wash bay (splashes) |

**Weather modifiers** (outdoor surfaces only, scaled by wetness / snow cover 0–1): wet Asphalt
×0.85, wet Stone/Tile ×0.75, wet Grass ×0.85, wet Dirt → mud ×0.80 and top speed ×0.90; snow
cover ×0.70 on all outdoor surfaces.

### 5.3 Battery charge & boost [VEH-3]
The battery **Charge meter** (0–100 %) is the skill mechanic.
- **Drift** charges up to 22 %/s, only above 40 % of top speed, after 0.4 s of continuous drift,
  at a slip angle of 10–45° (none while spinning), scaled by yaw rate.
- **Air time** beyond 0.3 s charges 15 %/s. A **clean landing** (all four wheels touch down within
  0.15 s, tilt < 20°) after > 0.8 s of air adds +10 %.
- **Drafting**: within 6 u behind another car, inside a ±15° cone, at ≥ 50 % of top speed: 10 %/s.
- **Spend:** hold **Boost** to drain 35 %/s for ×1.30 top speed and ×1.50 drive force; minimum
  15 % to activate.
- Feedback: drift sparks change **colour, size and sound pitch** with the gain rate; the meter
  glows when ≥ 15 %. AI uses the same rules.

### 5.4 Recovery [VEH-4]
- **Auto-right:** a car resting on its roof/side below 2 u/s for 1.0 s hops and rolls upright.
- **Reset button:** *tap* → flip upright in place if the car is upside down or on its side;
  *hold 0.4 s* → **respawn** at the track centre at the last respawn point (≈ every 15 u) that is
  not ahead of the last validated checkpoint, facing forward, with 1.5 s of ghosting (no car–car
  collision, blinking). Cooldown 2 s.
- **Auto-respawn** when: below the track's kill height; **off-track** for > 3 s (no wheel touches
  road or ground, or |lateral offset| > half-width + the segment's off-road margin, default 6 u);
  stuck (speed < 1 u/s with throttle held for 4 s).

### 5.5 Status effects [VEH-5]
Items and race events apply timed effects; physics reads the combined result.

| Effect | Parameters | Used by |
|---|---|---|
| `PowerLoss` | drive-force multiplier, duration | EMP, Storm Strike, stall |
| `Grip` | grip multiplier, duration | Oil Slick |
| `TopSpeed` | multiplier, duration | Leech Coil tow (+8 %) |
| `Boost` | top-speed mul, force mul, duration | Turbo Cell, perfect start |
| `Shield` | duration | Shield Bubble |
| `Ghost` | duration | respawn |
| *Tumble* | instant linear + angular impulse (not a timed effect) | rockets, orb, Goo Bomb, Spring Pad |

- **Stacking:** the same kind keeps the strongest magnitude, with duration = max(remaining, new).
  Different kinds multiply.
- **Boosts never stack:** the active boost is the strongest of charge boost (1.30 / 1.50),
  perfect start (1.30 / 1.50, 1 s), Turbo Cell (1.45 / 1.80, 2.5 s). Charge is not drained while
  Turbo is active. The 1.5× speed cap (§5.1) always applies.
- **Shield:** blocks every hit that produces a hit event (rocket, orb, EMP incl. its charge drain,
  Goo explosion, entering oil, Spring Pad launch, Leech Coil, Storm Strike), and every hit landing
  in the tick it pops; then it is removed. A shielded car cannot be handed a Goo Bomb.
- **Owner immunity:** your own projectiles and hazards ignore you for 1.0 s after they spawn; the
  Shock Orb never hits its owner; the Goo explosion does hit the carrier.
- **Heavier cars** receive tumble impulses scaled by (reference mass / mass)^0.5.

---

## 6. Items (pickups & weapons)

### 6.1 Pickups [ITEM-1]
- **Volt Cubes**: glowing translucent cubes with a lightning icon, floating & spinning, in rows
  of **4–5** across the track at 4–6 places per lap. Driving through one grants a random item if
  the racer holds none (a held item is never replaced). A taken cube respawns after **2 s**.
- Granting shows a 1.0 s **roulette** in the HUD before the item is usable.
- One item slot per racer.

### 6.2 Item set [ITEM-2]
Holding **Look back** while using an item fires it **backward** where noted.

| Item | Use | Effect | Counterplay |
|---|---|---|---|
| **Turbo Cell** | instant | Boost 1.45 / 1.80 for 2.5 s | — |
| **Spark Rocket** | fire forward (or backward) | Fast (55 u/s) homing rocket: locks the nearest car within 35° / 80 u ahead (backward: 35° / 60 u behind); turn rate 120°/s; hit = tumble (~1.5 s lost). Unlocked rockets fly straight and explode on walls. The target hears a lock-on beep and sees a HUD warning arrow. | Sharp turns break lock, shield, obstacles |
| **Leech Coil** | instant | Tethers you to the nearest car ahead within 40 u (else within 20 u any direction; waits up to 3 s for a target). For 3 s: moves 30 % of the target's charge to you (10 %/s) and tows you (+8 % top speed). Breaks beyond 45 u or without line of sight. | Break line of sight, shield |
| **Shock Orb** | fire forward | Slow (24 u/s) plasma ball that **follows the track**, bouncing off walls; knocks any car within 3 u sideways + up. Lives 10 s or 3 hits; never hits its owner. | Steer around it |
| **EMP Pulse** | instant | 0.4 s wind-up glow, then a 12 u radius pulse: other cars `PowerLoss(0)` for 2.0 s (steering works) and lose 50 % charge. | Keep distance, shield |
| **Goo Bomb** | instant | Sticks to your car with a 6 s fuse and beeping; its carrier drives at 1.1× top speed. **Bump another car — or pass within 0.6 u of it — to pass it on** (once per approach; 0.5 s no-pass-back). Explodes: tumble everyone within 7 u, the carrier included. | Pass it on! |
| **Oil Slick** | drop behind | 2.5 u puddle for 15 s, used up once it has caught 3 cars; driving through: `Grip(0.2)` for 1.2 s + spin torque | Steer around |
| **Spring Pad** | drop behind | 1.6 u springy pad for 15 s, used up after 3 launches; a car crossing it is launched (up 8 u/s, forward 2 u/s, small random spin). Skilled players can turn the air into charge. | Steer around, shield |
| **Shield Bubble** | instant | 10 s bubble (§5.5); bumping with it pushes others harder | — |
| **Storm Strike** | instant | *Rare, positions 6–8 only.* Lightning hits every car ahead, one by one 0.25 s apart: `PowerLoss(0)` 1.5 s + small hop. | Shield |

### 6.3 Odds by race position [ITEM-3]
Band from `f = (rank − 1) / (N − 1)`: rank 1 → **1st**; f ≤ 0.4 → **top**; f < 0.7 → **middle**;
otherwise **last** (N = 8: 1 | 2–3 | 4–5 | 6–8). Weights per row sum to 100.

| Band | Turbo | Rocket | Leech | Orb | EMP | Goo | Oil | Spring | Shield | Storm |
|---|---|---|---|---|---|---|---|---|---|---|
| 1st | 10 | 15 | 0 | 0 | 0 | 0 | 30 | 20 | 25 | 0 |
| top | 10 | 20 | 10 | 10 | 5 | 10 | 10 | 10 | 15 | 0 |
| middle | 15 | 20 | 15 | 15 | 10 | 10 | 5 | 5 | 5 | 0 |
| last | 25 | 15 | 20 | 15 | 15 | 0 | 0 | 0 | 0 | 10 |

Items can be disabled (Quick Race / Versus); Time Trial has none.

---

## 7. Tracks

### 7.1 Track list [TRK-1]
Six environments; each has a **Forward** and a **Reverse** layout, and any layout can be
**Mirrored** once unlocked → 24 variants.

| Track | Environment | Indoor | Diff. | Lap length | Default laps | Surfaces | Signature features |
|---|---|---|---|---|---|---|---|
| **Toy Town** | Toy store / playroom | yes | ★ | ~650 u | 4 | Plastic toy track, Carpet (play-mat), Wood | orange toy-track ribbon with banked turns & a helix; knockable block towers; dollhouse drive-through |
| **Sofa Circuit** | Family house ground floor | yes | ★ | ~400 u | 5 | Wood, Carpet (rug), Tile (kitchen) | book-stack ramp onto the coffee table and jump off; tunnel under the sofa; slalom through dining-chair legs; stair ramp |
| **Backyard Bash** | Back garden | no | ★★ | ~450 u | 5 | Grass, Dirt, Stone (patio), Wood (deck), Water (puddles) | flower-pot chicane; hose-reel spiral; sandbox jump; under the garden bench |
| **Aisle Rush** | Supermarket after closing | yes | ★★ | ~800 u | 3 | Tile (vinyl), Ice (freezer), Cardboard | long aisle straights; icy freezer section; cardboard-box ramps; knockable can pyramids; checkout lanes |
| **Wrench Works** | Auto-repair workshop | yes | ★★★ | ~700 u | 4 | Asphalt (concrete), Oil, Carpet (rubber mats), Wood (plywood ramps), Water (wash bay) | drive under cars on lifts; plywood ramp onto the workbench and jump; tyre-stack chicane; oil-stain slides; wash bay; paint-booth haze |
| **Maple Street** | Suburban street & front lawns | no | ★★★ | ~900 u | 3 | Asphalt, Grass (lawns), Stone (sidewalk) | drive under parked (giant) cars; curb hops; lawn shortcut; storm-drain dip; street lamps at night |

Racing width ≥ 8 u; chokepoints ≥ 5 u, at most 2 per lap. Time Trial always uses 3 laps.

### 7.2 Track content requirements [TRK-2]
Each layout provides: a closed loop with elevation changes (≥ 1 jump or ramp *that works in
that direction*), 4–6 pickup rows, ≥ 8 checkpoints, a starting grid for 8, respawn points, a
racing line for AI, medal target times, and an environment theme (props, lighting, ambience,
music). The **Forward** layout, mirrored or not, hides the track's one **Golden Battery** (off the
main line — a risky shortcut or ledge); Reverse layouts have none. Track features that only work
one way (jumps, ramps, drops, shortcuts) are flagged per direction.

Allowed presets:

| Track | Times of day | Weather |
|---|---|---|
| Sofa Circuit, Toy Town, Wrench Works | Morning, Noon, Sunset, Night | Clear, Cloudy, Rain, Storm (outside weather) |
| Aisle Rush | Sunset, Night | Clear |
| Backyard Bash, Maple Street | Morning, Noon, Sunset, Night | all six |

### 7.3 Track rules [TRK-3]
- Laps count only if checkpoints were passed in order. Intentional shortcuts lie inside the
  checkpoint corridors. Checkpoints passed while outside the corridor count once the car is back
  in it, unless the trip gained more track than the car drove (a cut): then "Missed checkpoint —
  go back".
- **Wrong way** warning after 2 s driving against the track direction above 5 u/s.
- **Lost** warning when a racer is outside the legal corridor for 0.5 s, or moves above 5 u/s with
  less than 30 % of it along the track for 2 s (driving sideways, circling an open floor): an
  arrow points back to the track (10 u ahead on the centreline); outside the corridor it also
  counts down to the automatic respawn. Off again after 0.5 s back in the corridor making
  progress. An automatic respawn says why it happened (off the track, fell, stuck).
- **Way-finding** (generated for every layout from its corners, so reverse and mirror get their
  own): where the ribbon is Invisible, the racing edges are taped onto the floor (themed:
  blue painter's tape in the house, store-blue lane tape in the supermarket, hazard yellow in
  the workshop, white line paint in the garden, orange chalk on the street) except along
  barriers; before every corner of ≥ 40° a floor arrow bends its way (hairpins get two);
  corners of ≥ 45° get chevron boards on the outside (one per 45°) facing the approaching cars;
  where the floor runs on straight ahead of a corner past the legal corridor, a taped cross
  closes it.
- Barriers keep cars on track where needed (themed: foam bumpers, books, curbs, plastic toy-
  track walls, shelf edges, tyre walls). Open areas (lawns, floors) are drivable off the racing
  ribbon with their own surfaces; leaving them triggers the off-track rule (§5.4).
- No vertical loops. Banked turns, half-pipe wall rides and helices are allowed.

---

## 8. AI opponents [AI-1]

### 8.1 Behaviour
- Follow a precomputed **racing line** with a speed profile from curvature; each AI drives a
  personal lateral offset for variety; overtakes by shifting lanes when blocked, and steers around
  a stopped or crawling car like an obstacle instead of queuing behind it; avoids hazards
  (oil, spring pads, orbs) it can see; brakes for sharp turns; ignores small dynamic props.
- Uses the charge boost on straights; drifts in long corners to charge.
- Item use: rockets when a target is in the lock cone (backward when a car is close behind and
  none ahead), Leech Coil when a car ahead is in range, oil/spring pad when someone is close
  behind, shield when a rocket locks on or when holding nothing better, passes the Goo Bomb by
  seeking the nearest car, Turbo on straights, EMP when ≥ 1 car is within radius.
- Recovery: reverses out when stuck against a wall; uses reset if still stuck; never blocks the
  race for more than a few seconds.
- **Catch-up** (option, on by default; off in Time Trial): AI behind the leading human get up to
  +6 % speed, AI far ahead lose up to −6 %; never visible as rubber-band teleporting.
- Perfect-start chance by difficulty (§3.2).

### 8.2 Difficulty
The speed scale multiplies the AI's own per-car speed profile.

| Level | Speed scale | Cornering margin | Mistakes | Item reaction |
|---|---|---|---|---|
| Rookie | 0.86 | wide | frequent small (late braking, wide lines) | 0.8–1.5 s |
| Pro | 0.94 | medium | occasional | 0.4–0.8 s |
| Elite | 1.00 | tight | rare | 0.2–0.4 s |
| Elite+ (Grand Volt Cup only) | 1.02 | tight | rare | 0.15–0.3 s, smarter item hoarding |

### 8.3 Rivals
Seven named rivals with a colour, a personality (aggression 0–1, consistency 0–1) and a car per
class (signature car in bold). In cups the rivals drive the cup's class; in Quick Race and Versus
they drive the class of the highest-class human car (Secret counts as Elite).

| Rival | Personality | Rookie | Pro | Elite |
|---|---|---|---|---|
| **Dash** | clean & fast | Sparky | **Hornet** | Phantom |
| **Mayhem** | aggressive | **Dumpster** | Torque | Volt GT |
| **Penny** | consistent | **Zip** | Hornet | Phantom |
| **Turbo Tom** | boost-happy | Zip | **Torque** | Volt GT |
| **Glitch** | erratic | Sparky | Hornet | **Volt GT** |
| **Rusty** | steady | **Sparky** | Torque | Phantom |
| **Nova** | item hoarder | Zip | Hornet | **Phantom** |

---

## 9. Progression & records [PROG-1]

- The **Profile** persists unlocks, trophies, medals, Golden Batteries, records, ghosts, stats,
  and which onboarding hints were seen.
- Unlock table:

| Trigger | Unlocks |
|---|---|
| start | Rookie Cup; Versus; Sofa Circuit, Toy Town, Backyard Bash, Aisle Rush (Fwd); Sparky, Dumpster, Zip; difficulties Rookie & Pro |
| Rookie Cup podium | Pro Cup; Maple Street, Wrench Works; **Reverse** layouts of all tracks; Torque, Hornet |
| Pro Cup podium | Elite Cup; **Mirror** mode; Phantom; difficulty Elite |
| Elite Cup podium | Grand Volt Cup; Volt GT |
| Grand Volt Cup gold | "Chrome" paint set; credits roll |
| 3 / 6 / 12 Time-Trial gold medals (Gold or Platinum) | "Neon" / "Carbon" / "Gold" paint sets |
| all 6 Golden Batteries | **Bolt** |

- **Medals** (Time Trial, per layout incl. mirror): target total times calibrated from an Elite
  AI driving a Phantom with items and catch-up off: Platinum = AI × 0.95, Gold × 0.98,
  Silver × 1.05, Bronze × 1.15.
- **Records:** best lap (from any single-player mode) and best race time (Time Trial only) per
  layout, each stored with the car used; shown in Track Select and Records. **Ghost** of the best
  Time Trial run per layout. Records and ghosts store a track-geometry hash and are discarded,
  with a notice, if the track changed.
- **Golden Batteries**: one per track, in the Forward layout (mirrored or not), collectable in
  any single-player mode; a toast shows and the profile is saved immediately.
- **Stats:** races, wins, podiums, distance driven, items used, hits landed, time played.
- New unlocks are announced with toasts on Results / Cup end and "NEW" badges in menus.
- Settings → Gameplay → **Reset progress** (double confirmation).

---

## 10. Weather & time of day [WTH-1]

### 10.1 Time of day
**Morning, Noon, Sunset, Night.** Outdoor tracks: physically-based sky with sun angle and colour,
stars at night, street lamps / porch lights at night. Indoor tracks: window light angle and colour
by time; at Night room lamps / work lights are on and windows show moonlight. At Night local
players' cars get headlights.

### 10.2 Weather
| Weather | Visuals | Gameplay (outdoor) | Audio |
|---|---|---|---|
| Clear | sharp shadows | — | birds (day) / crickets (night) |
| Cloudy | overcast sky, soft shadows | — | light wind |
| Rain | rain streaks around each camera, darker sky, wet darker & glossier ground, splashes, mist | wetness → grip | rain loop |
| Storm | heavy rain slanted by wind, lightning flashes + thunder | wetness; gusts push airborne cars slightly | heavy rain, thunder |
| Fog | dense distance fog (volumetric at Ultra) | — | muffled ambience |
| Snow | falling snow, gradual white ground cover, cold grading | snow cover → grip | wind, soft hush |

- **Wetness / snow cover** (0–1) ramp over ~20 s when precipitation starts and dry/melt slower.
- **Outside weather** (indoor tracks): Cloudy / Rain / Storm change the window light, show rain on
  windows and lightning flashes through them, and add muffled rain/thunder audio. No grip effect.
- **Random** picks one of the track's allowed presets (seeded).
- **Dynamic** (outdoor tracks): seeded; one change at 30–70 % of race distance to a neighbouring
  preset (Clear↔Cloudy↔Rain↔Storm, Cloudy↔Fog, Cloudy↔Snow) with a 20 s blend.
- Accessibility: *Reduce flashing* replaces lightning, Storm Strike, EMP rings, explosions and
  boost flashes with soft glows.

---

## 11. Camera [CAM-1]
- **Chase** (default), **Far chase**, **Hood** modes (cycle button). Spring-damped follow with
  velocity look-ahead; FOV widening up to +12° with speed/boost; subtle shake on impacts and
  landings (setting).
- **FOV setting** = horizontal FOV at 16:9 (Hor+ for other aspects); 2P top/bottom viewports are
  clamped to ≤ 100° horizontal.
- **Look back** while held. **Obstruction handling**: the camera pulls in when geometry is
  between it and the car (vital indoors under tables and cars).
- **Intro fly-by**, **finish orbit**, **spectate** (cycle cars) after finishing.
- **Split-screen layouts** [CAM-2]: 2P horizontal (top/bottom, default) or vertical (setting);
  3P = three quadrants + a 2D minimap/standings panel in the fourth; 4P = quadrants.
- Menus: a 3D **garage showroom** — the selected car on a turntable, slow camera drift; Car
  Select uses this backdrop (no separate preview render).

---

## 12. Controls [INP-1]

### 12.1 Driving actions (rebindable per scheme)
| Action | Gamepad (default) | Keyboard A | Keyboard B |
|---|---|---|---|
| Throttle | RT | W | ↑ |
| Brake / reverse | LT | S | ↓ |
| Steer | Left stick / D-pad ←→ | A / D | ← / → |
| Drift (handbrake) | B / RB | Space | Right Shift |
| Use item | A | E | Enter |
| Boost (spend charge) | X / LB | Left Shift | Right Ctrl |
| Look back (also: fire backward) | Y | Q | Numpad 0 / Delete |
| Reset (tap = flip, hold 0.4 s = respawn) | D-pad ↑ | R | Backspace |
| Camera mode | View/Back | C | Numpad . / End |
| Pause | Start | Esc | Esc |

- In single-player, keyboard A, keyboard B and any gamepad all control Player 1.
- Rebinding per scheme (keyboard A, keyboard B, gamepad); conflicts are checked across both
  keyboard schemes (they share one keyboard) and warned.
- Steer dead zone (default 0.12); analog triggers; digital steering ramp.
- **Rumble** on impacts, hits, landings and boost (setting, intensity slider).

### 12.2 Menu controls (fixed)
- Gamepad: D-pad / left stick move (stick threshold 0.5, release 0.3; hold repeats after 0.35 s,
  then every 0.1 s), A confirm, B back, LB/RB switch tabs, Start = pause/confirm.
- Keyboard: arrows or WASD move, Enter or Space confirm, Esc or Backspace back, Q/E switch tabs.
- **Versus lobby** (two keyboard players possible): scheme A uses WASD + Space (confirm) +
  Left Ctrl (back); scheme B uses arrows + Enter (confirm) + Backspace (back). The lobby warns
  that some keyboards cannot register many keys at once.
- Mouse: hover focuses, click activates, wheel scrolls lists.
- Button glyphs follow the last device used by that player (our own generic glyphs, no
  platform logos).

### 12.3 Device assignment [INP-2]
- Title screen: any Confirm continues; the device used becomes Player 1's glyph device.
- Versus lobby: "Press A / Enter / Space to join" for slots 1–4 (keyboard counts as two devices:
  scheme A and B); Back on a joined slot leaves. Each joined player picks a car with their own
  focus.
- **Controller disconnect** in a race: pause with "P2's controller disconnected — press A/Start
  on any controller, or Enter, to take over · [Drop player]". In Versus a dropped player's car is
  taken over by the AI. In the lobby the slot is freed; in menus nothing happens. Reassignment is
  always by button press, never by pad id.
- **Window focus loss** during a race auto-pauses; resuming needs input.

---

## 13. HUD [HUD-1]
Per player viewport (compact layout in split-screen; scaled by *HUD scale*):
- **Position** "2nd / 8" (top-left, large) and **Lap** "LAP 2/3"; "FINAL LAP" banner.
- **Timer**: total race time and current lap; best lap; in Time Trial a split delta vs the
  ghost/best at each checkpoint (green/red **and** +/− sign).
- **Item slot** (top-right) with roulette animation.
- **Speed** (digital, km/h or mph) and **Charge meter** (battery icon filling, glows when
  boost-ready, flashes while spending) — bottom-right.
- **Minimap** (bottom-left; 1P and 2P; in 3P the fourth quadrant): the track as a ribbon between
  its racing edges with the start line and chevrons for the direction of travel, racer dots in
  their colours. A player's own minimap is heading-up (it turns with their view), follows them
  (about 150 u across), shows them as an arrow in the middle and highlights their next
  checkpoint; the shared 3P map stays north-up and whole.
- **Turn indicator** (top centre): the corner ahead from ≈ 2.5 s before it (30–90 u) until its
  end — an arrow bending its way (slight, square, hairpin) coloured by how hard it is (white
  easy, yellow medium, red sharp), with words outside compact views ("HAIRPIN LEFT", "SHARP
  RIGHT", "LEFT", "EASY RIGHT"); hidden while lost or driving the wrong way.
- **Warnings**: rocket lock-on arrow (direction of the incoming rocket), Goo Bomb fuse, shield
  timer, wrong way, lost ("THIS WAY" / "OFF TRACK" with an arrow to the track and the respawn
  countdown, §7.3).
- **Countdown**, **notifications** ("Hit by Spark Rocket — Mayhem", "Fastest lap!", "Golden
  Battery found!", "Off the track too long — put back"), **onboarding hints** (once each, setting): drift → charge, boost ready, item
  ready, "Hold Reset" after 3 s stuck, wrong way.
- **Finish**: big "1st!" banner, then spectate hint; "Waiting for other players" (Versus);
  "Race ends in 10…" during the soft-lock guard.
- Opponent name tags and lock-on markers are drawn in 3D per camera (not UI overlays).
- Readable at 720p in 4-way split.

---

## 14. Front-end & screens [UI-1]
All menus are React (bevy-react) over a live 3D backdrop (garage showroom in menus; the track
during pause/results). Style: dark navy frosted-glass panels (translucent fills in split-screen),
electric cyan + volt-yellow accents, bold italic display font, springy motion; the glitch/charge
morph only on Main menu → mode transitions, plain fades elsewhere; a sound cue on every focus
change and confirmation.

| ID | Screen | Contents |
|---|---|---|
| UI-2 | Splash | short text card "Made with Bevy and bevy-react" (text only, no logos) |
| UI-3 | Title | ReCharged logo (animated), "Press any button", version |
| UI-4 | Main menu | Championship, Quick Race, Time Trial, Versus, Garage, Records, How to play, Settings, Credits, Quit (confirm) |
| UI-5 | Cup select | cup cards (tracks, difficulty, best trophy, lock reason) |
| UI-6 | Cup standings | race list with results; standings with animated points; Next race / Quit cup (confirm) |
| UI-7 | Track select | track cards with preview image, lock state, records & medals; layout toggles (Reverse/Mirror); options (laps, AI count & difficulty, items, weather, time of day) |
| UI-8 | Car select | car carousel over the showroom backdrop, stat bars, class, paint picker, lock reason |
| UI-9 | Versus lobby | 4 join slots (device glyph, P1–P4 name, slot colour, car, ready), then P1 picks the track |
| UI-10 | Loading | track name & art, tip, progress |
| UI-11 | Race HUD | §13, per viewport |
| UI-12 | Pause | Resume, Restart, How to play, Settings, Quit (confirm); dims the scene |
| UI-13 | Results | standings (pos, name, car, total, best lap, points), new records/medals, unlock toasts; mode actions (§3.3) |
| UI-14 | Garage | all cars (locked silhouettes with unlock hints), stats, paints |
| UI-15 | Records | per layout: best lap/total with car, medal; trophies; Golden Batteries; stats |
| UI-16 | Settings | tabs: Gameplay, Graphics, Audio, Controls, Accessibility (§15) — apply live, persist |
| UI-17 | Credits | team, tools, licences |
| UI-18 | Dialogs & toasts | confirm, controller disconnected, unlock toast, error toast/dialog |
| UI-19 | Navigation | full gamepad/keyboard/mouse parity; focus always visible; Back goes up one level; no dead ends; first focus on the most likely action |
| UI-20 | How to play | controls (with the player's glyphs), charge & boost, items, reset & recovery |
| UI-21 | Cup end | podium of the top 3 cars, trophy, unlocks, credits after Grand Volt gold |
| UI-22 | Versus scoreboard | session points per player across races |

Versus names are fixed **P1–P4** with slot colours **cyan, yellow, magenta, green** (also used
for 3D name tags).

---

## 15. Settings [SET-1]
All settings apply live and persist (written at most once per second).

| Tab | Settings |
|---|---|
| Gameplay | speed units (km/h, mph); default camera mode; camera shake; HUD scale (75–125 %); minimap; 2P split orientation; AI catch-up; default AI difficulty; hints; reset progress |
| Graphics | quality preset (Low / Medium / High / Ultra / Custom); display mode (windowed, borderless); window size (windowed); vsync; field of view (60–100° horizontal); shadows (off/low/high); anti-aliasing (off/FXAA/SMAA/TAA); ambient occlusion; bloom; motion blur; particle density (low/med/high); volumetric fog (Ultra only, never in split-screen) |
| Audio | master, music, sound effects, engines, UI, ambience volumes; mute when unfocused |
| Controls | binding lists per scheme (keyboard A, keyboard B, gamepad) with rebinding & reset; steering dead zone; steering sensitivity; vibration on/off & intensity |
| Accessibility | reduce flashing; colour-blind friendly racer markers; steering assist; auto-accelerate |

- **Steering assist**: gently steers toward the racing line and brakes gently before corners taken
  too fast, and after a crash or respawn turns the car back toward the track direction; off-track
  auto-respawn after 2 s instead of 3 s. Throttle-only play (no steering or braking for 2 s) gets
  round reliably at a relaxed pace (≈ 1.25× the medal reference: no medals by holding throttle);
  any steering or braking hands full control and pace back for 2 s.
- Split-screen applies a lower effective quality tier automatically.

---

## 16. Audio [AUD-1]

### 16.1 Music
Synthwave / electro-funk, energetic, loop-friendly; procedurally composed and rendered offline.
| Cue | Use |
|---|---|
| `menu_theme` | front-end loop |
| `race_1` … `race_4` | race loops (each track has a default); a synchronized *final-lap* variant/stem raises intensity |
| `results_jingle`, `podium_fanfare`, `cup_victory` | stingers |
Music ducks during pause and crossfades between screens. (If quality lags: 3 race loops.)

### 16.2 Sound effects (non-exhaustive)
Engines (3 timbres: buggy whine, truck growl, sports electric scream) with pitch by RPM and load;
tyre skid (hard surface) & scrub (soft); surface rumble (grass/dirt); landing thump; impacts
soft/hard; boost whoosh + electric crackle; drift spark sizzle (pitch rises with charge rate);
charge-ready chime; pickup grab; roulette ticks & lock; rocket launch / flight / explosion;
lock-on beep; leech coil hum & zap; orb hum & zap; EMP charge & blast; goo bomb beeps &
explosion; oil splat & slip; spring pad boing; shield up / hit / pop; storm strike; countdown
beeps (3× low + GO high); lap chime; final-lap fanfare; finish horn; wrong-way buzzer; respawn
materialise; Golden Battery jingle; UI hover, select, back, toggle, slider tick, error, unlock;
ambience: room tone, birds, crickets, wind, rain (light/heavy, muffled indoor variant), thunder
(3 variants), supermarket hum, workshop hum & distant tools.

### 16.3 Mixing
Buses: Master, Music, SFX, Engines, UI, Ambience (settings volumes). Other cars' sounds are
attenuated and panned by distance to the **nearest** local player's camera; in split-screen each
local player's own engine plays at −6 dB.

---

## 17. Visual direction [VIS-1]
- **Stylised realism**: PBR materials with saturated toy colours; beveled edges that catch
  highlights; rich bounce light; soft contact shadows (SSAO); bloom on emissives; filmic
  tonemapping; per-environment colour grading.
- **Environments** look like giant real places: real-world materials (wood grain, carpet, tiles,
  grass, asphalt, concrete), giant props. Budget per track: ≤ 6 hero props plus a shared kit of
  ~15 props per environment family; ≤ 40 dynamic (knockable) bodies per track.
- **Wet surfaces**: darker and glossier (roughness), with environment reflections — no
  screen-space reflections.
- **VFX**: drift sparks (charge-coloured & sized), boost flame + electric arcs, dust/grass/mud/
  water debris from wheels, skid marks, impact sparks, landing dust, rocket trails & explosions,
  leech tether beam, EMP ring, orb glow, oil sheen, spring pad bounce, shield bubble, lightning
  bolt, pickup glow & spin, respawn hologram materialise, finish confetti, rain streaks &
  splashes, snowfall, fog, lightning.
- Cars always readable against the environment (rim light; colour-blind markers option).
- An **art bible** (`docs/design/art-bible.md`) fixes per-environment palettes, bevel widths,
  texture density (px/u), material list and references before environment production starts.

---

## 18. Accessibility [ACC-1]
Reduce flashing (§10.2), camera shake toggle, HUD scale, colour-blind-safe racer markers, steering
assist, auto-accelerate, full rebinding, onboarding hints, all information conveyed by more than
colour alone (icons, text, signs, pitch).

---

## 19. Robustness [ROB-1]
- Data or asset load failure → error dialog and back to Menu, never a panic.
- Save failure → error toast, retried on the next save. Corrupt profile/settings → backed up,
  defaults used, toast shown. Saves are versioned with tested migrations.
- No audio device → the game runs muted. Missing gamepad support → keyboard still works.
- Logs of the last 3 sessions are kept in the user-data folder.

---

## 20. Out of scope
Online multiplayer, track editor, car damage, full replays (ghosts only), achievements/platform
services, localisation beyond English (strings kept in one place), mobile/web builds, exclusive
fullscreen, screen-space reflections, vertical loops. Stretch goals if time allows: Battle Tag
mode, photo mode, more tracks.

---

## 21. Milestones & definition of done

| Milestone | Contents | Exit criteria |
|---|---|---|
| **M1 — Walking skeleton** | workspace, contracts, app states & session lifecycle, React UI bundle with a menu, test track, placeholder car driving, chase camera | `cargo run` → menu → drive a lap on the test track; restart works |
| **M2 — Vertical slice** | **Toy Town** with props, a real car model, laps & positions, AI field, HUD, results, pause, **2P split-screen smoke test** | full race vs 7 AI on Toy Town; 2P race works |
| **M3 — Systems complete** | all items, weather & time of day, split-screen 1–4, rebinding, settings persisted, progression & save, all modes & post-race flows, audio system, onboarding | every mode playable end to end |
| **M4 — Content complete** | 8 cars; tracks in two waves (Toy Town, Maple Street, Sofa Circuit; then Backyard Bash, Aisle Rush, Wrench Works) with Fwd/Rev/Mirror; music, full SFX, VFX, previews, medal times | all content present and reachable |
| **M5 — Polish & QA** | balancing, performance, bugs, accessibility, credits | DoD below |

Optional user playtest checkpoints after M1 and M2 (driving feel); work continues meanwhile, and
vehicle/AI/item tuning files hot-reload in dev builds so feedback is cheap to apply.

**Definition of done [DOD]:**
1. All four modes playable end to end with keyboard only and with a gamepad only (verified with
   scripted input runs).
2. All 6 tracks × Fwd/Rev × Mirror drivable; automated headless test: an AI completes 3 laps on
   every variant within 1.5× the expected time with ≤ 2 respawns per lap.
3. Progression unlocks, records, medals, ghosts and settings persist across restarts.
4. 2–4 player split-screen with per-player HUD, mixed keyboard + gamepads.
5. Weather (all 6) and time of day (all 4) selectable and visibly/gameplay distinct.
6. All §16 music cues and the SFX list present and mixed via settings buses.
7. Performance targets of §1.3 met on the dev machine (RTX 3070), measured by the bench runner.
8. `cargo test --workspace` and UI typecheck/tests pass; clippy clean; deterministic replay of
   a seeded race; no panic and no entity leak in a 30-minute automated soak of chained races.
