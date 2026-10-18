# GDD Review 1
Verdict: APPROVED WITH REQUIRED CHANGES

Reviewed: `docs/GDD.md` (DRAFT v1), `docs/ARCHITECTURE.md` (DRAFT v1), `docs/PRACTICES.md`. The
`docs/tech/` digests did not exist yet at review time.

**Conditions.** The package is good: clear vision, sound layering, concrete numbers. It may go
to implementation once B1–B9 are fixed in the documents, and a short delta review (review 2)
will check only those fixes. Do not start M1 before B1–B4 are fixed, because they change
`rc_core`, which M1 creates.

**Verified against sources, not memory** (registry `~/.cargo/registry/src/index.crates.io-*`,
local `bevy-react` checkout):
- The stack is compatible: bevy_rapier3d 0.37.0 → bevy 0.20, bevy_kira_audio 0.27.1 → bevy_app 0.20,
  bevy-react 0.8.1 → bevy 0.20.
- Messages written in `FixedUpdate` survive until read. They are cleared only after a fixed tick
  (`bevy_time-0.20.0/src/lib.rs:101`).
- rapier propagates transforms in `SyncBackend`, so a respawn that writes `Transform` works.
- bevy_kira_audio only warns when there is no audio device (`audio_output.rs:43-45`).
- `IsDefaultUiCamera`, `Time<Fixed>::overstep_fraction`, `AnimationSet::Apply`, `<portal>` and
  `RapierConfiguration::physics_pipeline_active` all exist.
- I reproduced the B1 state bug with a running Bevy 0.20 probe. B2, B4 and R6/R7 are confirmed in
  the bevy-react and bevy_pbr source.

---

## Blocking issues (must be fixed before approval)

### B1 — The race-session lifecycle is broken as specified (scoping, restart, next race)
**Location:** ARCH §3.1 (`RaceSession`, `DespawnOnExit(RaceSession)`), §5.12, §6.1, §6.3; GDD §3.1
(restart), §3.2, UI-12, UI-13, Pillar 5 ("instant restart").

**Problem.**
1. In Bevy 0.20, computed states default to `ALLOW_SAME_STATE_TRANSITIONS = true`
   (`bevy_state-0.20.0/src/state/computed_states.rs:79`). On `Loading → InRace`, `RaceSession` is
   recomputed to the same value and an `exited == entered` transition event is sent. Despawn is
   skipped only when the flag is false (`state_scoped.rs:170`). I measured this with a probe
   app that spawns entities during `Loading`:
   - Default flag: `Loading → InRace` gives 1 → **0**. The track, colliders, racers and visuals
     are despawned the moment the race starts, and `OnExit`/`OnEnter(RaceSession)` re-run.
   - Flag set to `false`: `InRace → Loading` (next cup race, Versus "next track") gives 1 → **1**.
     The old session is never torn down, so tracks and racers are duplicated.
2. No transition graph is defined for restart, next cup race, rematch, next track, quit,
   results → menu, or a load failure.
3. Pillar 5 promises "instant restart". That cannot happen if restart rebuilds the track behind a
   loading screen that must show for at least 1 s.
4. The loading gate has a race. "Each loader adds/removes its key", and rc_race adds `"racers"`
   only once `Track` exists, so the set can be empty for a moment and `InRace` starts early.
5. Resources (`Track`, `Standings`, `RaceStatus`, `RaceResults`, `WeatherState`, …) are not
   despawned by `DespawnOnExit`, and no crate is named as responsible for cleaning them up.

**Fix (write into ARCH §3.1 and §6).**
- `AppState { Boot, Menu, Loading, InRace, Unloading }`. `Unloading` lasts one frame and is outside
  `RaceSession`. Every exit from a session, whether to `Menu` or to a new `Loading`, passes
  through `Unloading`.
- `impl ComputedStates for RaceSession { const ALLOW_SAME_STATE_TRANSITIONS: bool = false; … }`.
  Write the reason in the doc.
- Add a per-attempt scope:
  - `RacePhase { #[default] Setup, Intro, Countdown, Racing, Finished }`.
  - A computed `RaceAttempt`, present in every phase except `Setup`, with
    `ALLOW_SAME_STATE_TRANSITIONS = false`.
  - Per-attempt entities use `DespawnOnExit(RaceAttempt)`: racers, projectiles, hazards, the
    ghost car and the pickup state.
  - **Restart** = `RacePhase::Setup`. Attempt entities despawn. Each sim crate resets its attempt
    resources in `OnEnter(Setup)`. rc_race respawns the grid, dynamic props reset to their spawn
    poses, and the race goes to `Countdown` (Intro is skipped on restart).
  - The track, colliders, static visuals and loaded assets survive a restart. Target: under 0.5 s.
- Transition table:

  | From | Trigger | To |
  |---|---|---|
  | Menu | `race.start` | Loading |
  | Loading | all registered loaders done | InRace (`Setup` → `Intro`) |
  | Loading | load error | Unloading → Menu (+ error dialog) |
  | InRace | Restart / Retry / Rematch (same track) | `RacePhase::Setup` (no reload) |
  | InRace | Next cup race; Versus "Next track" | Unloading → Loading |
  | InRace | Quit; Results → Menu; cup finished | Unloading → Menu (React opens the right screen) |
