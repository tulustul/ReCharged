//! Spark Rocket (GDD §6.2). Fired forward, or backward with Look back; at launch it locks onto
//! the nearest car in its cone and range that is in sight (`ItemEvent::LockOn`) and homes with a
//! turn-rate limit; the lock breaks (`LockLost`) when the target leaves the cone (sharp turns),
//! the range or the line of sight, and an unlocked rocket flies straight. It skims ramps and
//! banks, detonates on walls and props, on a car near its path or when its life runs out, and
//! tumbles every car in the blast — its owner only after the immunity window.

use bevy::prelude::*;
use bevy_rapier3d::parry::shape::Ball;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::data::ItemTuning;
use crate::effects::{ItemClock, run_down};
use crate::hits::{
    Blow, Born, CAR_HALF_LENGTH_U, Hits, centre, immune, in_sight, tumble, world_filter,
};
use crate::pickups::UseItem;

/// Rockets start this far ahead of (or behind) the car's centre (u).
const LAUNCH_OFFSET_U: f32 = CAR_HALF_LENGTH_U + 0.4;
/// Surfaces whose normal is within 45° of up are ground: rockets skim them; steeper stops them.
const GROUND_NORMAL_Y: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// A skimming rocket keeps this clearance off the surface (u).
const SKIM_U: f32 = 0.05;

/// A rocket in flight.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rocket {
    /// Flight direction (unit).
    pub dir: Vec3,
    /// The lock holds within this range (u).
    pub range: f32,
}

/// `dir` turned toward `to` by at most `max_angle` radians (unit result; aims straight at it
/// when it is closer than that).
pub(crate) fn turn_towards(dir: Vec3, to: Vec3, max_angle: f32) -> Vec3 {
    let want = to.normalize_or(dir);
    if dir.angle_between(want) <= max_angle {
        return want;
    }
    let axis = dir.cross(want).normalize_or(dir.any_orthonormal_vector());
    (Quat::from_axis_angle(axis, max_angle) * dir).normalize()
}

/// `to` (from the rocket to a car) lies within `range` and the cone around `dir`.
pub(crate) fn in_cone(dir: Vec3, to: Vec3, cos_cone: f32, range: f32) -> bool {
    let d = to.length();
    d <= range && d > 1e-3 && to.dot(dir) >= d * cos_cone
}

/// The car a rocket fired from `from` along `dir` locks onto: the nearest car centre in the cone
/// and range that is in sight.
pub(crate) fn lock_target(
    from: Vec3,
    dir: Vec3,
    cos_cone: f32,
    range: f32,
    cars: impl IntoIterator<Item = (Entity, Vec3)>,
    sight: impl Fn(Vec3) -> bool,
) -> Option<Entity> {
    cars.into_iter()
        .filter(|&(_, c)| in_cone(dir, c - from, cos_cone, range) && sight(c))
        .min_by(|a, b| {
            a.1.distance_squared(from)
                .total_cmp(&b.1.distance_squared(from))
        })
        .map(|(e, _)| e)
}

