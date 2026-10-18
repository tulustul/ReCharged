//! Projectile visuals (`Projectile` + `Interpolated`, posed from the proxy): the Spark Rocket
//! (model oriented along its motion, smoke and spark trail), the Shock Orb (plasma core, arcs and
//! a faint field at its knock radius), the Leech Coil tether beam and the EMP wind-up shell.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::assets::{BubbleMaterial, VfxAssets};
use crate::cars::{CarFx, FxVisual, Poses};
use crate::fx::{Fx, segment, sprite, zigzag};
use crate::items::{Follows, root};
use crate::particles::{Facing, Look, Particle, Shape};
use crate::pool::Emitter;

/// Rocket trail: one smoke puff per this much travel, u.
const TRAIL_STEP_U: f32 = 0.18;

const CYAN: LinearRgba = LinearRgba::rgb(1.5, 6.0, 10.0);

/// The visual of a projectile.
#[derive(Component, Debug)]
struct ProjectileFx {
    kind: ItemKind,
    last: Option<Vec3>,
    dir: Vec3,
    trail: f32,
    emit: Emitter,
    age: f32,
}

const SMOKE_TRAIL: Look = Look {
    shape: Shape::Puff,
    color: [
        LinearRgba::new(0.78, 0.78, 0.8, 0.5),
        LinearRgba::new(0.85, 0.85, 0.88, 0.0),
    ],
    size: [0.18, 1.0],
    fade_in: 0.05,
    fade_out: 0.75,
    drag: 1.5,
    gravity: -0.6,
    ..Look::DEFAULT
};

const EXHAUST_SPARK: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Velocity,
    emissive: true,
    color: [
        LinearRgba::rgb(12.0, 7.0, 2.0),
        LinearRgba::rgb(4.0, 0.8, 0.1),
    ],
    size: [0.04, 0.02],
    fade_out: 0.5,
    drag: 2.0,
    gravity: 6.0,
    stretch: 0.02,
    aspect: 2.0,
    ..Look::DEFAULT
};

const ORB_ARC: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Axis,
    emissive: true,
    color: [
        LinearRgba::rgb(8.0, 6.0, 16.0),
        LinearRgba::rgb(4.0, 2.0, 12.0),
    ],
    size: [0.04, 0.03],
    fade_out: 0.5,
    ..Look::DEFAULT
};

const LEECH: LinearRgba = LinearRgba::rgb(3.0, 11.0, 1.5);

const BEAM: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Axis,
    emissive: true,
    color: [LEECH, LEECH],
    size: [0.07, 0.07],
    fade_out: 0.0,
    ..Look::DEFAULT
};

const MOTE: Look = Look {
    shape: Shape::Core,
    emissive: true,
    color: [CYAN, CYAN],
    size: [0.22, 0.14],
    fade_in: 0.2,
    fade_out: 0.3,
    ..Look::DEFAULT
};

pub(crate) fn build(app: &mut App) {
    app.add_observer(spawn_projectile).add_systems(
        Update,
        (rockets, orbs, tethers, emp_windup).in_set(crate::VfxSystems::Emit),
    );
}

