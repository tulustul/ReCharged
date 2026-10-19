//! Hazard and status visuals: the Goo Bomb riding its carrier (pulsing and blinking faster as the
//! fuse runs out, its blast radius telegraphed on the ground at the end), Oil Slick puddles and
//! Spring Pads drawn at their trigger radius (`ItemRadius`: what you see is what triggers), and
//! the Shield Bubble's burst when it pops (the shield itself is drawn by the car body's shader,
//! rc_render).

use bevy::light::NotShadowCaster;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::assets::VfxAssets;
use crate::cars::{FxVisual, Poses};
use crate::fx::{Fx, sprite};
use crate::items::Follows;
use crate::particles::{Facing, Look, Particle, Shape, pack_tag};
use crate::pool::Emitter;

/// Goo fuse (GDD §6.2) when the item carries no timer.
const GOO_FUSE_S: f32 = 6.0;
/// The Goo Bomb's blast ring shows for the last seconds of the fuse.
const GOO_WARN_S: f32 = 2.0;
/// Oil puddles spread over this long and dry up over the last `OIL_DRY_S` of their life.
const OIL_SPREAD_S: f32 = 0.4;
const OIL_DRY_S: f32 = 1.5;
/// Spring Pad bounce: duration (s) and plate travel (u).
const BOUNCE_S: f32 = 0.6;
const BOUNCE_U: f32 = 0.35;

const GOO_RED: LinearRgba = LinearRgba::rgb(16.0, 1.2, 0.6);
const SHIELD: LinearRgba = LinearRgba::rgb(0.4, 1.3, 2.4);

#[derive(Component, Debug)]
struct GooFx {
    blob: Entity,
    light: Entity,
    ring: Entity,
    drip: Emitter,
    age: f32,
}

#[derive(Component, Debug)]
struct OilFx {
    age: f32,
}

#[derive(Component, Debug)]
struct PadFx {
    plate: Entity,
    coils: [Entity; 3],
    ring: Entity,
    bounce: f32,
}

/// Marker of hazard parts posed by their visual (keeps their transform queries disjoint).
#[derive(Component, Debug)]
struct Part;

pub(crate) fn build(app: &mut App) {
    app.add_observer(spawn_hazard).add_systems(
        Update,
        (goo_bombs, oil_slicks, spring_pads, shields).in_set(crate::VfxSystems::Emit),
    );
}

