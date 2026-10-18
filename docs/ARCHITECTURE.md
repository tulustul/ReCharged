# ReCharged — Technical Architecture

| | |
|---|---|
| Status | **v2.1 — APPROVED** (reviews 1 & 2 in `docs/reviews/`; N1–N10 of review 2 applied) |
| Stack | Rust 2024 · Bevy 0.20 · bevy_rapier3d 0.37 · bevy_kira_audio 0.27 · bevy-react 0.8.1 (crate + npm) · React/TSX |
| Read with | `docs/GDD.md` (what), `docs/PRACTICES.md` (how we write code), `docs/tech/*.md` (verified API digests) |

---

## 1. Principles

1. **Simulation / presentation split.** Simulation crates (rules, physics, AI, items, weather
   state) run **headless** — no window, GPU or audio device. Presentation crates (graphics,
   VFX, cameras, audio, UI bridge) *observe* simulation state and messages; they never write
   gameplay state. Headless integration, determinism and soak tests depend on this.
2. **Contracts live in `rc_core`.** Crates talk only through types defined in `rc_core`:
   shared components, resources, messages, states, system sets and data/catalog types. A crate
   never reaches into a sibling crate's internals.
3. **Composition by decoration.** A gameplay entity (racer, pickup, projectile, prop) is
   spawned by its owning sim crate with `rc_core` components; other crates *decorate* it by
   reacting to its marker being added (an `On<Add<Racer>>` observer — Bevy 0.20 syntax — or an
   `Added<Racer>` system).
4. **One writer per piece of state.** Every component/resource has exactly one writing crate
   (tables in §4). The single documented exception is the steering-assist pass (§3.2).
5. **Data-driven content.** Cars, tracks, props, cups, rivals, item and AI tuning live in RON
   under `assets/data/`. Code holds behaviour; data holds numbers and content.
6. **Pure logic, thin systems.** Algorithms are plain Rust with unit tests; ECS systems are
   short adapters around them.
7. **Determinism.** Simulation results depend only on `RaceConfig` (incl. seed) and the input
   stream: fixed 60 Hz ticks, tick-counted clocks, a stateless keyed RNG, ordered systems, no
   parallel physics.
8. **The dependency DAG is enforced by Cargo.**

---

## 2. Workspace layout

```
bevy-volt/
├─ Cargo.toml                 workspace, shared deps, profiles
├─ crates/
│  ├─ rc_core/                L0  contracts: states, sets, components, messages, resources, catalog types, rng, paths, interpolation snapshots
│  ├─ rc_track/               L1  track data → spline, geometry, colliders, checkpoints, racing line, props, ground areas
│  ├─ rc_vehicle/             L1  RC car physics on rapier, charge/boost, telemetry, rapier configuration
│  ├─ rc_input/               L1  devices, bindings, player slots, intents → CarControl, menu navigation, rumble, lobby join, rebinding
│  ├─ rc_weather/             L1  weather plan, transitions, wetness/snow/wind/lightning → WeatherState
│  ├─ rc_race/                L2  racer spawning, phases, progress, laps, standings, reset/respawn decisions, finish, ghosts
│  ├─ rc_ai/                  L2  AI drivers, autopilot, steering assist
│  ├─ rc_items/               L2  pickups, roulette, item use, projectiles, hazards, status effects
│  ├─ rc_camera/              P   player cameras, split-screen viewports, camera modes, menu/overlay cameras, UI host camera
│  ├─ rc_render/              P   quality & post-processing, lighting/sky/time of day, materials, track/prop/car visuals, visual proxies, showroom, captures
│  ├─ rc_vfx/                 P   particles, skid marks, item/projectile visuals, precipitation, lightning, 3D name tags & lock-on markers
│  ├─ rc_audio/               P   kira buses, engines, SFX, music director, ambience
│  ├─ rc_game/                F1  app flow: boot, lifecycle transitions, modes, loading gate, pause, results, progression, persistence
│  ├─ rc_ui/                  F2  bevy-react bridge: typed channels per namespace, HUD data push, bindings export
│  ├─ rc_testkit/             T   headless app builders + helpers for integration tests (used only from `tests/`)
│  ├─ rc_pins/                —   build hygiene only: pins third-party features so per-crate builds reuse one Bevy build (no code)
│  └─ recharged/              BIN main.rs: assembles plugins, CLI flags (automation, capture, bench, bindings export)
├─ ui/                        React app (npm package; bundles → ui/dist/{vendor,app}.js)
├─ assets/                    data/ models/ textures/ audio/ fonts/ ui/   (all runtime content)
├─ tools/                     blender/ audio/ textures/ … asset generators (Python/Blender scripts)
└─ docs/                      GDD, ARCHITECTURE, PRACTICES, tech/ digests, design/ notes, tasks/ briefs, reviews/
```

### 2.1 Dependency rules
Layers: **L0** core → **L1** foundation sim → **L2** gameplay sim → **P** presentation →
**F1** flow → **F2** UI bridge → **BIN**. A crate may depend only on crates in lower layers and
only on those listed:

| Crate | May depend on (workspace) | External |
|---|---|---|
| rc_core | rc_pins (feature pins only) | bevy, serde, ron |
| rc_track | rc_core | bevy_rapier3d |
| rc_vehicle | rc_core | bevy_rapier3d |
| rc_input | rc_core | — |
| rc_weather | rc_core | — |
| rc_race | rc_core, rc_track | bevy_rapier3d |
| rc_ai | rc_core, rc_track | bevy_rapier3d |
| rc_items | rc_core, rc_track | bevy_rapier3d |
| rc_camera | rc_core, rc_track | bevy_rapier3d (obstruction rays) |
| rc_render | rc_core, rc_track | — |
| rc_vfx | rc_core, rc_track | — |
| rc_audio | rc_core | bevy_kira_audio |
| rc_game | rc_core, rc_track | — |
| rc_ui | rc_core, rc_game | bevy-react (+ serde, ts-rs as its macros require) |
| rc_testkit | rc_core, L1, L2 crates | bevy_rapier3d |
| recharged | everything | bevy-react |

- Sim crates (L0–L2) never depend on presentation or flow crates.
- Same-layer crates do not depend on each other; they share data via `rc_core`.
- `rc_testkit` is a `[dev-dependency]` used only by integration tests in `crates/*/tests/`
  (unit tests inside a crate must not use it — that would compile the crate twice).
- New external dependencies require orchestrator approval (PRACTICES §9). After any dependency
  change the orchestrator regenerates `rc_pins` (`python3 crates/rc_pins/update.py`, with no other
  build running) so `cargo test -p <crate>` keeps reusing the shared build.

