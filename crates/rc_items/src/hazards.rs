//! Hazards (GDD §6.2). Oil Slick and Spring Pad are dropped behind the car onto the ground below
//! (nothing below = nothing dropped) and live 15 s; a car *entering* one (centre over the disc,
//! near its surface; leaving the disc re-arms it) slips (`Grip` 0.2 + spin) or is launched.
//! The Goo Bomb sticks to its user: a bump (rapier `CollisionEvent::Started` between two racers),
//! or a near miss (the chassis within 0.6 u), passes it on — never to a shielded or ghosted car,
//! never back to the giver within 0.5 s; its carrier drives at 1.1× top speed to catch someone.
//! After the 6 s fuse it tumbles everyone within 7 u that the blast can see (not through walls),
//! the carrier included. A shielded car bumping another pushes it away (Shield Bubble).

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::data::ItemTuning;
use crate::effects::{ItemClock, run_down};
use crate::hits::{
    Blow, Born, CAR_HALF_LENGTH_U, Hits, centre, exposed, flat, ground_below, immune, tumble,
};
use crate::pickups::UseItem;

/// A car origin this far below…
const ZONE_BELOW_U: f32 = 1.0;
/// …to this far above a dropped hazard's surface touches it (u); higher cars fly over it.
const ZONE_ABOVE_U: f32 = 1.5;
/// Dropped hazards are looked for this far above and below the drop point (u).
const DROP_PROBE_U: f32 = 3.0;
/// The Goo Bomb sits this far above its carrier's origin (u).
const GOO_HEIGHT_U: f32 = 0.9;
/// Upward Δv of a Shield Bubble push (u/s).
const SHIELD_PUSH_UP_UPS: f32 = 1.5;
/// A Goo Bomb also passes to a car whose chassis comes this close to the carrier's (u), and its
/// carrier drives at this top-speed multiplier (refreshed every tick for this long, s): with
/// physical bumps only, two thirds of the bombs went off on their users (T19a).
// ponytail: code constants keep `items.ron` readable by older builds; move them into
// `goo_bomb` there with the next format change.
const GOO_PASS_GAP_U: f32 = 0.6;
const GOO_CARRIER_TOP_SPEED: f32 = 1.1;
const GOO_SPEED_REFRESH_S: f32 = 0.1;

/// The cars currently over a dropped hazard's trigger disc (its `ItemRadius`).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub(crate) struct Zone {
    pub inside: Vec<Entity>,
}

/// A Goo Bomb's passing state.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub(crate) struct Goo {
    /// Who passed it to the current carrier, and when (clock tick).
    pub giver: Option<(Entity, u64)>,
    /// The cars within the near-miss gap of the carrier last tick: only a car coming close
    /// takes it (like a bump), not one already alongside.
    pub close: Vec<Entity>,
}

/// The carrier of a Goo Bomb may pass it to `other` at tick `now`.
pub(crate) fn may_pass(
    goo: &Goo,
    other: Entity,
    other_shielded: bool,
    now: u64,
    no_back: u64,
) -> bool {
    let back = goo
        .giver
        .is_some_and(|(giver, at)| giver == other && now.saturating_sub(at) < no_back);
    !other_shielded && !back
}

/// Drops the Oil Slicks and Spring Pads of this tick's [`UseItem`]s behind the car, onto the
/// ground below (nothing below: nothing dropped).
pub(crate) fn drop_hazards(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    clock: Res<ItemClock>,
    rapier: ReadRapierContext,
    mut uses: MessageReader<UseItem>,
    cars: Query<&Transform, With<Racer>>,
    mut teleported: MessageWriter<Teleported>,
) {
    let ctx = rapier.single().ok();
    for u in uses.read() {
        let (kind, name, radius, life) = match u.kind {
            ItemKind::OilSlick => {
                let o = &tuning.oil_slick;
                (HazardKind::OilSlick, "oil slick", o.radius, o.life_secs)
            }
            ItemKind::SpringPad => {
                let s = &tuning.spring_pad;
                (HazardKind::SpringPad, "spring pad", s.radius, s.life_secs)
            }
            _ => continue,
        };
        let Ok(t) = cars.get(u.racer) else { continue };
        let behind = t.translation - flat(*t.forward()) * (CAR_HALF_LENGTH_U + radius + 0.2);
        let probe = behind + Vec3::Y * DROP_PROBE_U;
        let ground = ctx
            .as_ref()
            .and_then(|c| ground_below(c, probe, 2.0 * DROP_PROBE_U));
        let Some((at, normal)) = ground else { continue };
        let pose =
            Transform::from_translation(at).with_rotation(Quat::from_rotation_arc(Vec3::Y, normal));
        let entity = commands
            .spawn((
                Name::new(name),
                Hazard {
                    kind,
                    owner: u.racer,
                },
                Zone::default(),
                ItemRadius(radius),
                Born(clock.0),
                ItemTimer(life),
                pose,
                Interpolated,
                DespawnOnExit(RaceAttempt),
            ))
            .id();
        teleported.write(Teleported { entity });
    }
}

