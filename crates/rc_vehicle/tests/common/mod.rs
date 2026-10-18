//! Shared helpers of the rc_vehicle integration tests: a headless race with flat ground patches,
//! car spawning, control and measurement.
#![allow(dead_code)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;
use rc_testkit::*;
use rc_vehicle::{VehicleLibrary, VehicleParams};

/// Every car id of the roster.
pub const CARS: [&str; 8] = [
    "sparky", "dumpster", "zip", "torque", "hornet", "phantom", "volt_gt", "bolt",
];

/// A running race (Racing phase) with the vehicle plugin and a big asphalt floor (top at y = 0).
pub fn app() -> App {
    let mut app = headless_app(rc_vehicle::VehiclePlugin);
    start_race(&mut app, RaceConfig::default()).expect("race loads");
    ground(
        &mut app,
        SurfaceKind::Asphalt,
        Vec3::ZERO,
        Vec2::splat(2000.0),
    );
    app
}

/// A flat static ground box of `surface`, top face at `centre.y`, half size `half` (X, Z).
pub fn ground(app: &mut App, surface: SurfaceKind, centre: Vec3, half: Vec2) -> Entity {
    app.world_mut()
        .spawn((
            Collider::cuboid(half.x, 0.5, half.y),
            Transform::from_translation(centre - Vec3::Y * 0.5),
            Surface(surface),
            TrackPiece {
                kind: TrackPieceKind::Ground,
            },
            CollisionGroups::new(Group::from_bits_truncate(layers::TRACK), Group::ALL),
        ))
        .id()
}

/// A static box (wall, ramp block) with its centre, half extents and rotation.
pub fn block(app: &mut App, centre: Vec3, half: Vec3, rotation: Quat) -> Entity {
    app.world_mut()
        .spawn((
            Collider::cuboid(half.x, half.y, half.z),
            Transform::from_translation(centre).with_rotation(rotation),
            TrackPiece {
                kind: TrackPieceKind::Wall,
            },
            CollisionGroups::new(Group::from_bits_truncate(layers::TRACK), Group::ALL),
        ))
        .id()
}

/// Spawns a racer of `car` at `transform` (its body is built on the next tick).
pub fn spawn_car(app: &mut App, car: &str, transform: Transform) -> Entity {
    let index = app.world_mut().query::<&Racer>().iter(app.world()).count();
    let entity = app
        .world_mut()
        .spawn((
            Racer {
                index,
                name: car.into(),
                car: CarId::new(car),
                ..default()
            },
            transform,
        ))
        .id();
    step(app, 1);
    entity
}

/// The car's params from the loaded library.
pub fn params(app: &App, car: &str) -> VehicleParams {
    app.world()
        .resource::<VehicleLibrary>()
        .get(&CarId::new(car))
        .expect("car params")
        .clone()
}

pub fn control(app: &mut App, car: Entity) -> Mut<'_, CarControl> {
    app.world_mut().get_mut::<CarControl>(car).expect("racer")
}

pub fn telemetry(app: &App, car: Entity) -> VehicleTelemetry {
    *app.world().get::<VehicleTelemetry>(car).expect("racer")
}

pub fn pose(app: &App, car: Entity) -> Transform {
    *app.world().get::<Transform>(car).expect("racer")
}

pub fn velocity(app: &App, car: Entity) -> Velocity {
    *app.world().get::<Velocity>(car).expect("car body")
}

pub fn charge(app: &App, car: Entity) -> f32 {
    app.world().get::<Charge>(car).expect("racer").0
}

pub fn effects(app: &mut App, car: Entity) -> Mut<'_, StatusEffects> {
    app.world_mut()
        .get_mut::<StatusEffects>(car)
        .expect("racer")
}

/// Throttle/brake that hold `target` forward speed (P-controller), keeping the other inputs.
pub fn hold_speed(app: &mut App, car: Entity, target: f32) {
    let v = telemetry(app, car).forward_speed;
    let mut c = control(app, car);
    let e = target - v;
    c.throttle = (e * 0.6 + 0.35).clamp(0.0, 1.0);
    c.brake = if e < -1.5 { 0.6 } else { 0.0 };
}

