//! Shared helpers of the rc_items integration tests: a running race on `test_oval` with simple
//! box cars (rc_items may not depend on rc_vehicle, so [`TestVehicle`] stands in for its
//! request handling), item use, and a log of every item message.
#![allow(dead_code)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;
use rc_items::ItemsPlugin;
use rc_testkit::*;
use rc_track::{Track, TrackPlugin};

/// Height of the test_oval asphalt (top of the slab).
pub const ROAD_Y: f32 = 0.3;

/// Every item message seen, in order.
#[derive(Resource, Debug, Default)]
pub struct Log {
    pub items: Vec<ItemEvent>,
    pub hits: Vec<(u32, HitEvent)>,
    pub explosions: Vec<Explosion>,
}

#[derive(Resource, Debug, Default)]
struct Frame(u32);

fn record(
    mut frame: ResMut<Frame>,
    mut log: ResMut<Log>,
    mut items: MessageReader<ItemEvent>,
    mut hits: MessageReader<HitEvent>,
    mut explosions: MessageReader<Explosion>,
) {
    frame.0 += 1;
    log.items.extend(items.read().copied());
    let f = frame.0;
    log.hits.extend(hits.read().map(|h| (f, *h)));
    log.explosions.extend(explosions.read().copied());
}

/// rc_vehicle's handling of `ApplyImpulse` (× (2.6 / mass)^0.5), `DrainCharge` and `GrantCharge`.
pub struct TestVehicle;

impl Plugin for TestVehicle {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, impulses.in_set(SimSet::Vehicle))
            .add_systems(FixedUpdate, charge.in_set(SimSet::PostPhysics));
    }
}

fn impulses(
    mut requests: MessageReader<ApplyImpulse>,
    mut cars: Query<(&ReadMassProperties, &mut ExternalImpulse)>,
) {
    for r in requests.read() {
        if let Ok((mass, mut impulse)) = cars.get_mut(r.car) {
            let k = (REFERENCE_CAR_MASS / mass.get().mass).sqrt();
            impulse.impulse += r.linear * k;
            impulse.torque_impulse += r.angular * k;
        }
    }
}

fn charge(
    mut drains: MessageReader<DrainCharge>,
    mut grants: MessageReader<GrantCharge>,
    mut cars: Query<&mut Charge>,
) {
    for d in drains.read() {
        if let Ok(mut c) = cars.get_mut(d.car) {
            c.0 *= 1.0 - d.fraction.clamp(0.0, 1.0);
        }
    }
    for g in grants.read() {
        if let Ok(mut c) = cars.get_mut(g.car) {
            c.0 = (c.0 + g.amount).min(1.0);
        }
    }
}

/// A running race on `test_oval` (items on) with the track's colliders and cubes.
pub fn race(seed: u64) -> App {
    let mut app = headless_app((TrackPlugin, ItemsPlugin, TestVehicle));
    app.init_resource::<Log>()
        .init_resource::<Frame>()
        .add_systems(Update, record);
    let config = RaceConfig {
        variant: TrackVariant::forward("test_oval"),
        items: true,
        seed,
        ..default()
    };
    start_race(&mut app, config).expect("race loads");
    app
}

/// A box car (2.0 × 1.1 × 0.6 u, mass 2.6, frictionless so a push keeps it going) whose
/// origin is on the ground at `at`, facing `yaw` radians from −Z (positive turns left).
pub fn spawn_car(app: &mut App, at: Vec3, yaw: f32) -> Entity {
    let index = app.world_mut().query::<&Racer>().iter(app.world()).count();
    let body = Collider::cuboid(0.55, 0.3, 1.0);
    app.world_mut()
        .spawn((
            Racer {
                index,
                name: format!("car {index}"),
                ..default()
            },
            Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)),
            (
                RigidBody::Dynamic,
                Collider::compound(vec![(Vec3::Y * 0.3, Quat::IDENTITY, body)]),
                ColliderMassProperties::Mass(REFERENCE_CAR_MASS),
                Velocity::zero(),
                ExternalImpulse::default(),
                Sleeping::disabled(),
            ),
            (
                Friction {
                    coefficient: 0.0,
                    combine_rule: CoefficientCombineRule::Min,
                },
                CollisionGroups::new(Group::from_bits_truncate(layers::CAR), Group::ALL),
                ActiveEvents::COLLISION_EVENTS,
            ),
            DespawnOnExit(RaceAttempt),
        ))
        .id()
}

/// Gives `car` a ready `kind` and uses it this tick (`backward` = Look back held).
pub fn fire(app: &mut App, car: Entity, kind: ItemKind, backward: bool) {
    *app.world_mut().get_mut::<ItemSlot>(car).expect("racer") = ItemSlot {
        item: Some(kind),
        roulette: 0.0,
    };
    {
        let mut c = app.world_mut().get_mut::<CarControl>(car).expect("racer");
        c.use_item = true;
        c.look_back = backward;
    }
    step(app, 1);
    let mut c = app.world_mut().get_mut::<CarControl>(car).expect("racer");
    c.use_item = false;
    c.look_back = false;
}

/// Updates (= fixed ticks) run since the app started; `Log::hits` carry it.
pub fn frame(app: &App) -> u32 {
    app.world().resource::<Frame>().0
}

pub fn log(app: &App) -> &Log {
    app.world().resource::<Log>()
}

/// Hits on `victim` by `by` so far.
pub fn hits_on(app: &App, victim: Entity, by: ItemKind) -> Vec<HitEvent> {
    log(app)
        .hits
        .iter()
        .map(|(_, h)| *h)
        .filter(|h| h.victim == victim && h.by == by)
        .collect()
}

pub fn effects(app: &App, car: Entity) -> StatusEffects {
    *app.world().get::<StatusEffects>(car).expect("racer")
}

pub fn pose(app: &App, car: Entity) -> Transform {
    *app.world().get::<Transform>(car).expect("entity")
}

pub fn velocity(app: &App, car: Entity) -> Velocity {
    *app.world().get::<Velocity>(car).expect("car body")
}

pub fn set_velocity(app: &mut App, car: Entity, linear: Vec3) {
    app.world_mut()
        .get_mut::<Velocity>(car)
        .expect("car body")
        .linear = linear;
}

pub fn track(app: &App) -> &Track {
    app.world().resource::<Track>()
}

/// Entities with component `C`.
pub fn all<C: Component>(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .iter(app.world())
        .collect()
}
