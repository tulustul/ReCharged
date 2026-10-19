//! The real app flow (rc_game) driven by `GameCommand`s: transition table, loading gate,
//! placeholder phases, restart without reload, every exit through `Unloading` without leaks,
//! pause and its robustness triggers, Versus sessions, unlock validation, `--quick-race` and
//! `--soak` (ARCHITECTURE §3.1, §5.12, §8).

mod common;

use bevy::prelude::*;
use bevy::window::WindowFocused;
use common::*;
use rc_core::prelude::*;
use rc_game::{
    GameCommand, GameNotice, PlayerCar, RaceOutcome, RaceRequest, TrackPick, VersusSession,
};
use rc_testkit::*;

fn app(name: &str) -> App {
    app_in(&data_dir(name), None, None)
}

fn racers(app: &mut App) -> usize {
    count_entities::<With<Racer>>(app)
}

#[test]
fn boot_loads_the_catalog_and_opens_the_menu() {
    let app = app("boot");
    assert_eq!(app_state(&app), AppState::Menu);
    let world = app.world();
    assert!(
        world
            .resource::<Catalog>()
            .track(&"test_oval".into())
            .is_some()
    );
    assert!(world.get_resource::<Settings>().is_some());
    assert!(world.get_resource::<rc_game::Profile>().is_some());
    assert!(world.get_resource::<rc_game::MedalTable>().is_some());
    assert!(!world.resource::<LobbyState>().open);
}

#[test]
fn race_restart_next_race_and_quit_follow_the_transition_table() {
    let mut app = app("table");
    let baseline = count_entities::<()>(&mut app);

    start(&mut app, RaceRequest::quick("test_oval"));
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
    assert_eq!(racers(&mut app), MAX_RACERS);

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
    assert_eq!(racers(&mut app), MAX_RACERS);

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
    let mut app = app("setup-quit");
    let baseline = count_entities::<()>(&mut app);
    start(&mut app, RaceRequest::quick("test_oval"));
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
    let mut app = app("restart-quit");
    let baseline = count_entities::<()>(&mut app);
    start(&mut app, RaceRequest::quick("test_oval"));
    wait_for_phase(&mut app, RacePhase::Countdown);
    send(&mut app, GameCommand::Restart);
    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
}

#[test]
fn a_load_error_goes_back_to_the_menu_with_an_error() {
    let mut app = app("load-error");
    app.insert_resource(FailLoading);
    let baseline = count_entities::<()>(&mut app);
    start(&mut app, RaceRequest::quick("test_oval"));
    wait_for_state(&mut app, AppState::Unloading);
    wait_for_state(&mut app, AppState::Menu);
    assert_eq!(counters(&app).attempts, 0);
    assert_eq!(count_entities::<()>(&mut app), baseline);
    assert!(
        seen(&app)
            .notices
            .iter()
            .any(|n| matches!(n, GameNotice::Error(e) if e.contains("broken")))
    );
}

#[test]
fn pause_freezes_the_simulation_and_phases_until_resume() {
    let mut app = app("pause");
    start(&mut app, RaceRequest::quick("test_oval"));
    wait_for_phase(&mut app, RacePhase::Countdown);

    send(&mut app, GameCommand::Pause);
    step(&mut app, 2);
    let ticks = counters(&app).sim_ticks;
    step(&mut app, 120);
    assert_eq!(counters(&app).sim_ticks, ticks, "simulation frozen");
    assert_eq!(
        race_phase(&app),
        Some(RacePhase::Countdown),
        "phase clock frozen"
    );

    send(&mut app, GameCommand::Resume);
    send(&mut app, GameCommand::Resume); // idempotent
    wait_for_phase(&mut app, RacePhase::Racing);
    assert!(counters(&app).sim_ticks > ticks);

    app.world_mut().write_message(PauseRequest { slot: None });
    step(&mut app, 2);
    assert_eq!(
        *app.world().resource::<State<PauseState>>().get(),
        PauseState::Paused
    );
}

fn paused(app: &App) -> bool {
    app.world()
        .get_resource::<State<PauseState>>()
        .is_some_and(|s| *s.get() == PauseState::Paused)
}

