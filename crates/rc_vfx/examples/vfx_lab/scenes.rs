//! The lab's scripted scenes. Each spawns stand-in sim entities in `OnEnter(RaceAttempt)` and
//! animates them / writes messages before rc_vfx's emitters run.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_vfx::{Facing, Fx, Look, Particle, Shape, VfxSystems};

use crate::{Args, CARS, PATCHES};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Clock>()
        .add_systems(OnEnter(RaceAttempt), spawn_scene)
        .add_systems(
            Update,
            (tick, shapes, drive, chase, items, blasts, events, weather)
                .chain()
                .before(VfxSystems::Emit),
        );
}

/// Scene time (virtual seconds since the scene started) and the previous frame's.
#[derive(Resource, Default)]
struct Clock {
    now: f32,
    last: f32,
}

impl Clock {
    /// `period`-repeating trigger at `offset` s that fired this frame.
    fn fired(&self, period: f32, offset: f32) -> bool {
        let phase = |t: f32| ((t - offset) / period).floor();
        self.now >= offset && phase(self.now) != phase(self.last)
    }
}

fn tick(time: Res<Time>, mut clock: ResMut<Clock>) {
    clock.last = clock.now;
    clock.now += time.delta_secs();
}

/// A scripted car: (role, extra parameter).
#[derive(Component, Clone, Copy, PartialEq)]
enum Role {
    /// Circles `centre`; `charge` < 0 cycles the drift charge rate, else holds it.
    Driver {
        centre: Vec2,
        radius: f32,
        speed: f32,
        angle: f32,
        drift: bool,
        charge: f32,
    },
    Shielded,
    GooCarrier,
    Parked,
    Lander,
    Crasher,
    Respawner,
    Winner,
    Stunned,
    Booster,
}

