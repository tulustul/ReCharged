//! Headless races on `test_oval` (rc_testkit). The 8-car race runs the full sim stack
//! (`SimPlugins`: track, vehicles, weather, race, items) with scripted pilots instead of rc_ai.
//! The other tests use a kinematic stand-in for rc_vehicle — every car glides along the
//! centreline at its own speed — so the rules are checked exactly, without physics or AI.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_race::RacePlugin;
use rc_testkit::*;
use rc_track::{Track, TrackPlugin};

/// Speed of racer `index` (u/s): the pole is fastest.
fn speed(index: usize) -> f32 {
    50.0 - 2.0 * index as f32
}

/// Where the stand-in drives a car: along the centreline from its grid slot, or pinned.
#[derive(Component)]
struct Script {
    s: f32,
    lateral: f32,
    speed: f32,
    pin: Option<Vec3>,
}

/// Every racer event, phase entered, respawn request and effect seen.
#[derive(Resource, Default)]
struct Log {
    events: Vec<RaceEvent>,
    phases: Vec<RacePhase>,
    respawns: Vec<RespawnCar>,
    effects: Vec<ApplyEffect>,
}

impl Log {
    fn count(&self, pred: impl Fn(&RaceEventKind) -> bool) -> usize {
        self.events.iter().filter(|e| pred(&e.kind)).count()
    }
}

fn logging(app: &mut App) {
    app.init_resource::<Log>().add_systems(Update, record);
}

fn stand_in(app: &mut App) {
    logging(app);
    app.add_systems(
        FixedUpdate,
        (attach_scripts, drive).chain().in_set(SimSet::Vehicle),
    );
}

fn pilots(app: &mut App) {
    logging(app);
    app.add_systems(
        FixedUpdate,
        (attach_pilots, pilot).chain().in_set(SimSet::Drivers),
    );
}

/// Share of the track's reference speed profile the pilots drive at.
const PACE: f32 = 0.8;

/// AI-free control of a physical car: pure pursuit along the centreline at its grid column's
/// lateral offset.
#[derive(Component)]
struct Pilot {
    lane: f32,
    s: Option<f32>,
}

fn attach_pilots(
    mut commands: Commands,
    track: Option<Res<Track>>,
    racers: Query<(Entity, &Transform), Added<Racer>>,
) {
    let Some(track) = track else { return };
    for (car, transform) in &racers {
        let lane = track.project(transform.translation, None).lateral;
        commands.entity(car).insert(Pilot { lane, s: None });
    }
}

/// Countdown throttle: the pole times a perfect start, the last car holds it all along (stall).
fn launch(index: usize, countdown_tick: u32) -> f32 {
    match index {
        0 if countdown_tick > COUNTDOWN_TICKS - 10 => 1.0,
        7 => 1.0,
        _ => 0.0,
    }
}

fn pilot(
    phase: Res<State<RacePhase>>,
    status: Res<RaceStatus>,
    track: Option<Res<Track>>,
    mut racers: Query<(
        &Racer,
        &Transform,
        &VehicleTelemetry,
        &mut Pilot,
        &mut CarControl,
    )>,
) {
    let Some(track) = track else { return };
    let phase = *phase.get();
    for (racer, transform, telemetry, mut pilot, mut control) in &mut racers {
        let s = track.project(transform.translation, pilot.s).s;
        pilot.s = Some(s);
        let v = telemetry.forward_speed;
        let ahead = s + 4.0 + 0.3 * v.max(0.0);
        let target = track.frame_at(ahead).point(pilot.lane, 0.0);
        let local = transform.rotation.inverse() * (target - transform.translation);
        let want = track.target_speed_at(ahead) * PACE;
        let racing = phase == RacePhase::Racing;
        *control = CarControl {
            steer: (2.0 * local.x.atan2(-local.z)).clamp(-1.0, 1.0),
            throttle: match phase {
                RacePhase::Countdown => launch(racer.index, status.phase_ticks),
                _ if racing && v < want => 1.0,
                _ => 0.0,
            },
            brake: if racing && v > want + 2.0 { 0.6 } else { 0.0 },
            ..default()
        };
    }
}