---

## 3. Runtime model

### 3.1 States and the session lifecycle (`rc_core::state`)
```rust
#[derive(States, Default)] enum AppState { #[default] Boot, Menu, Loading, InRace, Unloading }

// Present while AppState ∈ {Loading, InRace}. Scope for session entities (track, colliders,
// static visuals, game cameras). ALLOW_SAME_STATE_TRANSITIONS = false — otherwise
// Loading → InRace recomputes the same value, fires an exit/enter pair and despawns the whole
// session (measured in review 1, B1). Bevy 0.20 has no ComputedStates derive: hand-write it.
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug)] struct RaceSession;
impl ComputedStates for RaceSession {
    type SourceStates = AppState;
    const ALLOW_SAME_STATE_TRANSITIONS: bool = false;
    fn compute(s: AppState) -> Option<Self> { matches!(s, AppState::Loading | AppState::InRace).then_some(Self) }
}

// Sub-state of AppState::InRace.
#[derive(SubStates, Default)] enum RacePhase { #[default] Setup, Intro, Countdown, Racing, Finished }

// Present while InRace and RacePhase != Setup. Scope for attempt entities (racers, pickups,
// projectiles, hazards, ghost car, golden battery). Attempt entities are SPAWNED in
// OnEnter(RaceAttempt) — never in OnEnter(Setup), where the scope is absent and a session that
// ends during Setup would leak them (review 2, N1).
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug)] struct RaceAttempt;
impl ComputedStates for RaceAttempt {
    type SourceStates = RacePhase;
    const ALLOW_SAME_STATE_TRANSITIONS: bool = false;
    fn compute(p: RacePhase) -> Option<Self> { (p != RacePhase::Setup).then_some(Self) }
}

// Sub-state of AppState::InRace.
#[derive(SubStates, Default)] enum PauseState { #[default] Running, Paused }
```
- Session entities: `DespawnOnExit(RaceSession)`; attempt entities: `DespawnOnExit(RaceAttempt)`;
  menu entities: `DespawnOnExit(AppState::Menu)` (or the exact 0.20 names from the digest).
- **Every exit from a session passes through `Unloading`** (one frame, outside `RaceSession`), so
  a following `Loading` always starts from a clean world.
- **Restart** = `RacePhase::Setup` without reloading: attempt entities despawn (`RaceAttempt`
  exits); every sim crate resets its attempt resources in `OnEnter(RacePhase::Setup)`; rc_track
  resets dynamic props to their spawn poses; then the phase leaves Setup, `RaceAttempt` enters and
  rc_race (and rc_items, …) **spawn attempt entities in `OnEnter(RaceAttempt)`**; the phase goes to
  `Countdown` (Intro only on the first attempt of a session). Target < 0.5 s. `PauseState` is set
  back to `Running`.
- **Resource cleanup**: each crate resets its attempt resources in `OnEnter(RacePhase::Setup)`
  and removes its session resources in `OnEnter(AppState::Unloading)`.
- **Game cameras are session-scoped** and re-bound to the new racers by slot in
  `OnEnter(RaceAttempt)`, so a restart never despawns the UI host camera (review 2, N4).

| From | Trigger | To |
|---|---|---|
| Boot | catalog/settings/profile loaded | Menu |
| Menu | race start request (`GameCommand::StartRace`) | Loading |
| Loading | every registered loader done (+ min display time) | InRace (`Setup` → `Intro`) |
| Loading | load error | Unloading → Menu (+ error dialog) |
| InRace | Restart / Retry / Rematch (same track) | `RacePhase::Setup` (no reload) |
| InRace | Next cup race; Versus "Next track" | Unloading → Loading |
| InRace | Quit; Results → Menu; cup finished | Unloading → Menu (React opens the right screen) |

**Loading gate** (`rc_core::LoadingTasks`): plugins that load something call
`LoadingTasks::register(app, "track")` in `build()` (build-time registration). On
`OnEnter(Loading)` every registered key is pending; a plugin reports completion with a
`LoadingDone("track")` message; **rc_game is the only runtime writer** of `LoadingTasks` and opens
the gate when all registered keys are done and the minimum display time has passed (1 s; 0 with
automation flags or headless). A plugin that is not added never blocks.

**Pause** (owned by rc_game; never pauses `Time<Virtual>` — bevy-react animates the UI with it):
- every `SimSet` is gated on `in_state(PauseState::Running)` (configured once in `CorePlugin`);
- rc_vehicle sets `RapierConfiguration::physics_pipeline_active = false` while paused;
- gameplay visuals (particles, skid marks, wheel spin, shake, precipitation) run
  `in_state(PauseState::Running)`; UI and menu backdrops keep animating;
- rc_audio pauses the Engines/Sfx/Ambience channels and ducks Music;
- race clocks count gated fixed ticks (`race_time = ticks × dt`); gameplay never reads
  `Time<Fixed>::elapsed()` or virtual time.
- Bevy (rc_input) **opens** the pause menu from a device; React **closes** it with the
  idempotent `race.resume` message; rc_input does not toggle pause while paused.

### 3.2 Schedules and system sets (`rc_core::schedule`)
```
PreUpdate
  InputSet::Collect      rc_input: devices → PlayerIntents (driving edges latched only while Running and no
                         menu/dialog is open, until a fixed tick consumes them; cleared on entering Running and
                         on OnEnter(Setup)), UiNav messages (suppressed while Racing and not paused), pause
                         requests, rebind capture
FixedUpdate (60 Hz; every SimSet gated on PauseState::Running, i.e. only in a running race)
  SimSet::Drivers        rc_input → CarControl (LocalPlayer & !AiDriver);  rc_ai → CarControl (AiDriver)
  SimSet::Assist         rc_ai steering assist: may change only `steer` of LocalPlayer racers with assist on
  SimSet::Effects        rc_items: ApplyEffect → StatusEffects (stacking rules), timers, shield pops
  SimSet::Items          rc_items: roulette, item use, projectile motion, hazards, pickups
  SimSet::Vehicle        rc_vehicle: suspension, tyres, drive, boost/charge, air control, RespawnCar execution
  ── bevy_rapier: SyncBackend → StepSimulation → Writeback ──
  SimSet::PostPhysics    rc_vehicle telemetry, impacts/landings; rc_items hit detection
  SimSet::Race           rc_race: progress, checkpoints, laps, standings, wrong-way, reset/respawn decisions, finish
  SimSet::Weather        rc_weather
FixedLast
  CoreSet::Snapshot      rc_core: PoseHistory snapshot of every `Interpolated` entity
Update (chained)
  FlowSet                rc_game: transitions, loading gate, results, persistence
  InterpolateSet         rc_render: pose visual proxies from PoseHistory + overstep fraction
  CameraSet              rc_camera
  VisualSet              rc_render / rc_vfx
  AudioSet               rc_audio
  UiSet                  rc_ui: forwards UiNav/events, HUD pushes, shared values (before bevy-react AnimationSet::Apply)
```
- `CorePlugin` configures and chains all sets once; crates only place systems in them.
- Systems in the same set that touch the same `ResMut`/component must be explicitly ordered;
  `rc_testkit` enables schedule-ambiguity detection (warn) for `FixedUpdate`.