#[derive(Component)]
struct Item(&'static str);

fn spawn_car(
    commands: &mut Commands,
    server: &AssetServer,
    i: usize,
    role: Role,
    at: Transform,
) -> Entity {
    let car = CARS[i % CARS.len()];
    let colors = [
        [0.1, 0.8, 1.0],
        [1.0, 0.85, 0.1],
        [1.0, 0.25, 0.8],
        [0.3, 0.9, 0.3],
    ];
    commands
        .spawn((
            Racer {
                index: i,
                name: [
                    "Mayhem",
                    "Penny",
                    "Turbo Tom",
                    "Vix",
                    "Bolt Boy",
                    "Nova",
                    "Rusty",
                    "Zed",
                ][i % 8]
                    .into(),
                car: CarId::new(car),
                color: colors[i % 4],
                ..default()
            },
            role,
            at,
            Visibility::default(),
            children![WorldAssetRoot(server.load(
                GltfAssetLabel::Scene(0).from_asset(format!("models/cars/{car}.glb"))
            ))],
            DespawnOnExit(RaceAttempt),
        ))
        .id()
}

fn facing(x: f32, z: f32, yaw: f32) -> Transform {
    Transform::from_xyz(x, 0.0, z).with_rotation(Quat::from_rotation_y(yaw))
}

fn spawn_scene(
    args: Res<Args>,
    mut commands: Commands,
    server: Res<AssetServer>,
    mut clock: ResMut<Clock>,
) {
    *clock = Clock::default();
    let c = &mut commands;
    let s = &server;
    if args.wet {
        c.insert_resource(WeatherState {
            wetness: 1.0,
            ..default()
        });
    }
    match args.scene.as_str() {
        "drive" => {
            for (i, (centre, ..)) in PATCHES.iter().enumerate() {
                let role = Role::Driver {
                    centre: *centre,
                    radius: 4.5,
                    speed: 16.0,
                    angle: i as f32 * 1.7,
                    drift: i != 1,
                    charge: -1.0,
                };
                spawn_car(
                    c,
                    s,
                    i * 2,
                    role,
                    Transform::from_xyz(centre.x, 0.0, centre.y),
                );
            }
        }
        "sparks" => {
            // Charge-rate ladder: just charging, mid, peak (GDD §5.3 drift gain up to 22 %/s).
            for (i, charge) in [0.02, 0.11, 0.22].into_iter().enumerate() {
                let role = Role::Driver {
                    centre: Vec2::new(-6.0 + 6.0 * i as f32, -1.0),
                    radius: 2.4,
                    speed: 11.0,
                    angle: i as f32 * 2.1,
                    drift: true,
                    charge,
                };
                spawn_car(c, s, 2 * i + 1, role, Transform::default());
            }
        }
        "weather" => {
            let role = Role::Driver {
                centre: Vec2::new(0.0, 0.0),
                radius: 6.0,
                speed: 15.0,
                angle: 0.0,
                drift: false,
                charge: 0.0,
            };
            let car = spawn_car(c, s, 0, role, Transform::default());
            c.entity(car).insert(LocalPlayer(PlayerSlot(0)));
            spawn_car(c, s, 3, Role::Parked, facing(-3.0, -5.0, 0.6));
        }
        "items" => {
            for k in 0..5 {
                c.spawn((
                    Pickup { row: 0, slot: k },
                    ItemTimer(0.0),
                    ItemRadius(1.5),
                    Transform::from_xyz(-4.5 + 2.25 * f32::from(k), 0.9, -6.0),
                    DespawnOnExit(RaceAttempt),
                ));
            }
            c.spawn((
                GoldenBattery,
                Transform::from_xyz(8.0, 1.2, -5.0),
                DespawnOnExit(RaceAttempt),
            ));
            let shielded = spawn_car(c, s, 0, Role::Shielded, facing(-5.5, 2.0, 0.5));
            let carrier = spawn_car(c, s, 1, Role::GooCarrier, facing(5.5, 2.0, -0.5));
            c.spawn((
                Hazard {
                    kind: HazardKind::GooBomb,
                    owner: carrier,
                },
                ItemTarget(Some(carrier)),
                ItemTimer(6.0),
                ItemRadius(7.0),
                Item("goo"),
                Transform::from_xyz(5.5, 0.9, 2.0),
                DespawnOnExit(RaceAttempt),
            ));
            let leech = spawn_car(c, s, 2, Role::Parked, facing(-1.5, 6.5, 0.0));
            let victim = spawn_car(c, s, 5, Role::Parked, facing(2.0, 1.0, 0.3));
            c.spawn((
                Projectile {
                    kind: ItemKind::LeechCoil,
                    owner: leech,
                },
                ItemTarget(Some(victim)),
                Item("leech"),
                Transform::from_xyz(-1.5, 0.0, 6.5),
                DespawnOnExit(RaceAttempt),
            ));
            c.spawn((
                Projectile {
                    kind: ItemKind::SparkRocket,
                    owner: leech,
                },
                ItemTarget(Some(shielded)),
                ItemRadius(1.8),
                Item("rocket"),
                Transform::from_xyz(0.0, 1.5, -2.0),
                DespawnOnExit(RaceAttempt),
            ));
            c.spawn((
                Projectile {
                    kind: ItemKind::ShockOrb,
                    owner: leech,
                },
                ItemRadius(3.0),
                Item("orb"),
                Transform::from_xyz(0.0, 0.7, -11.0),
                DespawnOnExit(RaceAttempt),
            ));
            c.spawn((
                Hazard {
                    kind: HazardKind::OilSlick,
                    owner: leech,
                },
                ItemRadius(2.5),
                ItemTimer(25.0),
                Transform::from_xyz(-8.5, 0.0, -2.0),
                DespawnOnExit(RaceAttempt),
            ));
            c.spawn((
                Hazard {
                    kind: HazardKind::SpringPad,
                    owner: leech,
                },
                ItemRadius(1.6),
                ItemTimer(25.0),
                Transform::from_xyz(8.5, 0.0, -1.0),
                DespawnOnExit(RaceAttempt),
            ));
        }
        "blasts" => {
            spawn_car(c, s, 4, Role::Parked, facing(0.0, -2.0, 0.4));
            spawn_car(c, s, 6, Role::Parked, facing(6.0, 1.0, -0.4));
        }
        "events" => {
            spawn_car(c, s, 0, Role::Lander, facing(-6.0, 0.0, 0.5));
            spawn_car(c, s, 1, Role::Crasher, facing(0.0, -10.6, 0.0));
            spawn_car(c, s, 2, Role::Respawner, facing(6.0, 0.0, -0.4));
            let winner = spawn_car(c, s, 3, Role::Winner, facing(0.0, 3.0, 0.0));
            c.entity(winner).insert(LocalPlayer(PlayerSlot(0)));
            spawn_car(c, s, 4, Role::Stunned, facing(-7.0, 6.0, 0.8));
            spawn_car(c, s, 6, Role::Booster, facing(7.5, 6.0, -0.8));
        }
        _ => {}
    }
}

/// Every shape twice (lit, emissive), held still in two rows.
fn shapes(args: Res<Args>, clock: Res<Clock>, mut fx: ResMut<Fx>) {
    if args.scene != "shapes" || !clock.fired(0.5, 0.0) {
        return;
    }
    let all = [
        Shape::Dot,
        Shape::Core,
        Shape::Streak,
        Shape::Puff,
        Shape::Ring,
        Shape::Star,
        Shape::Chip,
        Shape::Square,
        Shape::Drop,
        Shape::Bolt,
        Shape::Reticle,
    ];
    for (i, &shape) in all.iter().enumerate() {
        let x = (i as f32 - 5.0) * 1.3;
        for (row, emissive) in [(0.6, false), (2.2, true)] {
            let color = if emissive {
                LinearRgba::rgb(6.0, 3.0, 0.8)
            } else {
                LinearRgba::new(0.85, 0.5, 0.3, 1.0)
            };
            let look = Look {
                shape,
                facing: if shape == Shape::Streak {
                    Facing::Axis
                } else {
                    Facing::Camera
                },
                emissive,
                color: [color, color],
                size: [1.0, 1.0],
                fade_out: 0.0,
                ..Look::DEFAULT
            };
            fx.spawn(Particle::new(look, Vec3::new(x, row, 0.0), Vec3::ZERO, 0.6));
        }
    }
}

fn surface_at(p: Vec3) -> SurfaceKind {
    PATCHES
        .iter()
        .find(|(c, r, ..)| c.distance(p.xz()) < *r)
        .map_or(SurfaceKind::Asphalt, |(_, _, s, _)| *s)
}

/// Scripted drivers: circle, drift, boost every few seconds; telemetry as rc_vehicle writes it.
fn drive(
    time: Res<Time>,
    clock: Res<Clock>,
    mut cars: Query<(
        &mut Role,
        &mut Transform,
        &mut VehicleTelemetry,
        &mut StatusEffects,
    )>,
) {
    let dt = time.delta_secs();
    for (mut role, mut transform, mut t, mut effects) in &mut cars {
        let Role::Driver {
            centre,
            radius,
            speed,
            angle,
            drift,
            charge,
        } = &mut *role
        else {
            continue;
        };
        *angle += *speed / *radius * dt;
        let pos = *centre + Vec2::from_angle(*angle) * *radius;
        let vel = Vec2::new(-angle.sin(), angle.cos()) * *speed;
        let vdir = Vec3::new(vel.x, 0.0, vel.y).normalize();
        let slip = if *drift { 0.45 } else { 0.05 };
        let heading = Quat::from_rotation_y(slip) * vdir;
        transform.translation = Vec3::new(pos.x, 0.0, pos.y);
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, heading);
        let right = transform.rotation * Vec3::X;
        let fwd = transform.rotation * Vec3::NEG_Z;
        let surface = surface_at(transform.translation);
        let boosting = *charge < 0.0 && (clock.now % 5.0) > 3.4;
        *t = VehicleTelemetry {
            speed: *speed,
            forward_speed: *speed * vdir.dot(fwd),
            slip_angle: vdir.dot(right).atan2(vdir.dot(fwd)),
            drifting: *drift,
            surface: Some(surface),
            grounded_wheels: 4,
            boosting,
            up_dot: 1.0,
            charge_rate: match (*drift, *charge < 0.0) {
                (false, _) => 0.0,
                (true, true) => 0.04 + 0.2 * ((clock.now * 0.25).sin() * 0.5 + 0.5),
                (true, false) => *charge,
            },
            wheels: [WheelState {
                grounded: true,
                surface: Some(surface),
                slip: if *drift { 0.75 } else { 0.1 },
                ..default()
            }; 4],
            ..default()
        };
        effects.boost = (boosting && centre.x > 0.0).then_some(TimedBoost {
            top_speed: 1.45,
            force: 1.8,
            freeze_charge: true,
            remaining: 1.0,
        });
    }
}

