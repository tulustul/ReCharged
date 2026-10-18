//! The real app flow (rc_game) driven by `GameCommand`s: transition table, loading gate,
//! placeholder phases, restart without reload, every exit through `Unloading` without leaks,
//! and pause (ARCHITECTURE §3.1).

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_game::{GameCommand, GameFlowPlugin, RaceRequest};
use rc_testkit::*;

const SESSION_THINGS: usize = 3;
const ATTEMPT_THINGS: usize = 2;

#[derive(Component)]
struct SessionThing;

#[derive(Component)]
struct AttemptThing;

#[derive(Resource, Default)]
struct Counters {
    loads: u32,
    attempts: u32,
    race_ticks: u32,
    phases: Vec<RacePhase>,
}

#[derive(Resource)]
struct FailLoading;

/// Dummy loader + attempt spawner + a race-tick counter, as sim crates will do it.
fn dummy_world(app: &mut App) {
    LoadingTasks::register(app, "dummy");
    app.init_resource::<Counters>()
        .add_systems(OnEnter(AppState::Loading), dummy_loader)
        .add_systems(OnEnter(RaceAttempt), dummy_spawner)
        .add_systems(
            FixedUpdate,
            (|mut c: ResMut<Counters>| c.race_ticks += 1).in_set(SimSet::Race),
        )
        .add_systems(Update, record_phases);
}

fn dummy_loader(
    mut commands: Commands,
    mut counters: ResMut<Counters>,
    fail: Option<Res<FailLoading>>,
    mut done: MessageWriter<LoadingDone>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    counters.loads += 1;
    for _ in 0..SESSION_THINGS {
        commands.spawn((SessionThing, DespawnOnExit(RaceSession)));
    }
    if fail.is_some() {
        failed.write(LoadingFailed {
            key: "dummy",
            error: "broken".into(),
        });
    } else {
        done.write(LoadingDone("dummy"));
    }
}

fn dummy_spawner(mut commands: Commands, mut counters: ResMut<Counters>) {
    counters.attempts += 1;
    for index in 0..ATTEMPT_THINGS {
        commands.spawn((
            AttemptThing,
            Racer { index, ..default() },
            DespawnOnExit(RaceAttempt),
        ));
    }
}

fn record_phases(
    mut events: MessageReader<StateTransitionEvent<RacePhase>>,
    mut counters: ResMut<Counters>,
) {
    counters
        .phases
        .extend(events.read().filter_map(|e| e.entered));
}

fn app() -> App {
    headless_app((GameFlowPlugin, dummy_world))
}

fn send(app: &mut App, command: GameCommand) {
    app.world_mut().write_message(command);
}

fn counters(app: &App) -> &Counters {
    app.world().resource::<Counters>()
}

fn wait_for_phase(app: &mut App, phase: RacePhase) {
    step_until(app, &format!("{phase:?}"), |w| {
        w.get_resource::<State<RacePhase>>()
            .is_some_and(|s| *s.get() == phase)
    });
}

fn wait_for_state(app: &mut App, state: AppState) {
    step_until(app, &format!("{state:?}"), |w| {
        *w.resource::<State<AppState>>().get() == state
    });
}

#[test]
fn boot_loads_the_catalog_and_opens_the_menu() {
    let app = app();
    assert_eq!(app_state(&app), AppState::Menu);
    assert!(
        app.world()
            .resource::<Catalog>()
            .track(&"test_oval".into())
            .is_some()
    );
    assert!(app.world().get_resource::<Settings>().is_some());
}

