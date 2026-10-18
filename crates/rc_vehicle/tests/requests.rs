//! Requests other crates make of the vehicle (respawns, impulses), car–car contact and ghosting,
//! impact reports, and the pause freeze of rapier.

mod common;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use common::*;
use rc_core::prelude::*;
use rc_testkit::*;

#[test]
fn respawn_teleports_with_zero_velocity_and_reports() {
    let mut app = app();
    let car = spawn_car(&mut app, "sparky", Transform::from_xyz(0.0, 0.02, 0.0));
    control(&mut app, car).throttle = 1.0;
    step_seconds(&mut app, 1.5);
    let target = Transform::from_xyz(30.0, 0.5, 10.0).with_rotation(Quat::from_rotation_y(1.0));
    app.world_mut().write_message(RespawnCar {
        car,
        transform: target,
        kind: RespawnKind::ToTrack,
    });
    control(&mut app, car).throttle = 0.0;
    let mut teleports = app
        .world()
        .resource::<Messages<Teleported>>()
        .get_cursor_current();
    let respawned = step_collect::<CarRespawned>(&mut app, 1.0 / 60.0);
    assert_eq!(
        respawned,
        vec![CarRespawned {
            car,
            kind: RespawnKind::ToTrack
        }]
    );
    assert!(read_messages(&app, &mut teleports).contains(&Teleported { entity: car }));
    let p = pose(&app, car);
    assert!(p.translation.distance(target.translation) < 0.05, "{p:?}");
    assert!(p.rotation.angle_between(target.rotation) < 0.01);
    assert!(velocity(&app, car).linear.length() < 0.5, "velocity zeroed");
    step_seconds(&mut app, 1.0);
    let t = telemetry(&app, car);
    assert!(
        t.grounded_wheels == 4 && t.speed < 0.2,
        "lands and rests: {t:?}"
    );
}

#[test]
fn impulses_scale_with_weight() {
    let mut app = app();
    let cars = lanes(&mut app, &["zip", "dumpster"]);
    let j = Vec3::new(0.0, 0.0, -REFERENCE_CAR_MASS * 10.0);
    for &car in &cars {
        app.world_mut().write_message(ApplyImpulse {
            car,
            linear: j,
            angular: Vec3::ZERO,
        });
    }
    step(&mut app, 1);
    let dv: Vec<f32> = cars
        .iter()
        .map(|&c| velocity(&app, c).linear.z.abs())
        .collect();
    let masses = [params(&app, "zip").mass, params(&app, "dumpster").mass];
    for (i, m) in masses.iter().enumerate() {
        let expected = 10.0 * (REFERENCE_CAR_MASS / m).powf(1.5);
        assert!(
            (dv[i] - expected).abs() < 0.15 * expected,
            "Δv {} vs {expected}",
            dv[i]
        );
    }
    assert!(dv[0] > 1.8 * dv[1], "the light car flies further: {dv:?}");
}