/// `--chase`: the camera rides behind the first driver like the game's chase camera.
fn chase(
    args: Res<Args>,
    cars: Query<(&Racer, &Transform, &Role), Without<GameCamera>>,
    mut cams: Query<(&GameCamera, &mut Transform)>,
) {
    if !args.chase {
        return;
    }
    let Some((_, car, _)) = cars
        .iter()
        .find(|(r, _, role)| r.index == 0 && matches!(role, Role::Driver { .. }))
    else {
        return;
    };
    for (_, mut cam) in &mut cams {
        let back = car.rotation * Vec3::Z;
        let eye = car.translation + back * 4.6 + Vec3::Y * 1.7;
        *cam = Transform::from_translation(eye)
            .looking_at(car.translation + Vec3::Y * 0.6 - back * 2.0, Vec3::Y);
    }
}

#[allow(clippy::type_complexity)]
fn items(
    args: Res<Args>,
    clock: Res<Clock>,
    mut hits: MessageWriter<HitEvent>,
    mut cars: Query<(Entity, &Role, &Transform, &mut StatusEffects), Without<Item>>,
    mut pickups: Query<(&Pickup, &mut ItemTimer), Without<Item>>,
    mut things: Query<(&Item, &mut Transform, Option<&mut ItemTimer>), Without<Role>>,
) {
    if args.scene != "items" {
        return;
    }
    let t = clock.now;
    // Cube 2 is taken every 4 s and respawns after the 2 s countdown.
    for (pickup, mut timer) in &mut pickups {
        if pickup.slot == 2 {
            timer.0 = if t % 4.0 > 2.0 { 4.0 - t % 4.0 } else { 0.0 };
        }
    }
    let shielded = cars.iter().find(|c| *c.1 == Role::Shielded).map(|c| c.0);
    for (_, role, _, mut effects) in &mut cars {
        if *role == Role::Shielded {
            effects.shield = Some(10.0 - t % 10.0);
        }
    }
    if let Some(victim) = shielded
        && clock.fired(2.5, 1.0)
    {
        hits.write(HitEvent {
            victim,
            attacker: None,
            by: ItemKind::SparkRocket,
            blocked: true,
        });
    }
    for (item, mut transform, timer) in &mut things {
        match item.0 {
            "goo" => {
                if let Some(mut timer) = timer {
                    timer.0 = 6.0 - t % 6.0;
                }
            }
            "rocket" => {
                let a = t * 1.6;
                let pos = Vec3::new(a.cos() * 4.0, 1.4, -2.0 + a.sin() * 4.0);
                let dir = Vec3::new(-a.sin(), 0.0, a.cos());
                *transform = Transform::from_translation(pos).looking_to(dir, Vec3::Y);
            }
            "orb" => transform.translation.x = (t * 0.8).sin() * 6.0,
            _ => {}
        }
    }
}

