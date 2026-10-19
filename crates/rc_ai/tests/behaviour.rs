//! Headless behaviours of the AI on the full sim stack: starts, items, autopilot, steering
//! assist, catch-up, wall recovery, drift charge.

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
fn steering_assist_steers_toward_the_line_and_keeps_the_pedals_on_a_straight() {
    let (steer, throttle) = assisted_steer(true);
    // The grid slot is right of test_oval's racing line on the main straight (left edge).
    assert!(steer < -0.05, "the assist steers toward the line: {steer}");
    assert_eq!(
        throttle, 0.7,
        "under the speed plan the pedals are the player's"
    );
    let (steer, _) = assisted_steer(false);
    assert_eq!(steer, 0.0, "assist off: the player's steer is kept");
}

/// Full throttle for 6 s on test_oval (into turn 1), no steering or (`steering`) small corrections
/// every other tick: the lowest throttle and the strongest brake the car got.
fn assisted_pedals(assist: bool, steering: bool) -> (f32, f32) {
    let mut app = race_app();
    let mut settings = Settings::default();
    settings.accessibility.steering_assist = assist;
    app.insert_resource(settings);
    load_race(&mut app, config("test_oval", vec![human(false)], 1, 6)).expect("loads");
    run_to_go(&mut app, &mut Vec::new());
    let car = find(&mut app, 0);
    let (mut throttle, mut brake) = (1.0f32, 0.0f32);
    for tick in 0..secs_to_ticks(6.0) {
        *app.world_mut().get_mut::<CarControl>(car).expect("racer") = CarControl {
            throttle: 1.0,
            steer: if steering && tick % 2 == 0 { 0.15 } else { 0.0 },
            ..default()
        };
        step(&mut app, 1);
        let c = *app.world().get::<CarControl>(car).expect("racer");
        (throttle, brake) = (throttle.min(c.throttle), brake.max(c.brake));
    }
    (throttle, brake)
}