/// Two cars driving head-on at each other; returns (passed through each other, impacts).
fn head_on(ghosted: bool) -> (bool, Vec<CarImpact>) {
    let mut app = app();
    let a = spawn_car(&mut app, "hornet", Transform::from_xyz(0.0, 0.02, 0.0));
    let b = spawn_car(
        &mut app,
        "hornet",
        Transform::from_xyz(0.0, 0.02, -24.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
    );
    if ghosted {
        effects(&mut app, a).ghost = Some(1.5);
    }
    for car in [a, b] {
        control(&mut app, car).throttle = 1.0;
    }
    let impacts = step_collect::<CarImpact>(&mut app, 2.0);
    (
        pose(&app, a).translation.z < pose(&app, b).translation.z,
        impacts,
    )
}

#[test]
fn ghosted_cars_pass_through_cars_but_not_the_ground() {
    let (passed, impacts) = head_on(true);
    assert!(passed, "a ghosted car drives through the other");
    assert!(impacts.is_empty(), "no car–car contact: {impacts:?}");
    let (passed, impacts) = head_on(false);
    assert!(!passed, "solid cars collide");
    let strongest = impacts.iter().map(|i| i.strength).fold(0.0, f32::max);
    assert!(
        strongest > 0.5,
        "a head-on crash is a strong impact: {impacts:?}"
    );
    assert!(
        impacts.len() <= 6,
        "impacts are debounced: {}",
        impacts.len()
    );
    let i = impacts[0];
    assert!(i.other.is_some() && i.normal.y.abs() < 0.5, "{i:?}");
}

#[test]
fn wall_hits_bounce_and_glancing_hits_keep_speed() {
    let mut app = app();
    block(
        &mut app,
        Vec3::new(0.0, 1.5, -40.0),
        Vec3::new(30.0, 1.5, 0.25),
        Quat::IDENTITY,
    );
    let head = spawn_car(&mut app, "phantom", Transform::from_xyz(-10.0, 0.02, 0.0));
    // A wall at 15° to the driving line for a glancing hit.
    block(
        &mut app,
        Vec3::new(60.0, 1.5, -30.0),
        Vec3::new(0.25, 1.5, 40.0),
        Quat::from_rotation_y(15f32.to_radians()),
    );
    let glance = spawn_car(&mut app, "phantom", Transform::from_xyz(50.0, 0.02, 20.0));
    for car in [head, glance] {
        control(&mut app, car).throttle = 1.0;
    }
    let (mut bounced, mut head_hit) = (false, None);
    let (mut before, mut after) = (None, f32::MAX);
    let mut glance_ticks_since_hit = None;
    for _ in 0..(5 * 60) {
        let speed = telemetry(&app, glance).speed;
        let impacts = step_collect::<CarImpact>(&mut app, 1.0 / 60.0);
        for i in &impacts {
            if i.car == head {
                head_hit = head_hit.or(Some(i.strength));
            }
            if i.car == glance && before.is_none() {
                before = Some(speed);
                glance_ticks_since_hit = Some(0);
            }
        }
        if head_hit.is_some() && velocity(&app, head).linear.z > 1.0 {
            bounced = true;
            control(&mut app, head).throttle = 0.0;
        }
        if let Some(n) = glance_ticks_since_hit.as_mut() {
            *n += 1;
            if *n <= 18 {
                after = after.min(telemetry(&app, glance).speed);
            }
        }
    }
    assert!(
        head_hit.is_some_and(|s| s > 0.4),
        "head-on impact strength {head_hit:?}"
    );
    assert!(bounced, "bouncy off walls, never sticky");
    let before = before.expect("the glancing car reached its wall");
    assert!(
        after > 0.7 * before,
        "a glancing hit keeps most speed: {before:.1} → {after:.1}"
    );
}

#[test]
fn pause_freezes_the_cars_and_quitting_while_paused_does_not_leak() {
    let mut app = app();
    let car = spawn_car(&mut app, "bolt", Transform::from_xyz(0.0, 4.0, 0.0));
    control(&mut app, car).throttle = 1.0;
    step_seconds(&mut app, 0.2);
    pause(&mut app);
    let frozen = (pose(&app, car), telemetry(&app, car));
    step_seconds(&mut app, 1.0);
    assert_eq!(
        (pose(&app, car), telemetry(&app, car)),
        frozen,
        "paused frames change nothing"
    );
    resume(&mut app);
    step_seconds(&mut app, 0.2);
    assert_ne!(pose(&app, car), frozen.0, "resumes");
    pause(&mut app);
    quit_to_menu(&mut app);
    start_race(&mut app, RaceConfig::default()).expect("next race");
    let active = app
        .world_mut()
        .query::<&RapierConfiguration>()
        .iter(app.world())
        .all(|c| c.physics_pipeline_active);
    assert!(
        active,
        "physics runs again after quitting from the pause menu"
    );
}