fn attach_scripts(
    mut commands: Commands,
    track: Option<Res<Track>>,
    racers: Query<(Entity, &Racer, &Transform), Added<Racer>>,
) {
    let Some(track) = track else { return };
    for (car, racer, transform) in &racers {
        let tp = track.project(transform.translation, None);
        commands.entity(car).insert(Script {
            s: tp.s,
            lateral: tp.lateral,
            speed: speed(racer.index),
            pin: None,
        });
    }
}

fn drive(
    phase: Res<State<RacePhase>>,
    track: Option<Res<Track>>,
    mut racers: Query<(&mut Script, &mut Transform, &mut VehicleTelemetry)>,
) {
    let Some(track) = track else { return };
    let racing = *phase.get() == RacePhase::Racing;
    for (mut script, mut transform, mut telemetry) in &mut racers {
        if racing {
            script.s = track.wrap(script.s + script.speed * SIM_DT);
        }
        let frame = track.frame_at(script.s);
        *transform = match script.pin {
            Some(at) => Transform::from_translation(at),
            None => Transform::from_translation(frame.point(script.lateral, 0.15))
                .with_rotation(frame.rotation()),
        };
        *telemetry = VehicleTelemetry {
            grounded_wheels: 4,
            speed: if racing { script.speed } else { 0.0 },
            up_dot: 1.0,
            ..default()
        };
    }
}

fn record(
    mut events: MessageReader<RaceEvent>,
    mut phases: MessageReader<StateTransitionEvent<RacePhase>>,
    mut respawns: MessageReader<RespawnCar>,
    mut effects: MessageReader<ApplyEffect>,
    mut log: ResMut<Log>,
) {
    log.events.extend(events.read().copied());
    log.phases.extend(phases.read().filter_map(|t| t.entered));
    log.respawns.extend(respawns.read().copied());
    log.effects.extend(effects.read().copied());
}

fn app() -> App {
    headless_app((TrackPlugin, RacePlugin, stand_in))
}

/// Eight racers on test_oval; the listed grid indices are local players (slot 0, 1, …).
fn config(humans: &[usize], laps: u8) -> RaceConfig {
    let racers = (0..MAX_RACERS)
        .map(|i| RacerSetup {
            name: format!("R{i}"),
            car: CarId::new("zip"),
            controller: match humans.iter().position(|&h| h == i) {
                Some(slot) => Controller::Human {
                    slot: PlayerSlot(slot as u8),
                    autopilot: false,
                },
                None => Controller::Ai {
                    rival: RivalId::new("dash"),
                    difficulty: Difficulty::Pro,
                },
            },
            ..default()
        })
        .collect();
    RaceConfig {
        variant: TrackVariant::forward("test_oval"),
        laps,
        racers,
        seed: 1,
        ..default()
    }
}

fn racers(app: &mut App) -> Vec<(Entity, Racer, RaceProgress, Transform)> {
    let mut q = app
        .world_mut()
        .query::<(Entity, &Racer, &RaceProgress, &Transform)>();
    let mut out: Vec<_> = q
        .iter(app.world())
        .map(|(e, r, p, t)| (e, r.clone(), p.clone(), *t))
        .collect();
    out.sort_by_key(|r| r.1.index);
    out
}

fn status(app: &App) -> RaceStatus {
    app.world().resource::<RaceStatus>().clone()
}

fn wait_for(app: &mut App, phase: RacePhase) {
    step_until(app, &format!("{phase:?}"), |w| {
        w.get_resource::<State<RacePhase>>()
            .is_some_and(|s| *s.get() == phase)
    });
}

/// Steps until the phase is `phase`, for up to `secs` of simulated time.
fn run_until(app: &mut App, phase: RacePhase, secs: f32) {
    for _ in 0..secs_to_ticks(secs) {
        if race_phase(app) == Some(phase) {
            return;
        }
        app.update();
    }
    assert_eq!(race_phase(app), Some(phase), "after {secs} s");
}

