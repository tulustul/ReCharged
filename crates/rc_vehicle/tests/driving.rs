//! Driving-model behaviour of every car on flat ground (GDD §5.1): star targets, braking,
//! turning, drifts, recovery, landings, tunnelling, surfaces, effects, determinism.

mod common;

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use common::*;
use rc_core::prelude::*;
use rc_testkit::*;

fn tops(app: &App) -> Vec<f32> {
    CARS.iter().map(|c| params(app, c).top_speed).collect()
}

#[test]
fn zero_to_top_and_top_speed_match_the_stars() {
    let mut app = app();
    let cars = lanes(&mut app, &CARS);
    let tops = tops(&app);
    let results = straight(&mut app, &cars, &tops);
    let catalog = Catalog::load(&assets_dir()).expect("catalog");
    for (i, id) in CARS.iter().enumerate() {
        let stars = catalog.car(&CarId::new(*id)).expect("catalog car").stars;
        let target_t = zero_to_top_secs_for_stars(stars.accel);
        let target_v = top_speed_for_stars(stars.speed);
        let r = results[i];
        let t = r
            .zero_to_top
            .unwrap_or_else(|| panic!("{id} never reached top speed: {r:?}"));
        assert!(
            (t / target_t - 1.0).abs() <= 0.05,
            "{id}: 0→top {t:.2} s, target {target_t}"
        );
        assert!(
            (r.top / target_v - 1.0).abs() <= 0.05,
            "{id}: top {:.1}, target {target_v}",
            r.top
        );
        let d = r.braking.unwrap_or_else(|| panic!("{id} never stopped"));
        assert!(
            (6.0..=25.0).contains(&d),
            "{id}: braking distance {d:.1} u from top speed"
        );
    }
}

#[test]
fn turning_is_tight_responsive_and_follows_handling() {
    let mut app = app();
    let cars = lanes(&mut app, &CARS);
    let half: Vec<f32> = tops(&app).iter().map(|t| 0.5 * t).collect();
    let turns = turn(&mut app, &cars, &half);
    let catalog = Catalog::load(&assets_dir()).expect("catalog");
    let mut by_handling = [(0.0f32, 0.0f32); 6];
    for (i, id) in CARS.iter().enumerate() {
        let t = turns[i];
        assert!(
            (4.0..=12.0).contains(&t.radius),
            "{id}: turning radius {:.1} u",
            t.radius
        );
        assert!(
            t.max_slip_deg < 6.0,
            "{id}: grip cornering slides {:.1}°",
            t.max_slip_deg
        );
        assert!(
            t.min_up_dot > 0.9,
            "{id}: rolls in a turn (up {:.2})",
            t.min_up_dot
        );
        let response = t.response.expect("yaw settles");
        assert!(response <= 0.25, "{id}: steering response {response:.2} s");
        let h = usize::from(catalog.car(&CarId::new(*id)).expect("car").stars.handling);
        by_handling[h] = (by_handling[h].0 + t.lateral, by_handling[h].1 + 1.0);
    }
    let mean = |h: usize| by_handling[h].0 / by_handling[h].1;
    assert!(
        mean(4) > mean(3) && mean(3) > mean(2),
        "grip by handling stars: {by_handling:?}"
    );
}

#[test]
fn drift_starts_holds_and_recovers() {
    let mut app = app();
    let cars = lanes(&mut app, &CARS);
    let fast: Vec<f32> = tops(&app).iter().map(|t| 0.7 * t).collect();
    let drifts = drift(&mut app, &cars, &fast);
    for (i, id) in CARS.iter().enumerate() {
        let d = drifts[i];
        let start = d.start.unwrap_or_else(|| panic!("{id}: no drift {d:?}"));
        assert!(start <= 0.4, "{id}: drift took {start:.2} s to start");
        assert!(
            d.entry_max < 45.0 && d.hold.1 < 45.0,
            "{id}: spins out {d:?}"
        );
        assert!(d.hold.0 > 10.0, "{id}: drift collapses {d:?}");
        assert_eq!(
            d.drifting_ticks, 72,
            "{id}: telemetry drifting while holding"
        );
        let recover = d
            .recover
            .unwrap_or_else(|| panic!("{id}: never recovers {d:?}"));
        assert!(
            recover <= 0.8 && d.worst_after < 45.0,
            "{id}: recovery {recover:.2} s {d:?}"
        );
        assert!(d.speed_kept > 0.6, "{id}: the drift kills the speed {d:?}");
    }
}

