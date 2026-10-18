# Bevy 0.20 API digest for ReCharged

Target: **Bevy 0.20.0** (2026-10-08), **bevy_rapier3d 0.37.0** (see `rapier-notes.md`), **bevy_kira_audio 0.27.1** (see `kira-notes.md`), Rust 1.99, edition 2024.
Written for developers whose Bevy knowledge stops around 0.14–0.18. It covers only what changed or is easy to get wrong.

How this was checked (2026-10-18):
- Ground truth was the crate source in `~/.cargo/registry/src/index.crates.io-*/bevy_*-0.20.0/`, plus the official 0.20.0 examples (git tag `v0.20.0`) and the bevy.org migration guides 0.15→0.20.
- Tags:
  - **[C]** the snippet compiled (`cargo check`) in a scratch crate using `bevy = { default-features = false, features = ["3d", "ui"] }`, bevy_rapier3d 0.37 and bevy_kira_audio 0.27.
  - **[T]** a headless `cargo test` ran it and passed.
  - **[G]** it ran on the GPU (RTX 3070, Vulkan, windowless render to an `Image`) and the screenshot was inspected.
  - **UNVERIFIED** is stated wherever it applies.
- Snippets are excerpts from that crate. They assume `use bevy::prelude::*;` plus the imports shown.

---

## 0. The traps that will bite first

1. **Shaders are WESL now.** naga_oil is gone. A `.wgsl` file is passed to wgpu raw: no `#import`, no `#ifdef`, no `#{MATERIAL_BIND_GROUP}`. Any shader that imports `bevy_pbr::…` must be a `.wesl` file using `import …;`, `@if(DEF)` and `@group(constants::MATERIAL_BIND_GROUP)`. See §7. [G]
2. **TAA is broken on split-screen viewports.** `TemporalAntiAliasing` on a camera with `Camera::viewport = Some(..)` renders the frame squeezed to half width. The TAA pass sets the camera viewport but samples full-texture UVs (`bevy_anti_alias/src/taa/mod.rs:211`). Either use SMAA/FXAA/MSAA with viewports, or use the per-player `ViewportNode` layout from §9, which works with TAA. [G]
3. **Bundles are gone.** No `PbrBundle`, `Camera3dBundle`, `SceneBundle` and so on. Spawn tuples of components; required components fill in the rest.
   - A bundle tuple holds **at most 15 elements**. Nest tuples for more: `(a, b, (c, d, e), …)`. [C]
   - The same component type twice in one bundle **panics at runtime**. To override a component from a helper bundle, call `.insert(..)` afterwards.
4. **Events are split in two.** Buffered "events" are now **Messages**: `#[derive(Message)]`, `MessageWriter::write`, `MessageReader::read`, `add_message`, `on_message`. `#[derive(Event)]` is only for observers (`On<E>`, `commands.trigger`). [C][T]
5. **Lifecycle observer syntax changed (0.20).** It is `On<Add<Car>>`, `On<Remove<Car>>`, `On<Insert<..>>`, `On<Discard<..>>` (was `Replace`) and `On<Despawn<..>>`. Not `On<Add, Car>`. [C]
6. **Scenes were renamed.** `SceneRoot` → **`WorldAssetRoot`**, `SceneInstanceReady` → **`WorldInstanceReady`** (`bevy::world_serialization`). `bevy::scene` is now the new BSN system. [C]
7. **Resources are components on their own entities.** `Query<Entity>`, `Query<EntityRef>` and `Query<EntityMut>` match resource entities. A broad `Query<EntityMut>` next to `ResMut<T>` is an access conflict (B0002); add `Without<IsResource>`. [T]
8. **`AssetServer::load_with_settings` is removed.** Use `server.load_builder().with_settings(|s: &mut S| ..).load(path)`. [C]
9. **`Assets::get_mut` returns `Option<AssetMut<A>>`**, so bind it mutably: `let Some(mut m) = mats.get_mut(&h)`. `AssetEvent::Modified` fires only if you actually write through it. [C]
10. **Run-condition combinators were renamed.** `.and()`/`.or()` → `.and_then()`/`.or_else()`, or `.and_eager()`/`.or_eager()`. [C]
11. **`NextState::set(x)` re-runs `OnExit`/`OnEnter` even when `x` is already the current state.** Use `set_if_different(x)` to skip that. `set_if_neq` no longer exists.
12. **Hierarchy.** `Parent` → `ChildOf(e)`; `despawn()` is recursive (no `despawn_recursive`); `remove_children` → `detach_children`. [T]
13. **`Query::single()` returns `Result`** (`get_single` is gone). `Single<D, F>` and `Populated` skip the system when they don't match. A missing `Res<T>` **panics** unless you wrap it in `Option<..>` or the new `If<..>`. [T]
14. **Lights.** `shadows_enabled` → `shadow_maps_enabled`. The `AmbientLight` resource → `GlobalAmbientLight`; `AmbientLight` is now a per-camera component. [C]
15. **Camera fields became components.** `Camera.hdr` → `Hdr` component; `Camera.target` → `RenderTarget` component; `Msaa` is a per-camera component. [C][G]
16. **Rendering types moved crates.** Cameras are in `bevy::camera`, lights in `bevy::light`, bloom/DoF/motion blur in `bevy::post_process`, TAA/SMAA/FXAA in `bevy::anti_alias`, meshes in `bevy::mesh` and shaders in `bevy::shader`.
17. **Tests must finish the app themselves.** Call `app.finish(); app.cleanup();` before the first manual `app.update()`. Plugins such as rapier add required plugins in `finish()`. [T]
18. **Turn on `bevy/debug` in development.** Without it, system names in error messages print as `<Enable the debug feature to see the name>`. [T]
19. **`Tonemapping::None` is now a raw passthrough.** It skips grading and dither too. Use `Tonemapping::Linear` for "no curve". [C]
20. **`ScreenSpaceTransmission` is opt-in (0.20).** Glass and liquid transmission needs that component on the camera. [C]
21. **Computed states re-fire on unchanged values.** `ComputedStates::ALLOW_SAME_STATE_TRANSITIONS`
    defaults to `true`: when a source state changes but the computed value stays the same (e.g.
    `Loading → InRace` both mapping to `Some(RaceSession)`), an exit/enter pair fires and
    `DespawnOnExit(RaceSession)` entities are despawned. Scope states must hand-write the impl
    (there is **no `ComputedStates` derive**) with `const ALLOW_SAME_STATE_TRANSITIONS: bool = false;`
    (`bevy_state-0.20.0/src/state/computed_states.rs:79`; verified by the design review probe).

---

## 1. Cargo setup (features, kira, profiles)

```toml
[dependencies]
# Bevy's `default` = ["2d", "3d", "ui", "audio"]. `audio` (bevy_audio) clashes with
# bevy_kira_audio, so turn defaults off and pick collections.
bevy = { version = "0.20", default-features = false, features = ["3d", "ui"] }   # [C]
bevy_rapier3d = "0.37"
bevy_kira_audio = { version = "0.27", features = ["wav"] }   # its defaults: ogg + spatial

[features]
# cargo run --features dev
dev = ["bevy/dynamic_linking", "bevy/file_watcher", "bevy/debug"]

[profile.dev]
opt-level = 1
[profile.dev.package."*"]
opt-level = 3
```

- **Mixing `bevy` default features with kira breaks the build.** With the defaults on, `bevy::prelude::*` and `bevy_kira_audio::prelude::*` both export `AudioSource`, so `Handle<AudioSource>` fails with `error[E0659]: AudioSource is ambiguous` [C]. You also get two audio backends and two `.ogg` loaders. Cargo can't subtract a feature from `default`: list the collections yourself.
- **What the collections contain** (from `bevy-0.20.0/Cargo.toml`):
  - `3d` = `default_app` + `default_platform` + `3d_bevy_render` + `scene` + `picking`.
  - `3d_bevy_render` = pbr, gltf (+ `gltf_animation`), core_pipeline, post_process, anti_alias and gizmos_render.
  - `3d_api` = `bevy_light`, `bevy_mikktspace` (tangents), `ktx2`, `zstd_rust`, `tonemapping_luts`, `smaa_luts`, morph.
  - `common_api` = png, hdr, text, window, mesh, material, curve and so on.
  - `default_platform` = winit, x11, wayland, gilrs (gamepads), `default_font`, `multi_threaded`, `sysinfo_plugin`.
  - `ui` = bevy_ui + rendering + `bevy_ui_widgets`.
- **Not in any collection, add them yourself:**
  - images: `jpeg`, `webp`, `exr`, `basis-universal`
  - dev tools: `file_watcher` (asset hot reload), `bevy_dev_tools` or `dev` (FPS overlay, diagnostics overlay, render debug overlay)
  - `area_light_luts` (needed by `RectLight`)
  - `experimental_pbr_pcss` (soft shadows), `free_camera`, `pan_orbit_camera`, `meshlet`, `bevy_solari`, `dlss`
- **Input devices are cargo features** (`keyboard`, `mouse`, `gamepad`, `touch`). They are enabled by `bevy_window` and `bevy_gilrs`, both in `3d`/`ui`, so normally nothing to do.
- `dynamic_linking` is unchanged. Keep it out of release builds. UNVERIFIED with this toolchain.
- On x86_64 Linux, recent stable Rust links with `rust-lld` by default, so no mold/lld setup is needed.

---

## 2. App, plugins, schedules, sets, run conditions, time