#[allow(clippy::type_complexity)]
fn spawn_hazard(
    add: On<Add<Hazard>>,
    hazards: Query<(&Hazard, &Transform, Option<&ItemRadius>)>,
    assets: Res<VfxAssets>,
    mut commands: Commands,
) {
    let Ok((hazard, at, radius)) = hazards.get(add.entity) else {
        return;
    };
    let base = (
        Follows(add.entity),
        FxVisual,
        *at,
        Visibility::default(),
        DespawnOnExit(RaceAttempt),
    );
    let mesh = |m: &Handle<Mesh>, mat: &Handle<StandardMaterial>, t: Transform| {
        (
            Part,
            FxVisual,
            Mesh3d(m.clone()),
            MeshMaterial3d(mat.clone()),
            t,
            NotShadowCaster,
        )
    };
    match hazard.kind {
        HazardKind::GooBomb => {
            let blob = commands
                .spawn(mesh(
                    &assets.sphere,
                    &assets.goo,
                    Transform::from_scale(Vec3::new(0.75, 0.55, 0.75)),
                ))
                .id();
            let light_look = Look {
                shape: Shape::Core,
                emissive: true,
                color: [GOO_RED; 2],
                ..Look::DEFAULT
            };
            let light = commands
                .spawn((
                    Part,
                    FxVisual,
                    sprite(
                        &assets,
                        &light_look,
                        Transform::from_xyz(0.0, 0.32, 0.0).with_scale(Vec3::splat(0.5)),
                    ),
                ))
                .id();
            let ring_look = Look {
                shape: Shape::Ring,
                facing: Facing::Flat,
                emissive: true,
                color: [GOO_RED * 0.4; 2],
                ..Look::DEFAULT
            };
            let ring = commands
                .spawn((
                    Part,
                    FxVisual,
                    sprite(&assets, &ring_look, Transform::default()),
                    Visibility::Hidden,
                ))
                .id();
            commands
                .spawn((
                    base,
                    GooFx {
                        blob,
                        light,
                        ring,
                        drip: Emitter::default(),
                        age: 0.0,
                    },
                    Name::new("Goo Bomb visual"),
                ))
                .add_children(&[blob, light]);
            // The blast ring lies on the ground, not on the bomb: a separate root.
            commands
                .entity(ring)
                .insert((Follows(add.entity), DespawnOnExit(RaceAttempt)));
        }
        HazardKind::OilSlick => {
            let r = radius.map_or(2.5, |r| r.0);
            commands
                .spawn((base, OilFx { age: 0.0 }, Name::new("Oil Slick visual")))
                .with_child((
                    Mesh3d(assets.decal.clone()),
                    MeshMaterial3d(assets.oil.clone()),
                    Transform::from_xyz(0.0, 0.015, 0.0).with_scale(Vec3::new(
                        2.0 * r,
                        1.0,
                        2.0 * r,
                    )),
                    NotShadowCaster,
                ));
        }
        HazardKind::SpringPad => {
            let r = radius.map_or(1.6, |r| r.0);
            let s = r / 0.8;
            let plate = commands
                .spawn(mesh(
                    &assets.cylinder,
                    &assets.spring_plate,
                    Transform::from_xyz(0.0, 0.3 * s.min(1.5), 0.0).with_scale(Vec3::new(
                        1.3 * s,
                        0.08,
                        1.3 * s,
                    )),
                ))
                .id();
            let coils = [(); 3].map(|_| {
                commands
                    .spawn(mesh(
                        &assets.torus,
                        &assets.chrome,
                        Transform::from_scale(Vec3::splat(s)),
                    ))
                    .id()
            });
            let ring_look = Look {
                shape: Shape::Ring,
                facing: Facing::Flat,
                emissive: true,
                color: [LinearRgba::rgb(3.0, 2.2, 0.3); 2],
                ..Look::DEFAULT
            };
            let flat = Transform::from_xyz(0.0, 0.02, 0.0)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(2.0 * r));
            let ring = commands
                .spawn((Part, FxVisual, sprite(&assets, &ring_look, flat)))
                .id();
            let base_disc = commands
                .spawn(mesh(
                    &assets.cylinder,
                    &assets.black,
                    Transform::from_xyz(0.0, 0.04, 0.0).with_scale(Vec3::new(
                        1.5 * s,
                        0.08,
                        1.5 * s,
                    )),
                ))
                .id();
            commands
                .spawn((
                    base,
                    PadFx {
                        plate,
                        coils,
                        ring,
                        bounce: BOUNCE_S,
                    },
                    Name::new("Spring Pad visual"),
                ))
                .add_children(&[base_disc, plate, ring])
                .add_children(&coils);
        }
    }
}

/// Seconds between two goo blinks with `left` seconds of fuse (1 s when lit → 0.12 s at the end).
pub fn goo_blink_period(left: f32) -> f32 {
    let t = 1.0 - (left / GOO_FUSE_S).clamp(0.0, 1.0);
    1.0 + (0.12 - 1.0) * t
}

