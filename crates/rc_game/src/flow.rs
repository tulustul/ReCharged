//! Boot, session lifecycle transitions, loading gate and pause (ARCHITECTURE §3.1).
//!
//! | From | Trigger | To |
//! |---|---|---|
//! | Boot | catalog, medals, settings, profile ready | Menu |
//! | Menu | `StartRace` | Loading |
//! | Loading | every registered loader done (+ min display time) | InRace (Setup) |
//! | Loading | `LoadingFailed` | Unloading → Menu (+ `GameNotice::Error`) |
//! | InRace | `Restart` / `Rematch` | `RacePhase::Setup` (no reload) |
//! | InRace | `NextRace` / `NextTrack` | Unloading → Loading |
//! | Loading / InRace | `QuitToMenu`, last cup race done + `NextRace` | Unloading → Menu |

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::{Window, WindowFocused};
use rc_core::prelude::*;

use crate::medals::MedalTable;
use crate::profile::Profile;
use crate::save::{self, Loaded, SaveState};
use crate::series::CupRun;
use crate::{GameCommand, GameNotice, MIN_LOADING_SECS, Step, commands};

/// Where the flow goes after the one-frame `Unloading`.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub(crate) enum AfterUnload {
    /// Back to the menu (quit, load error, cup finished).
    #[default]
    Menu,
    /// Straight into the next session with this setup.
    Reload(Box<RaceConfig>),
}

/// Real time at which the current `Loading` began.
#[derive(Resource, Debug, Default)]
struct LoadingStarted(std::time::Duration);

pub(crate) fn build(app: &mut App) {
    app.init_resource::<AfterUnload>()
        .init_resource::<LoadingStarted>()
        .add_systems(
            OnEnter(AppState::Boot),
            (boot_catalog, boot_saves, open_menu).chain(),
        )
        .add_systems(OnEnter(AppState::Menu), commands::start_quick_race)
        .add_systems(OnEnter(AppState::Loading), begin_loading)
        .add_systems(Update, signals.in_set(Step::Signals))
        .add_systems(
            Update,
            (
                log_transitions,
                commands::start_race,
                // Before the session commands: a quit in the frame the gate opens wins.
                loading_gate.run_if(in_state(AppState::Loading)),
                commands::session_commands,
                commands::lobby_commands,
                pause_commands,
                quit_game,
                finish_unloading.run_if(in_state(AppState::Unloading)),
            )
                .chain()
                .in_set(Step::Transitions),
        );
}

/// An automated run that should stop on errors (`--soak` survives them by design).
pub(crate) fn exits_on_error(automation: Option<&AutomationConfig>) -> bool {
    automation.is_some_and(|a| a.is_automated() && a.soak_minutes.is_none())
}

/// Loads the catalog (unless a test inserted one) and the medal targets.
fn boot_catalog(
    mut commands: Commands,
    paths: Option<Res<UserPaths>>,
    catalog: Option<Res<Catalog>>,
    automation: Option<Res<AutomationConfig>>,
    mut notices: MessageWriter<GameNotice>,
    mut exit: MessageWriter<AppExit>,
) {
    let dir = paths.map_or_else(|| PathBuf::from("assets"), |p| p.assets_dir.clone());
    if catalog.is_none() {
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
                notices.write(GameNotice::Error(e.to_string()));
                commands.insert_resource(Catalog::default());
                if exits_on_error(automation.as_deref()) {
                    exit.write(AppExit::error());
                }
            }
        }
    }
    match MedalTable::load(&dir) {
        Ok(medals) => commands.insert_resource(medals),
        Err(e) => {
            error!("medal targets: {e}");
            commands.insert_resource(MedalTable::default());
        }
    }
}

/// Settings and profile from the user-data dir (defaults when missing; corrupt files are backed
/// up). Resources a test inserted beforehand are kept.
fn boot_saves(
    mut commands: Commands,
    paths: Option<Res<UserPaths>>,
    (settings, profile): (Option<Res<Settings>>, Option<Res<Profile>>),
    catalog: Option<Res<Catalog>>,
    automation: Option<Res<AutomationConfig>>,
    mut notices: MessageWriter<GameNotice>,
) {
    let dir = paths.map(|p| p.data_dir.clone());
    let read = |name: &str| -> Option<PathBuf> { dir.as_ref().map(|d| d.join(name)) };
    let mut restored = |file: &PathBuf, error: String, backup: Option<PathBuf>| {
        warn!("{error}; using defaults (backup: {backup:?})");
        notices.write(GameNotice::SaveRestored {
            file: file.display().to_string(),
            backup: backup.map(|b| b.display().to_string()),
        });
    };
    if settings.is_none() {
        let mut settings = Settings::default();
        if let Some(path) = read(save::SETTINGS_FILE) {
            match save::load::<Settings>(&path) {
                Loaded::Ok(loaded) => settings = loaded,
                Loaded::Missing => {}
                Loaded::Corrupt { error, backup } => restored(&path, error, backup),
            }
        }
        crate::prefs::sanitize(&mut settings);
        commands.insert_resource(settings);
    }
    let mut profile = match profile {
        Some(p) => p.clone(),
        None => match read(save::PROFILE_FILE) {
            Some(path) => match save::load::<Profile>(&path) {
                Loaded::Ok(loaded) => loaded,
                Loaded::Missing => Profile::default(),
                Loaded::Corrupt { error, backup } => {
                    restored(&path, error, backup);
                    Profile::default()
                }
            },
            None => Profile::default(),
        },
    };
    let automation = automation.as_deref();
    profile.unlock_all = automation.is_some_and(|a| a.unlock_all || a.soak_minutes.is_some());
    if let Some(catalog) = catalog.as_deref() {
        profile.refresh_unlocks(catalog); // earned while the game was closed: NEW badges only
    }
    commands.insert_resource(HintsSeen(profile.hints_seen.clone()));
    commands.insert_resource(SaveState::new(profile.stats.time_played_secs));
    commands.insert_resource(profile);
}