#[test]
fn the_assist_brakes_gently_before_a_corner_taken_too_fast() {
    let (throttle, brake) = assisted_pedals(true, false);
    assert_eq!(
        throttle, 0.0,
        "throttle only: the throttle lifts before turn 1"
    );
    assert!(
        brake > 0.0 && brake <= rc_ai::MAX_AUTO_BRAKE,
        "a gentle auto-brake: {brake}"
    );
    assert_eq!(
        assisted_pedals(true, true),
        (1.0, 0.0),
        "a player who steers keeps the pedals"
    );
    assert_eq!(
        assisted_pedals(false, false),
        (1.0, 0.0),
        "assist off: the pedals are the player's"
    );
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

#[test]
fn rockets_are_fired_only_at_cars_in_sight() {
    let mut app = race_app();
    let mut racers = ai_field(&app, 1, Difficulty::ElitePlus, CarClass::Pro);
    racers.push(human(false)); // parked: nobody writes its control
    load_race(&mut app, config("test_oval", racers, 3, 9)).expect("loads");
    run_to_go(&mut app, &mut Vec::new());
    let (shooter, target) = (find(&mut app, 0), find(&mut app, 1));
    let place = |app: &mut App, car, at: Vec3| {
        app.world_mut().write_message(RespawnCar {
            car,
            transform: Transform::from_translation(at).looking_to(Vec3::NEG_Z, Vec3::Y),
            kind: RespawnKind::ToTrack,
        });
    };
    // The shooter on the main straight heading −Z; the target 30 u ahead and 9.5 u left — in
    // the rocket cone and range, but on the grass behind the outer wall.
    place(&mut app, shooter, Vec3::new(0.0, 0.5, 20.0));
    place(&mut app, target, Vec3::new(-9.5, 0.2, -10.0));
    step(&mut app, 2);
    let rocket = ItemSlot {
        item: Some(ItemKind::SparkRocket),
        roulette: 0.0,
    };
    *app.world_mut().get_mut::<ItemSlot>(shooter).expect("racer") = rocket;
    step_seconds(&mut app, 1.0);
    let held = |app: &App| app.world().get::<ItemSlot>(shooter).expect("racer").item;
    assert_eq!(
        held(&app),
        Some(ItemKind::SparkRocket),
        "no shot at a hidden car"
    );
    // The same car on the track, in plain view: the shot goes after the reaction delay.
    place(&mut app, shooter, Vec3::new(0.0, 0.5, 20.0));
    place(&mut app, target, Vec3::new(-3.0, 0.5, -10.0));
    step_seconds(&mut app, 1.0);
    assert_eq!(held(&app), None, "fired at a car in sight");
}

#[test]
fn a_car_that_missed_a_gate_resets_to_drive_through_it() {
    let mut app = race_app();
    let racers = ai_field(&app, 1, Difficulty::Pro, CarClass::Pro);
    load_race(&mut app, config("test_oval", racers, 3, 10)).expect("loads");
    run_to_go(&mut app, &mut Vec::new());
    step_seconds(&mut app, 2.0); // over the line: lap 1, gate 1 next
    let car = find(&mut app, 0);
    let (gate, gate_s) = {
        let progress = app.world().get::<RaceProgress>(car).expect("racer");
        let track = app.world().resource::<rc_testkit::rc_track::Track>();
        let next = progress.next_checkpoint;
        (next, track.checkpoints()[next].s)
    };
    // Put the car 15 u past that gate: it skipped it.
    let pose = {
        let track = app.world().resource::<rc_testkit::rc_track::Track>();
        let f = track.frame_at(gate_s + 15.0);
        Transform::from_translation(f.point(0.0, 0.5)).with_rotation(f.rotation())
    };
    app.world_mut().write_message(RespawnCar {
        car,
        transform: pose,
        kind: RespawnKind::ToTrack,
    });
    let mut respawns = MessageCursor::<CarRespawned>::default();
    step(&mut app, 2);
    respawns
        .read(app.world().resource::<Messages<CarRespawned>>())
        .count();
    let mut reset = false;
    for _ in 0..secs_to_ticks(6.0) {
        step(&mut app, 1);
        let n = respawns
            .read(app.world().resource::<Messages<CarRespawned>>())
            .filter(|m| m.car == car)
            .count();
        reset |= n > 0;
    }
    assert!(reset, "the AI held reset after missing the gate");
    let progress = app.world().get::<RaceProgress>(car).expect("racer");
    assert_ne!(
        progress.next_checkpoint, gate,
        "the gate counted on the second pass"
    );
}

/// A throttle-only player with the steering assist (QA's kid-friendly path; auto-accelerate
/// gives the same input): its lap times against the track's reference lap, respawns and "Hold
/// Reset" hints — `AI_TRACK=backyard_bash AI_LAPS=1 [AI_REV=1] [AI_MIRROR=1] [AI_CAR=phantom]
/// cargo test -p rc_ai --test behaviour -- --ignored --nocapture assist_probe`.
#[test]
#[ignore = "tuning probe"]
fn assist_probe() {
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_owned());
    let mut app = race_app();
    let mut settings = Settings::default();
    settings.accessibility.steering_assist = true;
    app.insert_resource(settings);
    let laps: u8 = env("AI_LAPS", "1").parse().expect("AI_LAPS");
    let mut cfg = config(
        &env("AI_TRACK", "backyard_bash"),
        vec![human(false)],
        laps,
        1,
    );
    cfg.variant.reverse = env("AI_REV", "0") == "1";
    cfg.variant.mirror = env("AI_MIRROR", "0") == "1";
    cfg.racers[0].car = CarId::new(env("AI_CAR", "hornet"));
    load_race(&mut app, cfg).expect("loads");
    run_to_go(&mut app, &mut Vec::new());
    let car = find(&mut app, 0);
    // run_to_go's ticks after GO feed the assist its own steer back as input: a throttle-only
    // player has not touched the stick (rc_input writes every tick in the game).
    if let Some(mut st) = app.world_mut().get_mut::<rc_ai::AssistState>(car) {
        st.hands_off = rc_ai::HANDS_OFF_SECS;
    }
    let mut respawns = MessageCursor::<CarRespawned>::default();
    let mut events = MessageCursor::<RaceEvent>::default();
    let mut impacts = MessageCursor::<CarImpact>::default();
    let (mut count, mut stuck_hints) = (0, 0);
    let trace = std::env::var("AI_TRACE").is_ok();
    for tick in 0..secs_to_ticks(240.0) {
        *app.world_mut().get_mut::<CarControl>(car).expect("racer") = CarControl {
            throttle: 1.0,
            ..default()
        };
        step(&mut app, 1);
        if trace
            && tick
                % std::env::var("AI_TRACE_EVERY")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(6)
                == 0
        {
            let w = app.world();
            let (v, t, c) = (
                w.get::<rc_ai::TrackView>(car).expect("racer"),
                w.get::<VehicleTelemetry>(car).expect("racer"),
                w.get::<CarControl>(car).expect("racer"),
            );
            println!(
                "t {:6.2} s {:6.1} lat {:5.2} h {:5.2} v {:5.1} gw {} up {:4.2} | th {:.2} br {:.2} st {:5.2}",
                tick as f32 / 60.0,
                v.s,
                v.lateral,
                v.height,
                t.forward_speed,
                t.grounded_wheels,
                t.up_dot,
                c.throttle,
                c.brake,
                c.steer
            );
        }
        if trace {
            for m in impacts.read(app.world().resource::<Messages<CarImpact>>()) {
                if m.car == car {
                    let prop = m
                        .other
                        .and_then(|e| app.world().get::<Prop>(e))
                        .map(|p| p.kind.to_string());
                    println!(
                        "   impact {:.2} at {:.2} normal {:.2} other {prop:?}",
                        m.strength, m.point, m.normal
                    );
                }
            }
        }
        let n = respawns
            .read(app.world().resource::<Messages<CarRespawned>>())
            .filter(|m| m.car == car)
            .count();
        if n > 0 {
            let view = app.world().get::<rc_ai::TrackView>(car).expect("racer");
            println!("respawn near s {:.1} (lateral {:.1})", view.s, view.lateral);
        }
        count += n;
        stuck_hints += events
            .read(app.world().resource::<Messages<RaceEvent>>())
            .filter(|e| e.kind == RaceEventKind::Hint(HintKind::HoldReset))
            .count();
        if app
            .world()
            .get::<RaceProgress>(car)
            .is_some_and(|p| p.finished)
        {
            break;
        }
    }
    let reference = reference_lap(app.world().resource::<rc_testkit::rc_track::Track>());
    let p = app.world().get::<RaceProgress>(car).expect("racer");
    let laps: Vec<String> = p
        .lap_times
        .iter()
        .map(|&t| {
            format!(
                "{:.2} s ({:.2}×)",
                ticks_to_secs(t),
                ticks_to_secs(t) / reference
            )
        })
        .collect();
    println!(
        "assisted throttle-only: laps {laps:?} (reference {reference:.2} s), finished {}, \
         respawns {count}, Hold Reset hints {stuck_hints}",
        p.finished
    );
}

/// An Elite AI alone on test_oval (no drafting; air time charges only off the ground) earns
/// charge from its drifts: they outlast rc_vehicle's 0.4 s before a drift charges (measured
/// 0.055 over 3 laps; 0 while the edge room was measured to the drift line's own bound).
#[test]
fn elite_drifts_charge_the_battery() {
    let mut app = race_app();
    let racers = ai_field(&app, 1, Difficulty::Elite, CarClass::Pro);
    let cfg = config("test_oval", racers, 3, 7);
    let mut charged = 0.0;
    run_race_traced(&mut app, cfg, 60.0, |app, _| {
        let world = app.world_mut();
        let mut q = world.query::<&VehicleTelemetry>();
        for t in q.iter(world).filter(|t| t.drifting) {
            charged += t.charge_rate * SIM_DT;
        }
    });
    assert!(charged > 0.02, "drift charge over 3 laps: {charged:.3}");
}
