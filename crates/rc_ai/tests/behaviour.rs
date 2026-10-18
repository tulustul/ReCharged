//! Headless behaviours of the AI on the full sim stack: starts, items, autopilot, steering
//! assist, catch-up, wall recovery.

mod common;

use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use common::*;
use rc_core::prelude::*;
use rc_testkit::{load_race, race_phase, step, step_seconds};

/// Steps until the race is running (Intro and Countdown done), collecting race events.
fn run_to_go(app: &mut App, events: &mut Vec<RaceEvent>) {
    let mut cursor = MessageCursor::<RaceEvent>::default();
    for _ in 0..secs_to_ticks(10.0) {
        step(app, 1);
        events.extend(
            cursor
                .read(app.world().resource::<Messages<RaceEvent>>())
                .copied(),
        );
        if race_phase(app) == Some(RacePhase::Racing) {
            break;
        }
    }
    step(app, 2);
    events.extend(
        cursor
            .read(app.world().resource::<Messages<RaceEvent>>())
            .copied(),
    );
}

fn human(autopilot: bool) -> RacerSetup {
    RacerSetup {
        name: "Player".into(),
        car: CarId::new("hornet"),
        controller: Controller::Human {
            slot: PlayerSlot(0),
            autopilot,
        },
        ..default()
    }
}

fn find(app: &mut App, index: usize) -> Entity {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Racer)>();
    q.iter(world)
        .find(|(_, r)| r.index == index)
        .map(|(e, _)| e)
        .expect("racer spawned")
}

#[test]
fn perfect_starts_follow_the_difficulty_and_nobody_stalls() {
    let mut app = race_app();
    let racers = ai_field(&app, 8, Difficulty::ElitePlus, CarClass::Pro);
    load_race(&mut app, config("test_oval", racers, 1, 3)).expect("loads");
    let mut events = Vec::new();
    run_to_go(&mut app, &mut events);
    let perfect = events
        .iter()
        .filter(|e| e.kind == RaceEventKind::PerfectStart)
        .count();
    let stalls = events
        .iter()
        .filter(|e| e.kind == RaceEventKind::Stall)
        .count();
    assert_eq!(stalls, 0, "the AI never holds the throttle too early");
    assert!(perfect >= 2, "Elite+ (60 %): {perfect} perfect starts of 8");
    assert!(perfect < 8, "not every start is perfect");
}

#[test]
fn items_are_used_and_the_race_still_finishes() {
    let mut app = race_app();
    let racers = ai_field(&app, 8, Difficulty::Pro, CarClass::Pro);
    let mut cfg = config("test_oval", racers, 3, 4);
    cfg.items = true;
    let run = run_race(&mut app, cfg, 200.0);
    run.print("test_oval items");
    let used: u32 = run.items_used.iter().map(|(_, n)| n).sum();
    assert!(used >= 8, "items used: {:?}", run.items_used);
    assert!(run.hits.0 > 0, "something hit");
    assert!(run.racers.iter().all(|r| r.finished), "everyone finishes");
}

#[test]
fn autopilot_drives_a_local_player_to_the_finish() {
    let mut app = race_app();
    let mut racers = vec![human(true)];
    racers.extend(ai_field(&app, 3, Difficulty::Pro, CarClass::Pro));
    let run = run_race(&mut app, config("test_oval", racers, 1, 5), 60.0);
    let player = &run.racers[0];
    assert!(
        player.finished,
        "the autopilot finished the lap: {:?}",
        player.laps
    );
    assert!(player.total < 1.5 * run.reference_lap, "{}", player.total);
    assert_eq!(
        race_phase(&app),
        Some(RacePhase::Finished),
        "the human finished: race over"
    );
}