#[allow(clippy::type_complexity)]
fn goo_bombs(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    hazards: Query<(Option<&ItemTimer>, Option<&ItemRadius>)>,
    mut roots: Query<(&Follows, &mut GooFx, &mut Transform), (With<FxVisual>, Without<Part>)>,
    mut tagged: Query<
        (&mut Transform, &mut MeshTag, &mut Visibility),
        (With<Part>, With<FxVisual>),
    >,
    mut blobs: Query<&mut Transform, (With<Part>, With<FxVisual>, Without<MeshTag>)>,
) {
    let dt = time.delta_secs();
    for (follows, mut goo, mut transform) in &mut roots {
        let Ok((timer, radius)) = hazards.get(follows.0) else {
            continue;
        };
        let Some(pose) = poses.of(follows.0) else {
            continue;
        };
        goo.age += dt;
        let left = timer.map_or((GOO_FUSE_S - goo.age).max(0.0), |t| t.0);
        *transform = pose;
        let period = goo_blink_period(left);
        let phase = (goo.age / period).fract();
        let on = phase < 0.35;
        let pulse = 1.0
            + 0.1 * (std::f32::consts::TAU * phase).cos() * (1.0 + 2.0 * (1.0 - left / GOO_FUSE_S));
        if let Ok(mut blob) = blobs.get_mut(goo.blob) {
            blob.scale = Vec3::new(0.75, 0.55, 0.75) * pulse;
        }
        if let Ok((_, mut tag, _)) = tagged.get_mut(goo.light) {
            let c = if on { GOO_RED } else { GOO_RED * 0.08 };
            tag.value = pack_tag(c, true, Shape::Core, Facing::Camera);
        }
        // Telegraph the blast: a red ring of the real radius pulses on the ground at the end.
        if let Ok((mut ring, mut tag, mut vis)) = tagged.get_mut(goo.ring) {
            let warn = left < GOO_WARN_S;
            vis.set_if_neq(if warn {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            if warn {
                let r = radius.map_or(7.0, |r| r.0);
                let up = pose.rotation * Vec3::Y;
                ring.translation = pose.translation - up * 0.85;
                ring.rotation = Quat::from_rotation_arc(Vec3::Z, up);
                ring.scale = Vec3::splat(2.0 * r);
                let k = if fx.calm {
                    0.15
                } else if on {
                    0.6
                } else {
                    0.25
                };
                tag.value = pack_tag(GOO_RED * k, true, Shape::Ring, Facing::Flat);
            }
        }
        let n = goo.drip.count(5.0 * fx.density, dt);
        for _ in 0..n {
            let drop = Look {
                shape: Shape::Drop,
                facing: Facing::Velocity,
                color: [LinearRgba::new(0.15, 0.6, 0.05, 1.0); 2],
                size: [0.08, 0.06],
                gravity: 20.0,
                fade_out: 0.2,
                aspect: 1.4,
                ..Look::DEFAULT
            };
            let at = pose.translation
                + pose.rotation * Vec3::new(fx.rng.signed(0.25), -0.15, fx.rng.signed(0.25));
            fx.spawn(Particle::new(drop, at, Vec3::ZERO, 0.4));
        }
    }
}

fn oil_slicks(
    time: Res<Time>,
    poses: Poses,
    hazards: Query<Option<&ItemTimer>>,
    mut roots: Query<(&Follows, &mut OilFx, &mut Transform), With<FxVisual>>,
) {
    for (follows, mut oil, mut transform) in &mut roots {
        let (Ok(timer), Some(pose)) = (hazards.get(follows.0), poses.of(follows.0)) else {
            continue;
        };
        oil.age += time.delta_secs();
        let spread = (oil.age / OIL_SPREAD_S).clamp(0.0, 1.0);
        let dry = timer.map_or(1.0, |t| (t.0 / OIL_DRY_S).clamp(0.0, 1.0));
        let s = (1.0 - (1.0 - spread).powi(3)) * dry.sqrt();
        *transform = pose.with_scale(Vec3::splat(s.max(0.01)));
    }
}

#[allow(clippy::type_complexity)]
fn spring_pads(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    mut hits: MessageReader<HitEvent>,
    poses: Poses,
    hazards: Query<(&Hazard, Option<&ItemRadius>, Option<&ItemTimer>)>,
    mut roots: Query<(&Follows, &mut PadFx, &mut Transform), (With<FxVisual>, Without<Part>)>,
    mut parts: Query<&mut Transform, (With<Part>, With<FxVisual>)>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();
    let launched: Vec<Entity> = hits
        .read()
        .filter(|h| h.by == ItemKind::SpringPad && !h.blocked)
        .map(|h| h.victim)
        .collect();
    for (follows, mut pad, mut transform) in &mut roots {
        let (Ok((_, radius, timer)), Some(pose)) = (hazards.get(follows.0), poses.of(follows.0))
        else {
            continue;
        };
        // Like a puddle drying up: shrinks away over the last OIL_DRY_S of its timer (a used-up
        // pad gets exactly that long) instead of popping.
        let fade = timer.map_or(1.0, |t| (t.0 / OIL_DRY_S).clamp(0.0, 1.0));
        *transform = pose.with_scale(Vec3::splat(fade.sqrt().max(0.01)));
        let r = radius.map_or(1.6, |r| r.0);
        let near = launched.iter().any(|&car| {
            poses
                .of(car)
                .is_some_and(|c| c.translation.distance(pose.translation) < r + 1.5)
        });
        if near {
            pad.bounce = 0.0;
            let up = pose.rotation * Vec3::Y;
            for _ in 0..fx.n(14) {
                let a = fx.rng.angle();
                let out = pose.rotation * Vec3::new(a.cos(), 0.0, a.sin());
                let look = Look {
                    shape: Shape::Streak,
                    facing: Facing::Velocity,
                    emissive: true,
                    color: [
                        LinearRgba::rgb(8.0, 6.0, 1.0),
                        LinearRgba::rgb(3.0, 1.5, 0.1),
                    ],
                    size: [0.06, 0.03],
                    stretch: 0.05,
                    aspect: 2.0,
                    drag: 3.0,
                    ..Look::DEFAULT
                };
                let v = up * fx.rng.range(5.0, 9.0) + out * fx.rng.range(1.0, 3.0);
                let at = pose.translation + out * r * 0.6;
                fx.spawn(Particle::new(look, at, v, 0.3));
            }
        }
        pad.bounce += dt;
        let s = r / 0.8;
        let k = (pad.bounce / BOUNCE_S).clamp(0.0, 1.0);
        let spring = (-(k * 6.0)).exp() * (k * 18.0).cos() * (1.0 - k);
        let idle = 0.02 * (now * 3.0).sin();
        let lift = (0.3 * s.min(1.5) + BOUNCE_U * spring + idle).max(0.12);
        if let Ok(mut plate) = parts.get_mut(pad.plate) {
            plate.translation.y = lift;
        }
        for (i, &coil) in pad.coils.iter().enumerate() {
            if let Ok(mut c) = parts.get_mut(coil) {
                c.translation.y = 0.08 + lift * (i as f32 + 0.6) / 3.4;
                c.scale = Vec3::new(s * (1.0 - 0.07 * i as f32), s, s * (1.0 - 0.07 * i as f32));
            }
        }
        if let Ok(mut ring) = parts.get_mut(pad.ring) {
            ring.scale = Vec3::splat(2.0 * r * (1.0 + 0.03 * (now * 4.0).sin()));
        }
    }
}

/// A burst where a racer's shield ended (popped by a hit or expired).
fn shields(
    mut fx: ResMut<Fx>,
    poses: Poses,
    racers: Query<(Entity, &StatusEffects), With<Racer>>,
    mut shielded: Local<Vec<Entity>>,
) {
    for (racer, effects) in &racers {
        let had = shielded.contains(&racer);
        if effects.shielded() && !had {
            shielded.push(racer);
        } else if !effects.shielded() && had {
            shielded.retain(|&e| e != racer);
            if let Some(pose) = poses.of(racer) {
                shield_pop(&mut fx, pose.translation + pose.rotation * Vec3::Y * 0.45);
            }
        }
    }
    shielded.retain(|&e| racers.contains(e));
}

fn shield_pop(fx: &mut Fx, at: Vec3) {
    let shard = Look {
        shape: Shape::Streak,
        facing: Facing::Velocity,
        emissive: true,
        color: [SHIELD * 4.0, SHIELD],
        size: [0.06, 0.03],
        stretch: 0.03,
        aspect: 2.0,
        drag: 2.5,
        fade_out: 0.6,
        ..Look::DEFAULT
    };
    for _ in 0..fx.n(36) {
        let dir = fx.rng.dir();
        let v = dir * fx.rng.range(4.0, 9.0);
        let life = fx.rng.range(0.25, 0.45);
        fx.spawn(Particle::new(shard, at + dir * 1.2, v, life));
    }
    let flash = Look {
        shape: Shape::Dot,
        emissive: true,
        color: if fx.calm {
            [SHIELD * 0.4, SHIELD * 0.0]
        } else {
            [SHIELD * 2.5, SHIELD * 0.0]
        },
        size: [2.5, 3.5],
        fade_out: 1.0,
        ..Look::DEFAULT
    };
    fx.spawn(Particle::new(flash, at, Vec3::ZERO, 0.2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goo_beeps_speed_up_toward_the_blast() {
        assert!((goo_blink_period(6.0) - 1.0).abs() < 1e-5);
        assert!((goo_blink_period(0.0) - 0.12).abs() < 1e-5);
        assert!(goo_blink_period(2.0) < goo_blink_period(4.0));
        assert_eq!(goo_blink_period(20.0), 1.0, "clamped");
    }
}