```rust
use bevy::time::common_conditions::on_timer;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum RaceSet { Input, Vehicles, Rules }

pub fn plugin(app: &mut App) {        // plain fns are Plugins
    app.configure_sets(Update, (RaceSet::Input, RaceSet::Vehicles, RaceSet::Rules).chain())
        .configure_sets(FixedUpdate, RaceSet::Vehicles.run_if(in_state(RacePhase::Running)))
        .insert_resource(Time::<Fixed>::from_hz(120.0))   // default is 64 Hz
        .add_systems(Update, (
            count_laps.in_set(RaceSet::Rules),
            pause_toggle.in_set(RaceSet::Input),
            // .and() is gone: and_then (short-circuit) / and_eager
            countdown_done.run_if(in_state(RacePhase::Countdown).and_then(on_timer(Duration::from_secs(3)))),
            lap_debug.run_if(on_message::<LapCompleted>),
        ))
        .add_systems(FixedUpdate, fixed_step.in_set(RaceSet::Vehicles));
}
```
[C]

- Conditions in the prelude: `in_state`, `resource_exists::<T>`, `resource_exists_and(|r: &T| ..)` (new), `resource_changed`, `on_message::<M>`, `any_with_component::<C>`, `not(..)`, `run_once`, `condition_changed`.
- Time conditions in `bevy::time::common_conditions`: `on_timer`, `on_real_timer`, `once_after_delay`, `repeating_after_delay`, `paused`.
- Input conditions in `bevy::input::common_conditions`: `input_just_pressed(KeyCode::KeyP)`, `input_toggle_active`.
- Combinators: `and_then`, `and_eager`, `or_else`, `or_eager`, `nand_then`, `nor_else`, `xor`, `xnor`.
- **Observers take run conditions too:** `app.add_observer(on_x.run_if(in_state(S::Menu)))`. [C]
- **Weak ordering is new:** `.chain_weak()` / `.before_weak()` / `.after_weak()` only order systems that actually conflict. Some built-in render/UI sets now use it. `Update`, `FixedUpdate` and transform propagation are unaffected.
- Exclusive systems: `&mut World` may appear anywhere in the parameter list (0.20).

### Time

```rust
fn pause_toggle(keys: Res<ButtonInput<KeyCode>>, mut time: ResMut<Time<Virtual>>) {
    if keys.just_pressed(KeyCode::Escape) {
        if time.is_paused() { time.unpause() } else { time.pause() }   // also: time.toggle()
    }
    if keys.just_pressed(KeyCode::F2) {
        time.set_relative_speed(0.25);    // slow-mo; FixedUpdate also runs 4x less often
    }
}

fn fixed_step(time: Res<Time>, fixed: Res<Time<Fixed>>, real: Res<Time<Real>>) {
    let _dt = time.delta_secs();              // inside FixedUpdate, Res<Time> IS the fixed clock
    let _alpha = fixed.overstep_fraction();   // 0..1 for render interpolation
    let _wall = real.elapsed_secs();          // unaffected by pause / slow-mo
}
```
[C]

- `Res<Time>` is `Time<Virtual>` in `Update` and `Time<Fixed>` in `FixedUpdate`. Pausing `Time<Virtual>` stops `FixedUpdate`, so physics in FixedUpdate pauses too.
- `Time<Virtual>` max delta defaults to 250 ms. In 0.20 the clamp applies after speed scaling.
- **`Timer`:** `timer.tick(time.delta()).just_finished()`; `finished()`/`paused()` became `is_finished()`/`is_paused()`.
- **Delayed commands (0.19+, prelude):** `commands.delayed().secs(2.0).spawn(..)`, measured on virtual time (pauses with it). There is no cancellation. [C]
- Schedule order is unchanged: `First, PreUpdate, StateTransition, RunFixedMainLoop (FixedFirst … FixedUpdate, FixedPostUpdate … FixedLast), Update, PostUpdate, Last`.
- Order against the fixed loop with `RunFixedMainLoopSystems::{BeforeFixedMainLoop, FixedMainLoop, AfterFixedMainLoop}` in the `RunFixedMainLoop` schedule.

---

## 3. ECS

### Messages (buffered, pull) vs Events (immediate, observers)

```rust
#[derive(Message, Debug, Clone, Copy)]
pub struct LapCompleted { pub car: Entity, pub lap: u32 }

#[derive(Event, Debug)]
pub struct RaceStarted;                        // global event

#[derive(EntityEvent, Debug)]
pub struct Damaged { pub entity: Entity, pub amount: f32 }   // target = field named `entity`
                                               // (or #[event_target], or a 1-field tuple struct)
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]     // bubbles up ChildOf (part -> car root)
pub struct PartHit { pub entity: Entity }

app.add_message::<LapCompleted>()
   .add_observer(on_race_started)
   .add_observer(on_damaged)
   .add_observer(|add: On<Add<Car>>, cars: Query<&Car>| { let _ = cars.get(add.entity); })
   .add_observer(|rm: On<Remove<Health>>| debug!("health removed from {}", rm.entity));

fn lap_debug(mut w: MessageWriter<LapCompleted>, cars: Query<Entity, With<Car>>) {
    if let Some(car) = cars.iter().next() { w.write(LapCompleted { car, lap: 1 }); }
}
fn count_laps(mut laps: MessageReader<LapCompleted>, mut count: ResMut<LapCount>) {
    for lap in laps.read() { count.0 = count.0.max(lap.lap); }
}
fn on_damaged(e: On<Damaged>, mut q: Query<&mut Health>) {   // On<E> derefs to the event
    if let Ok(mut h) = q.get_mut(e.entity) { h.0 -= e.amount; }
}
// triggering:
commands.trigger(Damaged { entity: car, amount: 5.0 });   // runs global + entity observers
commands.entity(car).trigger(|entity| PartHit { entity });
// per-entity observer:
commands.spawn(Car::default()).observe(|hit: On<PartHit>| {
    debug!("hit via {}", hit.original_event_target())     // propagate(false) stops bubbling
});
```
[C]

- `world.trigger(..)` runs observers immediately. `commands.trigger` runs them when commands apply.
- Messages are double-buffered: a reader sees each message once, and a message lives two update cycles. Writing in `FixedUpdate` and reading in `Update` works. [T]
- `MessageMutator<M>` reads and writes the same type. `PopulatedMessageReader<M>` skips the system when no messages are queued.
- Built-in input and window streams (`KeyboardInput`, `WindowResized`, `GamepadConnectionEvent`, `AssetEvent<T>`, `AppExit`, …) are Messages: read them with `MessageReader<T>`.
- Writes from outside a system: `commands.write_message(m)` / `world.write_message(m)`. [T]

### Components: required components, hooks

```rust
#[derive(Component, Default)]
#[require(Health(100.0), Transform, Visibility, Name::new("Car"))]   // `A = expr()` also works
#[component(on_add = on_car_added)]   // hooks: on_add, on_insert, on_discard (was on_replace), on_remove, on_despawn
pub struct Car { pub player: u8 }

fn on_car_added(world: DeferredWorld, ctx: HookContext) {   // bevy::ecs::{world::DeferredWorld, lifecycle::HookContext}
    let player = world.get::<Car>(ctx.entity).map(|c| c.player);
    debug!("car {:?} added for player {:?}", ctx.entity, player);
}
```
[C]

- Required components are inserted only if missing, recursively. `Mesh3d` brings `Transform` and `Visibility`; `Camera3d` brings `Camera`, `Projection` and the rest.
- One hook per kind per component. Use observers for cross-cutting reactions.
- `#[component(summary_tick)]` (0.20) makes `Changed<T>` filters cheap for components that are queried often but rarely change (checkpoints, track pieces).
- Immutable components: `#[component(immutable)]`.

### Hierarchy and relationships

```rust
let car = commands.spawn((
    Car { player: 0 },
    DespawnOnExit(AppState::Race),
    children![
        (Name::new("Body"), Transform::default()),
        (Name::new("Wheel FL"), Transform::from_xyz(0.5, 0.0, -0.8)),
    ],
)).id();
commands.entity(car).with_child((Name::new("Spoiler"), Transform::default()));
// also: .with_children(|p: &mut ChildSpawnerCommands| { p.spawn(..); }),
//       Children::spawn((Spawn(b), SpawnIter(it), SpawnWith(|s: &mut RelatedSpawner<ChildOf>| ..))),
//       related!(Children[ .. ]), .add_child(e), .insert(ChildOf(parent))
```
[C]

- `despawn()` despawns all descendants [T]. Use `try_despawn()` for "maybe already gone"; `despawn_related::<Children>()` / `despawn_children()` despawn the children only.
- Bulk despawn by filter (0.20): `commands.despawn_all::<With<Skid>>()` and `commands.despawn_all_where::<&Health, ()>(|h| h.0 <= 0.0)`. [C]
- Renames (0.18): `remove_child(ren)` → `detach_child(ren)`, `clear_children` → `detach_all_children`. These detach; they do not despawn.
- Read the parent with `ChildOf::parent()`. Traverse with `children_q.iter_descendants(e)` and `parents_q.iter_ancestors(e)`, inherent on `Query<&Children>` / `Query<&ChildOf>`.
- `Name`: only `From<&'static str>`. Runtime strings need `Name::new(format!(..))`. [C]

### Queries and system params

```rust
fn fallible_system(cars: Query<&Transform, With<Car>>, single: Option<Single<&Car>>) -> Result {
    if let Some(car) = single { let _ = car.player; }   // Option<Single>: None for 0 or 2+ matches
    let Some(t) = cars.iter().next() else { return Ok(()) };
    if !t.translation.is_finite() { return Err("car transform is NaN".into()); }
    Ok(())
}
fn optional_settings(settings: If<Res<Settings>>) { let _ = settings.hud_scale; }  // skip if missing
fn contextual(q: Query<&Transform, With<Skid>>) -> Result {
    let n: u32 = "3".parse().context("parsing lap count")?;   // bevy::ecs::error::ContextExt
    Ok(())
}
```
[C]