#[test]
fn kicker_jumps_fly_and_land_on_the_wheels() {
    let mut app = app();
    let jumps = kicker_jumps(&mut app, &CARS);
    for (id, j) in CARS.iter().zip(jumps) {
        let l = j
            .landing
            .unwrap_or_else(|| panic!("{id}: no long jump {j:?}"));
        assert!(
            (0.5..=1.2).contains(&l.air_time),
            "{id}: air time {:.2} s",
            l.air_time
        );
        assert!(j.final_up > 0.95, "{id}: did not land on its wheels {j:?}");
        assert!(
            j.speed_after > 0.7 * j.takeoff,
            "{id}: the landing costs too much {j:?}"
        );
    }
}

#[test]
fn flipped_cars_right_themselves() {
    let mut app = app();
    let poses = [
        Quat::from_rotation_z(PI),
        Quat::from_rotation_z(PI / 2.0),
        Quat::from_rotation_z(-PI / 2.0) * Quat::from_rotation_y(1.0),
    ];
    let mut cars = Vec::new();
    for (i, id) in CARS.iter().enumerate() {
        for (k, rot) in poses.iter().enumerate() {
            let at = Vec3::new(i as f32 * 12.0, 1.2, k as f32 * 12.0);
            cars.push((
                *id,
                spawn_car(
                    &mut app,
                    id,
                    Transform::from_translation(at).with_rotation(*rot),
                ),
            ));
        }
    }
    step_seconds(&mut app, 3.5);
    for (id, car) in cars {
        let t = telemetry(&app, car);
        assert!(
            t.up_dot > 0.95 && t.speed < 1.0,
            "{id}: still flipped (up {:.2})",
            t.up_dot
        );
    }
}

#[test]
fn a_3u_drop_settles_without_bouncing() {
    let mut app = app();
    let cars: Vec<Entity> = CARS
        .iter()
        .enumerate()
        .map(|(i, id)| spawn_car(&mut app, id, Transform::from_xyz(i as f32 * 12.0, 3.0, 0.0)))
        .collect();
    let mut heights = vec![Vec::new(); cars.len()];
    for _ in 0..(3 * 60) {
        step(&mut app, 1);
        for (h, &car) in heights.iter_mut().zip(&cars) {
            h.push(pose(&app, car).translation.y);
        }
    }
    for (id, h) in CARS.iter().zip(heights) {
        // Touchdown is at ≈ 0.49 s; from 1.2 s on the car rests within ±0.02 u.
        let late = &h[72..];
        let (lo, hi) = late
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), y| (a.min(*y), b.max(*y)));
        assert!(
            hi - lo < 0.02 && lo.abs() < 0.03,
            "{id}: still bouncing {lo:.3}..{hi:.3}"
        );
    }
}

/// Rapier's velocity-based speculative contacts already stop these cases; CCD on the chassis is
/// the backstop for thinner geometry. This guards the requirement at the speed cap.
#[test]
fn no_tunnelling_through_a_thin_wall_at_the_speed_cap() {
    let mut app = app();
    let wall_z = -30.0;
    block(
        &mut app,
        Vec3::new(0.0, 1.5, wall_z),
        Vec3::new(40.0, 1.5, 0.25),
        Quat::IDENTITY,
    );
    for (i, id) in CARS.iter().enumerate() {
        let x = -28.0 + i as f32 * 8.0;
        let car = spawn_car(&mut app, id, Transform::from_xyz(x, 0.02, 0.0));
        let cap = params(&app, id).top_speed * TOP_SPEED_CAP;
        app.world_mut()
            .get_mut::<Velocity>(car)
            .expect("body")
            .linear = Vec3::NEG_Z * cap;
        control(&mut app, car).throttle = 1.0;
    }
    // Worst case: a flat body (upside down, so no wheel ray sees the slab) falling at the cap
    // onto a 0.5 u slab moves more than its own height plus the slab per tick.
    let slab_top = 20.0;
    block(
        &mut app,
        Vec3::new(0.0, slab_top - 0.25, 60.0),
        Vec3::new(40.0, 0.25, 20.0),
        Quat::IDENTITY,
    );
    let roof = Quat::from_rotation_z(PI);
    for (i, id) in CARS.iter().enumerate() {
        let at = Vec3::new(-28.0 + i as f32 * 8.0, slab_top + 3.0, 60.0);
        let car = spawn_car(
            &mut app,
            id,
            Transform::from_translation(at).with_rotation(roof),
        );
        let cap = params(&app, id).top_speed * TOP_SPEED_CAP;
        app.world_mut()
            .get_mut::<Velocity>(car)
            .expect("body")
            .linear = Vec3::NEG_Y * cap;
    }
    step_seconds(&mut app, 1.5);
    let mut cars = app.world_mut().query::<(&Racer, &Transform)>();
    for (racer, t) in cars.iter(app.world()) {
        let p = t.translation;
        if p.z < 30.0 {
            assert!(p.z > wall_z, "{} went through the wall: {p}", racer.car);
        } else {
            assert!(
                p.y > slab_top - 0.1,
                "{} fell through the slab: {p}",
                racer.car
            );
        }
    }
}