- Loading gate: each plugin registers its key in `build()` (`LoadingTasks::register("track")`) and
  reports done. The gate opens when every registered key is done and a minimum display time has
  passed (0 in automation and headless). A plugin that is absent never blocks.
- Every crate removes or resets its session resources in `OnEnter(AppState::Unloading)`.
- Add an rc_testkit test: start → restart ×2 → next race → menu. Assert that the entity count
  returns to baseline and that there is exactly one `Track` and one grid of racers.

### B2 — Pausing `Time<Virtual>` freezes the whole React UI
**Location:** ARCH §3.1 ("`PauseState::Paused` pauses `Time<Virtual>` (so `FixedUpdate` stops)");
GDD UI-12, §14 ("springy motion, glitch/charge morph").

**Problem.** bevy-react drives animations and transitions from `Res<Time>`, which in `Update` is
virtual time:
- animated values: `crates/core/src/animations/mod.rs:84-95, 330`, `tick_animations(time: Res<Time>)`;
- style transitions: `transition/mod.rs:280`;
- morph filters: `layer/render/morph.rs:277`.

With virtual time paused, the pause menu and the Settings screen opened from it cannot animate.
Any `withTiming` or `withSpring` started during pause never finishes, so a fade-in that starts
at opacity 0 leaves the menu invisible.

**Fix.**
- Do not pause `Time<Virtual>`. When paused:
  - (a) one run condition, configured in `CorePlugin`, gates every `SimSet` on `PauseState::Running`;
  - (b) `RapierConfiguration::physics_pipeline_active = false` (rc_vehicle owns the rapier config);
  - (c) gameplay visuals (particles, skid marks, wheel spin, shake, precipitation) run
    `in_state(PauseState::Running)`;
  - (d) rc_audio pauses the Engines, Sfx and Ambience channels and ducks Music.
- Race clocks count gated fixed ticks (`race_time = ticks × dt`). Never use
  `Time<Fixed>::elapsed()` or virtual time for gameplay. Add this to PRACTICES §4.
- Test: pause for 120 frames, then check that race time and car/projectile poses have not changed.

### B3 — `rc_core` contracts are ambiguous where crates meet
**Location:** ARCH §4.3–4.6, §5.3, §5.5–5.7, §8 (`--autopilot`); GDD §5.4, §5.5, §6.2; PRACTICES §4
("Exactly one writer per component").

**(a) `CarControl` has three writers on human racers:** rc_input; rc_ai steering assist
("blend on the human's `CarControl` after rc_input writes it"); rc_ai autopilot.

Fix:
- rc_input writes only `(With<LocalPlayer>, Without<AiDriver>)`; rc_ai writes `With<AiDriver>`.
- Autopilot: `Controller::Human { slot, autopilot: bool }`. rc_race then adds `AiDriver` to that
  racer.
- Assist: split `SimSet::Input` into `InputSet::Drivers → InputSet::Assist`. The assist pass may
  change only `steer`, and only for LocalPlayer racers that have assist on.
- Record this in PRACTICES §4 as the one allowed exception to the single-writer rule.

**(b) The reset button is decided twice.** rc_vehicle flips the car if it is upside down
(`SimSet::Vehicle`, before physics). rc_race respawns it if it is not flipped (`SimSet::Race`,
after physics). Both read the same edge in the same tick but see different poses, so a flip can
be followed by a respawn.

Fix:
- rc_race alone interprets `reset` and every respawn trigger, using a new
  `VehicleTelemetry.up_dot` field.
- `RespawnCar { car, transform, kind: FlipInPlace | ToTrack }`; rc_vehicle executes both kinds.
- rc_vehicle keeps only the physical auto-right.

**(c) Effects have no owner and no rules.**
- The `ApplyEffect → StatusEffects` system sits in rc_core, which §4 calls data-only and which is
  owned by the orchestrator. Nobody owns the stacking logic.
- Popping the shield needs a writer.
- EMP "−50 % charge" must write `Charge`, which rc_vehicle owns.

Fix:
- Move the effects system to **rc_items**, which becomes the single writer of `StatusEffects` and
  consumes `ApplyEffect` from both items and race.
- Add a `DrainCharge { car, fraction }` message from items to vehicle.
- Add this table to GDD §5.5:
  - **Kinds:** `PowerLoss(mul, dur)`, `Grip(mul, dur)`, `TopSpeed(mul, dur)`,
    `Boost(top_mul, force_mul, dur)`, `Shield(dur)`, `Ghost(dur)`. Tumble stays an `ApplyImpulse`.
  - **Stacking:** same kind keeps the strongest magnitude, with duration = max(remaining, new).
    Different kinds multiply.
  - **Boosts never stack.** The active boost is the strongest of: charge 1.30/1.50, perfect
    start 1.30/1.50 for 1 s, Turbo 1.45/1.80 (top-speed and drive-force multipliers). Charge is
    not drained while Turbo is active.
  - **Hard cap:** effective top speed ≤ 1.5 × the car's base top speed, after catch-up, AI scale
    and surface. Enable CCD on the chassis; barrier colliders ≥ 0.5 u thick. Unchecked, the
    stacks reach about 1.9×: 68 u/s, 1.1 u per tick, which can tunnel through thin walls.
  - **Shield:** it blocks every hit that produces a `HitEvent` (rocket, orb, EMP including its
    charge drain, Goo explosion, balloon, entering oil, Storm Strike). It blocks all hits that
    land in the tick it pops. rc_items removes it in that same tick. A shielded car cannot be
    handed a Goo Bomb.
  - **Owner immunity:** your own projectiles and hazards ignore you for 1.0 s after they spawn.
    The Shock Orb (24 u/s, slower than every car) never hits its owner. The Goo explosion does
    hit the carrier.