| Param | When it doesn't match |
|---|---|
| `Query<D, F>` | always valid |
| `q.single()` / `single_mut()` | returns `Result<_, QuerySingleError>` |
| `Single<D, F>` | system **skipped silently** for 0 or 2+ matches |
| `Populated<D, F>` | system skipped if empty |
| `Res<T>` / `ResMut<T>` | missing resource is an **error, and errors panic by default** [T] |
| `Option<Res<T>>`, `If<Res<T>>` | `None` / system skipped |

- **Errors.** Systems, observers and commands may return `Result` (`= Result<(), BevyError>`, in the prelude).
  - `BevyError` defaults to `Severity::Panic`, so returned errors panic. Panics inside systems are caught and routed to the same handler (0.20).
  - App-wide handler, set once: `app.set_error_handler(bevy::ecs::error::warn);` [C]. Per error: `.with_severity(Severity::Warning)`.
- `iter_many*` now yields `Result` items. Use `.iter_many(list).matched()` for the old behaviour.
- `get_many` / `get_many_mut` (not `many`).
- Disabled entities: inserting `bevy::ecs::entity_disabling::Disabled` hides an entity from every query. Opt back in with `With<Disabled>`, `Has<Disabled>` or `Allow<Disabled>`.
- **Resources as entities** [T]:
  - A fresh `World` already holds one resource entity, and `Query<Entity>` counts them.
  - Filter with `Without<bevy::ecs::resource::IsResource>` for "all game entities" loops and next to `ResMut` with `EntityMut`/`EntityRef` queries.
  - `#[derive(Component, Resource)]` on the same type is now impossible: split it into two types.

---

## 4. States

```rust
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState { #[default] Boot, Menu, Race }

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Race)]          // exists only while AppState::Race
pub enum RacePhase { #[default] Countdown, Running, Finished }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InRace;
impl ComputedStates for InRace {
    type SourceStates = AppState;             // tuples / Option<S> allowed
    fn compute(s: AppState) -> Option<Self> { (s == AppState::Race).then_some(InRace) }
}

app.init_state::<AppState>()                  // needs StatesPlugin (in DefaultPlugins)
   .add_sub_state::<RacePhase>()
   .add_computed_state::<InRace>()
   .add_systems(OnEnter(AppState::Race), spawn_race)
   .add_systems(OnExit(AppState::Race), |mut c: Commands| c.trigger(RaceStarted));

fn countdown_done(mut commands: Commands) { commands.set_state(RacePhase::Running); }
// or ResMut<NextState<S>>: .set(s) | .set_if_different(s) | NextState::Pending(s)
```
[C][T]

- **State-scoped entities are always on.** Use `DespawnOnExit(S::X)`, `DespawnOnEnter(S::X)`, `DespawnWhen::new(|t| ..)`, and `DisableOnExit`/`EnableOnEnter`/… (which toggle `Disabled`). Old `StateScoped` and `#[states(scoped_entities)]` don't compile. [T: children despawn too]
- `set(same_state)` re-runs `OnExit` + `OnEnter` and state-scoped despawns. That is handy for "restart race"; use `set_if_different` otherwise.
- Other pieces: `OnTransition { exited, entered }`, the `PreviousState<S>` resource, `State<S>::get()`, `StateTransitionEvent<S>` (a message), and `app.clear_messages_on_exit::<M>(S::X)`.

---

## 5. Assets, glTF

### Loading, settings, load state, hot reload

```rust
use bevy::asset::LoadState;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};

let def: Handle<TrackDef> = server.load("tracks/toytown.track.ron");
let asphalt = server.load_builder()                       // was load_with_settings
    .with_settings(|s: &mut ImageLoaderSettings| {
        s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..ImageSamplerDescriptor::linear()
        });
    })
    .load("textures/asphalt.png");

match server.load_state(&def) {
    LoadState::Loaded => {}
    LoadState::Failed(err) => error!("track failed: {err}"),
    _ => return,
}
if !server.is_loaded_with_dependencies(&gltf_handle) { return; }

fn track_changed(mut ev: MessageReader<AssetEvent<TrackDef>>) {   // hot reload: `file_watcher` feature
    for e in ev.read() { if let AssetEvent::Modified { id } = e { info!("track {id} reloaded"); } }
}
```
[C]

- Other `load_builder()` methods: `.override_unapproved()`, `.with_guard(g)`, `.load_untyped(p)`, `.load_erased(type_id, p)`.
- `asset_server.load(path)` and `load_folder` are unchanged.
- Inside a loader, `load_context.load_builder()` replaces `load_context.loader()`.
- `Assets<T>`:
  - `add`, `get`, `get_mut` (→ `AssetMut`, bind it `mut`), `get_mut_untracked`, `remove`, `iter`.
  - `insert(id, a)` returns `Result`.
  - Use `Option<AssetId>` instead of `AssetId::invalid()`.
- Asset root: `assets/` next to `Cargo.toml` (`CARGO_MANIFEST_DIR`), overridable with the `BEVY_ASSET_ROOT` env var. Running the binary directly from `target/` without either looks in `target/debug/assets`. [G]
- Embedded assets: `embedded_asset!(app, "shaders/x.wesl")` (path relative to the calling `.rs` file), then load `"embedded://<crate_name>/shaders/x.wesl"`. A WESL file imports it as `<crate_name>::shaders::x`. UNVERIFIED (from `examples/asset/embedded_asset.rs`).

### Custom asset loader (RON track files)

```rust
use bevy::asset::{AssetLoader, LoadContext, io::Reader};

#[derive(Asset, TypePath, Debug, serde::Deserialize)]
pub struct TrackDef { pub name: String, pub checkpoints: Vec<Vec3>, pub laps: u32 }

#[derive(Default, TypePath)]               // loaders must derive TypePath (0.18+)
pub struct TrackLoader;

#[derive(Debug, thiserror::Error)]
pub enum TrackLoaderError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("ron: {0}")] Ron(#[from] ron::error::SpannedError),
}

impl AssetLoader for TrackLoader {
    type Asset = TrackDef;
    type Settings = ();
    type Error = TrackLoaderError;          // any Into<BevyError>
    async fn load(&self, reader: &mut dyn Reader, _s: &(), _ctx: &mut LoadContext<'_>)
        -> Result<TrackDef, TrackLoaderError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron::de::from_bytes(&bytes)?)
    }
    fn extensions(&self) -> &[&str] { &["track.ron"] }
}
app.init_asset::<TrackDef>().init_asset_loader::<TrackLoader>();
```
[C] (`ron = "0.12"` is the version Bevy uses; `thiserror = "2"`.)

### glTF scenes, named nodes, extras, materials

```rust
use bevy::gltf::{Gltf, GltfExtras, GltfMaterialName, GltfMeshName};
use bevy::world_serialization::WorldInstanceReady;   // NOT in the prelude

commands.spawn((
    WorldAssetRoot(server.load(GltfAssetLabel::Scene(0).from_asset("tracks/toytown.glb"))),
    TrackRoot,
));                              // "cars/buggy.glb#Scene0" also works
app.add_observer(on_scene_ready);   // or .observe(..) on the spawned entity

fn on_scene_ready(
    ready: On<WorldInstanceReady>,            // fired on the WorldAssetRoot entity
    roots: Query<(), With<TrackRoot>>,
    children: Query<&Children>,
    nodes: Query<(&Name, Option<&GltfExtras>, Option<&GltfMeshName>, Option<&GltfMaterialName>)>,
) {
    if roots.get(ready.entity).is_err() { return; }
    for e in children.iter_descendants(ready.entity) {
        if let Ok((name, Some(extras), _, _)) = nodes.get(e) {
            let _json: &str = &extras.value;       // raw JSON of the node's glTF "extras"
            let _ = name.as_str();                 // Blender object name, e.g. "Checkpoint.003"
        }
    }
}
// Without spawning: Res<Assets<Gltf>> -> gltf.named_nodes["StartGrid"] (Handle<GltfNode>:
// .transform, .children, .mesh, .extras), gltf.named_meshes, gltf.scenes, gltf.default_scene.
```
[C]

- Extras come per level as components: `GltfExtras` (node/primitive), `GltfMeshExtras`, `GltfMaterialExtras` and `GltfSceneExtras`, plus `GltfMeshName`, `GltfMaterialName` and `GltfSceneName`.
- **Material sub-assets (0.19).**
  - `#Material0` is now a `Handle<GltfMaterial>` (renderer-agnostic). Load `"x.glb#Material0/std"` for the `StandardMaterial`.
  - Spawned mesh entities still get `MeshMaterial3d<StandardMaterial>`, so the recolor-per-player pattern below still works: clone the material, edit it, insert a new `MeshMaterial3d` (`examples/gltf/edit_material_on_gltf.rs`).
- **Axis conventions.** glTF forward is +Z, Bevy forward is −Z. Optional conversion: `GltfPlugin { convert_coordinates: GltfConvertCoordinates { rotate_scene_entity: true, rotate_meshes: true }, ..default() }` (off by default). Simpler: model the car facing −Z in Blender's glTF export, or rotate the root.
- Collision meshes from glTF are in `rapier-notes.md` (`AsyncSceneCollider`, `Collider::from_bevy_mesh`).

---

## 6. Meshes

```rust
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};

pub fn track_mesh() -> Mesh {
    let positions = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 0.0, -10.0], [0.0, 0.0, -10.0]];
    let normals = vec![[0.0, 1.0, 0.0]; 4];
    let uvs = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD, // keep CPU copy (colliders!)
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));   // CCW = front face
    mesh.generate_tangents().expect("tangents");   // for normal maps (bevy_mikktspace, in "3d")
    mesh
}
commands.spawn((Mesh3d(meshes.add(track_mesh())), MeshMaterial3d(mat), Transform::default()));
// primitives: meshes.add(Cuboid::new(1.0, 0.4, 2.0)), Sphere::new(0.2).mesh().uv(16, 8),
//             Plane3d::default().mesh().size(60.0, 60.0), Cylinder::new(r, h), Capsule3d …
```
[C][G]

