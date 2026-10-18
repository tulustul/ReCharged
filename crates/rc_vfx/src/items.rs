//! Visuals of collectibles: Volt Cubes (glow, spin, taken burst, respawn pop) and the Golden
//! Battery (gold, sparkle, collect burst). Item visuals are entities of their own, bound to their
//! item by [`Follows`] and despawned with it (also used by projectiles and hazards).

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::assets::VfxAssets;
use crate::cars::FxVisual;
use crate::fx::{Fx, sprite};
use crate::particles::{Facing, Look, Particle, Shape};
use crate::pool::Emitter;

/// Volt Cube edge, u.
const CUBE_U: f32 = 0.85;
/// Respawn pop duration, s.
const POP_S: f32 = 0.35;
/// An effect visual bound to a simulation entity; despawned when the entity is gone.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Follows(pub Entity);

/// The cube visual of a pickup.
#[derive(Component, Debug)]
struct CubeFx {
    spin: Entity,
    phase: f32,
    shown: bool,
    since: f32,
}

/// The visual of the Golden Battery.
#[derive(Component, Debug)]
struct BatteryFx {
    spin: Entity,
    sparkle: Emitter,
}

/// Marker of the spinning child of a cube or battery.
#[derive(Component, Debug)]
struct Spinner;

const CYAN: LinearRgba = LinearRgba::rgb(1.5, 6.0, 10.0);
const GOLD: LinearRgba = LinearRgba::rgb(10.0, 6.5, 1.5);

const TWINKLE: Look = Look {
    shape: Shape::Star,
    emissive: true,
    color: [CYAN, LinearRgba::rgb(0.5, 2.0, 4.0)],
    size: [0.3, 0.05],
    fade_out: 0.6,
    drag: 3.0,
    ..Look::DEFAULT
};

const BURST_RING: Look = Look {
    shape: Shape::Ring,
    facing: Facing::Flat,
    emissive: true,
    color: [CYAN, LinearRgba::rgb(0.2, 1.0, 2.0)],
    size: [0.5, 2.6],
    fade_out: 0.9,
    ..Look::DEFAULT
};

pub(crate) fn build(app: &mut App) {
    app.add_observer(spawn_cube)
        .add_observer(spawn_battery)
        .add_systems(
            Update,
            (
                despawn_orphans,
                (animate_cubes, animate_battery, battery_collected),
            )
                .chain()
                .in_set(crate::VfxSystems::Emit),
        );
}

/// Visuals whose item is gone go with it.
fn despawn_orphans(
    mut commands: Commands,
    entities: &bevy::ecs::entity::Entities,
    q: Query<(Entity, &Follows)>,
) {
    for (visual, follows) in &q {
        if !entities.contains(follows.0) {
            commands.entity(visual).despawn();
        }
    }
}

/// Root of an item visual: bound, posed by rc_vfx, scoped to the attempt.
pub(crate) fn root(target: Entity, at: Transform) -> impl Bundle {
    (
        Follows(target),
        FxVisual,
        at,
        Visibility::default(),
        DespawnOnExit(RaceAttempt),
    )
}

fn spawn_cube(
    add: On<Add<Pickup>>,
    pickups: Query<&Transform>,
    assets: Res<VfxAssets>,
    mut fx: ResMut<Fx>,
    mut commands: Commands,
) {
    let at = pickups.get(add.entity).copied().unwrap_or_default();
    let spin = commands
        .spawn((
            Spinner,
            FxVisual,
            Mesh3d(assets.cube.clone()),
            MeshMaterial3d(assets.cube_glass.clone()),
            Transform::from_scale(Vec3::splat(CUBE_U)),
            NotShadowCaster,
        ))
        .id();
    let bolt = Look {
        shape: Shape::Bolt,
        emissive: true,
        color: [LinearRgba::rgb(10.0, 8.5, 2.5); 2],
        ..Look::DEFAULT
    };
    let halo = Look {
        shape: Shape::Dot,
        emissive: true,
        color: [LinearRgba::rgb(0.25, 1.0, 1.6); 2],
        ..Look::DEFAULT
    };
    let phase = fx.rng.angle();
    commands
        .spawn((
            root(add.entity, Transform::from_translation(at.translation)),
            CubeFx {
                spin,
                phase,
                shown: true,
                since: POP_S,
            },
            Name::new("Volt Cube visual"),
        ))
        .add_child(spin)
        .with_child(sprite(
            &assets,
            &bolt,
            Transform::from_scale(Vec3::splat(CUBE_U * 0.72)),
        ))
        .with_child(sprite(
            &assets,
            &halo,
            Transform::from_scale(Vec3::splat(CUBE_U * 2.8)),
        ));
}