**(d) Determinism is impossible with one shared `RaceRng`.** Systems in rc_items, rc_ai,
rc_weather and rc_race all draw from it. When two systems in the same set have no ordering
between them, the executor decides who draws first, so PRACTICES §4 ("reproducible") and §5
("tests must be deterministic") cannot hold.

Fix:
- Replace it with a stateless keyed generator, `rc_core::rng::draw(seed, tick, stream, key) -> u64`
  (SplitMix64 over a hash), with one named stream per purpose.
- In rc_testkit, enable schedule ambiguity detection (warn) for `FixedUpdate`.
- Keep rapier's `parallel` feature off.

**(e) Contract items that are used but not defined:**
- `LobbyOpen` (read by rc_input; it appears only in §5.3) → add to §4.6.
- `TrackSummary { outline_2d, length, checkpoints }`, written by rc_track. rc_ui needs the minimap
  outline but may not depend on rc_track.
- `RaceConfig.golden_battery: bool`. rc_race cannot read the profile, which lives in rc_game.
- `RestartRace` and `LeaveRace` messages (B1), `UiNav` (B4), and the rebind request/result types.
- `CupRace.laps`.

### B4 — How menu input reaches React is undefined, and React cannot do spatial navigation
**Location:** ARCH §5.3, §5.13 (`nav/ … spatial focus manager (gamepad/keyboard)`); GDD §12.1
"Menus", §12.2, UI-9, UI-19, Pillar 4.

**Problem.**
- bevy-react 0.8.1 has "no built-in focus navigation" and "no arrow-key or gamepad navigation"
  (`docs/guide/events/gamepad.md`, `focus.md`, Limits sections).
- React cannot read laid-out rects, so a TypeScript *spatial* focus manager has no geometry to
  work with.
- The docs never say whether React reads the raw `gamepadInput`/`keyDown` built-ins or intents
  from Bevy. Reading raw input would bypass rebinding and dead zones. It would also handle presses
  that rc_input consumes too (lobby join, the pause toggle) a second time. And it cannot express
  per-player focus: in the Versus lobby (UI-9), four players pick cars at the same time.

**Fix.**
- **rc_input is the only thing that interprets menu input.** It emits a `ui.nav` React event:
  `{ action: Up|Down|Left|Right|Confirm|Back|TabPrev|TabNext|Pause, slot: Option<u8>, device_kind, repeat }`.
  - A stick counts as a direction at 0.5, released at 0.3.
  - Holding repeats after 0.35 s, then every 0.1 s.
  - Menu keys: D-pad, left stick, A, B, LB, RB, Start; on keyboard, arrows and WASD,
    Enter/Space, Esc, Q/E. These are fixed, not rebindable.
- React `nav/` uses a **logical** focus model: vertical list, horizontal list, grid with a
  column count, tab bar. It is pure TS, tested with `node --test`. The mouse uses pointer events.
  React never subscribes to raw `gamepadInput`/`keyDown`.
- Focus contexts per slot:
  - Versus lobby: each slot's widgets react only to that slot's `ui.nav`.
  - Versus track select: P1 drives it.
  - Pause menu: any joined player.
- Rebinding:
  - React sends `input.rebind { scheme, action }`.
  - rc_input suppresses `ui.nav`, captures the next press (Esc or Start cancels), and answers
    with `input.rebound`.
  - The conflict check covers both keyboard schemes, since they share one keyboard.
- "Primary device" (§12.2) only decides which button glyphs are shown. The `input.device` event
  reports the last-used device kind per slot.

### B5 — World scale and lap-time targets contradict each other
**Location:** GDD §4.2 [CAR-4]; §5.1 [VEH-1], Gravity row; §7.1 "Target lap time 30–45 s"; §1.3.

**Problem.**
- Three different scales are in use:
  - props at ×4.4 (car = 0.45 m, so 1 u = 0.225 m);
  - "car ≈ 2.0 u ≈ 1 m" and km/h = u/s × 1.8 (1 u = 0.5 m);
  - "~2.5 g ≈ 25 u/s²" (1 u = 1 m).
- At the prop scale, real gravity is 43.6 u/s². So 25 u/s² is 0.57 g, floatier than real, not
  "stronger than real".
- More importantly, lap length. A 30–45 s lap at a Pro car's ~22 u/s average is 650–1000 u, which
  is 150–225 m of real-scale path. A living room → hallway → kitchen route is about 30–40 m real
  (130–180 u), which gives a 6–8 s lap. A back garden is similar.