/// Messages of type `M` written since the last call with the same `cursor`.
pub fn read_messages<M: Message + Clone>(
    app: &App,
    cursor: &mut bevy::ecs::message::MessageCursor<M>,
) -> Vec<M> {
    let messages = app.world().resource::<Messages<M>>();
    cursor.read(messages).cloned().collect()
}

/// Distance between the parallel test lanes, u (a full-lock circle at top speed fits).
pub const LANE_GAP_U: f32 = 120.0;

/// Spawns `ids` side by side on the floor, resting after 0.6 s.
pub fn lanes(app: &mut App, ids: &[&str]) -> Vec<Entity> {
    let cars = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            spawn_car(
                app,
                id,
                Transform::from_xyz(i as f32 * LANE_GAP_U, 0.02, 0.0),
            )
        })
        .collect();
    step_seconds(app, 0.6);
    cars
}

/// Yaw rate (rad/s, + = left) of a car.
pub fn yaw_rate(app: &App, car: Entity) -> f32 {
    velocity(app, car)
        .angular
        .dot(pose(app, car).rotation * Vec3::Y)
}

/// Straight-line metrics on flat asphalt from standstill.
#[derive(Debug, Clone, Copy)]
pub struct Straight {
    /// Seconds to 99 % of the top speed.
    pub zero_to_top: Option<f32>,
    /// Highest forward speed in 8 s of full throttle.
    pub top: f32,
    /// Distance from top speed to standstill at full brake.
    pub braking: Option<f32>,
}

pub fn straight(app: &mut App, cars: &[Entity], tops: &[f32]) -> Vec<Straight> {
    let mut out = vec![
        Straight {
            zero_to_top: None,
            top: 0.0,
            braking: None
        };
        cars.len()
    ];
    for &car in cars {
        control(app, car).throttle = 1.0;
    }
    for tick in 1..=(8 * 60) {
        step(app, 1);
        for (i, &car) in cars.iter().enumerate() {
            let v = telemetry(app, car).forward_speed;
            out[i].top = out[i].top.max(v);
            if out[i].zero_to_top.is_none() && v >= rc_vehicle::TOP_SPEED_REACHED * tops[i] {
                out[i].zero_to_top = Some(tick as f32 / 60.0);
            }
        }
    }
    let start: Vec<Vec3> = cars.iter().map(|&c| pose(app, c).translation).collect();
    for &car in cars {
        let mut c = control(app, car);
        c.throttle = 0.0;
        c.brake = 1.0;
    }
    for _ in 0..(4 * 60) {
        step(app, 1);
        for (i, &car) in cars.iter().enumerate() {
            if out[i].braking.is_none() && telemetry(app, car).forward_speed < 0.5 {
                out[i].braking = Some(pose(app, car).translation.distance(start[i]));
            }
        }
    }
    for &car in cars {
        control(app, car).brake = 0.0;
    }
    out
}

/// Steady full-lock cornering at a held speed.
#[derive(Debug, Clone, Copy)]
pub struct Turn {
    pub speed: f32,
    pub radius: f32,
    /// Lateral acceleration v·ω, u/s².
    pub lateral: f32,
    /// Seconds until the yaw rate reaches 90 % of its steady value.
    pub response: Option<f32>,
    pub max_slip_deg: f32,
    pub min_up_dot: f32,
}