/// Sticks the Goo Bombs of this tick's [`UseItem`]s to their users.
pub(crate) fn attach_goo(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    mut uses: MessageReader<UseItem>,
    cars: Query<&Transform, With<Racer>>,
    mut teleported: MessageWriter<Teleported>,
) {
    for u in uses.read().filter(|u| u.kind == ItemKind::GooBomb) {
        let Ok(t) = cars.get(u.racer) else { continue };
        let entity = commands
            .spawn((
                Name::new("goo bomb"),
                Hazard {
                    kind: HazardKind::GooBomb,
                    owner: u.racer,
                },
                ItemTarget(Some(u.racer)),
                Goo::default(),
                ItemTimer(tuning.goo_bomb.fuse_secs),
                ItemRadius(tuning.goo_bomb.radius),
                riding(t),
                Interpolated,
                DespawnOnExit(RaceAttempt),
            ))
            .id();
        teleported.write(Teleported { entity });
    }
}

/// Pose of a Goo Bomb on a carrier posed at `t`.
fn riding(t: &Transform) -> Transform {
    Transform::from_translation(t.translation + t.rotation * Vec3::Y * GOO_HEIGHT_U)
        .with_rotation(t.rotation)
}

/// What entering a dropped hazard does to `car` (pose `t`, grid index `index`).
fn zone_blow(
    tuning: &ItemTuning,
    kind: HazardKind,
    t: &Transform,
    at: Vec3,
    seed: u64,
    tick: u64,
    index: usize,
) -> Blow {
    match kind {
        HazardKind::OilSlick => {
            let o = &tuning.oil_slick;
            // Spin away from the side the puddle is on.
            let side = if (at - t.translation).dot(*t.right()) >= 0.0 {
                -1.0
            } else {
                1.0
            };
            Blow {
                effect: Some(Effect::Grip {
                    mul: o.grip,
                    secs: o.secs,
                }),
                impulse: Some((Vec3::ZERO, Vec3::Y * o.spin * side)),
                drain: 0.0,
            }
        }
        _ => {
            let s = &tuning.spring_pad;
            let dv = Vec3::Y * s.up + flat(*t.forward()) * s.forward;
            let key = index as u64 * 16;
            let spin = Vec3::new(
                rng::range(seed, tick, RngStream::SpringPadSpin, key, -1.0, 1.0),
                rng::range(seed, tick, RngStream::SpringPadSpin, key + 1, -1.0, 1.0),
                rng::range(seed, tick, RngStream::SpringPadSpin, key + 2, -1.0, 1.0),
            ) * s.spin;
            Blow::impulse((dv * REFERENCE_CAR_MASS, spin))
        }
    }
}