/// Drives the human at 0.7 throttle without steering for 1.5 s: the strongest steer the car got
/// and its final throttle.
fn assisted_steer(assist: bool) -> (f32, f32) {
    let mut app = race_app();
    let mut settings = Settings::default();
    settings.accessibility.steering_assist = assist;
    app.insert_resource(settings);
    let mut racers = vec![human(false)];
    racers.extend(ai_field(&app, 1, Difficulty::Pro, CarClass::Pro));
    load_race(&mut app, config("test_oval", racers, 1, 6)).expect("loads");
    run_to_go(&mut app, &mut Vec::new());
    let car = find(&mut app, 0);
    let mut steer = 0.0f32;
    let mut throttle = 0.0f32;
    for _ in 0..secs_to_ticks(1.5) {
        *app.world_mut().get_mut::<CarControl>(car).expect("racer") = CarControl {
            throttle: 0.7,
            ..default()
        };
        step(&mut app, 1);
        let c = *app.world().get::<CarControl>(car).expect("racer");
        throttle = c.throttle;
        if c.steer.abs() > steer.abs() {
            steer = c.steer;
        }
    }
    (steer, throttle)
}

#[test]
fn steering_assist_steers_toward_the_line_and_touches_nothing_else() {
    let (steer, throttle) = assisted_steer(true);
    // The grid slot is right of test_oval's racing line on the main straight (left edge).
    assert!(steer < -0.05, "the assist steers toward the line: {steer}");
    assert_eq!(throttle, 0.7, "only steer is changed");
    let (steer, _) = assisted_steer(false);
    assert_eq!(steer, 0.0, "assist off: the player's steer is kept");
}

#[test]
fn catch_up_slows_rivals_ahead_of_the_human_but_never_in_time_trial() {
    let catch_up_after = |mode: GameMode| {
        let mut app = race_app();
        let mut racers = vec![human(false)];
        racers.extend(ai_field(&app, 3, Difficulty::Pro, CarClass::Pro));
        let mut cfg = config("test_oval", racers, 3, 7);
        cfg.ai_catch_up = true;
        cfg.mode = mode;
        load_race(&mut app, cfg).expect("loads");
        run_to_go(&mut app, &mut Vec::new());
        // The human stays parked on the grid (nobody writes its control).
        step_seconds(&mut app, 14.0);
        let world = app.world_mut();
        let mut q = world.query_filtered::<&CatchUp, (With<AiDriver>, Without<LocalPlayer>)>();
        q.iter(world).map(|c| c.0).collect::<Vec<_>>()
    };
    let quick = catch_up_after(GameMode::QuickRace);
    assert_eq!(quick.len(), 3);
    assert!(
        quick.iter().all(|&c| (-0.06..0.0).contains(&c)),
        "rivals ahead slow down: {quick:?}"
    );
    assert!(
        quick.iter().any(|&c| c <= -0.05),
        "the far leader is near the cap: {quick:?}"
    );
    let tt = catch_up_after(GameMode::TimeTrial);
    assert!(tt.iter().all(|&c| c == 0.0), "never in Time Trial: {tt:?}");
}

#[test]
fn a_car_put_against_a_wall_gets_back_into_the_race() {
    let mut app = race_app();
    let racers = ai_field(&app, 1, Difficulty::Pro, CarClass::Pro);
    load_race(&mut app, config("test_oval", racers, 3, 8)).expect("loads");
    run_to_go(&mut app, &mut Vec::new());
    // Past the start line first (lap 1), so rc_race counts the progress after the teleport.
    step_seconds(&mut app, 3.0);
    let car = find(&mut app, 0);
    // Nose against the outer (left) wall of the main straight, which heads −Z.
    let pose = Transform::from_xyz(-6.0, 0.5, -15.0).looking_to(Vec3::NEG_X, Vec3::Y);
    app.world_mut().write_message(RespawnCar {
        car,
        transform: pose,
        kind: RespawnKind::ToTrack,
    });
    step(&mut app, 2);
    let start = app
        .world()
        .get::<RaceProgress>(car)
        .expect("racer")
        .total_distance;
    step_seconds(&mut app, 12.0);
    let now = app
        .world()
        .get::<RaceProgress>(car)
        .expect("racer")
        .total_distance;
    assert!(
        now - start > 60.0,
        "back to racing: {start:.1} → {now:.1} u"
    );
}