#[test]
fn race_restart_next_race_and_quit_follow_the_transition_table() {
    let mut app = app();
    let baseline = count_entities::<()>(&mut app);

    send(
        &mut app,
        GameCommand::StartRace(RaceRequest::quick("test_oval")),
    );
    wait_for_phase(&mut app, RacePhase::Racing);
    assert_eq!(
        counters(&app).phases,
        [
            RacePhase::Setup,
            RacePhase::Intro,
            RacePhase::Countdown,
            RacePhase::Racing
        ]
    );
    assert_eq!(
        app.world().resource::<RaceConfig>().racers.len(),
        MAX_RACERS
    );
    assert_eq!(
        count_entities::<With<AttemptThing>>(&mut app),
        ATTEMPT_THINGS
    );

    app.world_mut().resource_mut::<Counters>().phases.clear();
    send(&mut app, GameCommand::Restart);
    wait_for_phase(&mut app, RacePhase::Countdown);
    wait_for_phase(&mut app, RacePhase::Racing);
    assert_eq!(
        counters(&app).phases,
        [RacePhase::Setup, RacePhase::Countdown, RacePhase::Racing],
        "no intro on restart"
    );
    assert_eq!(
        (counters(&app).loads, counters(&app).attempts),
        (1, 2),
        "restart does not reload"
    );
    assert_eq!(
        count_entities::<With<AttemptThing>>(&mut app),
        ATTEMPT_THINGS
    );

    send(&mut app, GameCommand::NextRace);
    wait_for_state(&mut app, AppState::Unloading);
    wait_for_phase(&mut app, RacePhase::Intro);
    assert_eq!(counters(&app).loads, 2);
    assert_eq!(
        count_entities::<With<SessionThing>>(&mut app),
        SESSION_THINGS,
        "one session at a time"
    );

    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
    assert!(app.world().get_resource::<RaceConfig>().is_none());
}

#[test]
fn quitting_while_still_in_setup_leaks_nothing() {
    let mut app = app();
    let baseline = count_entities::<()>(&mut app);
    send(
        &mut app,
        GameCommand::StartRace(RaceRequest::quick("test_oval")),
    );
    // The gate has opened: InRace (Setup) is entered on the next update — quit in that frame.
    step_until(&mut app, "the gate to open", |w| {
        matches!(
            *w.resource::<NextState<AppState>>(),
            NextState::PendingIfDifferent(AppState::InRace)
        )
    });
    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);
    assert_eq!(counters(&app).attempts, 0, "the attempt never started");
    assert_eq!(count_entities::<()>(&mut app), baseline);
}

#[test]
fn restart_and_quit_in_the_same_frame_leak_nothing() {
    let mut app = app();
    let baseline = count_entities::<()>(&mut app);
    send(
        &mut app,
        GameCommand::StartRace(RaceRequest::quick("test_oval")),
    );
    wait_for_phase(&mut app, RacePhase::Countdown);
    send(&mut app, GameCommand::Restart);
    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
}

#[test]
fn a_load_error_goes_back_to_the_menu() {
    let mut app = app();
    app.insert_resource(FailLoading);
    let baseline = count_entities::<()>(&mut app);
    send(
        &mut app,
        GameCommand::StartRace(RaceRequest::quick("test_oval")),
    );
    wait_for_state(&mut app, AppState::Unloading);
    wait_for_state(&mut app, AppState::Menu);
    assert_eq!(counters(&app).attempts, 0);
    assert_eq!(count_entities::<()>(&mut app), baseline);
}

#[test]
fn pause_freezes_race_ticks_and_phases_until_resume() {
    let mut app = app();
    send(
        &mut app,
        GameCommand::StartRace(RaceRequest::quick("test_oval")),
    );
    wait_for_phase(&mut app, RacePhase::Countdown);

    send(&mut app, GameCommand::Pause);
    step(&mut app, 2);
    let ticks = counters(&app).race_ticks;
    step(&mut app, 120);
    assert_eq!(counters(&app).race_ticks, ticks, "race ticks frozen");
    assert_eq!(
        race_phase(&app),
        Some(RacePhase::Countdown),
        "phase clock frozen"
    );

    send(&mut app, GameCommand::Resume);
    send(&mut app, GameCommand::Resume); // idempotent
    wait_for_phase(&mut app, RacePhase::Racing);
    assert!(counters(&app).race_ticks > ticks);

    app.world_mut().write_message(PauseRequest { slot: None });
    step(&mut app, 2);
    assert_eq!(
        *app.world().resource::<State<PauseState>>().get(),
        PauseState::Paused
    );
}