/// Holds `speed[i]` straight for 3 s, then full right lock for 5 s.
pub fn turn(app: &mut App, cars: &[Entity], speed: &[f32]) -> Vec<Turn> {
    for _ in 0..(3 * 60) {
        for (i, &car) in cars.iter().enumerate() {
            hold_speed(app, car, speed[i]);
            control(app, car).steer = 0.0;
        }
        step(app, 1);
    }
    let n = cars.len();
    let mut yaw = vec![Vec::new(); n];
    let (mut max_slip, mut min_up) = (vec![0.0f32; n], vec![1.0f32; n]);
    let mut acc = vec![(0.0f32, 0.0f32, 0.0f32); n];
    for tick in 0..(5 * 60) {
        for (i, &car) in cars.iter().enumerate() {
            hold_speed(app, car, speed[i]);
            control(app, car).steer = 1.0;
        }
        step(app, 1);
        for (i, &car) in cars.iter().enumerate() {
            let t = telemetry(app, car);
            let w = yaw_rate(app, car).abs();
            yaw[i].push(w);
            max_slip[i] = max_slip[i].max(t.slip_angle.abs().to_degrees());
            min_up[i] = min_up[i].min(t.up_dot);
            if tick >= 3 * 60 {
                acc[i] = (acc[i].0 + t.speed, acc[i].1 + w, acc[i].2 + 1.0);
            }
        }
    }
    for &car in cars {
        control(app, car).steer = 0.0;
    }
    (0..n)
        .map(|i| {
            let (v, w) = (acc[i].0 / acc[i].2, acc[i].1 / acc[i].2);
            Turn {
                speed: v,
                radius: v / w,
                lateral: v * w,
                response: yaw[i]
                    .iter()
                    .position(|y| *y >= 0.9 * w)
                    .map(|k| (k + 1) as f32 / 60.0),
                max_slip_deg: max_slip[i],
                min_up_dot: min_up[i],
            }
        })
        .collect()
}

/// A handbrake drift at full throttle from 70 % of top speed: 0.8 s full lock + handbrake,
/// 1.2 s half lock + handbrake, then release and centre the steering.
#[derive(Debug, Clone, Copy)]
pub struct Drift {
    /// Largest |slip| during the entry, degrees.
    pub entry_max: f32,
    /// Seconds until |slip| exceeds 10°.
    pub start: Option<f32>,
    /// |slip| range while holding, degrees.
    pub hold: (f32, f32),
    /// Lateral acceleration while holding (v·ω), u/s².
    pub lateral: f32,
    /// Seconds after release until |slip| < 5°.
    pub recover: Option<f32>,
    /// Largest |slip| after release, degrees.
    pub worst_after: f32,
    /// Speed at release / speed before the drift (full throttle throughout).
    pub speed_kept: f32,
    /// Ticks the telemetry reported `drifting` while holding (of 72).
    pub drifting_ticks: usize,
}

pub fn drift(app: &mut App, cars: &[Entity], speed: &[f32]) -> Vec<Drift> {
    for _ in 0..(4 * 60) {
        for (i, &car) in cars.iter().enumerate() {
            hold_speed(app, car, speed[i]);
            control(app, car).steer = 0.0;
        }
        step(app, 1);
    }
    let before: Vec<f32> = cars.iter().map(|&c| telemetry(app, c).speed).collect();
    let mut log = vec![Vec::new(); cars.len()];
    for tick in 0..(4 * 60) {
        let (steer, handbrake) = match tick {
            0..48 => (1.0, true),
            48..120 => (0.5, true),
            _ => (0.0, false),
        };
        for (i, &car) in cars.iter().enumerate() {
            // Full throttle while drifting (as players do), then hold the speed again.
            match handbrake {
                true => control(app, car).throttle = 1.0,
                false => hold_speed(app, car, speed[i]),
            }
            let mut c = control(app, car);
            c.brake = 0.0;
            c.steer = steer;
            c.drift = handbrake;
        }
        step(app, 1);
        for (i, &car) in cars.iter().enumerate() {
            let t = telemetry(app, car);
            log[i].push((
                t.slip_angle.to_degrees().abs(),
                t.speed,
                t.drifting,
                yaw_rate(app, car),
            ));
        }
    }
    for &car in cars {
        let mut c = control(app, car);
        c.drift = false;
        c.steer = 0.0;
    }
    log.iter()
        .zip(before)
        .map(|(l, before)| {
            let hold = &l[48..120];
            Drift {
                entry_max: l[..48].iter().map(|s| s.0).fold(0.0, f32::max),
                start: l
                    .iter()
                    .position(|s| s.0 > 10.0)
                    .map(|k| (k + 1) as f32 / 60.0),
                hold: (
                    hold.iter().map(|s| s.0).fold(f32::MAX, f32::min),
                    hold.iter().map(|s| s.0).fold(0.0, f32::max),
                ),
                lateral: l[110].1 * l[110].3.abs(),
                recover: l[120..]
                    .iter()
                    .position(|s| s.0 < 5.0)
                    .map(|k| k as f32 / 60.0),
                worst_after: l[120..].iter().map(|s| s.0).fold(0.0, f32::max),
                speed_kept: l[119].1 / before,
                drifting_ticks: hold.iter().filter(|s| s.2).count(),
            }
        })
        .collect()
}