- `RenderAssetUsages::RENDER_WORLD` alone frees the CPU data after GPU upload. After that, the plain accessors **panic**; the new `try_*` variants (`try_attribute`, `try_insert_attribute`, …) return errors instead.
  - Rapier needs CPU vertices, so keep `MAIN_WORLD` on meshes you build colliders from.
- **The `Aabb` updates automatically when the mesh asset changes** (frustum culling stays correct). Opt out per entity with `bevy::camera::visibility::NoAutoAabb`; skip culling with `NoFrustumCulling`.
- Renames:
  - `MeshAabb::compute_aabb` → `get_aabb` (`bevy::camera::primitives`).
  - `MeshTag(x)` → `MeshTag::new(x)`.
  - Primitives and bounding volumes moved to `bevy::shape` (still in the prelude); curves and easing moved to `bevy::curve`.
- Other `Mesh` helpers: `compute_smooth_normals` (now angle-weighted), `compute_flat_normals`, `merge`, `translated_by`/`rotated_by`/`scaled_by`, `merge_duplicate_vertices`, `triangles()`.

---

## 7. Materials and shaders

### StandardMaterial highlights

```rust
StandardMaterial {
    base_color: Color::srgb(0.8, 0.1, 0.1),
    base_color_texture: Some(server.load("textures/asphalt.png")),
    normal_map_texture: Some(server.load("textures/asphalt_n.png")),   // mesh needs tangents
    perceptual_roughness: 0.35,
    metallic: 0.6,
    clearcoat: 1.0, clearcoat_perceptual_roughness: 0.05,   // car paint
    emissive: LinearRgba::rgb(8.0, 6.0, 1.0),               // > 1 blooms (Hdr + Bloom)
    alpha_mode: AlphaMode::Blend,                           // Opaque | Mask(cutoff) | Blend | Premultiplied | Add | Multiply | AlphaToCoverage
    uv_transform: Affine2::from_scale(Vec2::splat(4.0)),    // bevy::math::Affine2, tiling
    unlit: false, double_sided: false, fog_enabled: true,
    cull_mode: Some(Face::Back),                            // bevy::render::render_resource::Face; None = two-sided
    ..default()
}
materials.add(Color::srgb(0.3, 0.5, 0.3))   // From<Color> shortcut
```
[C]

- These fields only work with extra features: clearcoat/anisotropy/specular/transmission **textures** need `pbr_multi_layer_material_textures`, `pbr_anisotropy_texture`, `pbr_specular_textures` and `pbr_transmission_textures`. The scalar fields always work.
- A two-channel RG `metallic_roughness_texture` is auto-detected (0.20).
- `specular_transmission` needs the `ScreenSpaceTransmission` camera component (opt-in since 0.20).

### WESL rules (0.20)

- Use `.wesl` for anything that imports Bevy shader modules. A plain `.wgsl` file is loaded verbatim with no preprocessing. GLSL support is removed.
- Imports come first, end with `;`, and follow the file layout:
  - `bevy_pbr::render::{forward_io, mesh_functions, mesh_view_bindings, pbr_fragment, pbr_functions, pbr_types, …}`
  - `bevy_pbr::prepass::io`
  - `bevy_pbr::deferred::functions`
  - `bevy_render::{view, globals, maths, utils}`
  - Old renames: `bevy_pbr::lighting` → `bevy_pbr::render::pbr_lighting`; `bevy_pbr::environment_map` → `bevy_pbr::light_probe::environment_map`.