- Gameplay never reads `ButtonInput`/gamepad state in `FixedUpdate`; it reads latched intents.

### 3.3 Interpolation of fixed-step motion
Anything simulated at 60 Hz that is shown on screen (racers, projectiles, hazards, dynamic props,
the 20 Hz ghost) carries `Interpolated`. `CorePlugin` snapshots `PoseHistory { prev, curr }` in
`FixedLast`. **rc_render** spawns a *visual proxy* entity per `Interpolated` sim entity, links
them with a one-to-one Bevy relationship (`VisualOf(sim)` on the proxy, relationship target
`Visual` on the sim entity with `linked_spawn`, so the proxy despawns with its sim entity) and
poses proxies in `InterpolateSet` using `Time<Fixed>::overstep_fraction()`. Teleports (spawn,
respawn, prop reset) send a `Teleported { entity }` message; the snapshot system then sets
`prev = curr` so nothing streaks across the screen. Models, wheels, VFX attachments,
cameras and audio emitters use the proxy. Nobody but rc_vehicle (and rapier) writes a rapier
body's `Transform`. Headless apps have no proxies.

### 3.4 Determinism
- Stateless keyed RNG: `rc_core::rng::draw(seed, tick, stream: RngStream, key: u64) -> u64`
  (SplitMix64 over a hash), helpers for floats/ranges/weighted picks. One `RngStream` per purpose
  (item roll, AI mistakes, weather plan, lightning, spawn jitter …); `key` is usually the
  racer index. No stateful shared RNG.
- rapier without the `parallel` feature; fixed timestep; tick-counted clocks.
- Test: two headless runs of the same seeded race produce identical result JSON.

---

## 4. Shared contracts (`rc_core`)

All types are **data only** (plus small pure helpers). Names are binding; fields may grow
additively. `rc_core` is split into modules by area (`catalog`, `race`, `vehicle`, `items`,
`input`, `weather`, `ui`, `flow`, `track`, …) so that a task brief can grant one agent additive
edits in one module.

### 4.1 Identifiers & catalog (`rc_core::catalog`)
```rust
struct CarId(String);  struct TrackId(String);  struct CupId(String);  struct RivalId(String);
struct PaintId(u8);    struct PlayerSlot(u8);   // 0..=3
enum CarClass { Rookie, Pro, Elite, Secret }
enum Difficulty { Rookie, Pro, Elite, ElitePlus }
struct TrackVariant { track: TrackId, reverse: bool, mirror: bool }
struct CarInfo { id, name, class, stars: CarStars { speed, accel, handling, weight }, drive: Drivetrain,
                 model: String, paints: Vec<PaintInfo>, unlock: UnlockRule, engine: EngineTimbre }
struct PaintInfo { id: PaintId, name, base: [f32;3], accent: [f32;3], metallic: f32, roughness: f32, set: PaintSet }
enum PaintSet { Base, Neon, Carbon, Gold, Chrome }
struct TrackInfo { id, name, environment: String, indoor: bool, difficulty: u8, preview: String, music: MusicCue,
                   default_laps: u8, allowed_times: Vec<TimeOfDay>, allowed_weather: Vec<WeatherKind>, unlock: UnlockRule,
                   hidden: bool /* dev tracks */ }
struct CupInfo { id, name, difficulty, races: Vec<CupRace { variant, weather: WeatherSetup, laps: Option<u8> }>, unlock: UnlockRule }
struct RivalInfo { id, name, color: [f32;3], aggression: f32, consistency: f32, cars: ClassCars { rookie, pro, elite } }
enum UnlockRule { Start, CupPodium(CupId), CupGold(CupId), GoldMedals(u32), GoldenBatteries(u32) }
enum Unlockable { Reverse, Mirror, Difficulty(Difficulty), PaintSet(PaintSet), Credits }
#[derive(Resource)] struct Catalog { cars, tracks, cups, rivals, unlocks: Vec<(Unlockable, UnlockRule)> }
```
`Catalog::load(root) -> Result<Catalog, CatalogError>` is pure (std::fs + ron) and validated
(every reference resolves, every cup race uses content unlocked no later than the cup).

### 4.2 Race setup (`rc_core::race`)
```rust
enum GameMode { Championship { cup: CupId, race_index: usize }, QuickRace, TimeTrial, Versus }
enum Controller { Human { slot: PlayerSlot, autopilot: bool }, Ai { rival: RivalId, difficulty: Difficulty } }
struct RacerSetup { name: String, car: CarId, paint: PaintId, color: [f32;3], controller: Controller }
enum WeatherKind { Clear, Cloudy, Rain, Storm, Fog, Snow }
enum TimeOfDay { Morning, Noon, Sunset, Night }
enum WeatherChoice { Fixed(WeatherKind), Random, Dynamic { start: WeatherKind } }
struct WeatherSetup { weather: WeatherChoice, time: TimeOfDay }
#[derive(Resource)] struct RaceConfig {
    mode, variant: TrackVariant, laps: u8, racers: Vec<RacerSetup> /* grid order */, items: bool,
    weather: WeatherSetup, ai_catch_up: bool, seed: u64,
    golden_battery: bool,            // spawn it (forward layout, single-player, not yet collected)
    ghost: Option<GhostData>,        // Time Trial
}
```
Written only by rc_game before `Loading` (and replaced for a next race); read-only otherwise.
`Human { autopilot: true }` → rc_race also adds `AiDriver` to that racer (camera/HUD stay).

### 4.3 The racer entity
Spawned by rc_race in `OnEnter(RaceAttempt)`, scoped `DespawnOnExit(RaceAttempt)`.