/// Lateral acceleration of `car` in a full-lock circle at 12 u/s on `surface` (own patch).
fn circle_on(surface: SurfaceKind, car: &str) -> f32 {
    let mut app = headless_app(rc_vehicle::VehiclePlugin);
    start_race(&mut app, RaceConfig::default()).expect("race loads");
    ground(&mut app, surface, Vec3::ZERO, Vec2::splat(300.0));
    let cars = lanes(&mut app, &[car]);
    turn(&mut app, &cars, &[12.0])[0].lateral
}

#[test]
fn surface_grip_orders_ice_tile_asphalt() {
    for car in ["hornet", "torque"] {
        let ice = circle_on(SurfaceKind::Ice, car);
        let tile = circle_on(SurfaceKind::Tile, car);
        let asphalt = circle_on(SurfaceKind::Asphalt, car);
        assert!(
            ice < tile && tile < asphalt,
            "{car}: ice {ice:.1} tile {tile:.1} asphalt {asphalt:.1}"
        );
    }
}

#[test]
fn effects_change_power_and_grip_and_the_cap_holds() {
    let mut app = app();
    ground(
        &mut app,
        SurfaceKind::ToyTrack,
        Vec3::new(0.0, 0.0, -1000.0),
        Vec2::new(100.0, 900.0),
    );
    let ids = ["hornet", "hornet", "hornet", "bolt"];
    let cars = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            spawn_car(
                &mut app,
                id,
                Transform::from_xyz(i as f32 * 20.0 - 30.0, 0.02, -150.0),
            )
        })
        .collect::<Vec<_>>();
    step_seconds(&mut app, 0.5);
    let timed = |mul| {
        Some(TimedMul {
            mul,
            remaining: 99.0,
        })
    };
    effects(&mut app, cars[0]).power_loss = timed(0.0);
    effects(&mut app, cars[1]).power_loss = timed(0.5);
    effects(&mut app, cars[3]).boost = Some(TimedBoost {
        top_speed: 1.45,
        force: 1.8,
        freeze_charge: true,
        remaining: 99.0,
    });
    effects(&mut app, cars[3]).top_speed = timed(1.08);
    app.world_mut().entity_mut(cars[3]).insert(CatchUp(0.06));
    for &car in &cars {
        control(&mut app, car).throttle = 1.0;
    }
    let mut bolt_max = 0.0f32;
    for tick in 0..(8 * 60) {
        step(&mut app, 1);
        bolt_max = bolt_max.max(telemetry(&app, cars[3]).speed);
        if tick == 60 {
            let speeds: Vec<f32> = cars.iter().map(|&c| telemetry(&app, c).speed).collect();
            assert!(speeds[0] < 0.3, "PowerLoss(0) still drives: {speeds:?}");
            assert!(
                speeds[1] < 0.75 * speeds[2] && speeds[1] > 0.3 * speeds[2],
                "{speeds:?}"
            );
        }
    }
    let cap = params(&app, "bolt").top_speed * TOP_SPEED_CAP;
    assert!(
        bolt_max <= cap + 1e-3,
        "speed {bolt_max:.2} above the cap {cap}"
    );
    assert!(
        bolt_max > 0.95 * cap,
        "stacked multipliers reach the cap: {bolt_max:.2}"
    );
    assert!(telemetry(&app, cars[3]).boosting);

    // Grip effect: the same circle with Grip(0.2) holds far less lateral acceleration.
    let mut grip_app = common::app();
    let cars = lanes(&mut grip_app, &["hornet", "hornet"]);
    effects(&mut grip_app, cars[1]).grip = timed(0.2);
    let t = turn(&mut grip_app, &cars, &[12.0, 12.0]);
    assert!(t[1].lateral < 0.5 * t[0].lateral, "Grip(0.2): {t:?}");
}