#[test]
fn eight_car_race_runs_to_the_finish() {
    let sim = SimPlugins.build().disable::<rc_ai::AiPlugin>();
    let mut app = headless_app((sim, pilots));
    load_race(&mut app, config(&[], 2)).expect("loads");
    wait_for(&mut app, RacePhase::Countdown);
    assert_eq!(status(&app).countdown, 3);
    let grid = racers(&mut app);
    assert_eq!(grid.len(), MAX_RACERS);
    assert!(
        grid.iter()
            .all(|r| r.2.total_distance <= 0.0 && r.2.lap == 0)
    );
    let standings = app.world().resource::<Standings>().0.clone();
    let by_grid: Vec<Entity> = grid.iter().map(|r| r.0).collect();
    assert_eq!(standings, by_grid, "the pole leads on the grid");

    run_until(&mut app, RacePhase::Racing, 4.0);
    step(&mut app, 1);
    assert_eq!(status(&app).countdown, 0, "GO: no number left on screen");
    run_until(&mut app, RacePhase::Finished, 120.0);
    let log = app.world().resource::<Log>();
    {
        use RacePhase::*;
        assert_eq!(log.phases, [Setup, Intro, Countdown, Racing, Finished]);
    }
    use RaceEventKind::*;
    let starts: Vec<RaceEvent> = log
        .events
        .iter()
        .filter(|e| matches!(e.kind, CountdownTick(_) | Go | PerfectStart | Stall))
        .copied()
        .collect();
    let (pole, last) = (Some(grid[0].0), Some(grid[7].0));
    let global = |kind| RaceEvent { racer: None, kind };
    assert_eq!(
        starts,
        [
            global(CountdownTick(3)),
            global(CountdownTick(2)),
            global(CountdownTick(1)),
            global(Go),
            RaceEvent {
                racer: pole,
                kind: PerfectStart
            },
            RaceEvent {
                racer: last,
                kind: Stall
            },
        ]
    );
    assert_eq!(
        log.count(|k| matches!(k, LapCompleted { .. })),
        2 * MAX_RACERS
    );
    assert_eq!(log.count(|k| *k == FinalLap), MAX_RACERS);
    let ranks: Vec<u8> = log
        .events
        .iter()
        .filter_map(|e| match e.kind {
            Finished { rank, .. } => Some(rank),
            _ => None,
        })
        .collect();
    assert_eq!(ranks, [1, 2, 3, 4, 5, 6, 7, 8]);

    let results = app.world().resource::<RaceResults>().clone();
    let hash = app.world().resource::<TrackSummary>().geometry_hash;
    assert_eq!(results.geometry_hash, hash);
    assert!(results.ghost.is_none(), "ghosts only in Time Trial");
    let final_racers = racers(&mut app);
    for (i, entry) in results.entries.iter().enumerate() {
        assert_eq!(entry.rank, i as u8 + 1);
        assert!(entry.finished && entry.slot.is_none());
        let progress = &final_racers[entry.index].2;
        assert_eq!(progress.lap_times.len(), 2);
        assert_eq!(entry.time_ticks, progress.lap_times.iter().sum::<u32>());
        assert_eq!(entry.best_lap_ticks, progress.best_lap);
    }
    assert!(
        results
            .entries
            .windows(2)
            .all(|w| w[0].time_ticks <= w[1].time_ticks)
    );
    let clock = status(&app).race_ticks;
    step(&mut app, 30);
    assert_eq!(status(&app).race_ticks, clock, "the race clock stopped");
}