#[allow(clippy::type_complexity)]
fn spawn_projectile(
    add: On<Add<Projectile>>,
    projectiles: Query<(&Projectile, &Transform, Option<&ItemRadius>)>,
    assets: Res<VfxAssets>,
    mut bubbles: ResMut<Assets<BubbleMaterial>>,
    mut commands: Commands,
) {
    let Ok((projectile, at, radius)) = projectiles.get(add.entity) else {
        return;
    };
    let mut visual = commands.spawn((
        root(add.entity, *at),
        ProjectileFx {
            kind: projectile.kind,
            last: None,
            dir: at.rotation * Vec3::NEG_Z,
            trail: 0.0,
            emit: Emitter::default(),
            age: 0.0,
        },
        Name::new(format!("{:?} visual", projectile.kind)),
    ));
    let part = |mesh: &Handle<Mesh>, mat: &Handle<StandardMaterial>, t: Transform| {
        (
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            t,
            NotShadowCaster,
        )
    };
    match projectile.kind {
        ItemKind::SparkRocket => {
            // Built along −Z (forward), 0.6 u long.
            let along = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
            let glow = Look {
                shape: Shape::Core,
                emissive: true,
                color: [LinearRgba::rgb(14.0, 8.0, 3.0); 2],
                ..Look::DEFAULT
            };
            visual.with_children(|p| {
                p.spawn(part(
                    &assets.cylinder,
                    &assets.white,
                    Transform::from_rotation(along).with_scale(Vec3::new(0.15, 0.42, 0.15)),
                ));
                p.spawn(part(
                    &assets.cone,
                    &assets.red,
                    Transform::from_xyz(0.0, 0.0, -0.29)
                        .with_rotation(along)
                        .with_scale(Vec3::new(0.15, 0.16, 0.15)),
                ));
                for k in 0..3 {
                    let roll = Quat::from_rotation_z(k as f32 * std::f32::consts::TAU / 3.0);
                    p.spawn(part(
                        &assets.cube,
                        &assets.red,
                        Transform::from_rotation(roll)
                            .with_translation(roll * Vec3::new(0.0, 0.1, 0.17))
                            .with_scale(Vec3::new(0.015, 0.12, 0.12)),
                    ));
                }
                p.spawn(part(
                    &assets.cylinder,
                    &assets.black,
                    Transform::from_xyz(0.0, 0.0, 0.23)
                        .with_rotation(along)
                        .with_scale(Vec3::new(0.11, 0.05, 0.11)),
                ));
                p.spawn(sprite(
                    &assets,
                    &glow,
                    Transform::from_xyz(0.0, 0.0, 0.3).with_scale(Vec3::splat(0.4)),
                ));
            });
        }
        ItemKind::ShockOrb => {
            let halo = Look {
                shape: Shape::Dot,
                emissive: true,
                color: [LinearRgba::rgb(2.5, 1.5, 6.0); 2],
                ..Look::DEFAULT
            };
            // The knock radius as a faint field: what you see is what knocks you.
            let reach = radius.map_or(3.0, |r| r.0);
            visual.with_children(|p| {
                p.spawn(part(
                    &assets.sphere,
                    &assets.plasma,
                    Transform::from_scale(Vec3::splat(0.55)),
                ));
                p.spawn(sprite(
                    &assets,
                    &halo,
                    Transform::from_scale(Vec3::splat(2.4)),
                ));
                p.spawn((
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(assets.orb_field.clone()),
                    Transform::from_scale(Vec3::splat(2.0 * reach)),
                    NotShadowCaster,
                ));
            });
        }
        ItemKind::EmpPulse => {
            let shell = bubbles.add(BubbleMaterial::new(LinearRgba::rgb(0.6, 1.2, 3.0), 1.5));
            visual.insert((
                Mesh3d(assets.sphere.clone()),
                MeshMaterial3d(shell),
                NotShadowCaster,
            ));
        }
        _ => {}
    }
}

/// Moves a projectile visual to its item's pose; returns (position, travel this frame, forward).
fn follow(
    poses: &Poses,
    follows: &Follows,
    fxp: &mut ProjectileFx,
    transform: &mut Transform,
) -> Option<(Vec3, f32)> {
    let pose = poses.of(follows.0)?;
    let pos = pose.translation;
    let travel = fxp.last.map_or(0.0, |l| l.distance(pos));
    if let Some(last) = fxp.last
        && travel > 1e-3
    {
        let dir = (pos - last) / travel;
        fxp.dir = fxp.dir.lerp(dir, 0.5).normalize_or(dir);
    }
    fxp.last = Some(pos);
    transform.translation = pos;
    Some((pos, travel))
}

fn rockets(
    mut fx: ResMut<Fx>,
    poses: Poses,
    mut visuals: Query<(&Follows, &mut ProjectileFx, &mut Transform), With<FxVisual>>,
) {
    for (follows, mut p, mut transform) in &mut visuals {
        if p.kind != ItemKind::SparkRocket {
            continue;
        }
        let Some((pos, travel)) = follow(&poses, follows, &mut p, &mut transform) else {
            continue;
        };
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, p.dir);
        let tail = pos - p.dir * 0.3;
        // Smoke every TRAIL_STEP_U of travel, spread back along this frame's path.
        p.trail += travel / (TRAIL_STEP_U / fx.density.max(0.3));
        let steps = p.trail.floor() as u32;
        p.trail -= steps as f32;
        for k in 0..steps {
            let back = p.dir * travel * k as f32 / steps.max(1) as f32;
            let at = tail - back + fx.rng.dir() * 0.04;
            let v = fx.rng.dir() * 0.4;
            let (life, spin) = (fx.rng.range(0.7, 1.1), fx.rng.angle());
            fx.spawn(Particle::new(SMOKE_TRAIL, at, v, life).spinning(spin, 1.0));
            let spark_v = -p.dir * fx.rng.range(3.0, 7.0) + fx.rng.dir() * 2.0;
            let life = fx.rng.range(0.12, 0.28);
            fx.spawn(Particle::new(EXHAUST_SPARK, tail, spark_v, life));
        }
    }
}

fn orbs(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    mut visuals: Query<(&Follows, &mut ProjectileFx, &mut Transform), With<FxVisual>>,
) {
    let dt = time.delta_secs();
    for (follows, mut p, mut transform) in &mut visuals {
        if p.kind != ItemKind::ShockOrb {
            continue;
        }
        let Some((pos, _)) = follow(&poses, follows, &mut p, &mut transform) else {
            continue;
        };
        p.age += dt;
        transform.scale = Vec3::splat(1.0 + 0.08 * (p.age * 23.0).sin());
        let n = p.emit.count(28.0 * fx.density, dt);
        for _ in 0..n {
            let to = pos + fx.rng.dir() * fx.rng.range(0.5, 1.0);
            zigzag(&mut fx, ORB_ARC, pos, to, 3, 0.12, Vec3::ZERO, 0.06);
        }
    }
}

