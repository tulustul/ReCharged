//! Pause gates the simulation (ARCHITECTURE §3.1): 120 paused frames change nothing — no fixed
//! race ticks, no movement in `SimSet`s, no rapier steps — and resuming continues.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;
use rc_testkit::*;

/// Fixed ticks counted in `SimSet::Race` (stands in for `RaceStatus::race_ticks`).
#[derive(Resource, Default)]
struct RaceTicks(u32);

#[derive(Component)]
struct Mover;

fn dummy_sim(app: &mut App) {
    app.init_resource::<RaceTicks>()
        .add_systems(FixedUpdate, count_ticks.in_set(SimSet::Race))
        .add_systems(FixedUpdate, move_forward.in_set(SimSet::Vehicle))
        .add_systems(OnEnter(RaceAttempt), spawn_bodies);
}

fn count_ticks(mut ticks: ResMut<RaceTicks>) {
    ticks.0 += 1;
}

fn move_forward(mut q: Query<&mut Transform, With<Mover>>) {
    for mut t in &mut q {
        t.translation.z -= 0.1;
    }
}

fn spawn_bodies(mut commands: Commands) {
    commands.spawn((Mover, Interpolated, DespawnOnExit(RaceAttempt)));
    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(0.5),
        Transform::from_xyz(0.0, 50.0, 0.0),
        DespawnOnExit(RaceAttempt),
    ));
}

/// (race ticks, mover z, pose snapshot z, ball height)
fn sim_state(app: &mut App) -> (u32, f32, f32, f32) {
    let ticks = app.world().resource::<RaceTicks>().0;
    let mut mover = app
        .world_mut()
        .query_filtered::<(&Transform, &PoseHistory), With<Mover>>();
    let (t, pose) = mover.single(app.world()).expect("one mover");
    let (z, snapshot) = (t.translation.z, pose.curr.translation.z);
    let mut ball = app
        .world_mut()
        .query_filtered::<&Transform, With<RigidBody>>();
    let y = ball.single(app.world()).expect("one ball").translation.y;
    (ticks, z, snapshot, y)
}

#[test]
fn paused_frames_change_nothing_and_resume_continues() {
    let mut app = headless_app(dummy_sim);
    start_race(&mut app, RaceConfig::default()).expect("loads");
    step(&mut app, 10);
    let running = sim_state(&mut app);
    assert!(
        running.0 > 0 && running.1 < 0.0 && running.3 < 50.0,
        "simulation runs: {running:?}"
    );

    pause(&mut app);
    let frozen = sim_state(&mut app);
    step(&mut app, 120);
    assert_eq!(
        sim_state(&mut app),
        frozen,
        "120 paused frames changed the simulation"
    );

    resume(&mut app);
    step(&mut app, 5);
    let after = sim_state(&mut app);
    assert!(
        after.0 > frozen.0 && after.1 < frozen.1 && after.3 < frozen.3,
        "{after:?}"
    );
}

#[test]
fn simulation_sets_do_not_run_outside_a_race() {
    let mut app = headless_app(dummy_sim);
    step(&mut app, 30);
    assert_eq!(
        app.world().resource::<RaceTicks>().0,
        0,
        "menus never tick the simulation"
    );
}