- So the M2 slice track cannot meet its own spec, and the first TrackDef will hit this.

**Fix.** Replace the text of §4.2 and the §7.1 sentence:
- "1 u = 22.5 cm of the real world; props are real size × 4.44. The car is ≈ 2.0 u long and
  ≈ 1.1 u wide. Displayed speed is toy-scale km/h = u/s × 1.8, deliberately not true scale.
  Gravity is 25 u/s² (≈ 0.57 g at world scale), a feel value chosen for big air; tune it in
  25–40."
- Per-track lap length targets in TRK-1:

  | Track | Lap length | Note |
  |---|---|---|
  | Sofa Circuit | ~400 u | whole ground floor plus a stair ramp; rooms may be up to 1.5× real floor area, furniture stays ×4.44 |
  | Backyard Bash | ~450 u | |
  | Toy Town | ~650 u | |
  | Night Museum | ~750 u | |
  | Aisle Rush | ~800 u | |
  | Maple Street | ~900 u | |
- Add `TrackDef.default_laps` (Sofa Circuit and Backyard Bash 5, others 3) so each race lasts
  1.5–2.5 min. Cups and the Quick Race default use it; Time Trial keeps 3 laps.
- Racing width ≥ 8 u; chokepoints ≥ 5 u, at most 2 per lap.

### B6 — Content and progression tables are inconsistent or incomplete
**Location:** GDD §3.1 cup table, §4.1, §5.2, §7.1, TRK-2, §8.3, §9, §10; ARCH §4.1.

1. **The cup table uses values that are not in the enums:** "Day", "Store lights", "Evening", and
   "Rain" with no time. `TimeOfDay` is Morning/Noon/Sunset/Night.
2. **The Grand Volt Cup races are not specified**, and "Dynamic weather" cannot apply to its four
   indoor tracks.
3. **Laps per cup race are not specified.** Use `default_laps` (B5).
4. **TRK-2's "allowed weather/time presets" are never listed.** "Night Museum" and Aisle Rush
   ("after closing") contradict Morning and Noon.

   Proposed normalized cups (track · layout · weather · time). Every race uses content that is
   already unlocked, and every weather kind and time appears at least once:

   | Cup | Races |
   |---|---|
   | Rookie | Sofa Circuit · Fwd · Clear · Noon; Toy Town · Fwd · Clear · Morning; Backyard Bash · Fwd · Clear · Morning; Aisle Rush · Fwd · Clear · Night |
   | Pro | Maple Street · Fwd · Cloudy · Noon; Night Museum · Fwd · Clear · Night; Backyard Bash · Rev · Rain · Noon; Sofa Circuit · Rev · Clear · Sunset |
   | Elite | Aisle Rush · Rev · Clear · Night; Maple Street · Rev · Storm · Night; Toy Town · Fwd+Mirror · Clear · Noon; Night Museum · Rev+Mirror · Clear · Night |
   | Grand Volt (6) | Toy Town · Rev+Mirror · Clear · Sunset; Backyard Bash · Fwd+Mirror · Snow · Morning; Sofa Circuit · Fwd+Mirror · Clear · Night; Aisle Rush · Fwd+Mirror · Clear · Sunset; Night Museum · Fwd+Mirror · Clear · Sunset; Maple Street · Rev+Mirror · Dynamic (starts Fog) · Sunset |

   Allowed presets:

   | Track | Times | Weather |
   |---|---|---|
   | Sofa Circuit, Toy Town | all 4 | Clear |
   | Aisle Rush, Night Museum | Sunset, Night | Clear |
   | Backyard Bash, Maple Street | all 4 | all 6 |
5. **Golden Battery.** TRK-2 says "each track variant provides 1"; §9 says "one hidden per
   track (forward layout)". Change TRK-2 to: "The Forward layout, mirrored or not, hides the
   track's one Golden Battery. Reverse layouts have none."
6. **Rival cars contradict the cup-class rule.** §8.3 gives each rival one car
   (`RivalInfo.car`), yet "in cups the field uses cars of the cup's class". As written, Glitch
   would bring his Volt GT to the Rookie Cup. Use `RivalInfo.cars { rookie, pro, elite }`
   (signature car in bold):

   | Rival | Rookie | Pro | Elite |
   |---|---|---|---|
   | Dash | Sparky | **Hornet** | Phantom |
   | Mayhem | **Dumpster** | Torque | Volt GT |
   | Penny | **Zip** | Hornet | Phantom |
   | Turbo Tom | Zip | **Torque** | Volt GT |
   | Glitch | Sparky | Hornet | **Volt GT** |
   | Rusty | **Sparky** | Torque | Phantom |
   | Nova | Zip | Hornet | **Phantom** |

   In Quick Race and Versus, the AI class is the class of the highest-class human car (Secret
   counts as Elite).
7. **Surfaces used by tracks are missing from §5.2:** cardboard, patio stone (already referenced
   by "wet tile/stone"), play-mat.
   - Add `Cardboard` (grip 0.90, top-speed factor 0.95) and `Stone` (1.00, 1.00, wet ×0.75).
   - Map the rest: play-mat → Carpet, deck boards → Wood, curbs/drains → Asphalt,
     stairs → Wood.
