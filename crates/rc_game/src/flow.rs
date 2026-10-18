//! Session lifecycle transitions, loading gate and pause (ARCHITECTURE §3.1).
//!
//! | From | Trigger | To |
//! |---|---|---|
//! | Boot | catalog + settings ready | Menu |
//! | Menu | `StartRace` | Loading |
//! | Loading | every registered loader done (+ min display time) | InRace (Setup) |
//! | Loading | `LoadingFailed` | Unloading → Menu |
//! | InRace | `Restart` | `RacePhase::Setup` (no reload) |
//! | InRace | `NextRace` | Unloading → Loading |
//! | Loading / InRace | `QuitToMenu` | Unloading → Menu |

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::Window;
use rc_core::prelude::*;

use crate::{GameCommand, MIN_LOADING_SECS, RaceRequest, build_race_config};

/// Where the flow goes after the one-frame `Unloading`.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
enum AfterUnload {
    /// Back to the menu (quit, load error).
    #[default]
    Menu,
    /// Straight into the next session (next race).
    Reload,
}

/// The command/transition systems (the placeholder phase driver runs after them).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Transitions;

/// Real time at which the current `Loading` began.
#[derive(Resource, Debug, Default)]
struct LoadingStarted(Duration);

pub(crate) fn build(app: &mut App) {
    app.init_resource::<AfterUnload>()
        .init_resource::<LoadingStarted>()
        .add_systems(OnEnter(AppState::Boot), boot)
        .add_systems(OnEnter(AppState::Menu), start_quick_race)
        .add_systems(OnEnter(AppState::Loading), begin_loading)
        .add_systems(
            Update,
            (
                log_transitions,
                start_race,
                // Before the commands: a quit in the frame the gate opens wins (last write).
                loading_gate.run_if(in_state(AppState::Loading)),
                session_commands,
                pause_commands,
                quit_game,
                finish_unloading.run_if(in_state(AppState::Unloading)),
            )
                .chain()
                .in_set(Transitions)
                .in_set(FlowSet),
        );
}

fn automated(automation: Option<&AutomationConfig>) -> bool {
    automation.is_some_and(AutomationConfig::is_automated)
}

/// Loads the catalog (unless a test inserted one) and default settings, then opens the menu.
fn boot(
    mut commands: Commands,
    paths: Option<Res<UserPaths>>,
    catalog: Option<Res<Catalog>>,
    automation: Option<Res<AutomationConfig>>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if catalog.is_none() {
        let dir = paths.map_or_else(|| PathBuf::from("assets"), |p| p.assets_dir.clone());
        match Catalog::load(&dir) {
            Ok(catalog) => {
                info!(
                    "catalog: {} cars, {} tracks, {} cups, {} rivals",
                    catalog.cars.len(),
                    catalog.tracks.len(),
                    catalog.cups.len(),
                    catalog.rivals.len()
                );
                commands.insert_resource(catalog);
            }
            Err(e) => {
                error!("{e}");
                commands.insert_resource(Catalog::default());
                if automated(automation.as_deref()) {
                    exit.write(AppExit::error());
                }
            }
        }
    }
    // ponytail: defaults only; T07 loads/persists settings.ron in UserPaths::data_dir.
    commands.init_resource::<Settings>();
    next.set_if_different(AppState::Menu);
}

/// `--quick-race`: start the race once, skipping the menu.
fn start_quick_race(
    automation: Option<Res<AutomationConfig>>,
    mut started: Local<bool>,
    mut out: MessageWriter<GameCommand>,
) {
    let Some(args) = automation.as_ref().and_then(|a| a.quick_race.as_ref()) else {
        return;
    };
    if !*started {
        *started = true;
        info!("--quick-race {}", args.track);
        out.write(GameCommand::StartRace(RaceRequest::from(args)));
    }
}

/// What building a `RaceConfig` reads.
#[derive(SystemParam)]
struct SetupInputs<'w> {
    catalog: Option<Res<'w, Catalog>>,
    settings: Option<Res<'w, Settings>>,
    automation: Option<Res<'w, AutomationConfig>>,
}

fn start_race(
    mut requests: MessageReader<GameCommand>,
    state: Res<State<AppState>>,
    inputs: SetupInputs,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    let automation = inputs.automation.as_deref();
    for command in requests.read() {
        let GameCommand::StartRace(request) = command else {
            continue;
        };
        if *state.get() != AppState::Menu {
            warn!("StartRace ignored in {:?}", state.get());
            continue;
        }
        let (Some(catalog), Some(settings)) =
            (inputs.catalog.as_deref(), inputs.settings.as_deref())
        else {
            error!("StartRace before boot finished");
            continue;
        };
        let autopilot = automation.is_some_and(|a| a.autopilot);
        let mut request = request.clone();
        request.seed = request.seed.or_else(|| Some(clock_seed()));
        match build_race_config(&request, catalog, settings, autopilot) {
            Ok(config) => {
                info!(
                    "starting {:?} on {} ({} racers, {} laps, seed {})",
                    config.mode,
                    config.variant.track,
                    config.racers.len(),
                    config.laps,
                    config.seed
                );
                commands.insert_resource(config);
                next.set(AppState::Loading);
            }
            Err(e) => {
                error!("cannot start race: {e}");
                if automated(automation) {
                    exit.write(AppExit::error());
                }
            }
        }
    }
}

