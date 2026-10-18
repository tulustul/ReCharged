# rc_core — contract layer (design note)

`rc_core` holds every type crates share (ARCHITECTURE §3–§4) plus the few generic systems the
architecture assigns to core: state registration, system-set configuration, message
registration, the fixed rate (`SIM_HZ`) and the `PoseHistory` snapshot. Everything is re-exported
at the crate root; subsystems write `use rc_core::prelude::*;` and add `CorePlugin` first (after
`DefaultPlugins`, or `MinimalPlugins` + `StatesPlugin` headless).

## Module map (module = area; briefs grant additive edits per module)
| Module | Contents |
|---|---|
| `state` | `AppState` (Boot, Menu, Loading, InRace, Unloading), computed `RaceSession` / `RaceAttempt` (hand-written, `ALLOW_SAME_STATE_TRANSITIONS = false`), sub-states `RacePhase` (default Setup), `PauseState` |
| `schedule` | `InputSet::Collect` (PreUpdate, after `InputSystems`), `SimSet::*` (FixedUpdate, chained, each gated on `PauseState::Running`), `CoreSet::Snapshot` (FixedLast), `FlowSet → InterpolateSet → CameraSet → VisualSet → AudioSet → UiSet` (Update, chained), `SIM_SETS` |
| `interp` | `Interpolated` (requires `Transform`, `PoseHistory`), `PoseHistory { prev, curr }` + `interpolate(alpha)`, `Teleported`, `VisualOf` / `Visual` (one-to-one, `linked_spawn`) |
| `catalog` | ids (`CarId`, `TrackId`, `CupId`, `RivalId`, `PaintId`, `PlayerSlot`), `CarInfo`, `PaintInfo`, `TrackInfo`, `CupInfo`/`CupRace`, `RivalInfo`/`ClassCars`, `UnlockRule`, `Unlockable`, `EngineTimbre`, `MusicCue`, `Catalog::{load, parse, validate, rule_implies}`, `CatalogError` |
| `race` | `GameMode`, `Controller`, `RacerSetup`, `RaceConfig`, `GhostData`, `Racer` (requires the shared racer components), `LocalPlayer`, `AiDriver`, `RaceProgress`, `RaceStatus`, `Standings`, `RaceResults`, `RaceEvent`/`RaceEventKind`, `HintKind`, `GoldenBattery`, `GhostCar` |
| `vehicle` | `CarControl`, `VehicleTelemetry`/`WheelState`, `Charge`, `CatchUp`, `CarImpact`, `CarLanded`, `RespawnCar`/`RespawnKind`, `CarRespawned`, `ApplyImpulse`, `DrainCharge`, `GrantCharge` |
| `items` | `ItemKind`, `HazardKind`, `Effect` (data), `StatusEffects` + getters, `ItemSlot`, `Pickup`, `Projectile`, `Hazard`, `ApplyEffect`, `ItemEvent`, `HitEvent`, `Explosion` |
| `input` | `InputDevice`, `DeviceKind`, `ControlScheme`, `DriveAction`, `Binding`, `ActionBindings` (GDD §12.1 defaults), `NavAction`, `UiNav`, `RebindRequest`/`RebindResult`, `DeviceEvent`, `PauseRequest`, `LocalPlayers` |
| `camera` | `GameCamera`, `MenuCamera`, `OverlayCamera`, `UiHost`, `PlayerViewports` |
| `weather` | `WeatherKind`, `TimeOfDay`, `WeatherChoice`, `WeatherSetup`, `WeatherState`, `WeatherEvent` |
| `track` | `SurfaceKind` + `props()` (GDD §5.2 incl. wet modifiers), `SurfaceProps::with_weather`, `TrackPiece`, `Surface`, `Prop`, `TrackSummary` |
| `ui` | `UiCue` (+ `stem()` = `assets/audio/sfx/<stem>.ogg`), `UiSound`, `MenuBackdrop`, `BackdropFocus` |
| `flow` | `LoadingTasks` (build-time `register`), `LoadingDone`, `LoadingFailed`, `LobbyState`, `AutomationConfig` (+ `QuickRaceArgs`, `ScreenshotPlan`, `is_automated`) |
| `settings` | `Settings { gameplay, graphics, audio, controls, accessibility }` with GDD §15 defaults, RON round-trip, `#[serde(default)]` everywhere |
| `paths` | `UserPaths`, pure `resolve_assets_dir` / `resolve_user_dir` (CLI → env → defaults) |
| `rng` | `draw(seed, tick, stream, key)` (SplitMix64 chain), `unit`, `range`, `index`, `chance`, `weighted`, `RngStream` (fixed discriminants) |
| `consts` | `GRAVITY`, `SIM_HZ`, `SIM_DT`, `SPEED_DISPLAY_KMH`, `MPH_PER_KMH`, `MAX_RACERS`, `MAX_LOCAL_PLAYERS`, `TOP_SPEED_CAP`, `SLOT_COLORS`, `layers::*`, tick helpers |