/// Blasts in sequence every 10 s: rocket (1 s), goo (3 s), EMP wind-up (5 s) and pulse (5.4 s),
/// Storm Strike (7 s), zaps (9 s).
#[allow(clippy::type_complexity)]
fn blasts(
    args: Res<Args>,
    clock: Res<Clock>,
    mut commands: Commands,
    mut explosions: MessageWriter<Explosion>,
    mut hits: MessageWriter<HitEvent>,
    cars: Query<(Entity, &Racer, &Transform)>,
    emps: Query<(Entity, &Projectile, &mut ItemTimer)>,
) {
    if args.scene != "blasts" {
        return;
    }
    let car = |i: usize| {
        cars.iter()
            .find(|(_, r, _)| r.index == i)
            .map(|(e, _, t)| (e, *t))
    };
    if clock.fired(10.0, 1.0) {
        explosions.write(Explosion {
            position: Vec3::new(-5.0, 0.5, -3.0),
            radius: 1.8,
            kind: ItemKind::SparkRocket,
        });
    }
    if clock.fired(10.0, 3.0) {
        explosions.write(Explosion {
            position: Vec3::new(4.0, 0.9, -4.0),
            radius: 7.0,
            kind: ItemKind::GooBomb,
        });
    }
    if clock.fired(10.0, 5.0)
        && let Some((owner, at)) = car(4)
    {
        commands.spawn((
            Projectile {
                kind: ItemKind::EmpPulse,
                owner,
            },
            ItemTimer(0.4),
            ItemRadius(12.0),
            at,
            DespawnOnExit(RaceAttempt),
        ));
    }
    let dt = clock.now - clock.last;
    for (entity, projectile, mut timer) in emps {
        timer.0 -= dt;
        if timer.0 <= 0.0 {
            let at = car(4).map_or(Vec3::ZERO, |(_, t)| t.translation);
            explosions.write(Explosion {
                position: at + Vec3::Y * 0.5,
                radius: 12.0,
                kind: projectile.kind,
            });
            commands.entity(entity).despawn();
        }
    }
    if clock.fired(10.0, 7.0)
        && let Some((victim, _)) = car(6)
    {
        hits.write(HitEvent {
            victim,
            attacker: None,
            by: ItemKind::StormStrike,
            blocked: false,
        });
    }
    if clock.fired(10.0, 9.0) {
        for (i, by) in [(4, ItemKind::ShockOrb), (6, ItemKind::OilSlick)] {
            if let Some((victim, _)) = car(i) {
                hits.write(HitEvent {
                    victim,
                    attacker: None,
                    by,
                    blocked: false,
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn events(
    args: Res<Args>,
    clock: Res<Clock>,
    mut landed: MessageWriter<CarLanded>,
    mut impacts: MessageWriter<CarImpact>,
    mut respawned: MessageWriter<CarRespawned>,
    mut race: MessageWriter<RaceEvent>,
    mut cars: Query<(
        Entity,
        &Role,
        &Transform,
        &mut VehicleTelemetry,
        &mut StatusEffects,
    )>,
) {
    if args.scene != "events" {
        return;
    }
    for (car, role, at, mut t, mut effects) in &mut cars {
        match role {
            Role::Lander => {
                t.surface = Some(SurfaceKind::Dirt);
                if clock.fired(2.0, 0.8) {
                    landed.write(CarLanded {
                        car,
                        air_time: 1.0,
                        strength: 0.9,
                        clean: true,
                    });
                }
            }
            Role::Crasher if clock.fired(1.5, 0.5) => {
                let point = at.translation + Vec3::new(0.3, 0.5, -1.0);
                impacts.write(CarImpact {
                    car,
                    other: None,
                    strength: 0.8,
                    point,
                    normal: Vec3::Z,
                });
            }
            Role::Respawner if clock.fired(2.5, 1.2) => {
                respawned.write(CarRespawned {
                    car,
                    kind: RespawnKind::ToTrack,
                });
            }
            Role::Winner if clock.fired(100.0, 1.5) => {
                race.write(RaceEvent {
                    racer: Some(car),
                    kind: RaceEventKind::Finished {
                        rank: 1,
                        time: 6000,
                    },
                });
            }
            Role::Stunned => {
                effects.power_loss = Some(TimedMul {
                    mul: 0.0,
                    remaining: 2.0,
                });
            }
            Role::Booster => {
                t.boosting = clock.now % 2.0 > 0.6;
                t.wheels = [WheelState {
                    grounded: true,
                    surface: Some(SurfaceKind::Asphalt),
                    ..default()
                }; 4];
            }
            _ => {}
        }
    }
}

/// Rain (or `--snow`) with a storm's wind and lightning every 2.5 s.
fn weather(
    args: Res<Args>,
    clock: Res<Clock>,
    mut commands: Commands,
    mut lightning: MessageWriter<WeatherEvent>,
) {
    if args.scene != "weather" {
        return;
    }
    commands.insert_resource(WeatherState {
        kind: if args.snow {
            WeatherKind::Snow
        } else {
            WeatherKind::Storm
        },
        time: if args.night {
            TimeOfDay::Night
        } else {
            TimeOfDay::Noon
        },
        precipitation: if args.snow { 0.5 } else { 1.0 },
        wetness: if args.snow { 0.0 } else { 1.0 },
        snow_cover: if args.snow { 1.0 } else { 0.0 },
        wind: Vec3::new(if args.snow { 3.0 } else { 12.0 }, 0.0, 2.0),
        cloudiness: 0.95,
        snowing: args.snow,
        ..default()
    });
    if !args.snow && clock.fired(2.5, 0.5) {
        lightning.write(WeatherEvent::Lightning { intensity: 1.0 });
    }
}