/// The pose after 10 s of a scripted drive (throttle, steering, drift, boost, reverse).
fn scripted_run() -> Vec<(Vec3, Quat)> {
    let mut app = app();
    block(
        &mut app,
        Vec3::new(0.0, 0.5, -60.0),
        Vec3::new(30.0, 0.5, 0.25),
        Quat::IDENTITY,
    );
    let cars: Vec<Entity> = ["zip", "dumpster", "phantom"]
        .iter()
        .enumerate()
        .map(|(i, id)| spawn_car(&mut app, id, Transform::from_xyz(i as f32 * 3.0, 0.02, 0.0)))
        .collect();
    for tick in 0..(10 * 60) {
        for (i, &car) in cars.iter().enumerate() {
            let t = tick as f32 / 60.0 + i as f32;
            let mut c = control(&mut app, car);
            c.throttle = if (t % 4.0) < 3.0 { 1.0 } else { 0.0 };
            c.brake = if (t % 4.0) >= 3.5 { 1.0 } else { 0.0 };
            c.steer = (t * 1.3).sin();
            c.drift = (t % 3.0) > 2.2;
            c.boost = (t % 5.0) > 4.0;
        }
        step(&mut app, 1);
    }
    cars.iter()
        .map(|&c| {
            let p = pose(&app, c);
            (p.translation, p.rotation)
        })
        .collect()
}

#[test]
fn same_inputs_give_identical_poses() {
    let a = scripted_run();
    let b = scripted_run();
    assert_eq!(a, b, "two runs of the same inputs diverged");
}

#[test]
fn cars_are_held_during_the_countdown() {
    let mut app = headless_app(rc_vehicle::VehiclePlugin);
    load_race(&mut app, RaceConfig::default()).expect("race loads");
    set_phase(&mut app, RacePhase::Countdown);
    ground(
        &mut app,
        SurfaceKind::Asphalt,
        Vec3::ZERO,
        Vec2::splat(50.0),
    );
    let car = spawn_car(&mut app, "bolt", Transform::from_xyz(0.0, 0.02, 0.0));
    step_seconds(&mut app, 0.5);
    let start = pose(&app, car).translation;
    {
        let mut c = control(&mut app, car);
        c.throttle = 1.0;
        c.boost = true;
    }
    step_seconds(&mut app, 1.0);
    let t = telemetry(&app, car);
    assert!(
        pose(&app, car).translation.distance(start) < 0.05,
        "moved while held"
    );
    assert!(t.rpm > 0.9 && !t.boosting, "revs but no boost: {t:?}");
    set_phase(&mut app, RacePhase::Racing);
    step_seconds(&mut app, 1.0);
    assert!(telemetry(&app, car).forward_speed > 5.0, "drives after GO");
}

#[test]
fn brake_from_standstill_reverses_to_the_reverse_speed() {
    let mut app = app();
    let cars = lanes(&mut app, &["torque"]);
    control(&mut app, cars[0]).brake = 1.0;
    step_seconds(&mut app, 4.0);
    let v = telemetry(&app, cars[0]).forward_speed;
    let target = params(&app, "torque").reverse_speed;
    assert!(
        v < 0.0 && (v.abs() / target - 1.0).abs() < 0.1,
        "reverse {v:.1}, target {target}"
    );
}

#[test]
fn wet_and_snowy_outdoor_ground_loses_grip() {
    let lateral = |wetness: f32, snow_cover: f32| {
        let mut app = app();
        app.world_mut().insert_resource(WeatherState {
            wetness,
            snow_cover,
            ..default()
        });
        let cars = lanes(&mut app, &["hornet"]);
        turn(&mut app, &cars, &[14.0])[0].lateral
    };
    let (dry, wet, snow) = (lateral(0.0, 0.0), lateral(1.0, 0.0), lateral(0.0, 1.0));
    assert!(
        wet < 0.95 * dry && snow < wet,
        "dry {dry:.1} wet {wet:.1} snow {snow:.1}"
    );
}