/// Dropped hazards: lifetime, entering cars (owner immunity), re-arming.
pub(crate) fn zones(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    clock: Res<ItemClock>,
    config: Option<Res<RaceConfig>>,
    mut zones: Query<(
        Entity,
        &Hazard,
        &Born,
        &mut Zone,
        &ItemRadius,
        &mut ItemTimer,
        &Transform,
    )>,
    cars: Query<(Entity, &Racer, &Transform)>,
    mut hits: Hits,
) {
    let seed = config.map_or(0, |c| c.seed);
    let immunity = u64::from(secs_to_ticks(tuning.owner_immunity_secs));
    for (entity, hazard, born, mut zone, radius, mut timer, at) in &mut zones {
        if !run_down(&mut timer.0) {
            commands.entity(entity).despawn();
            continue;
        }
        for (car, racer, t) in &cars {
            let local = at.rotation.inverse() * (t.translation - at.translation);
            let over = Vec2::new(local.x, local.z).length() <= radius.0;
            let was = zone.inside.contains(&car);
            if !over {
                zone.inside.retain(|&e| e != car);
                continue;
            }
            if was || !(-ZONE_BELOW_U..=ZONE_ABOVE_U).contains(&local.y) {
                continue;
            }
            zone.inside.push(car);
            if immune(car, hazard.owner, born.0, clock.0, immunity) {
                continue;
            }
            let by = match hazard.kind {
                HazardKind::OilSlick => ItemKind::OilSlick,
                _ => ItemKind::SpringPad,
            };
            let blow = zone_blow(
                &tuning,
                hazard.kind,
                t,
                at.translation,
                seed,
                clock.0,
                racer.index,
            );
            hits.land(car, hazard.owner, by, entity, blow);
        }
    }
}

/// Racer pairs whose contact started (rapier reports collider entities; a car's collider is on
/// its racer entity).
fn bumps<'a>(
    events: impl Iterator<Item = &'a CollisionEvent> + 'a,
    is_racer: impl Fn(Entity) -> bool + 'a,
) -> impl Iterator<Item = (Entity, Entity)> + 'a {
    events.filter_map(move |e| match *e {
        CollisionEvent::Started(a, b, _) if is_racer(a) && is_racer(b) => Some((a, b)),
        _ => None,
    })
}

/// Goo Bombs: passing on bumps, riding on the carrier, the fuse and the explosion.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn goo_bombs(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    clock: Res<ItemClock>,
    rapier: ReadRapierContext,
    mut collisions: MessageReader<CollisionEvent>,
    mut bombs: Query<(
        Entity,
        &mut Hazard,
        &mut ItemTarget,
        &mut ItemTimer,
        &mut Goo,
        &mut Transform,
    )>,
    cars: Query<(Entity, &Transform), (With<Racer>, Without<Goo>)>,
    bodies: Query<&Collider, With<Racer>>,
    (mut hits, mut explosions): (Hits, MessageWriter<Explosion>),
    (mut contacts, mut near): (Local<Vec<(Entity, Entity)>>, Local<Vec<Entity>>),
) {
    let g = &tuning.goo_bomb;
    let ctx = rapier.single().ok();
    contacts.clear();
    contacts.extend(bumps(collisions.read(), |e| cars.contains(e)));
    let no_back = u64::from(secs_to_ticks(g.no_pass_back_secs));
    for (entity, mut hazard, mut carrier, mut fuse, mut goo, mut t) in &mut bombs {
        let Some(holder) = carrier.0 else { continue };
        near.clear();
        if let (Some(ctx), Ok((_, ht)), Ok(body)) = (&ctx, cars.get(holder), bodies.get(holder)) {
            near_cars(ctx, holder, ht, body, GOO_PASS_GAP_U, &mut near);
        }
        let bumped = contacts
            .iter()
            .filter_map(|&(a, b)| match (a == holder, b == holder) {
                (true, _) => Some(b),
                (_, true) => Some(a),
                _ => None,
            });
        let closing = near.iter().copied().filter(|c| !goo.close.contains(c));
        let pass = bumped
            .chain(closing)
            .find(|&other| may_pass(&goo, other, hits.shielded(other), clock.0, no_back));
        goo.close.clone_from(&near);
        if let Some(other) = pass {
            debug!("items: goo passed {holder} -> {other}");
            goo.giver = Some((holder, clock.0));
            hazard.owner = holder;
            carrier.0 = Some(other);
        }
        let Some(Ok((carrying, ct))) = carrier.0.map(|c| cars.get(c)) else {
            commands.entity(entity).despawn();
            continue;
        };
        *t = riding(ct);
        let chase = Effect::TopSpeed {
            mul: GOO_CARRIER_TOP_SPEED,
            secs: GOO_SPEED_REFRESH_S,
        };
        hits.effect(carrying, chase, entity);
        if run_down(&mut fuse.0) {
            continue;
        }
        let at = t.translation;
        debug!(
            "items: goo exploded on {:?} (passed: {})",
            carrier.0,
            goo.giver.is_some()
        );
        explosions.write(Explosion {
            position: at,
            radius: g.radius,
            kind: ItemKind::GooBomb,
        });
        for (car, c) in &cars {
            let to = centre(c) - at;
            if to.length() <= g.radius && exposed(ctx.as_ref(), at, centre(c)) {
                let blow = Blow::impulse(tumble(&g.tumble, to));
                hits.land(car, hazard.owner, ItemKind::GooBomb, entity, blow);
            }
        }
        commands.entity(entity).despawn();
    }
}