| Component | Writer | Readers | Meaning |
|---|---|---|---|
| `Racer { index, name, car, paint, color }` | race | all | identity |
| `LocalPlayer(PlayerSlot)` | race | input, camera, ui, audio, vfx | human (or autopilot) slot |
| `AiDriver { rival, difficulty }` | race | ai | AI-driven |
| `CarControl { throttle, brake, steer, drift, boost, use_item, look_back, reset_held }` | input (`LocalPlayer`, `!AiDriver`) / ai (`AiDriver`); assist may adjust `steer` (§3.2) | vehicle, items, race | driving intent; `use_item` is an edge consumed in the tick; `reset_held` is a level — rc_race times the hold in fixed ticks |
| `VehicleTelemetry { speed, forward_speed, rpm, throttle_load, grounded_wheels, airborne_time, slip_angle, drifting, surface, wheels: [WheelState;4], boosting, up_dot, clean_landing }` | vehicle | render, vfx, audio, ui, ai, race | physical state |
| `Charge(f32)` (0–1) | vehicle | ui, ai, vfx, audio | battery meter |
| `RaceProgress { lap, next_checkpoint, distance, total_distance, rank, finished, lap_times, best_lap, wrong_way, last_valid_checkpoint, off_track_time }` | race | ui, ai, items, game, audio, camera | standing in the race |
| `ItemSlot { item: Option<ItemKind>, roulette: f32 }` | items | ui, ai | held item |
| `StatusEffects` (timed effects; getters `power_mul()`, `grip_mul()`, `top_speed_mul()`, `boost()`, `shielded()`, `ghosted()`) | items | vehicle, render, vfx, ui, ai | effect state |
| `CatchUp(f32)` (±0.06) | ai | vehicle | catch-up top-speed scaling |
| `Interpolated`, `PoseHistory` | race (marker) / core (snapshot) | render | interpolation |
| `Visual` (relationship target of the proxy's `VisualOf`) | render | camera, vfx, audio | the visual proxy |

Rapier components on the racer are private to rc_vehicle.

### 4.4 Other gameplay entities (`rc_core` markers)
| Component | Spawned by (scope) | Decorated by |
|---|---|---|
| `TrackPiece { kind }`, `Surface(SurfaceKind)` | rc_track (session) | — (visuals come from `TrackGeometry`) |
| `Prop { kind, dynamic }` (+ `Interpolated` if dynamic) | rc_track (session; reset on Setup) | rc_render (glb scene on the proxy) |
| `Pickup { row, slot }` | rc_items (attempt) | rc_vfx |
| `Projectile { kind, owner }` (+ `Interpolated`) | rc_items (attempt) | rc_vfx, rc_audio |
| `Hazard { kind: OilSlick / SpringPad / GooBomb …, owner }` | rc_items (attempt) | rc_vfx, rc_audio |
| `GoldenBattery` | rc_race (attempt) | rc_vfx |
| `GhostCar { car, paint }` (+ `Interpolated`) | rc_race (attempt) | rc_render |
| `GameCamera { slot }`, `MenuCamera`, `OverlayCamera`, `UiHost` marker | rc_camera | rc_render (post-processing), rc_audio (listener), rc_vfx (precipitation layer) |
| `VisualOf(Entity)` (relationship, `linked_spawn` target `Visual`) | rc_render | — |

### 4.5 Messages (`rc_core::messages`)
| Message | Producer | Consumers |
|---|---|---|
| `CarImpact { car, other, strength, point, normal }` | vehicle | audio, vfx, input (rumble), camera (shake) |
| `CarLanded { car, air_time, strength, clean }` | vehicle | audio, vfx, input, camera |
| `RespawnCar { car, transform, kind: FlipInPlace / ToTrack }` | race | vehicle |
| `CarRespawned { car, kind }` | vehicle | audio, vfx, ui, ai |
| `ApplyEffect { car, effect: Effect, source: Option<Entity> }` | items, race | items (effects system) |
| `ApplyImpulse { car, linear, angular }` | items | vehicle |
| `DrainCharge { car, fraction }` / `GrantCharge { car, amount }` | items | vehicle |
| `ItemEvent { racer, kind: Picked / Ready(ItemKind) / Used(ItemKind) / LockOn { target } / LockLost }` | items | ui, audio, vfx, ai |
| `HitEvent { victim, attacker, by: ItemKind, blocked }` | items | ui, audio, vfx, input, game (stats) |
| `Explosion { position, radius, kind }` | items | vfx, audio, camera |
| `RaceEvent { racer: Option<Entity>, kind }` — CountdownTick(n), Go, PerfectStart, Stall, CheckpointSplit(delta), LapCompleted{lap,time,best}, FinalLap, Finished{rank,time}, WrongWay(bool), GoldenBattery, RaceEndsIn(secs), Hint(HintKind) | race | ui, audio, game, vfx |
| `WeatherEvent::{ Lightning { intensity }, Changed(WeatherKind) }` | weather | render, vfx, audio |
| `UiNav { action: NavAction, slot: Option<PlayerSlot>, device: DeviceKind, repeat: bool }` | input | ui (→ `ui.nav` React event) |
| `RebindRequest { scheme, action }` / `RebindResult { scheme, action, binding: Option<Binding>, conflict }` | ui / input | input / ui |
| `DeviceEvent::{ Joined { slot, device }, Left { slot }, Disconnected { slot }, LastUsed { slot, kind } }` | input | game, ui |
| `PauseRequest { slot: Option<PlayerSlot> }` | input, game (focus loss) | game |
| `UiSound(UiCue)` | ui | audio |
| `MusicStinger(MusicCue)` | ui / game (Cup-end ceremony) | audio |
| `Teleported { entity }` | vehicle (respawn), track (prop reset), race/items (spawns) | core (snapshot snap) |
| `LoadingDone(&'static str)` | every loader | game (`LoadingTasks`) |
| `LoadingFailed { key, error }` | any loader | game (→ Unloading → Menu + error dialog) |

### 4.6 Resources
| Resource | Writer | Purpose |
|---|---|---|
| `Settings { gameplay, graphics, audio, controls, accessibility }` | game (applies UI edits, persists) | GDD §15 |
| `Catalog` | game (Boot) | content metadata |
| `RaceConfig` | game | session setup |
| `RaceStatus { phase_ticks, race_ticks, countdown, total_laps, leader_lap, ends_in, attempt }` | race | ui, audio, camera |
| `Standings(Vec<Entity>)` | race | ui, items (odds), ai |
| `RaceResults { entries, ghost: Option<GhostData>, geometry_hash }` | race (on finish) | game, ui |
| `TrackSummary { outline_2d, length, checkpoints, geometry_hash }` | track | ui (minimap), game (records) |
| `WeatherState { kind, time, precipitation, wetness, snow_cover, wind, fog, cloudiness, lightning, outside_only }` | weather | vehicle, render, vfx, audio, ui |
| `LocalPlayers { slots: Vec<SlotBinding { slot, devices, last_used: DeviceKind }> }` | input | game, ui, camera |
| `LobbyState { open: bool, max_slots }` | game | input |
| `PlayerViewports(Vec<(PlayerSlot, Rect)>)` (logical px) | camera | ui |
| `MenuBackdrop { car, paint, focus }` | ui | render, camera |
| `LoadingTasks` | game (runtime; loaders register at build and send `LoadingDone`) | loading gate |
| `AutomationConfig` | binary (CLI) | game, ai, camera, render, ui, input (`--input-script`) |
| `UserPaths { data_dir, assets_dir }` | binary (CLI/env) | game (saves, logs), track/vehicle/items/ai (data) |

### 4.7 Shared tables & constants
- `SurfaceKind` (GDD §5.2: Asphalt, Stone, ToyTrack, Wood, Tile, Carpet, Cardboard, Grass, Dirt,
  Ice, Oil, Water) + `props()`.
- `ItemKind`, `HazardKind`, `Effect` (GDD §5.5), `NavAction`, `DeviceKind`, `UiCue`, `MusicCue`,
  `EngineTimbre`, `HintKind`.
- Collision layers as `u32` bits: `CAR, TRACK, PROP, PROJECTILE, PICKUP, GHOST_CAR`.
- `GRAVITY` (25 u/s²), `SIM_HZ` (60), `SPEED_DISPLAY_KMH` (1.8), `MAX_RACERS` (8),
  `MAX_LOCAL_PLAYERS` (4), `TOP_SPEED_CAP` (1.5), `SLOT_COLORS` (cyan, yellow, magenta, green).
- `rc_core::paths`: assets root (`--assets-dir`, env `RECHARGED_ASSETS`, `./assets`, next to the
  exe) and user-data dir (`--user-data-dir`, env `RECHARGED_USER_DIR`, else the OS config dir).

---

## 5. Subsystems
Each subsystem owns its crate, its data files and a short note `docs/design/<crate>.md`.

### 5.1 `rc_track`
- **TrackDef** (`assets/data/tracks/<id>.ron`): closed list of control points
  `{ pos, width, bank_deg, surface, wall_left, wall_right, offroad_margin }` (the loop's shape is
  identical in both directions), ribbon `style`
  (`ToyTrack | Asphalt | CarpetRunner | Invisible` — Invisible lies on the environment's floor),
  `gap` spans (jumps, with a takeoff direction), `ground` polygons with surfaces (lawns, floors,
  furniture tops), item rows, props `{ kind, pos, yaw, dir }`, shortcuts, golden battery, kill
  height. Every *feature* (gaps, ramps/props, item rows, shortcuts, golden battery, point flags)
  carries `dir: Both | Fwd | Rev`; a different path in one direction is modelled as a shortcut.
- **Variants:** reverse = traverse backwards and keep only `Both|Rev` features; mirror = negate X
  of positions and mirror yaw (prop meshes are never mirrored, so text stays readable), fix winding.
- **Build (pure):** centripetal Catmull-Rom → arc-length samples (0.5 u) → frames (bank; helices OK,
  no loops) → `TrackGeometry` (ribbon runs per surface, walls ≥ 0.5 u thick, curbs) → checkpoints
  (≥ 8), respawn points (15 u), grid (8 slots), item rows (4–5 cubes), racing line + speed
  profile, `geometry_hash`.
- **Runtime:** `Track` resource with `project(pos, hint_s)`, `frame_at`, `racing_line_at`,
  `target_speed_at`, `respawn_before(s, last_valid_checkpoint)`, `checkpoint_of`, `length`,
  `is_on_drivable(pos)`; writes `TrackSummary`. Spawns colliders (ribbon runs and ground areas
  tagged `Surface`, walls, room shell), props from `assets/data/props.ron` (primitive colliders,
  static or dynamic, ≤ 40 dynamic, sleeping, reset on Setup).
- **Tools:** `cargo run -p rc_track --example plot -- <track> [--rev] [--mirror]` → top-down SVG
  and side-elevation SVG (height & bank vs s, jump arcs at target speed).
- Tests: every track file loads; loops close; projection continuity; reverse/mirror invariants;
  racing line within margins; speed profile limits; checkpoint wrap-around; feature `dir` filtering.

### 5.2 `rc_vehicle`
- Spawns the body on `Added<Racer>` from `assets/data/vehicles/<car>.ron` (`VehicleParams`);
  wheel geometry must match the car model's wheel metadata (test).
- 4 raycast/shape-cast wheels, spring-damper suspension, tyre model (longitudinal + slip-based
  lateral grip × surface × weather × effects), drift, air control (weight-scaled), downforce,
  boost/charge rules (GDD §5.3, §5.5), speed cap, CCD on the chassis, auto-right, executes
  `RespawnCar` (flip in place / to track), `ApplyImpulse`, `DrainCharge`/`GrantCharge`.
- Owns `RapierConfiguration` (gravity, `physics_pipeline_active` while paused).
- Outputs `VehicleTelemetry`, `Charge`, `CarImpact`, `CarLanded`, `CarRespawned`.
- Dev builds hot-reload `vehicles/*.ron` (mtime poll).
- Tests (headless): 0→top time per accel star and top speed per speed star within 5 %; braking
  distance; turning circle; drift recovers; flip auto-right; 3 m drop is stable; no tunnelling at
  the speed cap through a 0.5 u wall; determinism.

### 5.3 `rc_input`
- Devices `KeyboardA`, `KeyboardB`, `Gamepad(Entity)`; bindings per scheme in `Settings.controls`
  (defaults GDD §12.1).
- `LocalPlayers`: single-player → slot 0 accepts all devices; Versus lobby (`LobbyState.open`) →
  Confirm on an unassigned device joins the next slot, Back leaves (`DeviceEvent`).
- PreUpdate: intents (analog + latched edges, digital steering ramp, dead zones, sensitivity,
  auto-accelerate) and the reset hold timer; FixedUpdate `SimSet::Drivers`: writes `CarControl`.
- **Menu navigation**: emits `UiNav` from the fixed menu keys (GDD §12.2: stick 0.5/0.3, repeat
  0.35 s then 0.1 s), tagged with the slot (or `None` when unassigned) and device kind. Suppressed
  while a text field has focus and while capturing a rebind.
- Pause: the Pause action → `PauseRequest` (only when not already paused). Disconnects →
  `DeviceEvent::Disconnected`; `LastUsed` for glyphs.
- Rebinding: `RebindRequest` → capture the next press (Esc/Start cancels) → conflict check across
  both keyboard schemes → `RebindResult`; **rc_game** writes `Settings` (single writer).
- Driving edges are latched only while `PauseState::Running` and no menu/dialog is open; all
  latched edges are cleared on entering `Running` and on `OnEnter(RacePhase::Setup)` (so the
  Confirm that clicks "Resume" never fires an item). `UiNav` is suppressed while
  `RacePhase::Racing` and not paused.
- Reset: rc_input forwards the button level as `CarControl.reset_held`; rc_race times the hold.
- Rumble for local players' pads from `CarImpact`, `HitEvent`, `CarLanded`, boost start.

### 5.4 `rc_weather`
Builds the weather plan from `RaceConfig.weather` and the track's indoor flag (indoor → outside
weather only: `outside_only = true`, no wetness/snow); Random/Dynamic seeded via the keyed RNG
(Dynamic: one change at 30–70 % race distance to a neighbouring preset, 20 s blend);
wetness/snow accumulation and drying; wind gusts; lightning → `WeatherEvent`. Writes
`WeatherState` only; resets in `OnEnter(Setup)`.

### 5.5 `rc_race`
- `OnEnter(RacePhase::Setup)`: reset race resources only. `OnEnter(RaceAttempt)`: spawn racers on
  the grid (`Interpolated`, scoped to the attempt), `GoldenBattery` if `RaceConfig.golden_battery`,
  `GhostCar` in Time Trial.
- Phases Intro → Countdown → Racing → Finished; perfect start / stall (`ApplyEffect`); AI
  perfect-start chance is decided by rc_ai's throttle timing.
- Progress with continuity hints; checkpoint validation; lap timing in ticks; standings (finished
  by time, then `total_distance`); wrong way (> 5 u/s for 2 s); soft-lock guard (GDD §3.2) with
  `RaceEndsIn`; projected times.
- **Sole owner of reset & respawn decisions:** reset tap (flip if `up_dot` < 0.3) / hold 0.4 s
  (respawn), kill height, off-track (> 3 s, assist 2 s), stuck (4 s) → `RespawnCar { kind }`.
- Ghost recording at 20 Hz; `RaceResults` (with ghost & geometry hash) when the race ends;
  onboarding `Hint` events (rules in GDD §13, filtered by rc_game's `hints_seen`).

### 5.6 `rc_ai`
- For `AiDriver` racers: pure-pursuit steering on an offset racing line, throttle/brake from the
  speed profile × difficulty × per-car profile, overtaking lanes, hazard avoidance, drift & boost,
  item decisions (GDD §8.1) with reaction delays, perfect-start chance, stuck recovery
  (reverse, then hold reset), `CatchUp`.
- Autopilot: an `AiDriver` on a `LocalPlayer` racer (from `RaceConfig`).
- Steering assist (`SimSet::Assist`): blends only `steer` toward the racing line for LocalPlayer
  racers with assist on.
- Data `assets/data/ai.ron`; dev hot reload.
- Tests: headless AI-only race completes 3 laps on every layout within 1.5× the expected time with
  ≤ 2 respawns per lap (`#[ignore]`, run by QA); unit tests for steering/speed maths.

### 5.7 `rc_items`
- Pickup rows → sensor overlap → roulette → item ready (`ItemEvent`); odds from
  `assets/data/items.ron` by the band formula (GDD §6.3) using `Standings`.
- Use on `CarControl.use_item` (backward when `look_back`); projectiles are kinematic with
  shape casts; the Shock Orb follows the track via `Track`; rockets home with a turn-rate limit
  and emit `LockOn`.
- **Single writer of `StatusEffects`**: consumes `ApplyEffect` from items and race, applies the
  stacking/shield/immunity rules (GDD §5.5); emits `ApplyImpulse`, `DrainCharge`/`GrantCharge`,
  `HitEvent`, `Explosion`.
- Tests: odds rows & bands; rocket hits a stationary target; shield blocks and pops; goo passes on
  contact and explodes; oil slip; spring pad launch; leech transfer; owner immunity; stacking.

### 5.8 `rc_camera` — cameras & the UI host
- **UI host rig** (bevy-react renders into the camera marked `IsDefaultUiCamera`; exactly one at a
  time, managed here):
  - *Single view* (Boot/Menu showroom, Loading, 1-player race): the full-window 3D camera itself
    hosts the UI (`IsDefaultUiCamera`) — proven by the bevy-react examples; HDR, bloom and
    frosted-glass `backdropFilter` all work.
  - *Split-screen* (2–4 players): player cameras keep HDR/post-processing with viewports; a
    full-window `OverlayCamera` hosts the UI: `Camera2d`, order 10, `clear_color:
    ClearColorConfig::Custom(Color::NONE)`, `output_mode: CameraOutputMode::Write { blend_state:
    Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING), clear_color: ClearColorConfig::None }`,
    `Msaa::Off`, `IsDefaultUiCamera` + `UiHost` (`rc_camera::overlay_camera()`; verified with the
    real React UI by T04, see `docs/design/rc_camera.md`). A `ClearColorConfig::None` overlay does
    **not** work: over HDR players its LDR texture is never cleared, so the UI accumulates frame
    over frame (translucent panels turn opaque, removed UI stays). React panels use translucent
    fills in split-screen (`backdropFilter` has no 3D frame to sample on the overlay).
  - Anti-aliasing: **TAA only in single view** (TAA is broken with `Camera::viewport` in 0.20.0);
    split-screen maps TAA → SMAA. SSAO requires `Msaa::Off`, so MSAA is never chosen while SSAO is
    on (Bevy's per-camera default is `Msaa::Sample4` — set it explicitly). rc_render owns this
    mapping.
- `GameCamera { slot }` per local player (session-scoped, re-bound to the slot's racer in
  `OnEnter(RaceAttempt)`) with `Viewport` from the split layout
  (pure function, tested); writes `PlayerViewports`. 3P: no 3D camera in the fourth quadrant
  (React shows a minimap/standings panel there).
- Modes chase/far/hood, look-back, obstruction pull-in (rapier rays, ignoring cars), FOV Hor+ with
  speed/boost widening and the 2P clamp, shake (setting), intro fly-by, finish orbit, spectate.
- Follows visual proxies (`Visual`), not physics bodies.

### 5.9 `rc_render`
- **Visual proxies** (§3.3) and their interpolation.
- **Quality:** `Settings.graphics` + split-screen tier → camera components (HDR, tonemapping,
  bloom, SSAO, AA, motion blur, distance fog; volumetric fog only at Ultra in single view) and
  shadow/cascade config. No SSR. Display mode windowed/borderless, vsync, window size.
- **Lighting & sky:** per environment (`assets/data/environments/<env>.ron`) and `TimeOfDay`: sun
  + atmosphere (outdoor), window light + lamps/work lights (indoor), environment light probes;
  blends from `WeatherState` (cloudiness, fog, lightning, wetness: darker & glossier ground,
  outside-only weather through windows). Headlights (shadowless spots) on local players' cars at
  Night.
- **Materials:** `MaterialLibrary` of PBR surface materials and car paints.
- **Track visuals:** meshes from `TrackGeometry` (styled ribbon or none for Invisible), barriers,
  ground, room shell, prop glb scenes, decorative-only props.
- **Car visuals:** car glb on the proxy, paint, wheel spin/steer/suspension from telemetry,
  brake/head lights, ghost/respawn translucency.
- **Showroom** for menus from `MenuBackdrop`; **captures**: track previews, car thumbnails,
  `--track-tour`.

### 5.10 `rc_vfx`
Pooled particle system (CPU sim, shared meshes/materials for batching; counts by quality);
emitters from telemetry (drift sparks by charge rate, wheel debris by surface, boost flames,
landing dust, splashes) and messages (impacts, explosions, EMP ring, lightning, respawn,
confetti); skid marks; visuals for `Pickup`, `Projectile`, `Hazard`, `GoldenBattery`, leech
tether; per-`GameCamera` precipitation on its own `RenderLayers`; 3D name tags (slot colours) and
lock-on markers per camera; reduce-flashing variants.

### 5.11 `rc_audio`
Kira channels: Music, Sfx, Engines, Ui, Ambience (volumes = master × bus). Engines per racer
(playback rate from rpm & load; other racers attenuated/panned by distance to the nearest
`GameCamera`; own engine −6 dB each in split-screen); one-shots from messages; projectile loops;
stingers; UI cues from `UiSound`; music director (state, track cue, synchronized final-lap
variant, pause duck, crossfades); ambience by environment, time, weather (muffled outside rain
indoors). Pause pauses Engines/Sfx/Ambience. No audio device → muted, no panic.

### 5.12 `rc_game` (F1)
- Boot: `Catalog`, `Settings`, `Profile` (from `UserPaths.data_dir`), window settings → Menu.
- `GameCommand` messages (from rc_ui): `StartRace(RaceRequest)`, `Restart`, `NextRace`,
  `Rematch`, `NextTrack(TrackVariant)`, `QuitToMenu`, `Pause/Resume`, `OpenLobby/CloseLobby`,
  `ApplySettings(Settings)`, `ResetProgress`, `QuitGame`. Validates unlocks (`--unlock-all`
  overrides in memory only).
- Lifecycle transitions (§3.1), loading gate, pause (incl. window focus loss and controller
  disconnect), cup runs (`CupRun`), Time Trial ghosts, Versus lobby & `VersusSession` scoreboard,
  post-race flows (GDD §3.3).
- Results: points, records, medals, trophies, Golden Batteries, stats, unlock evaluation (pure,
  tested) → save → UI.
- Persistence in the user-data dir: `settings.ron` (≤ 1 write/s), `profile.ron`,
  `ghosts/<variant>.ghost`; versioned with tested migrations; atomic writes; corrupt → backup +
  defaults + toast; logs of the last 3 sessions in `logs/`.

### 5.13 `rc_ui` (F2) + `ui/`
- **Rust**: one module per namespace (`app`, `menu`, `race`, `hud`, `lobby`, `settings`,
  `input`, `profile`, `ui`), one owner each. All bevy-react channels; `register_bindings(app)`
  shared by the app and the `export_bindings` example (`cargo run -p rc_ui --example
  export_bindings -- ui/src/bevy.ts`, no full game build needed).
  - events (Bevy → React): `app.state`, `ui.nav`, `input.device`, `race.setup`, `race.hud`
    (batched, all players, ≤ 10 Hz), `race.event`, `race.results`, `cup.standings`,
    `versus.score`, `profile.unlocked`, `lobby.slots`, `viewports`, `loading.progress`,
    `toast`.
  - messages (React → Bevy): `ui.ready`, `ui.error`, `ui.sound`, `menu.backdrop`, `race.start`,
    `race.restart`, `race.next`, `race.resume`, `race.quit`, `lobby.open/close`,
    `settings.apply`, `input.rebind`, `profile.reset`, `hud.bind/unbind`, `app.quit`.
  - requests: `app.snapshot`, `race.snapshot`, `catalog.get`, `profile.get`, `settings.get`,
    `records.get`, `bindings.get`, `lobby.snapshot` — **every store fetches a snapshot on mount**
    (events are not buffered).
  - per-frame visuals (gauges, charge bar, minimap dots) via shared values set from Rust
    `.after(AnimationSet::Tick).before(AnimationSet::Apply)`.
  - `ReactUiPlugin.hot_reload(cfg!(debug_assertions))`; missing `ui.ready` 15 s after start is
    an error (automation runs exit non-zero).
  - rc_ui owns Bevy's `UiScale` = min(w/1920, h/1080) of the window's logical size (menus are
    authored at 1080p); converting window logical px (e.g. `PlayerViewports`) to UI px divides by
    it.
- **React (`ui/src`)**:
  ```
  index.tsx · App.tsx (ErrorBoundary → ui.error; switch on app.state) · bevy.ts (generated)
  state/      stores over events + snapshot requests (hmr-safe singletons on globalThis)
  nav/        logical focus model (list, row, grid, tabs) + per-slot focus contexts, driven ONLY by ui.nav (+ pointer)
  theme/      design tokens (colours, fonts, spacing, motion)
  components/ Button, Panel, Tabs, Slider, Toggle, Select, Carousel, StatBar, Toast, Dialog, Glyph…
  screens/    Title, MainMenu, CupSelect, CupStandings, CupEnd, TrackSelect, CarSelect, Lobby, Garage, Records, HowToPlay, Settings, Credits, Loading, Pause, Results, VersusScore
  hud/        Hud (per viewport rect from `viewports`), Position, LapTimer, ItemSlot, Speed, ChargeMeter, Minimap, Countdown, Notifications, Warnings, Hints, Finish
  strings/    <area>.ts — every user-facing string
  sound.ts    UI cue helper
  ```
  React never subscribes to raw `gamepadInput`/`keyDown` for navigation; an on-screen keyboard is
  not needed (Versus names are fixed P1–P4).

---

## 6. Key flows

### 6.1 Start → race → results
```
React race.start(setup) → rc_ui → GameCommand::StartRace → rc_game validates, writes RaceConfig
  → Loading: rc_track builds Track/colliders ("track"); rc_render loads/builds visuals ("visuals");
             rc_audio loads banks ("audio"); … each marks its LoadingTasks key
  → gate open → InRace/Setup (attempt resources reset) → RaceAttempt enters: rc_race spawns racers →
             decorators attach physics (vehicle), brains (ai), item slots (items), proxies & models
             (render), engines (audio); session game cameras re-bind to the new racers
  → Intro → Countdown → Racing … → Finished: rc_race writes RaceResults
  → rc_game: points/records/unlocks → save → rc_ui pushes race.results → React Results screen
```
### 6.2 Restart / next race / quit
- Restart: `race.restart` → rc_game → `RacePhase::Setup` (+ `PauseState::Running`) → attempt
  scope despawns, resets run → Countdown.
- Next cup race / next track: rc_game writes the new `RaceConfig` → `Unloading` → `Loading`.
- Quit: `Unloading` → `Menu`; React opens the screen the mode returns to.

### 6.3 One fixed tick
`intents → CarControl (drivers) → assist → effects → items → vehicle forces → rapier →
telemetry & contacts → race → weather → (FixedLast) pose snapshots`; then in `Update`:
`flow → interpolate proxies → cameras → visuals/vfx → audio → UI`.

---

## 7. Data files

| Path | Owner | Content |
|---|---|---|
| `assets/data/catalog.ron` | orchestrator (designer) | cars, paints, tracks, cups, rivals, unlocks |
| `assets/data/medals.ron` | generated (`--calibrate-medals`) | medal targets per layout |
| `assets/data/vehicles/<car>.ron` | vehicle | physics tuning |
| `assets/data/tracks/<track>.ron` | track designers | layout, surfaces, ground areas, props, items, features with `dir` |
| `assets/data/environments/<env>.ron` | render | lighting/sky per time of day, materials, ambience ids, windows |
| `assets/data/props.ron` | track + render | prop kind → model path, collider primitives, mass, surface, light attachments |
| `assets/data/items.ron` | items | odds table, item tuning |
| `assets/data/ai.ron` | ai | difficulty profiles, personalities |
| `assets/models/cars/<car>.glb` + `<car>.wheels.ron` | art | model (nodes `body`, `wheel_fl/fr/rl/rr`; materials `paint`, `paint_accent`, `light_head`, `light_tail`, `light_brake`) + wheel metadata |
| `assets/models/props/<env>/<prop>.glb` | art | prop models |
| `assets/textures/<set>/…` | art/render | PBR texture sets |
| `assets/audio/{music,sfx,ambience}/*.ogg` | audio | sounds |
| `assets/fonts/*` | ui | fonts (+ licence files) |
| `assets/ui/…` | ui / capture | track previews, car thumbnails, icons, glyphs |

All RON types derive `Serialize, Deserialize` (with `#[serde(default)]` where sensible); every data
file is loaded and validated by a test of its owning crate.

---

## 8. Automation & test hooks
`recharged` CLI → `AutomationConfig`:

| Flag | Effect |
|---|---|
| `--user-data-dir <path>` / `--assets-dir <path>` | redirect saves/logs and assets (agents always pass `--user-data-dir target/agent-data/<task>`) |
| `--unlock-all` | everything unlocked, in memory only |
| `--quick-race <track> [--reverse] [--mirror] [--car <id>] [--ai <n>] [--laps <n>] [--weather <w>] [--time <t>] [--players <n>] [--seed <n>] [--mode tt\|versus\|cup:<id>]` | skip menus |
| `--autopilot` | local players driven by the AI |
| `--input-script <file>` | timed `UiNav` + driving intents (DoD-1 tests without a human) |
| `--exit-after <secs>` | quit after N seconds of wall time |
| `--screenshot <path> [--screenshot-at <s>] [--screenshot-every <s>]` | captures |
| `--headless-race` | no window/render/audio: run the race sim as fast as possible, print result JSON |
| `--soak <minutes>` | chain random races through `Unloading`; assert entity/resource counts return to baseline |
| `--bench <scenario>` | p50/p95 frame times as JSON |
| `--calibrate-medals` | Elite AI in a Phantom, items & catch-up off, on every layout → `medals.ron` |
| `--capture-previews`, `--track-tour <track>` | preview images; screenshots along the racing line |
| `--export-bindings <path>` | write the TS bindings and exit (also `cargo run -p rc_ui --example export_bindings`) |

`rc_testkit`: headless app builders (MinimalPlugins, states, rapier, sim plugins, manual time,
ambiguity detection), race helpers (enter race, step seconds, restart, next race), lifecycle
test (start → restart ×2 → next race → menu, **and leaving the session while still in `Setup`
via quit and via a load error**: entity counts back to baseline, one `Track`, one grid), pause
test (120 paused frames: no change), determinism test.

---

## 9. Performance budget (1080p, RTX 3070, single-player High)
| Area | Budget |
|---|---|
| Fixed sim tick (8 cars, items, rapier) | ≤ 2.0 ms |
| Draw calls | ≤ 1500 per view (instanced props & particles) |
| Particles alive | ≤ 4000 total (scaled by quality) |
| Track geometry | ≤ 1.5 M triangles visible per view |
| Car model | ≤ 25 k triangles |
| Dynamic props | ≤ 40 per track, sleeping when at rest |
| UI | no per-frame React renders; HUD events ≤ 10 Hz |
| Load time | ≤ 3 s Loading → Intro; restart ≤ 0.5 s |
Split-screen tiers: fewer shadow cascades, no SSAO and no volumetric fog at 3–4 players.

---

## 10. Risks & mitigations
| Risk | Mitigation |
|---|---|
| APIs newer than agents' knowledge | verified digests in `docs/tech/`; read registry sources; compile early |
| Physics instability | substeps / 120 Hz option; clamped forces; speed cap; CCD; headless stability tests |
| Parallel agents breaking builds | crate ownership; rc_core & manifests changed only by the orchestrator or the named agent; per-crate `cargo check -p` |
| Session leaks / soft-locks | §3.1 lifecycle, lifecycle & soak tests |
| Split-screen rig unverified | prototype first in the camera task, screenshot-verified |
| Asset volume | scripted generators, art bible, prop kits, two environment waves |
| Driving feel without a human | metric tests, hot-reloaded tuning, optional user playtests |
| Split-screen GPU cost | quality tiers; per-camera precipitation kept cheap |