8. **The unlock data model cannot express half of the unlock table.** In ARCH §4.1, only cars,
   tracks and cups carry an `UnlockRule`.
   - Add `Catalog.unlocks: Vec<(Unlockable, UnlockRule)>`, with
     `Unlockable::{ Reverse, Mirror, Difficulty(..), PaintSet(..), Credits }`.
   - Say that "gold medals" means Gold or Platinum.
   - Say that each paint set adds one paint to every car.
9. **Small fixes:**
   - "Eight … cars in three classes (+1 secret)" → "seven cars in three classes plus one secret
     car (8 total)".
   - Elite+ is cup-only; it is not offered in Quick Race.
   - Versus is available from the start.

### B7 — The track data model cannot express the GDD's tracks
**Location:** ARCH §5.1 (TrackDef; variants as "pure transforms"); GDD §7.1–7.3; §5.4 ("> 3 s far
off the drivable area").

**Problem.**
1. **Reverse is not a pure transform for gameplay geometry.** Ramps, jumps and drops only work in
   one direction:
   - a forward jump (ramp, gap, lower landing) becomes a drop onto the back of a ramp, or a wall;
   - the same goes for the book-stack ramp onto the coffee table, the sandbox jump, the exhibit
     stairs and the table edges.

   With 12 layouts, these failures would only surface at DoD-2 in M4.
2. **Most environments are open spaces, not a ribbon road with walls.** Floors, lawns and aisles
   are driven on directly: the floor itself, rugs, furniture tops. §7.3 also allows open lawns.
   Yet "off-track" and the "drivable area" have no data definition.
3. **Vertical loops** (Toy Town, "loop sections") break several systems: the bank-only up vector,
   `project()`, wrong-way, the camera, auto-right and the AI.

**Fix.**
- Every TrackDef feature carries `dir: Both | Fwd | Rev`: point flags, `gap` jumps (with their
  takeoff direction), props/ramps, item rows, the Golden Battery and shortcuts. Each layout must
  pass the AI 3-lap test before art polish.
- Ribbon `style: ToyTrack | Asphalt | CarpetRunner | Invisible`. Invisible lies on the floor of
  the environment, so rooms and gardens are not paved with a road strip.
- `ground` polygons (lawns, floors) with a surface.
  - **Off-track** = no wheel touches Road or Ground for > 3 s, or
    |lateral| > half-width + `offroad_margin` (per segment, default 6 u) for > 3 s.
  - A respawn point is never ahead of the last validated checkpoint.
- Cut vertical loops. Keep banked turns, half-pipe wall rides and the hose-reel spiral (a helix
  works with hint-based projection).

### B8 — A "complete game" still lacks these flows and states
**Location:** GDD §3, §12.2, §13, §14, §15; ARCH §5.12.

1. **Onboarding.** The only help is a "tip of the day".
   - Add UI-20 **How to play**, from the Main menu and Pause: controls with the slot's glyphs,
     charge/boost, items, reset.
   - Add one-time contextual HUD hints: drift → charge, boost ready, item ready, "Reset" after
     3 s stuck, wrong way.
   - Add a Gameplay setting "Hints", and `Profile.hints_seen`.
2. **The post-race flow** is called "mode-specific" but never defined, and there is no cup-end
   screen even though `podium_fanfare` and `cup_victory` exist.
   - Championship: Results → Continue (standings, UI-6) → Next race.
     - After the last race: **UI-21 Cup end** (top-3 podium, trophy, unlocks, credits on Grand
       Volt gold) → Cup select.
     - Restart is allowed from Pause and from Results; it reverts that race's points.
     - Quit cup asks for confirmation ("progress in this cup will be lost").
     - A cup in progress is not saved across app restarts.
   - Quick Race: Restart, Change track/options, Menu.
   - Time Trial: Retry (instant, B1), Change track, Menu. Show the medal, the delta to the next
     medal and "ghost saved".
   - Versus: Rematch, Next track (P1 picks), Lobby, Menu.
3. **Controller disconnect.**
   - In a race: pause with "P2's controller disconnected — press A/Start on any controller, or
     Enter, to take over · [Drop player]". In Versus, the AI takes over a dropped player's car.
   - In the lobby: the slot is freed.
   - In menus: nothing happens.
   - A reconnected pad gets a new id, so reassign by button press, never by id.
4. **Focus loss.** When the window loses focus during a race, the game auto-pauses. It resumes
   only on input.
5. **Failure states.**
   - Data or asset load failure → error dialog and back to Menu, never a panic.
   - Save failure → error toast, retried on the next save.
   - Corrupt profile or settings → keep a backup, start fresh, and show a toast.
   - No audio device → run muted.
6. **HUD.**
   - A "Race ends in 10…" countdown while the soft-lock guard runs.
   - "Waiting for other players" for a finished player in Versus.
7. **Settings and menu.**
   - "Reset progress" with a double confirmation.
   - Quitting from the Main menu asks for confirmation.

### B9 — PRACTICES conflicts with the user's rules and with parallel work
**Location:** PRACTICES §0.3, §1, §2, §7, §8; ARCH §4.7 (`paths`), §7 (fonts), §8.

1. **Every `cargo run` writes outside the repo.** It writes `settings.ron`, `profile.ron` and
   ghosts to `$XDG_CONFIG_HOME/recharged`, which PRACTICES §0.3 forbids. That directory is shared
   by all running agents and holds the user's real profile.
   - Add `--user-data-dir <path>` (and env `RECHARGED_USER_DIR`), honoured by `rc_core::paths`.
   - PRACTICES §2: every agent run uses `--user-data-dir target/agent-data/<task>`, and tests use
     temp dirs.
   - Add `--unlock-all` (held in memory, never saved) so agents can test locked content.
2. **Several agents will edit the same shared files at once**, causing read-modify-write races
   that break everyone's build: `crates/rc_core/**`, workspace `Cargo.toml`/`Cargo.lock`,
   `ui/src/strings.ts`, `ui/src/bevy.ts`, `assets/CREDITS.md`.
   - rc_core and the workspace manifests are edited only by the orchestrator, or by the one agent
     whose brief names them. Other agents request changes under Requests.
   - Split strings into `ui/src/strings/<area>.ts`.
   - Only the rc_ui owner regenerates `bevy.ts`.
   - Credits go in the agent's report; the orchestrator merges them.
3. **Fonts.** "fonts: OFL" (PRACTICES §7, ARCH §7) breaks the user's rule that assets are
   original or CC0. Use CC0 fonts (the Kenney font set is CC0), or get the user's explicit
   approval for OFL before any font is added.

---

## Non-blocking recommendations

**R1 — Many of the leader's items do nothing.** (§6.2–6.3)
- In the "1st" row, Rocket 15 + Balloons 20 can only fire forward, and Goo 10 has nobody to
  pass to: 45 % of the leader's pulls are useless.
- Fix: holding Look back while firing sends Spark Rocket, Rocket Trio and Water Balloons backward
  (the rocket homes on the nearest car behind, 35°/60 u). Set "1st" Goo to 0, Shield to 25 and
  Oil to 30.
- Telegraph rockets (Pillar 3): when a rocket locks on, show a HUD arrow and play a lock-on beep.

**R2 — Strengthen originality.** (§1.2, §6.2, §7.1)
- 9 of the 10 items map one-to-one onto Re-Volt pickups: Firework, Firework pack, Shockwave,
  Electro pulse, Putty bomb, Oil, Water balloons ×3, Battery, Global pulse. Only the Shield is new.
- 4 of the 6 environments match Re-Volt's: Toys in the Hood, SuperMarket, a museum at night, Toy
  World.
- Replace 2–3 items with items built around charge. For example:
  - *Leech Coil*: tethers you to the car ahead for 3 s and moves 30 % of its charge to you.
  - *Spring Pad*: a dropped pad that launches cars into the air.
  - *Static Field*: a dropped zone that drains charge.
- Consider replacing Night Museum with a setting Re-Volt never had (garage workshop, attic,
  kids' bedroom).

**R3 — Give weather a bigger role.** (§10)
- Only 2 of 6 tracks can have weather, and the Rookie Cup never shows any.
- On indoor tracks, show the weather through the windows: sky, rain on the glass, lightning
  flashes, rain and thunder audio. This is cheap and has no gameplay effect.
- Define Dynamic weather:
  - seeded;
  - one change at 30–70 % of race distance, to a neighbouring preset (Clear↔Cloudy↔Rain↔Storm,
    Cloudy↔Fog, Cloudy↔Snow);
  - a 20 s blend.

**R4 — Smooth everything that moves at 60 Hz, not only racers.** (ARCH §3.3)
- Rockets (55 u/s, 0.9 u per tick), orbs, balloons, dynamic props and the 20 Hz ghost will stutter
  on 120/144 Hz screens.
- Add an `Interpolated` marker:
  - poses are snapshotted in `FixedLast`;
  - the visual entity is posed in `Update` by rc_render.
- This also takes the Update-time proxy code out of the sim crate rc_vehicle.
- Never write interpolated poses into a rapier body's `Transform`.

**R5 — Stop charge farming.** (§5.3)
- Drift charges only:
  - above 40 % of top speed;
  - after 0.4 s of continuous drift;
  - at a slip angle of 10–45° (none while spinning);
  - scaled by yaw rate.
- A "clean landing" means all four wheels touch down within 0.15 s, with tilt under 20°.
- Draft cone: ±15°, at ≥ 50 % of top speed.
- Show the charge rate by spark size and pitch as well as colour (ACC-1 forbids colour-only
  information).

**R6 — Split-screen.** (GDD §11 CAM-2, §13, §16.3; ARCH §5.8)
- Add a 2P smoke test to M2. HUD, camera, listener and precipitation are all per viewport, and
  retrofitting them later is expensive.
- FOV setting = horizontal FOV at 16:9 (Hor+). Clamp the top/bottom 2P viewport to ≤ 100°
  horizontal: a 70° vertical FOV on a 1920×540 viewport is 136° horizontal.
- In 3P, put a 2D React minimap in the fourth quadrant instead of a fourth 3D camera.
- Mix local players' own engines at −6 dB each.
- `<anchor>` cannot serve race overlays: it projects through `IsDefaultUiCamera` (the full-window
  UiCamera) and draws on the main screen only (`anchor.md:84,151`). Make name tags and lock-on
  markers per-camera 3D billboards in rc_vfx.

**R7 — Rendering scope.** (GDD §10.2, §15 Graphics; ARCH §5.9)
- Bevy 0.20 SSR requires deferred rendering (`bevy_pbr-0.20.0/src/ssr/mod.rs:54`), so it cannot be
  a live toggle and it would push every material into the deferred path. Drop SSR.
- Do wet ground with roughness and darkening plus an environment-map light probe.
- Volumetric fog only at Ultra, never in split-screen.
- Shadowless headlight spots on local players' cars only.
- Offer windowed and borderless display modes, not exclusive fullscreen.

**R8 — Automation for the DoD.** (ARCH §8)
- `--input-script <file>`: timed `UiNav` and driving inputs, so DoD-1 (keyboard only / gamepad
  only) can be tested without a human.
- `--soak <min>`: chains random races through `Unloading` and asserts that entity counts return to
  baseline, which catches B1-type leaks.
- `--bench <scenario>`: p50/p95 frame times as JSON, for DoD-7.
- `--calibrate-medals`: Elite AI in a Phantom, items and catch-up off; prints targets.
- A determinism test: run the same seed twice and get identical result JSON.
- A stricter DoD-2: finish within 1.5× the expected time with ≤ 2 respawns per lap.
- Add a fast `rc_ui` example for `--export-bindings`, so codegen does not build the whole game.

**R9 — Let track authors see their work.** (ARCH §5.1 Tools)
- The top-down SVG cannot show heights or jumps. Add a side-elevation plot (height and bank
  against s, plus jump arcs at the target speed).
- Add `--track-tour <track>`, which saves screenshots along the racing line.
- Mirror prop placement and yaw, never prop meshes: mirrored meshes show text backwards.

**R10 — Art bible and asset budget, before M2.** (GDD §7, §17; PRACTICES §7)
- Per-environment palette, bevel widths, texture density (px/u), material list and references.
- Per track: ≤ 6 hero props plus a shared kit of about 15.
- At most ~40 dynamic bodies per track. Keep them light (cars barely deflect), let them sleep,
  reset them on restart, and have the AI ignore them.

**R11 — Driving feel needs a human.** (GDD §5.1, §20)
- User playtest gates at the end of M1 (test track) and M2 (the slice).
- Dev builds hot-reload `vehicles/*.ron`, `ai.ron` and `items.ron`.
- Map accel stars to 0→top time (1★ 3.5 s … 5★ 2.5 s) and assert it in tests.

**R12 — Controls.** (§12)
- Reset on D-pad ↑ sits next to D-pad steering, so respawns will happen by accident. Make respawn
  a 0.4 s hold; flipping stays instant.
- WASD also navigates menus.
- Warn in the lobby about key ghosting when two players share a keyboard.
- Right Alt is AltGr on European layouts (including Polish). Default keyboard B Look back to
  Numpad 0 / Delete.

**R13 — Race end and start.** (GDD §3.2 RACE-1, §15)
- The 45 s guard cuts off slow children. Make it 60 s after the winner, start it when the first
  human finishes in multi-human races, and show the countdown (B8).
- Auto-accelerate must never trigger the start stall.
- Give the AI a perfect-start chance by difficulty: 5 / 20 / 40 / 60 %.

**R14 — Pickups.** (GDD §6.1) Eight cars through rows of 3–4 cubes on a 5 s respawn leave the back of the
pack, who need items most, with empty rows. Use rows of 4–5 and a 2 s respawn.

**R15 — Car balance.** (GDD §4.1, §5.1)
- Bolt at 5/5/5/1 is strictly the best car and becomes the only pick in Versus. Use 5/5/3/1.
- Give Weight a cost (less air control; heavier landings), so it is a trade-off and not a free
  stat.

**R16 — Pin these numbers.** (GDD §6.3, §8.2, §9)
- Odds band: `f = (rank−1)/(N−1)`:
  - rank 1 → "1st";
  - f ≤ 0.4 → top;
  - f < 0.7 → middle;
  - otherwise last.

  With N = 8 that gives 1 | 2–3 | 4–5 | 6–8, matching Storm Strike's "6–8".
- Medals: Platinum = Elite AI time × 0.95, Gold × 0.98, Silver × 1.05, Bronze × 1.15, measured
  with a Phantom.
- Records:
  - best lap from any single-player mode;
  - best race time from Time Trial only;
  - store the car with the record.
- "AI speed scale" multiplies the AI's own per-car speed profile.

**R17 — Versus.** (GDD MODE-4, UI-9)
- A session scoreboard using Championship points.
- Names fixed as P1–P4 (no text entry).
- Fixed slot colours (cyan, yellow, magenta, green), also used for name tags.

**R18 — Saves.** (ARCH §5.12; GDD §9)
- Store a track-geometry hash with every ghost and record; discard them with a notice when it no
  longer matches.
- Migrations: serde defaults for additions; pure, tested functions on version bumps.
- Write settings at most once per second.
- Keep logs in `<user-data>/logs` (last 3 sessions).

**R19 — Layering.** (ARCH §2.1, §5.13)
- rc_ui depends on its same-layer sibling rc_game, which §2.1 forbids. Declare two layers:
  F1 = rc_game, F2 = rc_ui.
- Split rc_ui into per-namespace modules, each with one owner.

**R20 — PRACTICES additions.** (PRACTICES header, §0, §4, §5)
- A task brief may narrow scope but cannot override §1, §2 or §7.
- Bevy gotchas:
  - never read `ButtonInput` in `FixedUpdate`;
  - systems that share a `ResMut` within one set must be ordered;
  - no `<anchor>` in the race HUD;
  - only rc_vehicle writes a rapier body's `Transform`.
- Dev profile: `opt-level = 1` for the workspace, `3` for dependencies.
- Use rc_testkit only from `tests/`. Unit tests would compile the crate twice through the
  dev-dependency cycle.
- Write the `docs/tech/` digests before M1.

**R21 — UI scope.** (GDD UI-8, §14)
- Car select uses the 3D showroom backdrop, not a second `<portal>` render.
- Use the glitch/charge morph only on Main menu → mode transitions; plain fades elsewhere.

**R22 — Text fixes.** (GDD §18, §7.3, UI-2, §12, §15)
- "Reduce flashing" must also cover Storm Strike, the EMP ring, explosions and boost flashes.
- Show "Wrong way" only above 5 u/s.
- Splash: text only, no Bevy logo (PRACTICES forbids real logos).
- Draw button glyphs ourselves, without platform logos.
- "Steering assist … recovery help": define what "recovery help" does.

---

## Scope / risk assessment

| # | Risk | Why | Mitigation |
|---|---|---|---|
| 1 | Asset volume and quality | 8 cars, ~80 props, 6 environments and PBR textures, all made by agents with scripts | Art bible and kits (R10); preview review; build environments in waves |
| 2 | Driving feel | Agents cannot feel it, and it is Pillar 2 | User playtest gates and hot-reloaded tuning (R11); metric tests |
| 3 | Track authoring | 12 direction-specific layouts with in-room verticality, checked only through plots | B7, B5, R9; AI 3-lap test per layout before art |
| 4 | Split-screen cost | Shadows and post-processing per view; weather per camera | Quality tiers, R6, R7 |
| 5 | UI breadth | 15+ screens, navigation, a 4-cursor lobby, rebinding, live settings | B4, R21 |
| 6 | Session lifecycle | Restart, next race and pause cause leaks and soft-locks | B1, B2, R8 soak |
| 7 | Synthesized music | Procedural synthwave can sound amateur | 60–90 s loops with 3 stems for the final lap; 3 race loops if quality lags |
| 8 | Build contention | One target dir, Bevy + V8 builds | Orchestrator pre-builds in M1; ≤ 4 parallel coding agents per wave |

**Suggested cuts. None of them removes a feature the user asked for.**
- Vertical loops.
- SSR.
- Volumetric fog below Ultra.
- The `<portal>` car preview.
- The 3P 3D overview camera.
- Domino rows and large dynamic stacks.
- Morph transitions outside the main menu.
- Exclusive fullscreen.

**Phasing.**
- M1 does not start until B1–B4 are in the docs.
- M2 slice:
  - Sofa Circuit is the hardest first track (invisible ribbon over furniture, verticality, B5).
    Use **Toy Town** (its toy track is literally a ribbon) for the slice and do Sofa Circuit next.
  - Add the 2P split-screen smoke test and the first user playtest.
- M4: build environments in two waves (Toy Town, Maple Street, Sofa Circuit first; then Backyard
  Bash, Aisle Rush, Night Museum). Cups stay data-driven, so a track that slips can be swapped
  without code changes.

---

## What is good
- **Vision and process.** Clear vision and pillars. Requirement IDs and a measurable DoD
  (headless AI laps on every variant, a soak test, performance targets on the actual dev GPU).
- **Core mechanic.** Charge/boost is a good skill mechanic that ties drift, air and drafting
  together. It is the strongest step beyond Re-Volt.
- **Items.** Every row of the odds table sums to 100, the counterplay column is good design
  discipline, and there are no long stuns.
- **Unlock chain.** Every cup uses only content that is already unlocked, and there are no
  circular or impossible unlocks.
- **Architecture.**
  - Sim/presentation split with headless testing.
  - A dependency DAG enforced by Cargo, with no cycles.
  - Pure logic first; data-driven RON with a load test for every file.
  - The render proxy; shared values for per-frame HUD data.
  - Persistence that is atomic and versioned, with corrupt-file backup.
  - Explicit performance budgets; CLI automation hooks.
- **PRACTICES.** Specific and enforceable: ownership, build hygiene for a shared tree, testing
  rules, asset budgets with axis conventions (the Blender +Y → Bevy −Z mapping is correct), and a
  useful report template.
- **Stack.** The chosen stack is mutually compatible as pinned, and the docs already anticipate
  newer-than-training APIs (digests, reading the registry source).