/// Pop curve of a respawning cube: 0 → overshoot → 1.
fn pop_scale(t: f32) -> f32 {
    let x = (t / POP_S).clamp(0.0, 1.0);
    let back = 1.70158 * 1.5;
    let y = x - 1.0;
    1.0 + (back + 1.0) * y * y * y + back * y * y
}

#[allow(clippy::type_complexity)]
fn animate_cubes(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    pickups: Query<(&Transform, Option<&ItemTimer>), (With<Pickup>, Without<FxVisual>)>,
    mut cubes: Query<
        (&Follows, &mut CubeFx, &mut Transform, &mut Visibility),
        (With<FxVisual>, Without<Spinner>),
    >,
    mut spinners: Query<&mut Transform, (With<Spinner>, With<FxVisual>)>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();
    for (follows, mut cube, mut transform, mut visibility) in &mut cubes {
        let Ok((at, timer)) = pickups.get(follows.0) else {
            continue;
        };
        let available = timer.is_none_or(|t| t.0 <= 0.0);
        if available != cube.shown {
            cube.shown = available;
            cube.since = 0.0;
            if !available {
                cube_taken(&mut fx, at.translation);
            }
        }
        cube.since += dt;
        let scale = if available {
            pop_scale(cube.since)
        } else {
            (1.0 - cube.since / 0.1).max(0.0)
        };
        visibility.set_if_neq(if scale > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        transform.translation = at.translation + Vec3::Y * 0.12 * (now * 2.2 + cube.phase).sin();
        transform.scale = Vec3::splat(scale.max(1e-3));
        if let Ok(mut spin) = spinners.get_mut(cube.spin) {
            spin.rotation = Quat::from_rotation_y(now * 1.7 + cube.phase)
                * Quat::from_euler(EulerRot::XYZ, 0.62, 0.0, 0.78);
        }
    }
}

fn cube_taken(fx: &mut Fx, at: Vec3) {
    for _ in 0..fx.n(18) {
        let v = fx.rng.dir() * fx.rng.range(2.0, 6.0);
        let life = fx.rng.range(0.35, 0.6);
        fx.spawn(Particle::new(TWINKLE, at, v, life));
    }
    let ring = Look {
        color: if fx.calm {
            [CYAN * 0.3, CYAN * 0.05]
        } else {
            BURST_RING.color
        },
        ..BURST_RING
    };
    fx.spawn(
        Particle::new(ring, at, Vec3::ZERO, 0.3)
            .rotated(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    );
}

fn spawn_battery(
    add: On<Add<GoldenBattery>>,
    batteries: Query<&Transform>,
    assets: Res<VfxAssets>,
    mut commands: Commands,
) {
    let at = batteries.get(add.entity).copied().unwrap_or_default();
    let part = |mesh: &Handle<Mesh>, mat: &Handle<StandardMaterial>, t: Transform| {
        (Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), t)
    };
    let spin = commands
        .spawn((
            Spinner,
            FxVisual,
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn(part(
                &assets.cylinder,
                &assets.gold,
                Transform::from_scale(Vec3::new(0.62, 0.95, 0.62)),
            ));
            p.spawn(part(
                &assets.cylinder,
                &assets.black,
                Transform::from_xyz(0.0, 0.3, 0.0).with_scale(Vec3::new(0.64, 0.14, 0.64)),
            ));
            p.spawn(part(
                &assets.cylinder,
                &assets.chrome,
                Transform::from_xyz(0.0, 0.53, 0.0).with_scale(Vec3::new(0.24, 0.12, 0.24)),
            ));
        })
        .id();
    let halo = Look {
        shape: Shape::Dot,
        emissive: true,
        color: [LinearRgba::rgb(1.6, 1.0, 0.25); 2],
        ..Look::DEFAULT
    };
    let bolt = Look {
        shape: Shape::Bolt,
        emissive: true,
        color: [LinearRgba::rgb(9.0, 5.5, 1.0); 2],
        ..Look::DEFAULT
    };
    commands
        .spawn((
            root(add.entity, Transform::from_translation(at.translation)),
            BatteryFx {
                spin,
                sparkle: Emitter::default(),
            },
            Name::new("Golden Battery visual"),
        ))
        .add_child(spin)
        .with_child(sprite(
            &assets,
            &halo,
            Transform::from_scale(Vec3::splat(3.2)),
        ))
        .with_child(sprite(
            &assets,
            &bolt,
            Transform::from_xyz(0.0, 0.05, 0.0).with_scale(Vec3::splat(0.5)),
        ));
}

#[allow(clippy::type_complexity)]
fn animate_battery(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    batteries: Query<&Transform, (With<GoldenBattery>, Without<FxVisual>)>,
    mut visuals: Query<
        (&Follows, &mut BatteryFx, &mut Transform),
        (With<FxVisual>, Without<Spinner>),
    >,
    mut spinners: Query<&mut Transform, (With<Spinner>, With<FxVisual>)>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();
    for (follows, mut battery, mut transform) in &mut visuals {
        let Ok(at) = batteries.get(follows.0) else {
            continue;
        };
        transform.translation = at.translation + Vec3::Y * 0.15 * (now * 1.8).sin();
        if let Ok(mut spin) = spinners.get_mut(battery.spin) {
            spin.rotation = Quat::from_rotation_y(now * 1.2) * Quat::from_rotation_z(0.25);
        }
        let n = battery.sparkle.count(14.0 * fx.density, dt);
        let gold = Look {
            color: [GOLD, LinearRgba::rgb(3.0, 1.5, 0.2)],
            fade_in: 0.3,
            fade_out: 0.5,
            drag: 0.5,
            gravity: -0.4,
            ..TWINKLE
        };
        for _ in 0..n {
            let from = transform.translation + fx.rng.dir() * fx.rng.range(0.5, 1.1);
            let v = fx.rng.dir() * 0.3;
            let (life, scale) = (fx.rng.range(0.6, 1.1), fx.rng.range(0.6, 1.3));
            fx.spawn(Particle::new(gold, from, v, life).scaled(scale));
        }
    }
}

fn battery_collected(
    mut events: MessageReader<RaceEvent>,
    mut fx: ResMut<Fx>,
    visuals: Query<&Transform, With<BatteryFx>>,
) {
    for ev in events.read() {
        if ev.kind != RaceEventKind::GoldenBattery {
            continue;
        }
        for at in &visuals {
            let at = at.translation;
            for _ in 0..fx.n(60) {
                let gold = Look {
                    color: [GOLD, LinearRgba::rgb(3.0, 1.2, 0.1)],
                    size: [0.45, 0.05],
                    gravity: 2.0,
                    ..TWINKLE
                };
                let v = fx.rng.dir() * fx.rng.range(2.0, 9.0) + Vec3::Y * 2.0;
                let life = fx.rng.range(0.6, 1.3);
                fx.spawn(Particle::new(gold, at, v, life));
            }
            let ring = Look {
                color: if fx.calm {
                    [GOLD * 0.25, GOLD * 0.05]
                } else {
                    [GOLD, GOLD * 0.1]
                },
                size: [0.6, 5.0],
                ..BURST_RING
            };
            fx.spawn(
                Particle::new(ring, at, Vec3::ZERO, 0.5)
                    .rotated(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respawn_pop_overshoots_then_settles() {
        assert!(pop_scale(0.0).abs() < 1e-5);
        let peak = (1..35)
            .map(|i| pop_scale(i as f32 * 0.01))
            .fold(0.0, f32::max);
        assert!(peak > 1.05 && peak < 1.3, "overshoot {peak}");
        assert!((pop_scale(POP_S) - 1.0).abs() < 1e-5);
        assert!((pop_scale(10.0) - 1.0).abs() < 1e-5);
    }
}