#[test]
fn a_disconnected_controller_or_lost_focus_pauses_the_race() {
    let mut app = app("robust");
    app.world_mut().write_message(DeviceEvent::Disconnected {
        slot: PlayerSlot(0),
    });
    step(&mut app, 2);
    assert!(seen(&app).notices.is_empty(), "nothing happens in menus");

    start(&mut app, RaceRequest::quick("test_oval"));
    wait_for_phase(&mut app, RacePhase::Countdown);
    app.world_mut().write_message(DeviceEvent::Disconnected {
        slot: PlayerSlot(0),
    });
    step(&mut app, 2);
    assert!(paused(&app));
    assert!(
        seen(&app)
            .notices
            .contains(&GameNotice::ControllerDisconnected(PlayerSlot(0)))
    );

    send(&mut app, GameCommand::Resume);
    step(&mut app, 2);
    assert!(!paused(&app));
    let window = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(WindowFocused {
        window,
        focused: true,
    });
    step(&mut app, 2);
    assert!(!paused(&app), "gaining focus does nothing");
    app.world_mut().write_message(WindowFocused {
        window,
        focused: false,
    });
    step(&mut app, 2);
    assert!(paused(&app), "losing focus pauses; resuming needs input");

    // Automated runs often never get the focus: no auto-pause there.
    let automation = AutomationConfig {
        exit_after: Some(600.0),
        ..default()
    };
    let mut app = app_in(&data_dir("robust-automated"), Some(automation), None);
    start(&mut app, RaceRequest::quick("test_oval"));
    wait_for_phase(&mut app, RacePhase::Countdown);
    let window = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(WindowFocused {
        window,
        focused: false,
    });
    step(&mut app, 2);
    assert!(!paused(&app));
}

#[test]
fn finished_races_cannot_be_paused() {
    let mut app = app_in(&data_dir("finished-pause"), None, Some(5));
    start(&mut app, RaceRequest::quick("test_oval"));
    wait_for_phase(&mut app, RacePhase::Finished);
    send(&mut app, GameCommand::Pause);
    step(&mut app, 2);
    assert!(!paused(&app));
}

#[test]
fn locked_content_is_rejected_unless_unlock_all() {
    let mut app = app("locked");
    start(&mut app, RaceRequest::quick("maple_street"));
    step(&mut app, 3);
    assert_eq!(app_state(&app), AppState::Menu);
    assert!(
        seen(&app)
            .notices
            .iter()
            .any(|n| matches!(n, GameNotice::Error(e) if e.contains("locked")))
    );

    let all = AutomationConfig {
        unlock_all: true,
        ..default()
    };
    let mut app = app_in(&data_dir("unlock-all"), Some(all), None);
    start(&mut app, RaceRequest::quick("maple_street"));
    wait_for_state(&mut app, AppState::Loading);
    let profile = app.world().resource::<rc_game::Profile>();
    assert!(profile.unlocked.is_empty(), "--unlock-all earns nothing");
}

fn versus_request(track: &str) -> RaceRequest {
    RaceRequest {
        mode: GameMode::Versus,
        humans: vec![
            PlayerCar {
                slot: PlayerSlot(0),
                car: "zip".into(),
                paint: PaintId(0),
            },
            PlayerCar {
                slot: PlayerSlot(1),
                car: "sparky".into(),
                paint: PaintId(2),
            },
        ],
        ai: Some(2),
        ..RaceRequest::quick(track)
    }
}

/// The grid: player names, "AI" for rivals.
fn grid(app: &App) -> Vec<String> {
    let config = app.world().resource::<RaceConfig>();
    let name = |r: &RacerSetup| match r.controller {
        Controller::Human { .. } => r.name.clone(),
        Controller::Ai { .. } => "AI".into(),
    };
    config.racers.iter().map(name).collect()
}

fn versus_points(app: &App, slot: u8) -> u32 {
    app.world()
        .resource::<VersusSession>()
        .points(PlayerSlot(slot))
}