#[test]
fn restart_mid_race_recreates_the_grid() {
    let mut app = app();
    load_race(&mut app, config(&[0], 3)).expect("loads");
    run_until(&mut app, RacePhase::Racing, 10.0);
    step_seconds(&mut app, 8.0);
    let before = racers(&mut app);
    assert!(before.iter().all(|r| r.2.lap >= 1), "everyone is racing");

    app.world_mut().resource_mut::<Log>().phases.clear();
    request(&mut app, RacePhase::Setup);
    request(&mut app, PauseState::Running);
    wait_for(&mut app, RacePhase::Countdown);
    let after = racers(&mut app);
    let grid = app.world().resource::<Track>().grid().to_vec();
    assert_eq!(after.len(), MAX_RACERS);
    for (i, (car, _, progress, transform)) in after.iter().enumerate() {
        assert!(before.iter().all(|b| b.0 != *car), "new entities");
        assert!(transform.translation.distance(grid[i].translation) < 0.01);
        assert!(progress.lap == 0 && progress.total_distance <= 0.0);
    }
    assert_eq!(status(&app).attempt, 2);
    assert_eq!(status(&app).race_ticks, 0);
    run_until(&mut app, RacePhase::Racing, 5.0);
    use RacePhase::*;
    let log = app.world().resource::<Log>();
    assert_eq!(
        log.phases,
        [Setup, Countdown, Racing],
        "no intro on restarts"
    );
}

#[test]
fn leaving_during_setup_leaks_nothing() {
    let mut app = app();
    let baseline = count_entities::<()>(&mut app);
    load_race(&mut app, config(&[0], 3)).expect("loads");
    assert_eq!(race_phase(&app), Some(RacePhase::Setup));
    assert_eq!(count_entities::<With<Racer>>(&mut app), 0);
    quit_to_menu(&mut app);
    assert_eq!(app_state(&app), AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
    let world = app.world();
    assert!(world.get_resource::<RaceStatus>().is_none());
    assert!(world.get_resource::<Standings>().is_none());
    assert!(world.get_resource::<RaceResults>().is_none());

    // And after a whole race.
    start_race(&mut app, config(&[0], 1)).expect("loads");
    step_seconds(&mut app, 2.0);
    quit_to_menu(&mut app);
    assert_eq!(count_entities::<()>(&mut app), baseline);
    assert!(app.world().get_resource::<RaceStatus>().is_none());
}

#[test]
fn pause_freezes_race_time() {
    let mut app = app();
    start_race(&mut app, config(&[0], 3)).expect("loads");
    step_seconds(&mut app, 2.0);
    let ticks = status(&app).race_ticks;
    assert!(ticks > 100, "{ticks}");
    pause(&mut app);
    let frozen = (status(&app), racers(&mut app));
    step(&mut app, 120);
    assert_eq!(
        (status(&app), racers(&mut app)),
        frozen,
        "nothing moves while paused"
    );
    resume(&mut app);
    step(&mut app, 10);
    assert!(status(&app).race_ticks > frozen.0.race_ticks);
}

#[test]
fn respawns_battery_and_dropped_players() {
    let mut app = app();
    let mut race = config(&[0, 1], 3);
    race.golden_battery = true;
    start_race(&mut app, race).expect("loads");
    step(&mut app, 5);
    let cars = racers(&mut app);
    let (p1, p2, ai) = (cars[0].0, cars[1].0, cars[2].0);

    // P1 drops below the kill height: a respawn to the track with ghosting, then a 2 s cooldown.
    let below = Vec3::new(0.0, -30.0, 0.0);
    app.world_mut().get_mut::<Script>(p1).expect("script").pin = Some(below);
    step(&mut app, 2);
    let log = app.world().resource::<Log>();
    assert_eq!(log.respawns.len(), 1);
    let respawn = log.respawns[0];
    assert_eq!((respawn.car, respawn.kind), (p1, RespawnKind::ToTrack));
    let validated = app.world().get::<RaceProgress>(p1).expect("progress");
    let validated = validated.last_valid_checkpoint;
    let track = app.world().resource::<Track>();
    let on_track = track.project(respawn.transform.translation, None);
    assert!(on_track.lateral.abs() < 0.1, "on the centreline");
    assert_eq!(
        track.checkpoint_of(on_track.s),
        validated,
        "never past the next gate"
    );
    assert!(
        log.effects
            .iter()
            .any(|e| e.car == p1 && e.effect == Effect::Ghost { secs: 1.5 })
    );
    step(&mut app, 100);
    assert_eq!(app.world().resource::<Log>().respawns.len(), 1, "cooldown");
    step(&mut app, 30);
    assert_eq!(
        app.world().resource::<Log>().respawns.len(),
        2,
        "still below"
    );

    // P2 drives through the Golden Battery.
    let battery = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Transform, With<GoldenBattery>>();
        q.single(app.world()).expect("one battery").translation
    };
    app.world_mut().get_mut::<Script>(p2).expect("script").pin = Some(battery);
    step(&mut app, 2);
    let log = app.world().resource::<Log>();
    assert!(
        log.events
            .iter()
            .any(|e| e.racer == Some(p2) && e.kind == RaceEventKind::GoldenBattery)
    );
    assert_eq!(
        count_entities::<With<GoldenBattery>>(&mut app),
        0,
        "collected"
    );

    // P2's controller is dropped: the AI takes over that car only.
    app.world_mut().write_message(PlayerDropped {
        slot: PlayerSlot(1),
    });
    step(&mut app, 1);
    assert!(app.world().get::<AiDriver>(p2).is_some());
    assert!(app.world().get::<AiDriver>(p1).is_none());
    assert!(app.world().get::<AiDriver>(ai).is_some());
}