fn open_menu(mut next: ResMut<NextState<AppState>>) {
    next.set_if_different(AppState::Menu);
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
    (mut next, mut after): (ResMut<NextState<AppState>>, ResMut<AfterUnload>),
    (mut notices, mut exit): (MessageWriter<GameNotice>, MessageWriter<AppExit>),
) {
    for LoadingDone(key) in done.read() {
        if !tasks.complete(key) {
            debug!("loading: ignored LoadingDone({key})");
        }
    }
    if let Some(failure) = failed.read().last() {
        error!("loading failed ({}): {}", failure.key, failure.error);
        notices.write(GameNotice::Error(failure.error.clone()));
        *after = AfterUnload::Menu;
        next.set(AppState::Unloading);
        if exits_on_error(automation.as_deref()) {
            exit.write(AppExit::error());
        }
        return;
    }
    let automated = automation
        .as_deref()
        .is_some_and(AutomationConfig::is_automated);
    let min = if automated || windows.is_empty() {
        0.0
    } else {
        MIN_LOADING_SECS
    };
    if tasks.is_done() && (time.elapsed() - started.0).as_secs_f32() >= min {
        next.set_if_different(AppState::InRace);
    }
}

/// After the one frame of `Unloading`: the next session, or the menu (dropping the session's
/// setup and an unfinished cup).
fn finish_unloading(
    mut after: ResMut<AfterUnload>,
    mut next: ResMut<NextState<AppState>>,
    cup: Option<Res<CupRun>>,
    mut commands: Commands,
) {
    match std::mem::take(&mut *after) {
        AfterUnload::Reload(config) => {
            commands.insert_resource(*config);
            next.set(AppState::Loading);
        }
        AfterUnload::Menu => {
            commands.remove_resource::<RaceConfig>();
            commands.remove_resource::<commands::ActiveRequest>();
            if cup.is_some_and(|c| !c.finished()) {
                info!("cup abandoned");
                commands.remove_resource::<CupRun>();
            }
            next.set(AppState::Menu);
        }
    }
}

/// The race phases a pause may interrupt.
fn pausable(phase: Option<&State<RacePhase>>) -> bool {
    phase.is_some_and(|p| {
        matches!(
            p.get(),
            RacePhase::Intro | RacePhase::Countdown | RacePhase::Racing
        )
    })
}

/// Window focus loss → pause request (not in automated runs: window managers often never focus
/// the game window there); controller disconnect in a race → pause + dialog.
fn signals(
    (mut focus, mut devices): (MessageReader<WindowFocused>, MessageReader<DeviceEvent>),
    phase: Option<Res<State<RacePhase>>>,
    automation: Option<Res<AutomationConfig>>,
    mut requests: MessageWriter<PauseRequest>,
    mut notices: MessageWriter<GameNotice>,
) {
    let in_race = phase.is_some();
    let lost_focus = focus.read().any(|f| !f.focused);
    let automated = automation.is_some_and(|a| a.is_automated());
    if lost_focus && !automated && pausable(phase.as_deref()) {
        info!("window lost focus: pausing");
        requests.write(PauseRequest { slot: None });
    }
    for event in devices.read() {
        if let DeviceEvent::Disconnected { slot } = *event
            && in_race
        {
            warn!("P{}'s controller disconnected", slot.0 + 1);
            requests.write(PauseRequest { slot: Some(slot) });
            notices.write(GameNotice::ControllerDisconnected(slot));
        }
    }
}

/// Pause from devices (`PauseRequest`) or React; resume from React. Never touches `Time<Virtual>`.
fn pause_commands(
    mut requests: MessageReader<GameCommand>,
    mut pause_requests: MessageReader<PauseRequest>,
    pause: Option<Res<State<PauseState>>>,
    phase: Option<Res<State<RacePhase>>>,
    mut next_pause: ResMut<NextState<PauseState>>,
) {
    let wants_pause = pause_requests.read().count() > 0;
    let mut target = wants_pause.then_some(PauseState::Paused);
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
    if target == Some(PauseState::Paused) && !pausable(phase.as_deref()) {
        return;
    }
    if let Some(target) = target {
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
