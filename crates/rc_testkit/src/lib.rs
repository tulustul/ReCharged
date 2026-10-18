//! # rc_testkit — headless test harness (T)
//!
//! For integration tests in `crates/*/tests/` only (a dev-dependency; never from unit tests,
//! which would compile the crate under test twice). No window, GPU or audio device; manual time
//! (every `update()` = exactly one fixed tick of 1/`SIM_HZ` s); deterministic.
//!
//! - [`headless_app`]: `MinimalPlugins` + transforms + states + rapier (fixed schedule, no debug
//!   render) + `CorePlugin` + the plugins under test; `FixedUpdate` ambiguity detection = warn;
//!   starts in `AppState::Menu`, with `UserPaths` on the workspace `assets/` and a temp data dir.
//! - [`SimPlugins`]: every headless sim plugin (track, vehicle, weather, race, items, AI); the sim
//!   crates are re-exported for tests that add individual plugins.
//! - Stepping: [`step`], [`step_seconds`], [`step_until`]; queries: [`app_state`],
//!   [`race_phase`], [`count_entities`] (resource entities excluded).
//! - Lifecycle helpers [`load_race`], [`start_race`], [`restart`], [`next_race`],
//!   [`quit_to_menu`], [`pause`], [`resume`]: a headless stand-in for rc_game's transitions and
//!   loading gate (ARCHITECTURE §3.1) and for the phase driver. Do not combine them with rc_game's
//!   `GameFlowPlugin` — rc_game's own tests drive the app with `GameCommand`s instead.

use std::path::PathBuf;

use bevy::app::Plugins;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::resource::IsResource;
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::log::tracing_subscriber::{self, EnvFilter};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::state::state::FreelyMutableState;
use bevy::time::TimeUpdateStrategy;
use rc_core::prelude::*;

/// The sim crates, re-exported so integration tests of any crate can add their plugins without
/// extra dev-dependencies (e.g. `rc_testkit::rc_vehicle::VehiclePlugin`).
pub use {rc_ai, rc_input, rc_items, rc_race, rc_track, rc_vehicle, rc_weather};

/// Every headless-capable simulation plugin of a race: track, vehicle, weather, race, items, AI.
/// (Not `GameInputPlugin`, which needs Bevy's input plugin; physics setup is already part of
/// [`headless_app`].) Use `headless_app(SimPlugins)` for full AI races.
pub struct SimPlugins;

impl PluginGroup for SimPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        bevy::app::PluginGroupBuilder::start::<Self>()
            .add(rc_track::TrackPlugin)
            .add(rc_vehicle::VehiclePlugin)
            .add(rc_weather::WeatherPlugin)
            .add(rc_race::RacePlugin)
            .add(rc_items::ItemsPlugin)
            .add(rc_ai::AiPlugin)
    }
}

/// Longest a helper waits for something to happen, in simulated seconds.
pub const TIMEOUT_SECS: f32 = 10.0;

/// The workspace `assets/` directory.
pub fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// Loader reports seen since the last `load_race` / `next_race`.
#[derive(Resource, Debug, Default)]
struct LoadingLog {
    done: Vec<&'static str>,
    failed: Option<LoadingFailed>,
}

fn record_loading(
    mut done: MessageReader<LoadingDone>,
    mut failed: MessageReader<LoadingFailed>,
    mut log: ResMut<LoadingLog>,
) {
    log.done.extend(done.read().map(|d| d.0));
    if let Some(f) = failed.read().last() {
        log.failed = Some(f.clone());
    }
}

/// A finished headless app with `plugins` added (use `()` for none), already in `AppState::Menu`.
pub fn headless_app<M>(plugins: impl Plugins<M>) -> App {
    // Warnings (e.g. schedule ambiguities) reach the test output; `RUST_LOG` overrides the level.
    // `try_init` is a no-op once a test in this process has installed the subscriber.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, StatesPlugin))
        .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
        .insert_resource(UserPaths {
            assets_dir: assets_dir(),
            data_dir: std::env::temp_dir().join(format!("recharged-test-{}", std::process::id())),
        })
        .add_plugins((CorePlugin, rc_vehicle::PhysicsPlugin))
        .init_resource::<LoadingLog>()
        .add_systems(Update, record_loading);
    app.edit_schedule(FixedUpdate, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            ..default()
        });
    });
    app.add_plugins(plugins);
    app.finish();
    app.cleanup();
    // The first update runs startup + `OnEnter(Boot)` (rc_game's boot, if added, opens the menu)
    // with a zero delta, i.e. no fixed tick yet.
    app.update();
    if app_state(&app) != AppState::Menu {
        request(&mut app, AppState::Menu);
        app.update();
    }
    app
}