#[test]
fn versus_keeps_a_scoreboard_over_rematches_and_tracks_and_ai_takes_over_dropped_players() {
    let mut app = app_in(&data_dir("versus"), None, Some(5));
    let baseline = count_entities::<()>(&mut app);
    send(&mut app, GameCommand::OpenLobby);
    step(&mut app, 2);
    assert!(app.world().resource::<LobbyState>().open);

    start(&mut app, versus_request("test_oval"));
    wait_for_outcome(&mut app);
    assert!(
        !app.world().resource::<LobbyState>().open,
        "the lobby closes for the race"
    );
    // The rivals fill the front of the grid, P1 and P2 start behind them in slot order.
    assert_eq!(grid(&app), ["AI", "AI", "P1", "P2"]);
    // P1 and P2 finish first, then the rivals.
    assert_eq!(
        app.world().resource::<RaceOutcome>().points,
        [Some(10), Some(8), None, None],
        "only the players score in Versus"
    );
    assert_eq!((versus_points(&app, 0), versus_points(&app, 1)), (10, 8));

    app.world_mut().resource_mut::<FinishOrder>().0 = vec![1, 0, 2, 3];
    send(&mut app, GameCommand::Rematch);
    wait_for_phase(&mut app, RacePhase::Countdown);
    assert_eq!(
        grid(&app),
        ["AI", "AI", "P2", "P1"],
        "the leader starts last"
    );
    wait_for_outcome(&mut app);
    assert_eq!(counters(&app).loads, 1, "rematch does not reload");
    assert_eq!((versus_points(&app, 0), versus_points(&app, 1)), (18, 18));

    send(&mut app, GameCommand::Restart); // replays the rematch: its points are reverted
    step(&mut app, 2);
    assert_eq!((versus_points(&app, 0), versus_points(&app, 1)), (10, 8));
    wait_for_phase(&mut app, RacePhase::Countdown);
    assert_eq!(
        grid(&app),
        ["AI", "AI", "P2", "P1"],
        "a restart keeps the race's grid"
    );
    wait_for_outcome(&mut app);
    assert_eq!(
        app.world()
            .resource::<VersusSession>()
            .series
            .races_played(),
        2
    );

    send(&mut app, GameCommand::DropPlayer(PlayerSlot(1)));
    step(&mut app, 2);
    assert_eq!(seen(&app).dropped, [PlayerSlot(1)]);
    let autopilot = |app: &App| {
        app.world().resource::<RaceConfig>().racers.iter().any(|r| {
            matches!(
                r.controller,
                Controller::Human {
                    slot: PlayerSlot(1),
                    autopilot: true
                }
            )
        })
    };
    assert!(autopilot(&app));

    let next = TrackVariant::forward("toy_town");
    send(&mut app, GameCommand::NextTrack(next.clone()));
    wait_for_next_outcome(&mut app);
    assert_eq!(app.world().resource::<RaceConfig>().variant, next);
    assert!(autopilot(&app), "the AI keeps the dropped player's car");
    assert_eq!(
        app.world()
            .resource::<VersusSession>()
            .series
            .races_played(),
        3
    );

    // "Next track" with options: the picked laps and items apply, the players stay.
    let pick = TrackPick {
        variant: TrackVariant::forward("test_oval"),
        laps: Some(2),
        items: Some(false),
        ..default()
    };
    send(&mut app, GameCommand::ChangeTrack(Box::new(pick)));
    wait_for_next_outcome(&mut app);
    let config = app.world().resource::<RaceConfig>();
    assert_eq!(
        (config.variant.track.as_str(), config.laps, config.items),
        ("test_oval", 2, false)
    );
    assert!(autopilot(&app), "the players and their cars stay");

    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
    assert!(
        app.world().contains_resource::<VersusSession>(),
        "back to the lobby: session kept"
    );
    start(&mut app, versus_request("test_oval"));
    wait_for_outcome(&mut app);
    assert_eq!(
        app.world()
            .resource::<VersusSession>()
            .series
            .races_played(),
        5
    );
    assert!(
        !autopilot(&app),
        "a race from the lobby has its players back"
    );
    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);
    send(&mut app, GameCommand::CloseLobby);
    step(&mut app, 2);
    assert!(!app.world().contains_resource::<VersusSession>());
}

#[test]
fn quick_race_flag_starts_the_cup_race_on_its_track() {
    let automation = AutomationConfig {
        unlock_all: true,
        quick_race: Some(QuickRaceArgs {
            track: "backyard_bash".into(),
            mode: QuickRaceMode::Cup("rookie_cup".into()),
            ..default()
        }),
        ..default()
    };
    let mut app = app_in(&data_dir("quick-cup"), Some(automation), None);
    wait_for_state(&mut app, AppState::InRace);
    let config = app.world().resource::<RaceConfig>();
    assert_eq!(
        config.mode,
        GameMode::Championship {
            cup: "rookie_cup".into(),
            race_index: 2
        }
    );
    assert_eq!(config.racers.len(), MAX_RACERS);
}

fn soak(app: &mut App) -> AppExit {
    for _ in 0..60 * 60 * 2 {
        if let Some(exit) = app.should_exit() {
            return exit;
        }
        app.update();
    }
    panic!("the soak never ended");
}

fn soak_app(name: &str, minutes: f32) -> App {
    let automation = AutomationConfig {
        soak_minutes: Some(minutes),
        soak_seed: Some(7),
        ..default()
    };
    app_in(&data_dir(name), Some(automation), Some(30))
}

#[test]
fn a_short_soak_chains_races_without_leaks() {
    let mut app = soak_app("soak", 1.0);
    assert_eq!(soak(&mut app), AppExit::Success);
    assert!(
        counters(&app).loads >= 5,
        "races were chained: {} loads",
        counters(&app).loads
    );
}

/// A plugin bug the soak must catch: an attempt entity without a scope.
fn leaky(app: &mut App) {
    app.add_systems(OnEnter(RaceAttempt), |mut commands: Commands| {
        commands.spawn(Transform::default());
    });
}

#[test]
fn the_soak_catches_a_leak() {
    let mut app = soak_app("soak-leak", 1.0);
    leaky(&mut app);
    assert!(matches!(soak(&mut app), AppExit::Error(_)));
}