- Conditionals: `@if(DEF)`, `@elif`, `@else` on declarations, imports and statements (also blocks: `@if(X) { … } @else { … }`). Integer defs are read as `constants::NAME`.
- Material bindings: `@group(constants::MATERIAL_BIND_GROUP) @binding(N)`. (The 0.20 migration guide has this backwards; the form here is the one Bevy's own shaders use.)
- Your own modules: `import super::util::hsv_to_rgb;` (relative) or by asset path.
- Shader defs from Rust only apply to `.wesl`. IDE support: wgsl-analyzer.

### Verified custom material: `ExtendedMaterial<StandardMaterial, CarPaint>` [G]

```rust
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct CarPaint {
    #[uniform(100)] pub flake: Vec4,      // extension bindings start at 100 (0..99 = StandardMaterial)
    #[uniform(101)] pub tint: LinearRgba,
}
impl MaterialExtension for CarPaint {
    fn fragment_shader() -> ShaderRef { "shaders/car_paint.wesl".into() }
    // deferred_fragment_shader(), vertex_shader(), prepass_*_shader(), enable_oit() also exist
}
pub type CarPaintMaterial = ExtendedMaterial<StandardMaterial, CarPaint>;

app.add_plugins(MaterialPlugin::<CarPaintMaterial>::default());
commands.spawn((
    Mesh3d(meshes.add(Cuboid::new(1.6, 0.6, 3.0))),
    MeshMaterial3d(paints.add(CarPaintMaterial {
        base: StandardMaterial { base_color: Color::srgb(0.8, 0.1, 0.1), metallic: 0.5,
                                 perceptual_roughness: 0.3, clearcoat: 1.0, ..default() },
        extension: CarPaint { flake: Vec4::new(0.25, 40.0, 0.0, 0.0), tint: LinearRgba::WHITE },
    })),
));
```

`assets/shaders/car_paint.wesl`:
```wgsl
import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_view_bindings::globals,
};
@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> flake: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<uniform> tint: vec4<f32>;

fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let sparkle = hash13(floor(in.world_position.xyz * flake.y)) * flake.x;
    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * tint.rgb + vec3<f32>(sparkle),
        pbr_input.material.base_color.a,
    );
    pbr_input.material.base_color = alpha_discard(pbr_input.material.flags, pbr_input.material.alpha_cutoff, pbr_input.material.base_color);
    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);   // fog, tonemap if !Hdr
    out.color = vec4<f32>(out.color.rgb * (0.95 + 0.05 * sin(globals.time)), out.color.a);
}
    return out;
}
```

- A fully custom `Material` (no PBR) implements `Material { fragment_shader, vertex_shader, alpha_mode, enable_prepass, enable_shadows, specialize }`. See the rain material in §16 for a complete vertex+fragment one [G].
- `AsBindGroup` field attributes:
  - `#[uniform(N)]` (a `Vec4`/`LinearRgba`/`ShaderType` struct)
  - `#[texture(N)]` + `#[sampler(N+1)]` on `Handle<Image>` / `Option<Handle<Image>>`
  - `#[storage(N, read_only)]`
  - `#[bind_group_data(Key)]` for specialization
- **WebGL2 only:** uniform structs must be 16-byte aligned.
- Shader data:
  - `globals.time` comes from `bevy_pbr::render::mesh_view_bindings::globals`; the camera from `view` (`view.world_position`, `view.clip_from_world`, `view.world_from_view`).
  - Per-instance data: `get_world_from_local(instance_index)` and `get_tag(instance_index)` (from `MeshTag::new(u32)`).

---

## 8. Lighting

```rust
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, NotShadowCaster, light_consts::lux};

app.insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 200.0, ..default() }) // was AmbientLight resource
   .insert_resource(DirectionalLightShadowMap { size: 2048 });                                 // per-cascade texture size
commands.spawn((
    DirectionalLight {
        illuminance: lux::RAW_SUNLIGHT,     // with Atmosphere; otherwise ~lux::AMBIENT_DAYLIGHT (10_000, default)
        shadow_maps_enabled: true,          // was shadows_enabled
        ..default()                         // also: contact_shadows_enabled, shadow_depth_bias, shadow_normal_bias
    },
    CascadeShadowConfigBuilder { num_cascades: 3, first_cascade_far_bound: 12.0, maximum_distance: 120.0, ..default() }.build(),
    Transform::from_xyz(10.0, 20.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),   // shines along -Z of its transform
));
commands.spawn((PointLight { intensity: 200_000.0 /* lumens */, range: 20.0, shadow_maps_enabled: false, ..default() },
                Transform::from_xyz(0.0, 4.0, 0.0)));
commands.spawn((SpotLight { intensity: 400_000.0, outer_angle: 0.6, inner_angle: 0.4, ..default() },
                Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::NEG_Z, Vec3::Y)));   // headlight
commands.spawn((Mesh3d(m), MeshMaterial3d(glass), NotShadowCaster));   // also NotShadowReceiver
```
[C][G]

- Units:
  - `DirectionalLight::illuminance` is in **lux**. Presets: `lux::{MOONLESS_NIGHT, FULL_MOON_NIGHT, CIVIL_TWILIGHT, OVERCAST_DAY, AMBIENT_DAYLIGHT, FULL_DAYLIGHT, DIRECT_SUNLIGHT, RAW_SUNLIGHT}`.
  - Point and spot `intensity` is in **lumens** (`light_consts::lumens::*`). Default camera exposure is `Exposure::BLENDER`; pair `Exposure::SUNLIGHT` with real sun values.
- `AmbientLight` is now a **camera component** that overrides `GlobalAmbientLight` for that view. [C]
- Default cascades: 4, max distance 150, first bound 10. Shadow costs:
  - **Directional shadow maps render per view, so split screen multiplies them.**
  - Point and spot shadow maps render once and are shared by all cameras. With several cameras, put `ShadowLodOrigin` on the camera that should drive their LOD. [C]
- `RectLight { color, intensity, range, width, height }` is a new area light. It needs the `area_light_luts` feature and casts no shadows.
- Environment and IBL (all camera components) [C]:
  - `EnvironmentMapLight { diffuse_map, specular_map, intensity, rotation, .. }` (prefiltered KTX2 pair). Helpers: `EnvironmentMapLight::solid_color(&mut images, c)` and `hemispherical_gradient(..)`.
  - `GeneratedEnvironmentMapLight { environment_map: cubemap, intensity, .. }` filters any power-of-two cubemap on the GPU at runtime.
  - `Skybox { image: Some(cubemap), brightness, .. }` (now `bevy::light::Skybox`; `image` is an `Option`).
  - `LightProbe` is no longer a unit struct: use `LightProbe::default()`.

### Procedural sky and time of day [G]

```rust
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, AtmosphereEnvironmentMapLight, SunDisk, VolumetricLight};
use bevy::pbr::AtmosphereSettings;

// The planet is its own entity (0.19+). Default transform puts the ground at y = 0.
commands.spawn(Atmosphere::earth(media.add(ScatteringMedium::earth(256, 256))));   // ResMut<Assets<ScatteringMedium>>
// The sun: illuminance lux::RAW_SUNLIGHT, plus SunDisk::EARTH (visible disc) and VolumetricLight (god rays).
// Each camera that should show the sky:
commands.spawn((Camera3d::default(), AtmosphereSettings::default() /* requires Hdr */, Exposure::SUNLIGHT,
                AtmosphereEnvironmentMapLight::default() /* IBL from the sky */, Bloom::NATURAL));

fn time_of_day(time: Res<Time>, mut sun: Single<&mut Transform, With<DirectionalLight>>) {
    sun.rotate_x(-time.delta_secs() * 0.01);   // sky color, IBL and shadows follow
}
```
- Multiple cameras are supported since 0.20; this was verified with two viewports.
- `AtmosphereEnvironmentMapLight` renders a cubemap **per camera** (default 128×128 since 0.20; set `size`).
- `AtmosphereSettings` controls LUT sizes and sample counts. `rendering_method: AtmosphereMode::{LookupTexture, Raymarched}`.

Fog:
- `DistanceFog { color, falloff: FogFalloff::{Linear{start,end}, Exponential{density}, ExponentialSquared{..}, Atmospheric{..}}, directional_light_color, directional_light_exponent }` goes on the camera (cheap, `bevy::pbr`).
- Volumetric fog: `VolumetricFog { ambient_intensity, step_count, jitter, .. }` on the camera, `FogVolume::default()` + `Transform` (a box), and `VolumetricLight` on lights. All in `bevy::light`. It is expensive.

---

## 9. Cameras and split screen

```rust
use bevy::camera::{Exposure, Hdr, ImageRenderTarget, RenderTarget, Viewport};
use bevy::camera::visibility::RenderLayers;
use bevy::ui::{IsDefaultUiCamera, UiTargetCamera};
use bevy::window::{PrimaryWindow, WindowResized};

fn player_camera(i: usize) -> impl Bundle {
    (
        Camera3d::default(),                      // NOT bare `Camera3d` (it has fields)
        Camera { order: i as isize, ..default() },// distinct orders on one target
        Projection::from(PerspectiveProjection { fov: 70f32.to_radians(), near: 0.05, far: 2000.0, ..default() }),
        Hdr,                                      // was Camera { hdr: true }
        Tonemapping::TonyMcMapface,               // bevy::core_pipeline::tonemapping (re-export)
        Bloom::NATURAL,                           // requires Hdr (auto-inserted)
        Exposure::SUNLIGHT,
        AtmosphereSettings::default(),
        AtmosphereEnvironmentMapLight::default(),
        DistanceFog { color: Color::srgba(0.35, 0.48, 0.66, 1.0),
                      falloff: FogFalloff::Linear { start: 80.0, end: 400.0 }, ..default() },
        Msaa::Sample4,                            // per camera (was a resource)
        RenderLayers::from_layers(&[0, 1 + i]),   // 0 = world, 1+i = only this player sees it
        PlayerCam(i),
    )
}
```
[C]

### Option A: viewports on one window (SMAA/FXAA/MSAA only, no TAA)

```rust
fn layout_split_screen(
    mut resized: MessageReader<WindowResized>,       // also fires once at startup
    window: Single<&Window, With<PrimaryWindow>>,
    mut cams: Query<(&PlayerCam, &mut Camera)>,
) {
    if resized.read().last().is_none() { return; }
    let size = window.physical_size();
    let n = cams.iter().count() as u32;
    for (PlayerCam(i), mut cam) in &mut cams {
        let i = *i as u32;
        let (cols, rows) = if n <= 2 { (1, n.max(1)) } else { (2, 2) };   // 2p = top/bottom
        let cell = UVec2::new(size.x / cols, size.y / rows);
        cam.viewport = Some(Viewport {
            physical_position: UVec2::new(i % cols, i / cols) * cell,
            physical_size: cell,
            ..default()                                // depth: 0.0..1.0
        });
    }
}
// Per-player HUD laid out inside that player's viewport:
commands.spawn((UiTargetCamera(cam),
    Node { width: percent(100), height: percent(100), ..default() },
    children![(Text::new("P1"), TextFont::from_font_size(28.0))]));
```
[C][G with 2 viewports]

- The projection aspect ratio follows the viewport automatically.
- Since 0.20, viewport cameras no longer pay for an MSAA writeback.
- **Do not put `TemporalAntiAliasing` on these cameras.** [G] The output is squeezed into half the viewport (§0, trap 2).

### Option B: per-player image targets shown by `ViewportNode` (works with TAA) [G]

```rust
use bevy::ui::widget::ViewportNode;
let mut cams = Vec::new();
for i in 0..2 {
    // Each player renders into its own Image (start tiny; ViewportNode resizes it to the node):
    let img = images.add(Image::new_target_texture(16, 16, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb)));
    let cam = commands
        .spawn((player_camera(i), RenderTarget::Image(ImageRenderTarget::from(img))))
        // insert() replaces player_camera's Msaa::Sample4. Never put the same component
        // twice in one spawn tuple: "Bundle … has duplicate components" panic at runtime.
        .insert((Msaa::Off, TemporalAntiAliasing::default()))
        .id();
    cams.push(cam);
}
// One UI layout, drawn by the window's UI camera (the overlay camera below):
commands.spawn((Node { width: percent(100), height: percent(100), ..default() }, children![
    (ViewportNode::new(cams[0]), Node { width: percent(50), height: percent(100), ..default() }),
    (ViewportNode::new(cams[1]), Node { width: percent(50), height: percent(100), ..default() }),
]));
```
[C][G]
- Each player gets a full main texture, so TAA, SSAO and motion blur behave as they would full-screen.
- `UiTargetCamera(player_cam)` HUDs render into the player's image. The window needs one UI camera (the overlay below).
- The image is resized when the node size changes, so window resizes are handled.

### Overlay camera, render to texture, layers

```rust
// Full-window 2D/UI camera on top of all player views (pause menu, results, timers):
commands.spawn((
    Camera2d,
    Camera { order: 100, clear_color: ClearColorConfig::None, ..default() },  // don't erase the views
    IsDefaultUiCamera,                                                        // un-targeted UI goes here
));
// Rear-view mirror: camera -> Image -> UI ImageNode (or a material texture)
let mirror = images.add(Image::new_target_texture(512, 256, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb)));
commands.spawn((Camera3d::default(), Camera { order: -1, clear_color: Color::BLACK.into(), ..default() },
                RenderTarget::Image(ImageRenderTarget::from(mirror.clone())),   // was Camera.target
                Transform::from_xyz(0.0, 2.0, 0.0).looking_to(Vec3::Z, Vec3::Y)));
commands.spawn((ImageNode::new(mirror), Node { width: px(256), height: px(128), ..default() }));
```
[C][G]

- `ClearColorConfig::{Default, Custom(c), None}`; the global default comes from the `ClearColor` resource.
- `Camera { is_active: false }` turns a camera off.
- `RenderLayers::layer(n)`, `.with(m)`, `from_layers(&[..])`. Entities without the component are on layer 0. Lights obey layers too.
- Gizmo layers: `GizmoConfig::render_layers`.
- Mixed-`Hdr` camera stacks on one target now alpha-composite instead of overwriting (0.20).
- `Camera::viewport_to_world(&gt, cursor) -> Result<Ray3d, _>` and `world_to_viewport` return `Result`.

---

## 10. Post-processing and anti-aliasing

| Component (path) | Notes |
|---|---|
| `Hdr` (`bevy::camera`) | Needed for bloom, auto exposure and atmosphere; `Bloom`/`AutoExposure`/`AtmosphereSettings` insert it |
| `Tonemapping` (`bevy::core_pipeline::tonemapping`, really in `bevy::render::view`) | `TonyMcMapface` is the 3D default. `AcesFitted`, `AgX`, `BlenderFilmic` and others exist. **`None` = raw passthrough; use `Linear` for "no curve".** Camera2d defaults to `Linear` |
| `Bloom` (`bevy::post_process::bloom`) | Presets `NATURAL`, `ANAMORPHIC`, `OLD_SCHOOL`, `SCREEN_BLUR`. Since 0.19 luma is linear (dimmer): raise `intensity` or emissive |
| `Msaa` (component) | `Off`/`Sample2`/`Sample4` (default)/`Sample8`. **Must be `Off` for TAA and deferred** (so SSR). The official AA example also turns it off for SMAA/FXAA |
| `TemporalAntiAliasing` (`bevy::anti_alias::taa`) | Requires depth + motion-vector prepasses (auto) and `Msaa::Off`. Good with SSAO. **Broken with `Camera::viewport`** (§9 option B) |
| `Smaa { preset: SmaaPreset::High }` (`bevy::anti_alias::smaa`) | Good default for split screen with viewports [G] |
| `Fxaa` (`bevy::anti_alias::fxaa`) | Cheapest post AA [G] |
| `ContrastAdaptiveSharpening` (`bevy::anti_alias::contrast_adaptive_sharpening`) | Use after TAA. Not compiled here |
| `ScreenSpaceAmbientOcclusion` (`bevy::pbr`) | Requires depth + normal prepasses (auto). Noisy without TAA; not on WebGL2. Medium–high cost |
| `ScreenSpaceReflections` (`bevy::pbr`) | Requires depth + **deferred** prepasses (auto). Fields: `min/max_perceptual_roughness: Range`. High cost |
| `ContactShadows` (`bevy::pbr`) | Requires depth prepass. Also set `contact_shadows_enabled: true` on the light. Good for tyres on the road |
| `ScreenSpaceTransmission` (`bevy::pbr`) | Opt-in since 0.20 |
| `DistanceFog` (`bevy::pbr`) | Cheap |
| `VolumetricFog` + `FogVolume` + `VolumetricLight` (`bevy::light`) | Expensive (raymarch per light) |
| `MotionBlur { shutter_angle, samples }` (`bevy::post_process::motion_blur`) | Requires the motion-vector prepass (auto). Medium cost |
| `DepthOfField { mode: DepthOfFieldMode::{Bokeh, Gaussian}, focal_distance, aperture_f_stops, .. }` (`bevy::post_process::dof`) | Needs depth. Bokeh is expensive. Photo mode only |
| `AutoExposure` (`bevy::post_process::auto_exposure`) | Requires `Hdr`; compute shader. Metering with viewports is UNVERIFIED |
| `ChromaticAberration`, `Vignette` (0.19), `LensDistortion` (0.19) (`bevy::post_process::effect_stack`) | One cheap combined pass. Good boost/speed effects |
| `ColorGrading` (`bevy::render::view`), `Exposure` (`bevy::camera`) | Per-camera grading, EV100 |

[C] for every row except CAS; [G] where marked.

- Every prepass a camera has (depth, normal, motion vectors, deferred) multiplies with the number of players.
- **Split-screen budget suggestion:** SMAA or MSAA ×4, Bloom, DistanceFog, cheap effect stack, no SSR/volumetrics.
  - Per player: directional shadows render per view. Use 2–3 cascades with a short `maximum_distance`, and `DirectionalLightShadowMap { size: 1024..2048 }`.
- Quality preset swap [C]:
  ```rust
  commands.entity(cam).insert((Msaa::Off, TemporalAntiAliasing::default(), ScreenSpaceAmbientOcclusion::default()));
  commands.entity(cam).insert((Msaa::Off, Smaa { preset: SmaaPreset::High }));
  commands.entity(cam).insert((MotionBlur { shutter_angle: 0.5, samples: 4 },
                               ChromaticAberration { intensity: 0.02, ..default() }, Vignette::default()));
  ```
- New in 0.20: `RenderDebugOverlay` shows depth, normals and motion vectors. It needs the `render_dev_tools` feature (in `dev`). F1/F2 when keybindings are enabled.

---

## 11. Input

```rust
use bevy::input::gamepad::{AxisSettings, GamepadConnection, GamepadConnectionEvent,
                           GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::input::keyboard::KeyboardInput;

#[derive(Component)] pub struct PlayerPad(pub usize);

fn pad_connections(mut commands: Commands, mut ev: MessageReader<GamepadConnectionEvent>,
                   mut settings: Query<&mut GamepadSettings>, assigned: Query<&PlayerPad>) {
    for e in ev.read() {
        match &e.connection {
            GamepadConnection::Connected { name, .. } => {
                let slot = assigned.iter().count();
                info!("pad {name} -> player {slot}");
                commands.entity(e.gamepad).insert(PlayerPad(slot));     // gamepads ARE entities
                if let Ok(mut s) = settings.get_mut(e.gamepad) {
                    // (livezone_lower, deadzone_lower, deadzone_upper, livezone_upper, threshold)
                    s.default_axis_settings = AxisSettings::new(-1.0, -0.12, 0.12, 1.0, 0.01).unwrap();
                }
            }
            GamepadConnection::Disconnected => {}   // Gamepad component removed; entity stays
        }
    }
}

fn read_pads(pads: Query<(&Gamepad, &PlayerPad)>) {
    for (pad, PlayerPad(slot)) in &pads {
        let throttle = pad.get(GamepadButton::RightTrigger2).unwrap_or(0.0);  // analog 0..1
        let brake    = pad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0);
        let steer    = pad.left_stick().x;               // or pad.get(GamepadAxis::LeftStickX)
        let fire     = pad.just_pressed(GamepadButton::South);
        let _ = (slot, throttle, brake, steer, fire, pad.pressed(GamepadButton::Start));
    }
}

fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut text: MessageReader<KeyboardInput>) {
    let throttle = keys.pressed(KeyCode::KeyW) as i32 as f32;
    let _ = (throttle, keys.any_just_pressed([KeyCode::Space, KeyCode::Enter]));
    for k in text.read() { let _ = (&k.logical_key, k.key_code, k.state); }
}

fn rumble(pads: Query<Entity, With<Gamepad>>, mut rumble: MessageWriter<GamepadRumbleRequest>) {
    for gamepad in &pads {
        rumble.write(GamepadRumbleRequest::Add {
            gamepad, duration: Duration::from_millis(250),
            intensity: GamepadRumbleIntensity::strong_motor(0.8),   // MAX, WEAK_MAX, weak_motor(x), {strong_motor, weak_motor}
        });                                                         // ::Stop { gamepad }
    }
}
```
[C]

- `GamepadSettings` is a component on each gamepad entity.
  - Defaults: axis deadzone ±0.05, button press/release thresholds 0.75/0.65, trigger axis `ButtonAxisSettings { high: 0.95, low: 0.05 }`.
  - Per-axis overrides go in `settings.axis_settings: HashMap<GamepadAxis, AxisSettings>`.
- Unknown buttons and axes now arrive as `GamepadButton::Other(u32)` / `GamepadAxis::Other(u32)` (0.20).
- Raw messages: `GamepadButtonChangedEvent`, `GamepadAxisChangedEvent`, `GamepadButtonStateChangedEvent`. Vendor info: `pad.vendor_id()`, `product_id()`, plus a `Name` component.
- Mouse: `AccumulatedMouseMotion`, `AccumulatedMouseScroll` (use `MouseScrollPixelsPerLine` for unit conversion). Cursor grab: the `CursorOptions` component on the window entity.

---

## 12. Windows and exit

```rust
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, VideoModeSelection, WindowMode, WindowResolution};

DefaultPlugins.set(WindowPlugin {
    primary_window: Some(Window {
        title: "ReCharged".into(),
        resolution: WindowResolution::new(1600, 900),   // u32; requested in logical px at creation
        present_mode: PresentMode::AutoVsync,           // AutoNoVsync, Fifo, FifoRelaxed, Immediate, Mailbox
        ..default()                                     // also: mode, position, resizable, decorations,
    }),                                                 //   desired_maximum_frame_latency, …
    ..default()                                         // primary_cursor_options, exit_condition, close_when_requested
})

fn window_controls(keys: Res<ButtonInput<KeyCode>>, mut window: Single<&mut Window, With<PrimaryWindow>>,
                   mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::F11) {
        window.mode = match window.mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            _ => WindowMode::Windowed,
        };
    }
    if keys.just_pressed(KeyCode::F10) {   // exclusive; falls back to borderless if no mode matches
        window.mode = WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current);
    }
    if keys.just_pressed(KeyCode::KeyV) { window.present_mode = PresentMode::AutoNoVsync; }
    if keys.just_pressed(KeyCode::F9) { window.resolution.set(1920.0, 1080.0); }   // logical size
    if keys.just_pressed(KeyCode::KeyQ) { exit.write(AppExit::Success); }        // AppExit is a Message
}
```
[C]

- Scale factor: `window.resolution.scale_factor()`, `set_scale_factor_override(Some(1.0))`. Read sizes with `window.width()`/`height()` (logical) and `physical_size()`.
- `fn main() -> AppExit { App::new()…run() }` is allowed.
- Window-close exit systems run in `Last` (0.19+).
- `CursorOptions` is its own component (`Single<&mut CursorOptions, With<PrimaryWindow>>`).

---

## 13. Audio

**We use bevy_kira_audio, not bevy_audio.** See `kira-notes.md`.

- Keep `bevy`'s `audio` feature off (§1).
- For reference, bevy_audio 0.20 is `AudioPlayer::new(handle)` + `PlaybackSettings::{LOOP, ONCE, DESPAWN, REMOVE}` and `AudioSink` with `set_volume(Volume::Linear(..))`. Its only default codec is `vorbis` (`wav`, `mp3`, `flac`, `aac`, `mp4` are opt-in). Not compiled here.

---

## 14. Screenshots and diagnostics

```rust
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

commands.spawn(Screenshot::primary_window()).observe(save_to_disk("screenshots/shot.png"));   // [C]
commands.spawn(Screenshot::image(render_target_handle)).observe(save_to_disk("frame.png"));  // [G] headless
// Screenshot::window(entity) also exists. save_to_disk writes synchronously in the observer.
// Float (Hdr image) targets: save to .exr/.hdr (needs the `exr` feature) or values get clipped.

app.add_plugins(FrameTimeDiagnosticsPlugin::default());   // not a unit struct any more
fn fps(diags: Res<DiagnosticsStore>) {
    if let Some(fps) = diags.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed()) { let _ = fps; }
}
```
[C]

- `LogDiagnosticsPlugin::default()` prints to the log.
- On-screen FPS needs the `bevy_dev_tools` feature: `bevy::dev_tools::fps_overlay::{FpsOverlayPlugin, FpsOverlayConfig}`, or `DiagnosticsOverlayPlugin` + `commands.spawn(DiagnosticsOverlay::fps())` (0.19). UNVERIFIED (feature not enabled in the scratch crate).

---

## 15. Gizmos

```rust
use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore};

fn debug_draw(mut gizmos: Gizmos, cams: Query<&GlobalTransform, With<PlayerCam>>) {
    for t in &cams {
        let p = t.translation();
        gizmos.line(p, p + Vec3::Y, Color::WHITE);
        gizmos.arrow(p, p + t.forward() * 2.0, Color::srgb(1.0, 0.0, 0.0));
        gizmos.sphere(Isometry3d::from_translation(p), 0.25, Color::srgb(0.0, 1.0, 0.0));  // Vec3 also converts
    }
}
fn gizmo_config(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 3.0;
    config.depth_bias = -0.1;
    config.render_layers = RenderLayers::layer(0);   // which cameras see gizmos
}
```
[C]

- Other shapes: `cube(transform, c)` (was `cuboid`), `rect`, `circle(iso, r, c)`, `grid`, `cross`, `ray`, `short_arc_3d_between`, `primitive_3d(&shape, iso, c)`, `curve_gradient_3d`, `axes(transform, len)`.
- `gizmos.text(iso, "120 km/h", 20.0, Vec2::ZERO, color)` is a stroke-font debug label (0.19).
- Custom groups: `#[derive(Default, Reflect, GizmoConfigGroup)] struct PhysicsGizmos;` + `app.init_gizmo_group::<PhysicsGizmos>()` + `Gizmos<PhysicsGizmos>`.
- `gizmos.arrow(..).with_double_end()` (0.20).
- Rapier's debug renderer is drawn with gizmos, so it obeys the gizmo config.

---

## 16. Many small objects (rain, sparks, skid marks)

- **Batching is automatic.** Entities sharing the same `Mesh3d` handle **and** the same `MeshMaterial3d` handle become one instanced draw, with GPU preprocessing and culling.
  - Reuse handles; don't `materials.add(..)` per entity.
  - Per-instance shader data comes from `MeshTag::new(u32)` → `get_tag(instance_index)`.
  - Thousands of entities are fine; hundreds of thousands are better as one mesh.
- **There is no built-in particle system.** For rain and weather, build one mesh of N quads and animate it entirely in a vertex shader: one draw call, zero CPU per drop. Add `NoFrustumCulling`, because the CPU AABB no longer matches the moved vertices. [G]

```rust
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct RainMaterial {
    #[uniform(0)] pub params: Vec4,   // (fall speed m/s, box height m, streak length m, streak width m)
    #[uniform(1)] pub color: LinearRgba,
}
impl Material for RainMaterial {
    fn vertex_shader() -> ShaderRef { "shaders/rain.wesl".into() }
    fn fragment_shader() -> ShaderRef { "shaders/rain.wesl".into() }
    fn alpha_mode(&self) -> AlphaMode { AlphaMode::Premultiplied }
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }
    fn specialize(_: &MaterialPipeline, descriptor: &mut RenderPipelineDescriptor,
                  layout: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>)
        -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),   // per-drop seed position (same for 4 corners)
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),       // corner (-0.5..0.5, 0..1)
        ])?];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
// imports: bevy::pbr::{MaterialPipeline, MaterialPipelineKey}, bevy::mesh::MeshVertexBufferLayoutRef,
//          bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError}
commands.spawn((Mesh3d(meshes.add(rain_mesh(20_000, Vec3::new(40.0, 20.0, 40.0)))),
                MeshMaterial3d(mats.add(RainMaterial { params: Vec4::new(9.0, 20.0, 0.6, 0.015),
                                                       color: LinearRgba::new(0.7, 0.75, 0.8, 0.35) })),
                NoFrustumCulling, Transform::default()));   // move the emitter with the camera(s)
```

`assets/shaders/rain.wesl` (vertex stage):
```wgsl
import bevy_pbr::render::{ mesh_functions::get_world_from_local, mesh_view_bindings::{globals, view} };
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> color: vec4<f32>;
struct Vertex { @builtin(instance_index) instance_index: u32, @location(0) seed: vec3<f32>, @location(1) corner: vec2<f32> };
struct VertexOutput { @builtin(position) clip: vec4<f32>, @location(0) corner: vec2<f32> };
@vertex
fn vertex(v: Vertex) -> VertexOutput {
    let world_from_local = get_world_from_local(v.instance_index);
    let y = fract((v.seed.y - globals.time * params.x) / params.y) * params.y;   // fall + wrap
    let drop = (world_from_local * vec4<f32>(v.seed.x, y, v.seed.z, 1.0)).xyz;
    let right = normalize(view.world_from_view[0].xyz);                          // camera-facing
    let world = drop + right * v.corner.x * params.w - vec3<f32>(0.0, 1.0, 0.0) * v.corner.y * params.z;
    var out: VertexOutput;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    out.corner = v.corner;
    return out;
}
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let a = color.a * (1.0 - abs(in.corner.x) * 2.0) * (1.0 - in.corner.y);
    return vec4<f32>(color.rgb * a, a);   // premultiplied
}
```
- `rain_mesh` builds 4 vertices and 6 indices per drop, with `RenderAssetUsages::RENDER_WORLD`.
- In split screen the view bindings belong to whichever camera is rendering, so streaks face every player correctly.
- **Skid marks:** append quads to one dynamic mesh (`MAIN_WORLD | RENDER_WORLD`, mutate it through `meshes.get_mut`), or use pooled decal entities that share one mesh and material.

---

## 17. Headless simulation tests [T]

```rust
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, StatesPlugin))
        // Each app.update() advances exactly one fixed step (Time<Fixed> timestep):
        .insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        // or ::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)) for wall-clock-like stepping
    app
}

#[test]
fn rapier_headless_ball_falls_and_rests() {
    let mut app = headless_app();
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(TimestepMode::Fixed { dt: 1.0 / 60.0, substeps: 1 })
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule());
    app.finish();    // REQUIRED when you call update() yourself: plugins' finish() hooks
    app.cleanup();   // (rapier adds AssetPlugin + MeshPlugin there). app.run() does this for you.
    app.world_mut().spawn((Collider::cuboid(10.0, 0.1, 10.0), Transform::default()));
    let ball = app.world_mut()
        .spawn((RigidBody::Dynamic, Collider::ball(0.5), Transform::from_xyz(0.0, 5.0, 0.0))).id();
    for _ in 0..240 { app.update(); }
    let y = app.world().get::<Transform>(ball).unwrap().translation.y;
    assert!((y - 0.6).abs() < 0.05);   // measured: 0.600
}
```

- `MinimalPlugins` = `TaskPoolPlugin`, `FrameCountPlugin`, `TimePlugin`, `ScheduleRunnerPlugin`. No transforms, assets, states, input or window. Add what the code under test needs:
  - `TransformPlugin` for `GlobalTransform` propagation (rapier queries use it)
  - `StatesPlugin` for `init_state`
  - `AssetPlugin::default()` + `bevy::mesh::MeshPlugin` for `Assets<Mesh>`
  - `InputPlugin` for `ButtonInput`
- **The first `update()` has a zero delta**, so FixedUpdate does not run on it. 10 updates gave 9 fixed runs with `FixedTimesteps(1)`. `ManualDuration(1/30 s)` × 30 updates gave 0.967 s and 57 fixed runs at 60 Hz.
- A system needing a missing resource **panics** (e.g. `Res<Assets<Mesh>>` without `MeshPlugin`). With `bevy/debug` the message names the system.
- Drive input by writing resources (`app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW)`) or, better, write to your own `DriveInput` component.
- Messages: `world.write_message(m)`. Events: `world.trigger(e)` (observers run immediately). [T]
- No GPU needed. To render in tests or CI, use the windowless pattern from `examples/app/headless_renderer.rs`:
  - `DefaultPlugins.set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() }).disable::<WinitPlugin>()` + `ScheduleRunnerPlugin::run_loop(..)`.
  - Cameras render to an `Image`; capture with `Screenshot::image(h)` [G].
- Determinism: `ScheduleBuildSettings { shuffle_seed: Some(seed), .. }` (needs `bevy/debug`) shuffles system order to flush out ordering bugs (0.20).

---

## 18. Renames since 0.14–0.16 (old → new in 0.20)

| Old | 0.20 |
|---|---|
| `PbrBundle{mesh,material,transform}` | `(Mesh3d(h), MeshMaterial3d(m), Transform)` |
| `Camera3dBundle` / `Camera2dBundle` | `Camera3d::default()` (+ `Camera{..}`, `Transform`) / `Camera2d` |
| `SpatialBundle`, `TransformBundle`, `VisibilityBundle` | `(Transform, Visibility)` (required components fill the rest) |
| `SceneBundle` / `SceneRoot` / `Handle<Scene>` | `WorldAssetRoot(h)` / `Handle<WorldAsset>` |
| `SceneInstanceReady`, `SceneSpawner`, `DynamicScene` | `WorldInstanceReady`, `WorldInstanceSpawner`, `DynamicWorld` (`bevy::world_serialization`) |
| `PointLightBundle` etc. | `PointLight { .. }` component |
| `shadows_enabled` | `shadow_maps_enabled` |
| `AmbientLight` resource | `GlobalAmbientLight` (resource); `AmbientLight` = camera component |
| `Camera { hdr: true }` | `Hdr` component |
| `Camera { target: .. }` | `RenderTarget::Image(h.into())` component |
| `Msaa` resource | `Msaa` component per camera |
| `BloomSettings`, `FogSettings`, `*Settings` post FX | `Bloom`, `DistanceFog`, `Smaa`, `DepthOfField`, `AutoExposure`, `ScreenSpaceAmbientOcclusion`, … |
| `TemporalAntiAliasBundle` | `TemporalAntiAliasing` |
| `bevy::core_pipeline::{bloom, dof, motion_blur, fxaa, smaa, experimental::taa}` | `bevy::post_process::*`, `bevy::anti_alias::*` |
| `bevy::render::camera::*`, `bevy::render::view::{Visibility, RenderLayers}` | `bevy::camera::*`, `bevy::camera::visibility::*` |
| `bevy::pbr::{PointLight, DirectionalLight, CascadeShadowConfigBuilder, NotShadowCaster, …}` | `bevy::light::*` |
| `bevy::render::mesh::*`, `render_resource::{Shader, ShaderRef}`, `render_asset::RenderAssetUsages`, `texture::Image` | `bevy::mesh::*`, `bevy::shader::*`, `bevy::asset::RenderAssetUsages`, `bevy::image::*` |
| `bevy::math::primitives::*`, `bounding::*`, `Ray3d` | `bevy::shape::*` (prelude unchanged) |
| `Tonemapping::None` (meaning "linear") | `Tonemapping::Linear` |
| naga_oil `.wgsl` `#import bevy_pbr::forward_io::VertexOutput` | `.wesl` `import bevy_pbr::render::forward_io::VertexOutput;` |
| `#ifdef X` / `@group(2) @binding(0)` / `#{MATERIAL_BIND_GROUP}` | `@if(X)` / `@group(constants::MATERIAL_BIND_GROUP) @binding(0)` |
| `EventWriter<T>` / `.send()` / `EventReader<T>` / `Events<T>` | `MessageWriter<T>` / `.write()` / `MessageReader<T>` / `Messages<T>` |
| `add_event::<T>()`, `on_event::<T>()`, `send_event` | `add_message::<T>()`, `on_message::<T>`, `write_message` |
| `Trigger<E>`, `trigger.entity()/target()` | `On<E>`, field `e.entity` (EntityEvent) |
| `Trigger<OnAdd, C>` → `On<Add, C>` | `On<Add<C>>` (also `Insert`, `Discard` (was `Replace`/`OnReplace`), `Remove`, `Despawn`) |
| `world.trigger_targets(E, e)` | `#[derive(EntityEvent)] struct E { entity }` + `commands.trigger(E { entity: e })` |
| `app.observe(..)` | `app.add_observer(..)` / `entity.observe(..)` |
| `Parent`, `parent.get()` | `ChildOf(e)`, `child_of.parent()` |
| `despawn_recursive()` / `despawn_descendants()` | `despawn()` / `despawn_children()` |
| `push_children`, `remove_children`, `clear_children` | `add_children`, `detach_children`, `detach_all_children` |
| `ChildBuilder`, `parent_entity()` | `ChildSpawnerCommands`, `target_entity()` |
| `q.get_single()` / `q.single()` (panicking) | `q.single()` → `Result` |
| `q.many(..)` | `q.get_many(..)` → `Result` |
| `iter_many` items `T` | items `Result<T,_>` (`.matched()` for old behaviour) |
| `IntoSystemConfigs` | `IntoScheduleConfigs` |
| `Condition` trait, `.and()`, `.or()` | `SystemCondition`, `.and_then()`/`.and_eager()`, `.or_else()`/`.or_eager()` |
| `TransformSystem`, `UiSystem`, `InputSystem`, `TimeSystem`, `RenderSet`, `PickSet` | `TransformSystems`, `UiSystems`, `InputSystems`, `TimeSystems`, `RenderSystems`, `PickingSystems` |
| `GLOBAL_ERROR_HANDLER`, `DefaultErrorHandler` | `app.set_error_handler(..)`, `FallbackErrorHandler` |
| `commands.add(cmd)` | `commands.queue(cmd)` |
| `StateScoped(S)`, `enable_state_scoped_entities` | `DespawnOnExit(S)` (always enabled) |
| `NextState::set_if_neq` | `NextState::set_if_different` |
| `load_with_settings(p, f)`, `load_untyped(p)` | `load_builder().with_settings(f).load(p)`, `load_builder().load_untyped(p)` |
| `assets.get_mut(h) -> Option<&mut A>` | `Option<AssetMut<A>>` (bind `mut`) |
| `Handle::weak_from_u128`, `weak_handle!` | `uuid_handle!("…")` |
| `#Material0` → `Handle<StandardMaterial>` | `#Material0` → `Handle<GltfMaterial>`, `#Material0/std` → `StandardMaterial` |
| `NodeBundle{style: Style{..}}`, `Style` | `Node { .. }` (+ `BackgroundColor`, …) |
| `TextBundle::from_section(s, TextStyle{..})` | `(Text::new(s), TextFont{ font: h.into(), font_size: FontSize::Px(30.0), .. }, TextColor(c))` |
| `TextFont { font_size: 30.0 }` | `font_size: FontSize::Px(30.0)` (`TextFont::from_font_size(30.0)`); default is now `FontSize::Rem(1.0)` |
| `TargetCamera(cam)` | `UiTargetCamera(cam)` |
| `UiImage` / `ImageBundle` | `ImageNode::new(h)` |
| `ButtonBundle` + `Interaction` | `bevy::ui_widgets::Button` + `On<Activate>`, `Hovered`, `Pressed` (old ones deprecated) |
| `Pointer<Click>` (observer event) | `PointerClick` (flat; `.pointer.position`) |
| `Val::Px(10.)` | still valid; also `px(10)`, `percent(50)`, `vw()`, `vh()`, `Val::Rem`, `Val::Em` |
| `Res<Gamepads>`, `ButtonInput<GamepadButton>`, `Axis<GamepadAxis>` | `Query<(Entity, &Gamepad)>`; `gamepad.pressed(..)`, `gamepad.get(..)` |
| `GamepadButtonType::South` | `GamepadButton::South` |
| `ReceivedCharacter` | `KeyboardInput { logical_key, text, .. }` |
| `window.cursor.grab_mode` | `CursorOptions` component |
| `WindowResolution::new(1280.0, 720.0)` | `WindowResolution::new(1280, 720)` |
| `EventWriter<AppExit>`, `AppExit` unit | `MessageWriter<AppExit>`, `AppExit::Success` |
| `time.delta_seconds()`, `timer.finished()` | `time.delta_secs()`, `timer.is_finished()` |
| `ScreenshotManager` | `commands.spawn(Screenshot::primary_window()).observe(save_to_disk(p))` |
| `gizmos.cuboid` | `gizmos.cube` |
| `bevy::utils::HashMap`, `bevy::utils::Instant` | `bevy::platform::collections::HashMap`, `bevy::platform::time::Instant` |
| `Name::from(&string)` | `Name::new(string)` (`From<&'static str>` only) |
| `Atmosphere` on the camera, `bottom_radius/top_radius` | `Atmosphere::earth(medium)` entity + `AtmosphereSettings` on camera; `inner/outer_radius` |
| `Skybox { image: h }` | `Skybox { image: Some(h), .. }` |
| `ScreenSpaceReflections { perceptual_roughness_threshold }` | `min_/max_perceptual_roughness: Range<f32>` |
| `FrameTimeDiagnosticsPlugin` (unit) | `FrameTimeDiagnosticsPlugin::default()` |

---

## 19. 0.19 / 0.20 features useful for a split-screen racer

- **0.20**
  - Atmosphere works with several cameras.
  - Viewport cameras skip MSAA writeback.
  - Mixed `Hdr` stacks composite correctly.
  - `despawn_all`, used by `DespawnOnExit`, makes race resets faster.
  - `#[component(summary_tick)]` gives cheap change detection.
  - `chain_weak` adds parallelism.
  - Schedule shuffling finds order bugs.
  - Panics are caught as errors; `.context()` adds detail.
  - WESL brings IDE support for custom track/road/skid shaders.
  - Per-material OIT opt-out.
  - `Tonemapping::Linear`.
  - Retained UI rendering makes the HUD cheaper.
  - `Val::Em`/`Val::Rem` + `RemSize` give scalable HUD text; `FixedNode` positions HUD elements against the target camera's viewport.
  - `PanOrbitCamera` (`pan_orbit_camera` feature) for a track editor or replay.
  - `RenderDebugOverlay`.
- **0.19**
  - Vignette and LensDistortion (boost effect).
  - Contact shadows (tyres).
  - `RectLight` area lights.
  - Parallax-corrected reflection probes.
  - Physically based SSR.
  - Atmosphere as an entity, `SunDisk`.
  - Delayed commands.
  - `If<P>` and observer run conditions.
  - `DisableOnExit`/`EnableOnEnter`, `PreviousState`.
  - Text gizmos, transform gizmo, infinite grid (dev tools).
  - Diagnostics overlay.
  - Render-device-loss recovery (`RenderErrorHandler`).
  - Big draw/culling speedups (useful with 4 views).
  - `ShadowLodOrigin` for split screen.
  - BSN `bsn!` scenes (optional; plain tuples keep working).

---

## Appendix: verification log

The scratch crate is in the session scratchpad (`research-bevy/verify`): Bevy 0.20.0, bevy_rapier3d 0.37.0 (rapier3d 0.36.1), bevy_kira_audio 0.27.1 (kira 0.12.5).

- `cargo check --all-targets` passes for every [C] snippet: ECS, assets, rendering, input/window, rain, rapier car, kira.
- `cargo test`: 10 headless tests pass.
  - FixedTimesteps vs ManualDuration stepping.
  - rapier ball rest height 0.600.
  - raycast car settles on four wheels at y ≈ 0.42 and drives −Z at 18 m/s after 2 s of throttle.
  - the car collects a sensor pickup through `CollisionEvent`.
  - built-in `RayCastVehicleController` axis conventions.
  - state-scoped recursive despawn.
  - messages + observers.
  - resources counted by `Query<Entity>`.
  - missing `Res` panics.
- GPU (windowless, two players on one 1280×720 target), all with zero warnings:
  - WESL `ExtendedMaterial` and vertex-animated rain render.
  - Atmosphere + AtmosphereEnvironmentMapLight on two cameras.
  - UI overlay camera with `ClearColorConfig::None` and per-player `UiTargetCamera` HUD.
  - SMAA/FXAA/MSAA/off correct with `Viewport`; **TAA squeezed to half width with `Viewport`**.
  - TAA correct with per-player image targets and with `ViewportNode`.