## Decisions and deviations from ARCHITECTURE
- **Computed states derive no `States`**: Bevy implements `States` for every `ComputedStates`
  (blanket impl), so ARCH §3.1's `#[derive(States)]` on them does not compile.
- **`LoadingFailed { key, error }`** added next to `LoadingDone`: the §3.1 "load error →
  Unloading → Menu" path needs a signal from loaders.
- **`camera` module** added (camera markers were not assigned to a module). T04's grant names only
  `input.rs`; it should cover `camera.rs` too.
- Messages live in their area modules (no `messages` module); weather enums live in `weather`.
- **`Racer` requires** `Interpolated, CarControl, VehicleTelemetry, Charge, RaceProgress,
  ItemSlot, StatusEffects`, so any spawner (rc_race, tests) gets a complete racer.
- **`PoseHistory`** starts at the spawn pose (insert hook) and a freshly added history counts as a
  teleport, so spawns never streak even without a `Teleported` message.
- `Effect::Boost` carries `freeze_charge` (Turbo Cell stops charge drain, GDD §5.5).
- `Settings` has no version field; rc_game (T07) versions the file envelope.
- Bevy's `serialize` feature is on (serde for `KeyCode`/`GamepadButton` in bindings, `Vec3` in ghosts).
- Rapier setup is `rc_vehicle::PhysicsPlugin` (fixed schedule, `SIM_DT`, gravity, set edges,
  frozen while paused), shared by the game and `rc_testkit`.
- **`rc_pins`** (build hygiene, no code): Cargo resolves features per invocation, so
  `cargo test -p rc_core` and `cargo build -p recharged` resolved shared crates (`glam`, `uuid`,
  `serde_json`, …) differently and each compiled its own Bevy (+2.5 min, +1.4 GB per variant).
  `rc_core` depends on `rc_pins`, which pins those crates' features to the full workspace's, so any
  `-p` build reuses the one shared build (verified: `rc_core` tests after a full build compile only
  `rc_core`). After changing dependencies run `python3 crates/rc_pins/update.py` (reads Cargo's unit
  graph; run it while no other build is running). To drop it: remove `rc_pins` from `rc_core`.

## Tests
`cargo test -p rc_core`: RNG determinism, pinned value and stream independence; catalog file +
validation (references, unlock order of cup content, presets, classes, duplicates, parse errors);
settings RON round-trip and defaults; surface table and weather scaling; path precedence; pose
interpolation, snapshot and teleport snapping; proxy despawn; loading gate; bindings defaults.
Lifecycle and pause behaviour: `crates/rc_testkit/tests/`, `crates/rc_game/tests/flow.rs`.

## Additions after T01 (orchestrator-maintained)
| Item | Module | Added by | Purpose |
|---|---|---|---|
| `MusicStinger(MusicCue)` message | `audio` | orchestrator (T09 request) | UI/flow-timed stingers (Cup-end ceremony) |
| `WeatherState.snowing` | `weather` | T15 | precipitation is snow (also during Snow blends) |
| `CycleCamera { slot }` message | `input` | T04 | camera-mode cycle / spectate target change |
| `SHOWROOM_FOCUS` const | `camera` | T04 | where rc_render builds the menu turntable (car faces −Z) |
| `LoadingFailed { key, error }` message | `flow` | T01 | loader error → Unloading → Menu + error dialog |
| `ShowroomReady` resource | `ui` | T05 | rc_render inserts it while the menu showroom renders the car; React leaves the centre to the 3D car |
| `HintsSeen(Vec<HintKind>)` resource | `flow` | T07 | written by rc_game from the profile; rc_race filters hints |
| `PlayerDropped { slot }` message | `flow` | T07 | rc_game → rc_race: AI takes over the slot's racer |
| `AutomationConfig::soak_seed` | `flow` | T07 | `--soak-seed <n>` |
| `ItemTarget(Option<Entity>)`, `ItemTimer(f32)`, `ItemRadius(f32)` components | `items` | T12 | rocket lock / leech target / goo carrier; cube respawn, lifetimes, fuse; drawn size = trigger size |
| `COUNTDOWN_TICKS` const | `race` | T06 | 180; race tick 1 is the 181st tick after entering Countdown |
| `RaceEventKind::MissedCheckpoint` | `race` | orchestrator (B15) | rc_race: a gate passed outside it; HUD "go back" notice |
