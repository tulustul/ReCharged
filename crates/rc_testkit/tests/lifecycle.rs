//! Session lifecycle with the testkit's stand-in flow (ARCHITECTURE §3.1, review 2 N1): session
//! entities from a loader and attempt entities from an `OnEnter(RaceAttempt)` spawner come back to
//! the baseline on every exit path; there is one session at a time; restarts rebuild the attempt.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_testkit::*;

const SESSION_THINGS: usize = 3;
const ATTEMPT_THINGS: usize = 2;

#[derive(Component)]
struct SessionThing;

#[derive(Component)]
struct AttemptThing;

/// How many times the dummy loader ran.
#[derive(Resource, Default)]
struct Loads(u32);

/// Makes the dummy loader fail.
#[derive(Resource)]
struct FailLoading;

fn dummy_world(app: &mut App) {
    LoadingTasks::register(app, "dummy");
    app.init_resource::<Loads>()
        .add_systems(OnEnter(AppState::Loading), dummy_loader)
        .add_systems(OnEnter(RaceAttempt), dummy_spawner);
}

fn dummy_loader(
    mut commands: Commands,
    mut loads: ResMut<Loads>,
    fail: Option<Res<FailLoading>>,
    mut done: MessageWriter<LoadingDone>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    loads.0 += 1;
    for _ in 0..SESSION_THINGS {
        commands.spawn((SessionThing, DespawnOnExit(RaceSession)));
    }
    if fail.is_some() {
        failed.write(LoadingFailed {
            key: "dummy",
            error: "tracks/broken.ron: bad data".into(),
        });
    } else {
        done.write(LoadingDone("dummy"));
    }
}

fn dummy_spawner(mut commands: Commands) {
    for index in 0..ATTEMPT_THINGS {
        commands.spawn((
            AttemptThing,
            Racer { index, ..default() },
            DespawnOnExit(RaceAttempt),
        ));
    }
}

fn attempt_ids(app: &mut App) -> Vec<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<AttemptThing>>();
    q.iter(app.world()).collect()
}

fn loads(app: &App) -> u32 {
    app.world().resource::<Loads>().0
}

#[test]
fn restart_and_next_race_keep_one_session_and_rebuild_the_attempt() {
    let mut app = headless_app(dummy_world);
    let baseline = count_entities::<()>(&mut app);

    start_race(&mut app, RaceConfig::default()).expect("loads");
    assert_eq!(race_phase(&app), Some(RacePhase::Racing));
    assert_eq!(
        count_entities::<With<SessionThing>>(&mut app),
        SESSION_THINGS
    );
    assert_eq!(count_entities::<With<Racer>>(&mut app), ATTEMPT_THINGS);

    let mut previous = attempt_ids(&mut app);
    for _ in 0..2 {
        restart(&mut app);
        let now = attempt_ids(&mut app);
        assert_eq!(now.len(), ATTEMPT_THINGS, "attempt entities recreated");
        assert!(
            now.iter().all(|e| !previous.contains(e)),
            "old attempt despawned"
        );
        assert_eq!(
            count_entities::<With<SessionThing>>(&mut app),
            SESSION_THINGS
        );
        assert_eq!(loads(&app), 1, "restart does not reload");
        previous = now;
    }

    next_race(&mut app, None).expect("reloads");
    assert_eq!(loads(&app), 2);
    assert_eq!(
        count_entities::<With<SessionThing>>(&mut app),
        SESSION_THINGS,
        "one session"
    );
    assert_eq!(
        count_entities::<With<AttemptThing>>(&mut app),
        ATTEMPT_THINGS
    );

    quit_to_menu(&mut app);
    assert_eq!(app_state(&app), AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
    assert!(app.world().get_resource::<RaceConfig>().is_none());
}

#[test]
fn leaving_during_setup_leaks_nothing() {
    let mut app = headless_app(dummy_world);
    let baseline = count_entities::<()>(&mut app);

    // Quit while the first attempt has not started yet.
    load_race(&mut app, RaceConfig::default()).expect("loads");
    assert_eq!(race_phase(&app), Some(RacePhase::Setup));
    assert_eq!(count_entities::<With<AttemptThing>>(&mut app), 0);
    quit_to_menu(&mut app);
    assert_eq!(count_entities::<()>(&mut app), baseline);

    // Restart and quit in the same frame.
    start_race(&mut app, RaceConfig::default()).expect("loads");
    request(&mut app, RacePhase::Setup);
    quit_to_menu(&mut app);
    assert_eq!(app_state(&app), AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
}

#[test]
fn load_error_returns_to_the_menu_without_leaks() {
    let mut app = headless_app(dummy_world);
    app.insert_resource(FailLoading);
    let baseline = count_entities::<()>(&mut app);

    let failure = start_race(&mut app, RaceConfig::default()).expect_err("loader fails");
    assert_eq!(failure.key, "dummy");
    assert_eq!(app_state(&app), AppState::Menu);
    assert_eq!(count_entities::<With<SessionThing>>(&mut app), 0);
    assert_eq!(count_entities::<()>(&mut app), baseline);
}