/// Where along `a → b` (0–1) the point closest to `p` lies, and its distance to `p`.
fn closest_on_segment(a: Vec3, b: Vec3, p: Vec3) -> (f32, f32) {
    let ab = b - a;
    let k = ((p - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
    (k, a.lerp(b, k).distance(p))
}

/// Moves a rocket `step` u along `dir` from `p`: it skims ground-like surfaces (`dir` turns
/// parallel to them) and stops at anything steeper. Returns the new position and whether a
/// wall stopped it.
fn advance(ctx: &RapierContext, p: Vec3, dir: &mut Vec3, step: f32, radius: f32) -> (Vec3, bool) {
    let motion = *dir * step;
    let opts = ShapeCastOptions {
        max_time_of_impact: 1.0,
        stop_at_penetration: false,
        ..default()
    };
    let ball = Ball::new(radius);
    let Some((_, hit)) = ctx.cast_shape(p, Quat::IDENTITY, motion, &ball, opts, world_filter())
    else {
        return (p + motion, false);
    };
    let at = p + motion * hit.time_of_impact;
    let normal = hit.details.map_or(Vec3::ZERO, |d| d.normal1);
    if normal.y < GROUND_NORMAL_Y {
        return (at, true);
    }
    *dir = (*dir - normal * dir.dot(normal)).normalize_or(*dir);
    (at + normal * SKIM_U, false)
}

/// Fires the rockets of this tick's [`UseItem`]s.
#[allow(clippy::too_many_arguments)]
pub(crate) fn launch_rockets(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    clock: Res<ItemClock>,
    rapier: ReadRapierContext,
    mut uses: MessageReader<UseItem>,
    cars: Query<(Entity, &Transform), With<Racer>>,
    mut events: MessageWriter<ItemEvent>,
    mut teleported: MessageWriter<Teleported>,
) {
    let r = &tuning.spark_rocket;
    let ctx = rapier.single().ok();
    for u in uses.read().filter(|u| u.kind == ItemKind::SparkRocket) {
        let Ok((_, t)) = cars.get(u.racer) else {
            continue;
        };
        let dir = if u.backward { t.back() } else { t.forward() };
        let start = centre(t) + *dir * LAUNCH_OFFSET_U;
        let range = if u.backward {
            r.lock_range_back
        } else {
            r.lock_range
        };
        let others = cars
            .iter()
            .filter(|&(e, _)| e != u.racer)
            .map(|(e, t)| (e, centre(t)));
        let sight = |c: Vec3| ctx.as_ref().is_none_or(|ctx| in_sight(ctx, start, c));
        let cos_cone = r.lock_cone_deg.to_radians().cos();
        let target = lock_target(start, *dir, cos_cone, range, others, sight);
        let rocket = commands
            .spawn((
                Name::new("spark rocket"),
                Projectile {
                    kind: ItemKind::SparkRocket,
                    owner: u.racer,
                },
                Rocket { dir: *dir, range },
                ItemTarget(target),
                ItemTimer(r.life_secs),
                ItemRadius(r.blast_radius),
                Born(clock.0),
                Transform::from_translation(start).looking_to(dir, Vec3::Y),
                Interpolated,
                DespawnOnExit(RaceAttempt),
            ))
            .id();
        teleported.write(Teleported { entity: rocket });
        if let Some(target) = target {
            events.write(ItemEvent {
                racer: u.racer,
                kind: ItemEventKind::LockOn { target },
            });
        }
    }
}

/// Racer cars as the rocket systems see them.
type Cars<'w, 's> = Query<'w, 's, (Entity, &'static Transform), (With<Racer>, Without<Rocket>)>;

/// Locked rockets turn toward their target, or lose it (`LockLost`) when it leaves the cone,
/// the range or the line of sight.
pub(crate) fn home_rockets(
    tuning: Res<ItemTuning>,
    rapier: ReadRapierContext,
    mut rockets: Query<(&Projectile, &mut Rocket, &mut ItemTarget, &Transform)>,
    cars: Cars,
    mut events: MessageWriter<ItemEvent>,
) {
    let Ok(ctx) = rapier.single() else { return };
    let r = &tuning.spark_rocket;
    let cos_cone = r.lock_cone_deg.to_radians().cos();
    let turn = r.turn_rate_deg.to_radians() * SIM_DT;
    for (projectile, mut rocket, mut target, t) in &mut rockets {
        let Some(locked) = target.0 else { continue };
        let p = t.translation;
        let aim = cars.get(locked).ok().map(|(_, c)| centre(c)).filter(|&a| {
            in_cone(rocket.dir, a - p, cos_cone, rocket.range) && in_sight(&ctx, p, a)
        });
        match aim {
            Some(a) => rocket.dir = turn_towards(rocket.dir, a - p, turn),
            None => {
                target.0 = None;
                events.write(ItemEvent {
                    racer: projectile.owner,
                    kind: ItemEventKind::LockLost,
                });
            }
        }
    }
}

/// Moves the rockets and detonates them on walls, near a car or at the end of their life.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fly_rockets(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    clock: Res<ItemClock>,
    rapier: ReadRapierContext,
    mut rockets: Query<(
        Entity,
        &Projectile,
        &Born,
        &mut Rocket,
        &ItemTarget,
        &mut ItemTimer,
        &mut Transform,
    )>,
    cars: Cars,
    (mut hits, mut events, mut explosions): (
        Hits,
        MessageWriter<ItemEvent>,
        MessageWriter<Explosion>,
    ),
) {
    let Ok(ctx) = rapier.single() else { return };
    let r = &tuning.spark_rocket;
    let immunity = u64::from(secs_to_ticks(tuning.owner_immunity_secs));
    for (entity, projectile, born, mut rocket, target, mut timer, mut t) in &mut rockets {
        let owner = projectile.owner;
        let spared = |car: Entity| immune(car, owner, born.0, clock.0, immunity);
        let p0 = t.translation;
        let (p1, wall) = advance(&ctx, p0, &mut rocket.dir, r.speed * SIM_DT, r.radius);
        let struck = cars
            .iter()
            .filter(|&(car, _)| !spared(car))
            .filter_map(|(_, c)| {
                let (k, d) = closest_on_segment(p0, p1, centre(c));
                (d <= r.hit_radius).then_some(k)
            })
            .min_by(f32::total_cmp);
        let alive = run_down(&mut timer.0);
        let at = match struck {
            Some(k) => p0.lerp(p1, k),
            None if wall || !alive => p1,
            None => {
                *t = Transform::from_translation(p1).looking_to(rocket.dir, Vec3::Y);
                continue;
            }
        };
        explosions.write(Explosion {
            position: at,
            radius: r.blast_radius,
            kind: ItemKind::SparkRocket,
        });
        for (car, c) in &cars {
            if !spared(car) && centre(c).distance(at) <= r.blast_radius {
                let blow = Blow::impulse(tumble(&r.tumble, rocket.dir));
                hits.land(car, owner, ItemKind::SparkRocket, entity, blow);
            }
        }
        if target.0.is_some() {
            events.write(ItemEvent {
                racer: owner,
                kind: ItemEventKind::LockLost,
            });
        }
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homing_turns_at_most_the_rate_limit() {
        let max = 120f32.to_radians() * SIM_DT; // 2° per tick
        let dir = Vec3::NEG_Z;
        for target in [
            Vec3::X,
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::Z,
            Vec3::new(0.0, 1.0, -0.2),
        ] {
            let next = turn_towards(dir, target, max);
            assert!((next.length() - 1.0).abs() < 1e-5);
            assert!((dir.angle_between(next) - max).abs() < 1e-4, "{target}");
            assert!(next.angle_between(target) < dir.angle_between(target));
        }
        // Within the limit it aims straight at the target.
        let near = Vec3::new(0.01, 0.0, -1.0);
        assert!(turn_towards(dir, near, max).angle_between(near) < 1e-5);
        // A target dead behind still turns (about some axis), never NaN.
        assert!(turn_towards(dir, Vec3::Z, max).is_finite());
    }

    #[test]
    fn a_full_turn_takes_the_time_the_rate_allows() {
        // 120 °/s: turning 90° takes 0.75 s = 45 ticks (± float rounding of the last step).
        let max = 120f32.to_radians() * SIM_DT;
        let mut dir = Vec3::NEG_Z;
        let mut ticks = 0;
        while dir.angle_between(Vec3::X) > 1e-4 && ticks < 100 {
            dir = turn_towards(dir, Vec3::X, max);
            ticks += 1;
        }
        assert!((45..=46).contains(&ticks), "{ticks} ticks");
    }

    #[test]
    fn lock_picks_the_nearest_car_in_cone_range_and_sight() {
        let e = |i| Entity::from_raw_u32(i).expect("entity");
        let cos = 35f32.to_radians().cos();
        let cars = [
            (e(1), Vec3::new(0.0, 0.0, -50.0)),  // ahead, far
            (e(2), Vec3::new(10.0, 0.0, -30.0)), // ahead, 18° off: nearest valid
            (e(3), Vec3::new(30.0, 0.0, -20.0)), // 56° off
            (e(4), Vec3::new(0.0, 0.0, 20.0)),   // behind
            (e(5), Vec3::new(0.0, 0.0, -90.0)),  // out of range
        ];
        let all = |_| true;
        assert_eq!(
            lock_target(Vec3::ZERO, Vec3::NEG_Z, cos, 80.0, cars, all),
            Some(e(2))
        );
        let hidden = |c: Vec3| c.x == 0.0; // car 2 behind a wall
        assert_eq!(
            lock_target(Vec3::ZERO, Vec3::NEG_Z, cos, 80.0, cars, hidden),
            Some(e(1))
        );
        // Backward: 60 u behind.
        assert_eq!(
            lock_target(Vec3::ZERO, Vec3::Z, cos, 60.0, cars, all),
            Some(e(4))
        );
        assert_eq!(
            lock_target(Vec3::ZERO, Vec3::NEG_X, cos, 80.0, cars, all),
            None
        );
    }

    #[test]
    fn segment_distance() {
        let (k, d) = closest_on_segment(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -2.0),
            Vec3::new(1.0, 0.0, -1.0),
        );
        assert!((k - 0.5).abs() < 1e-6 && (d - 1.0).abs() < 1e-6);
        let (k, _) = closest_on_segment(Vec3::ZERO, Vec3::ZERO, Vec3::X);
        assert_eq!(k, 0.0);
    }
}