#[test]
fn time_trial_records_and_races_a_ghost() {
    let mut app = app();
    let mut tt = config(&[0], 1);
    tt.mode = GameMode::TimeTrial;
    tt.racers.truncate(1);
    start_race(&mut app, tt.clone()).expect("loads");
    run_until(&mut app, RacePhase::Finished, 30.0);
    let results = app.world().resource::<RaceResults>().clone();
    let ghost = results.ghost.clone().expect("a ghost");
    assert_eq!(ghost.time_ticks, results.entries[0].time_ticks);
    assert_eq!(ghost.geometry_hash, results.geometry_hash);
    let expected = ghost.time_ticks / (SIM_HZ as u32 / GHOST_HZ) + 1;
    assert!((ghost.samples.len() as i64 - i64::from(expected)).abs() <= 1);

    // Retry against it at the same speed: the ghost car keeps pace and the splits are ~0.
    app.world_mut().resource_mut::<RaceConfig>().ghost = Some(ghost.clone());
    app.world_mut().resource_mut::<Log>().events.clear();
    restart(&mut app);
    step_seconds(&mut app, 3.0);
    let ghost_pos = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Transform, With<GhostCar>>();
        q.single(app.world()).expect("a ghost car").translation
    };
    let car_pos = racers(&mut app)[0].3.translation;
    assert!(
        ghost_pos.distance(car_pos) < 0.5,
        "{ghost_pos} vs {car_pos}"
    );
    let splits: Vec<f32> = app
        .world()
        .resource::<Log>()
        .events
        .iter()
        .filter_map(|e| match e.kind {
            RaceEventKind::CheckpointSplit(d) => Some(d),
            _ => None,
        })
        .collect();
    assert!(!splits.is_empty());
    assert!(splits.iter().all(|d| d.abs() < 0.05), "{splits:?}");
}

#[test]
fn soft_lock_guard_classifies_a_human_who_never_finishes() {
    let mut app = app();
    start_race(&mut app, config(&[3], 1)).expect("loads");
    step(&mut app, 2);
    let human = racers(&mut app)[3].0;
    app.world_mut()
        .get_mut::<Script>(human)
        .expect("script")
        .speed = 0.0;
    run_until(&mut app, RacePhase::Finished, 90.0);

    let log = app.world().resource::<Log>();
    let ends_in: Vec<u8> = log
        .events
        .iter()
        .filter_map(|e| match e.kind {
            RaceEventKind::RaceEndsIn(n) => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(ends_in, [10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    let results = app.world().resource::<RaceResults>().clone();
    let winner = results.entries[0].time_ticks;
    let clock = status(&app).race_ticks;
    assert_eq!(clock, winner + 3600, "60 s after the winner");
    let last = results.entries.last().expect("entries");
    assert_eq!(
        (last.index, last.finished),
        (3, false),
        "classified last by distance"
    );
    assert!(last.time_ticks > clock, "projected beyond the end");
    assert!(results.entries[..7].iter().all(|e| e.finished));
    assert!(status(&app).ends_in.is_none());
}