/// Steps `secs` and collects every `M` written meanwhile (messages only live ~2 updates).
pub fn step_collect<M: Message + Clone>(app: &mut App, secs: f32) -> Vec<M> {
    let mut cursor = app.world().resource::<Messages<M>>().get_cursor_current();
    let mut out = Vec::new();
    for _ in 0..secs_to_ticks(secs) {
        step(app, 1);
        out.extend(read_messages(app, &mut cursor));
    }
    out
}

/// The arena's kicker: a 14° wooden ramp rising toward −Z whose foot is at `foot` (top edge
/// 2.9 u high, 11.6 u further).
pub fn kicker(app: &mut App, foot: Vec3) -> Entity {
    let (half, angle) = (Vec3::new(5.0, 0.25, 6.0), 14f32.to_radians());
    let rot = Quat::from_rotation_x(angle);
    let centre = foot - rot * Vec3::new(0.0, half.y, half.z);
    app.world_mut()
        .spawn((
            Collider::cuboid(half.x, half.y, half.z),
            Transform::from_translation(centre).with_rotation(rot),
            Surface(SurfaceKind::Wood),
            TrackPiece {
                kind: TrackPieceKind::Ground,
            },
            CollisionGroups::new(Group::from_bits_truncate(layers::TRACK), Group::ALL),
        ))
        .id()
}

/// A run over the arena kicker with the throttle held all the way (also in the air).
#[derive(Debug, Clone, Copy)]
pub struct Jump {
    /// Speed when the last wheel left the ground.
    pub takeoff: f32,
    /// The first landing after more than 0.4 s of air.
    pub landing: Option<CarLanded>,
    /// Speed 0.5 s after that landing.
    pub speed_after: f32,
    /// `up_dot` at the end (6 s).
    pub final_up: f32,
}

/// Every car in `ids` drives over its own kicker from 50 u away at full throttle.
pub fn kicker_jumps(app: &mut App, ids: &[&str]) -> Vec<Jump> {
    let mut cars = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let x = i as f32 * 20.0;
        kicker(app, Vec3::new(x, 0.0, -20.0));
        cars.push(spawn_car(app, id, Transform::from_xyz(x, 0.02, 30.0)));
    }
    step_seconds(app, 0.5);
    for &car in &cars {
        control(app, car).throttle = 1.0;
    }
    let mut out = vec![
        Jump {
            takeoff: 0.0,
            landing: None,
            speed_after: 0.0,
            final_up: 0.0
        };
        cars.len()
    ];
    let mut countdown = vec![None; cars.len()];
    let mut was_grounded = vec![true; cars.len()];
    for _ in 0..(6 * 60) {
        let landed = step_collect::<CarLanded>(app, 1.0 / 60.0);
        for (i, &car) in cars.iter().enumerate() {
            let t = telemetry(app, car);
            if was_grounded[i] && t.grounded_wheels == 0 && out[i].landing.is_none() {
                out[i].takeoff = t.speed;
            }
            was_grounded[i] = t.grounded_wheels > 0;
            if out[i].landing.is_none()
                && let Some(l) = landed.iter().find(|l| l.car == car && l.air_time > 0.4)
            {
                out[i].landing = Some(*l);
                countdown[i] = Some(30);
            }
            if let Some(n) = countdown[i].as_mut() {
                *n -= 1;
                if *n == 0 {
                    out[i].speed_after = t.speed;
                    countdown[i] = None;
                }
            }
            out[i].final_up = t.up_dot;
        }
    }
    out
}