/// The Leech Coil: an acid-green beam wobbling between owner and target, with charge motes
/// flowing to the owner; sparks around the owner while it still searches.
fn tethers(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    projectiles: Query<(&Projectile, Option<&ItemTarget>)>,
    cars: Query<&CarFx>,
    mut visuals: Query<(&Follows, &mut ProjectileFx), With<FxVisual>>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();
    for (follows, mut p) in &mut visuals {
        if p.kind != ItemKind::LeechCoil {
            continue;
        }
        let Ok((projectile, target)) = projectiles.get(follows.0) else {
            continue;
        };
        let Some(owner) = poses.of(projectile.owner) else {
            continue;
        };
        let roof = |e: Entity| cars.get(e).map_or(0.8, |c| c.shape.roof());
        let from = owner.transform_point(Vec3::Y * roof(projectile.owner));
        let Some(target) = target
            .and_then(|t| t.0)
            .and_then(|t| poses.of(t).map(|pose| (t, pose)))
        else {
            // Searching: sparks orbit the coil.
            let n = p.emit.count(30.0 * fx.density, dt);
            for _ in 0..n {
                let a = fx.rng.angle();
                let v = Vec3::new(a.cos(), fx.rng.range(0.5, 1.5), a.sin()) * 2.0;
                let life = fx.rng.range(0.15, 0.3);
                fx.spawn(Particle::new(MOTE, from, v, life).scaled(0.5));
            }
            continue;
        };
        let to = target.1.transform_point(Vec3::Y * roof(target.0));
        let d = to - from;
        let len = d.length();
        if len < 0.1 {
            continue;
        }
        let side = d.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = side.cross(d / len);
        const SEGMENTS: usize = 12;
        for strand in 0..2 {
            let phase = strand as f32 * std::f32::consts::PI;
            let mut prev = from;
            for k in 1..=SEGMENTS {
                let t = k as f32 / SEGMENTS as f32;
                let sag = (t * std::f32::consts::PI).sin();
                let wobble = (t * 9.0 - now * 14.0 + phase).sin() * 0.18 * sag;
                let next = from
                    + d * t
                    + side * wobble
                    + up * (0.12 * sag * (now * 6.0 + phase).cos())
                    + if k == SEGMENTS {
                        Vec3::ZERO
                    } else {
                        fx.rng.dir() * 0.04
                    };
                let look = Look {
                    size: [0.07 - 0.03 * strand as f32; 2],
                    ..BEAM
                };
                segment(&mut fx, look, prev, next, Vec3::ZERO, dt * 1.6);
                prev = next;
            }
        }
        // Charge flowing from the victim to the leech.
        let n = p.emit.count(26.0 * fx.density, dt);
        for _ in 0..n {
            let v = -d / 0.45;
            let from = to + fx.rng.dir() * 0.15;
            fx.spawn(Particle::new(MOTE, from, v, 0.45));
        }
    }
}

/// EMP wind-up: a blue shell swelling around the user, sparks drawn into it.
fn emp_windup(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    projectiles: Query<(&Projectile, Option<&ItemTimer>)>,
    mut visuals: Query<(&Follows, &mut ProjectileFx, &mut Transform), With<FxVisual>>,
) {
    let dt = time.delta_secs();
    for (follows, mut p, mut transform) in &mut visuals {
        if p.kind != ItemKind::EmpPulse {
            continue;
        }
        let Ok((projectile, timer)) = projectiles.get(follows.0) else {
            continue;
        };
        let Some(owner) = poses.of(projectile.owner) else {
            continue;
        };
        p.age += dt;
        // 0.4 s wind-up (GDD §6.2); the timer says how much is left.
        let left = timer.map_or((0.4 - p.age).max(0.0), |t| t.0);
        let k = 1.0 - (left / 0.4).clamp(0.0, 1.0);
        let centre = owner.translation + owner.rotation * Vec3::Y * 0.5;
        transform.translation = centre;
        transform.scale = Vec3::splat(2.2 + 1.6 * k * k);
        let n = p.emit.count(70.0 * fx.density, dt);
        for _ in 0..n {
            let from = centre + fx.rng.dir() * fx.rng.range(1.5, 2.5);
            let v = (centre - from) / 0.25;
            let look = Look {
                color: [
                    LinearRgba::rgb(3.0, 6.0, 14.0),
                    LinearRgba::rgb(6.0, 9.0, 16.0),
                ],
                ..MOTE
            };
            fx.spawn(Particle::new(look, from, v, 0.25).scaled(0.6));
        }
    }
}