/// The racers whose chassis come within `gap` of `car`'s (its `body` grown by `gap`, posed at
/// `t`); ghosted cars (no car contacts) never count.
fn near_cars(
    ctx: &RapierContext,
    car: Entity,
    t: &Transform,
    body: &Collider,
    gap: f32,
    out: &mut Vec<Entity>,
) {
    let Some(grown) = grown(body.as_unscaled_typed_shape(), gap) else {
        return;
    };
    let cars = Group::from_bits_truncate(layers::CAR);
    let filter = QueryFilter::new()
        .exclude_sensors()
        .exclude_collider(car)
        .groups(CollisionGroups::new(cars, cars));
    ctx.intersect_shape(t.translation, t.rotation, &grown, filter, |e, _| {
        out.push(e);
        true
    });
    out.sort(); // deterministic choice among several
}

/// `shape` (a chassis: boxes, rounded or not, possibly in a compound) grown by `by`.
fn grown(shape: ColliderView, by: f32) -> Option<Collider> {
    match shape {
        ColliderView::Cuboid(c) => {
            let h = c.half_extents();
            Some(Collider::round_cuboid(h.x, h.y, h.z, by))
        }
        ColliderView::RoundCuboid(c) => {
            let h = c.inner_shape().half_extents();
            Some(Collider::round_cuboid(
                h.x,
                h.y,
                h.z,
                c.border_radius() + by,
            ))
        }
        ColliderView::Compound(c) => {
            let parts: Vec<_> = c
                .shapes()
                .filter_map(|(at, rot, part)| Some((at, rot, grown(part, by)?)))
                .collect();
            (!parts.is_empty()).then(|| Collider::compound(parts))
        }
        _ => None,
    }
}

/// A car with its shield up pushes the cars it bumps away (not a hit).
pub(crate) fn shield_push(
    tuning: Res<ItemTuning>,
    mut collisions: MessageReader<CollisionEvent>,
    cars: Query<(&Transform, &StatusEffects), With<Racer>>,
    mut impulses: MessageWriter<ApplyImpulse>,
) {
    for (a, b) in bumps(collisions.read(), |e| cars.contains(e)) {
        let (Ok((ta, sa)), Ok((tb, sb))) = (cars.get(a), cars.get(b)) else {
            continue;
        };
        for (pusher, shielded, victim, vt) in
            [(ta, sa.shielded(), b, tb), (tb, sb.shielded(), a, ta)]
        {
            if !shielded {
                continue;
            }
            let away = flat(vt.translation - pusher.translation);
            let dv = away * tuning.shield_bubble.push + Vec3::Y * SHIELD_PUSH_UP_UPS;
            impulses.write(ApplyImpulse {
                car: victim,
                linear: dv * REFERENCE_CAR_MASS,
                angular: Vec3::ZERO,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goo_passes_on_but_not_straight_back_nor_to_shields() {
        let (a, b, c) = (
            Entity::from_raw_u32(1).expect("e"),
            Entity::from_raw_u32(2).expect("e"),
            Entity::from_raw_u32(3).expect("e"),
        );
        let fresh = Goo::default();
        assert!(may_pass(&fresh, b, false, 10, 30));
        assert!(
            !may_pass(&fresh, b, true, 10, 30),
            "shielded cars cannot take it"
        );
        let passed = Goo {
            giver: Some((a, 100)),
            ..Goo::default()
        }; // a gave it to the carrier at tick 100
        assert!(
            !may_pass(&passed, a, false, 129, 30),
            "no pass back within 0.5 s"
        );
        assert!(may_pass(&passed, a, false, 130, 30));
        assert!(may_pass(&passed, c, false, 101, 30), "anyone else at once");
    }
}