#[test]
fn wheel_telemetry_describes_the_suspension() {
    let mut app = app();
    let cars = lanes(&mut app, &["dumpster"]);
    let t = telemetry(&app, cars[0]);
    for w in t.wheels {
        assert!(w.grounded && w.offset.abs() < 0.01, "rest pose {w:?}");
        assert!((0.2..0.8).contains(&w.compression) && w.surface == Some(SurfaceKind::Asphalt));
    }
    app.world_mut()
        .get_mut::<Transform>(cars[0])
        .expect("car")
        .translation
        .y += 5.0;
    step(&mut app, 3);
    let t = telemetry(&app, cars[0]);
    let droop = params(&app, "dumpster").suspension.droop;
    for w in t.wheels {
        assert!(
            !w.grounded && w.compression == 0.0 && (w.offset + droop).abs() < 1e-4,
            "{w:?}"
        );
    }
    control(&mut app, cars[0]).steer = 1.0;
    step(&mut app, 2);
    let t = telemetry(&app, cars[0]);
    assert!(
        t.wheels[0].steer > 0.3 && t.wheels[2].steer == 0.0,
        "front wheels steer right"
    );
}

/// A gently bumpy trimesh floor like rc_track's ribbons (top near y = 0, `FIX_INTERNAL_EDGES`).
fn bumpy_trimesh(app: &mut App, size: f32, amplitude: f32) {
    let n = 80u32;
    let mut vertices = Vec::new();
    for z in 0..=n {
        for x in 0..=n {
            let (fx, fz) = (
                x as f32 / n as f32 * size - size / 2.0,
                z as f32 / n as f32 * size - size / 2.0,
            );
            vertices.push(Vec3::new(
                fx,
                amplitude * (fx * 0.35).sin() * (fz * 0.45).cos(),
                fz,
            ));
        }
    }
    let mut indices = Vec::new();
    for z in 0..n {
        for x in 0..n {
            let (i, row) = (z * (n + 1) + x, n + 1);
            indices.push([i, i + row, i + 1]);
            indices.push([i + 1, i + row, i + row + 1]);
        }
    }
    let collider =
        Collider::trimesh_with_flags(vertices, indices, TriMeshFlags::FIX_INTERNAL_EDGES)
            .expect("valid trimesh");
    app.world_mut().spawn((
        collider,
        Transform::default(),
        Surface(SurfaceKind::Grass),
        TrackPiece {
            kind: TrackPieceKind::Ground,
        },
        CollisionGroups::new(Group::from_bits_truncate(layers::TRACK), Group::ALL),
    ));
}

#[test]
fn bumpy_trimesh_ground_is_driven_smoothly() {
    let mut app = headless_app(rc_vehicle::VehiclePlugin);
    start_race(&mut app, RaceConfig::default()).expect("race loads");
    bumpy_trimesh(&mut app, 400.0, 0.15);
    let ids = ["zip", "dumpster", "bolt"];
    let cars: Vec<Entity> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            spawn_car(
                &mut app,
                id,
                Transform::from_xyz(i as f32 * 8.0 - 8.0, 0.5, 150.0),
            )
        })
        .collect();
    step_seconds(&mut app, 0.5);
    for &car in &cars {
        control(&mut app, car).throttle = 1.0;
    }
    let mut min_up = [1.0f32; 3];
    let mut grounded = [0usize; 3];
    for _ in 0..(6 * 60) {
        step(&mut app, 1);
        for (i, &car) in cars.iter().enumerate() {
            let t = telemetry(&app, car);
            min_up[i] = min_up[i].min(t.up_dot);
            grounded[i] += usize::from(t.grounded_wheels >= 3);
        }
    }
    for (i, id) in ids.iter().enumerate() {
        let t = telemetry(&app, cars[i]);
        let top = params(&app, id).top_speed * SurfaceKind::Grass.props().top_speed;
        assert!(
            t.speed > 0.85 * top,
            "{id}: {:.1} of {top:.1} u/s on bumpy grass",
            t.speed
        );
        assert!(
            min_up[i] > 0.85,
            "{id}: bumps tip the car (up {:.2})",
            min_up[i]
        );
        assert!(
            grounded[i] > 300,
            "{id}: wheels on the ground {} of 360 ticks",
            grounded[i]
        );
        assert_eq!(t.surface, Some(SurfaceKind::Grass));
    }
}