fn clock_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

/// Restart, next race and quit.
fn session_commands(
    mut requests: MessageReader<GameCommand>,
    state: Res<State<AppState>>,
    mut next_app: ResMut<NextState<AppState>>,
    mut next_phase: ResMut<NextState<RacePhase>>,
    mut next_pause: ResMut<NextState<PauseState>>,
    mut after: ResMut<AfterUnload>,
) {
    let state = *state.get();
    for command in requests.read() {
        match (command, state) {
            (GameCommand::Restart, AppState::InRace) => {
                info!("restart");
                next_phase.set(RacePhase::Setup);
                next_pause.set(PauseState::Running);
            }
            (GameCommand::NextRace, AppState::InRace) => {
                *after = AfterUnload::Reload;
                next_app.set(AppState::Unloading);
            }
            (GameCommand::QuitToMenu, AppState::Loading | AppState::InRace) => {
                *after = AfterUnload::Menu;
                next_app.set(AppState::Unloading);
            }
            (GameCommand::Restart | GameCommand::NextRace | GameCommand::QuitToMenu, _) => {
                warn!("{command:?} ignored in {state:?}");
            }
            _ => {}
        }
    }
}

/// Pause from devices (`PauseRequest`) or React; resume from React. Never touches `Time<Virtual>`.
fn pause_commands(
    mut requests: MessageReader<GameCommand>,
    mut pause_requests: MessageReader<PauseRequest>,
    pause: Option<Res<State<PauseState>>>,
    mut next_pause: ResMut<NextState<PauseState>>,
) {
    let wants_pause = pause_requests.read().count() > 0;
    let mut target = None;
    for command in requests.read() {
        match command {
            GameCommand::Pause => target = Some(PauseState::Paused),
            GameCommand::Resume => target = Some(PauseState::Running),
            _ => {}
        }
    }
    let Some(current) = pause.map(|p| *p.get()) else {
        return; // not in a race
    };
    if let Some(target) = target.or(wants_pause.then_some(PauseState::Paused)) {
        next_pause.set_if_different(target);
        if target != current {
            info!("{target:?}");
        }
    }
}

fn quit_game(mut requests: MessageReader<GameCommand>, mut exit: MessageWriter<AppExit>) {
    if requests.read().any(|c| *c == GameCommand::QuitGame) {
        info!("quit");
        exit.write(AppExit::Success);
    }
}

fn begin_loading(
    mut tasks: ResMut<LoadingTasks>,
    mut started: ResMut<LoadingStarted>,
    time: Res<Time<Real>>,
) {
    tasks.reset();
    started.0 = time.elapsed();
    info!("loading: waiting for {:?}", tasks.registered());
}

/// Completes loader keys; opens the gate when none is pending; leaves on a failure.
fn loading_gate(
    (mut done, mut failed): (MessageReader<LoadingDone>, MessageReader<LoadingFailed>),
    mut tasks: ResMut<LoadingTasks>,
    (started, time): (Res<LoadingStarted>, Res<Time<Real>>),
    (automation, windows): (Option<Res<AutomationConfig>>, Query<(), With<Window>>),
    mut next: ResMut<NextState<AppState>>,
    mut after: ResMut<AfterUnload>,
    mut exit: MessageWriter<AppExit>,
) {
    for LoadingDone(key) in done.read() {
        if !tasks.complete(key) {
            debug!("loading: ignored LoadingDone({key})");
        }
    }
    if let Some(failure) = failed.read().last() {
        error!("loading failed ({}): {}", failure.key, failure.error);
        *after = AfterUnload::Menu;
        next.set(AppState::Unloading);
        if automated(automation.as_deref()) {
            exit.write(AppExit::error());
        }
        return;
    }
    let min = if automated(automation.as_deref()) || windows.is_empty() {
        0.0
    } else {
        MIN_LOADING_SECS
    };
    if tasks.is_done() && (time.elapsed() - started.0).as_secs_f32() >= min {
        next.set_if_different(AppState::InRace);
    }
}

fn finish_unloading(
    mut after: ResMut<AfterUnload>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    match std::mem::take(&mut *after) {
        AfterUnload::Reload => next.set(AppState::Loading),
        AfterUnload::Menu => {
            commands.remove_resource::<RaceConfig>();
            next.set(AppState::Menu);
        }
    }
}

fn log_transitions(
    mut app: MessageReader<StateTransitionEvent<AppState>>,
    mut phase: MessageReader<StateTransitionEvent<RacePhase>>,
) {
    for e in app.read() {
        info!("AppState {:?} -> {:?}", e.exited, e.entered);
    }
    for e in phase.read() {
        if let Some(entered) = e.entered {
            info!("RacePhase -> {entered:?}");
        }
    }
}