/// Runs `updates` frames (= fixed ticks).
pub fn step(app: &mut App, updates: u32) {
    for _ in 0..updates {
        app.update();
    }
}

/// Runs `secs` of simulated time.
pub fn step_seconds(app: &mut App, secs: f32) {
    step(app, secs_to_ticks(secs));
}

/// Steps until `done` holds; panics naming `what` after [`TIMEOUT_SECS`].
pub fn step_until(app: &mut App, what: &str, mut done: impl FnMut(&World) -> bool) {
    for _ in 0..secs_to_ticks(TIMEOUT_SECS) {
        if done(app.world()) {
            return;
        }
        app.update();
    }
    assert!(
        done(app.world()),
        "timed out after {TIMEOUT_SECS} s waiting for {what}"
    );
}

/// The current app state.
pub fn app_state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

/// The current race phase (`None` outside `AppState::InRace`).
pub fn race_phase(app: &App) -> Option<RacePhase> {
    app.world()
        .get_resource::<State<RacePhase>>()
        .map(|s| *s.get())
}

/// Entities matching `F`, excluding resource entities (`count_entities::<()>` = all others).
pub fn count_entities<F: QueryFilter>(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<(), (F, Without<IsResource>)>();
    query.iter(app.world()).count()
}

/// Queues a state change for the next update.
pub fn request<S: FreelyMutableState>(app: &mut App, state: S) {
    app.world_mut().resource_mut::<NextState<S>>().set(state);
}

/// Menu → Loading with `config` → (every registered loader reports `LoadingDone`) → InRace in
/// `RacePhase::Setup`. A `LoadingFailed` leaves through Unloading → Menu and is returned.
pub fn load_race(app: &mut App, config: RaceConfig) -> Result<(), LoadingFailed> {
    assert_eq!(
        app_state(app),
        AppState::Menu,
        "load_race starts in the menu"
    );
    app.world_mut().insert_resource(config);
    request(app, AppState::Loading);
    open_gate(app)
}

/// [`load_race`], then leaves Setup straight to `RacePhase::Racing` (skipping Intro/Countdown):
/// the attempt is running when this returns.
pub fn start_race(app: &mut App, config: RaceConfig) -> Result<(), LoadingFailed> {
    load_race(app, config)?;
    set_phase(app, RacePhase::Racing);
    Ok(())
}

/// Sets the race phase and runs one update.
pub fn set_phase(app: &mut App, phase: RacePhase) {
    request(app, phase);
    app.update();
}

/// Restart without reloading: Setup (+ Running) → Racing; the attempt is rebuilt.
pub fn restart(app: &mut App) {
    assert_eq!(app_state(app), AppState::InRace, "restart needs a race");
    request(app, RacePhase::Setup);
    request(app, PauseState::Running);
    app.update();
    set_phase(app, RacePhase::Racing);
}

/// Next race: (optionally a new `config`) → Unloading → Loading → InRace → Racing.
pub fn next_race(app: &mut App, config: Option<RaceConfig>) -> Result<(), LoadingFailed> {
    assert_eq!(app_state(app), AppState::InRace, "next_race needs a race");
    if let Some(config) = config {
        app.world_mut().insert_resource(config);
    }
    request(app, AppState::Unloading);
    app.update();
    request(app, AppState::Loading);
    open_gate(app)?;
    set_phase(app, RacePhase::Racing);
    Ok(())
}

/// Leave the session: Unloading → Menu (also from Loading or Setup).
pub fn quit_to_menu(app: &mut App) {
    request(app, AppState::Unloading);
    app.update();
    app.world_mut().remove_resource::<RaceConfig>();
    request(app, AppState::Menu);
    app.update();
}

/// Pause the race (gates every `SimSet` and rapier) and run one update.
pub fn pause(app: &mut App) {
    request(app, PauseState::Paused);
    app.update();
}

/// Resume the race and run one update.
pub fn resume(app: &mut App) {
    request(app, PauseState::Running);
    app.update();
}

fn open_gate(app: &mut App) -> Result<(), LoadingFailed> {
    *app.world_mut().resource_mut::<LoadingLog>() = LoadingLog::default();
    app.update(); // enters Loading: loaders start
    for _ in 0..secs_to_ticks(TIMEOUT_SECS) {
        if let Some(failure) = app.world_mut().resource_mut::<LoadingLog>().failed.take() {
            quit_to_menu(app);
            return Err(failure);
        }
        let world = app.world();
        let log = world.resource::<LoadingLog>();
        let tasks = world.resource::<LoadingTasks>();
        if tasks.registered().iter().all(|key| log.done.contains(key)) {
            request(app, AppState::InRace);
            app.update();
            return Ok(());
        }
        app.update();
    }
    let log = app.world().resource::<LoadingLog>();
    panic!("loading timed out; done so far: {:?}", log.done);
}
