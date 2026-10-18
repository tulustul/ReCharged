//! Leech Coil (GDD §6.2): a tether entity at its owner searches for up to 3 s for the nearest
//! car ahead in the race (`Standings`) within 40 u, else the nearest car within 20 u, always in
//! sight. Attaching is a hit (a shield blocks it and the tether fizzles). For 3 s it then moves
//! 30 % of the target's charge (as it was at attach time) evenly to its owner
//! (`DrainCharge`/`GrantCharge`) and tows the owner (`TopSpeed` 1.08). It breaks beyond 45 u,
//! without line of sight, or when the target raises a shield.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::data::ItemTuning;
use crate::effects::run_down;
use crate::hits::{Blow, Hits, centre, in_sight};
use crate::pickups::UseItem;

/// The tow is re-applied every tick for this long, so it ends right after the tether (s).
const TOW_REFRESH_S: f32 = 0.1;

/// Charge moved per tick once attached.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Tether {
    pub per_tick: f32,
}

/// A possible Leech Coil target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Candidate {
    pub car: Entity,
    pub centre: Vec3,
    /// Ahead of the user in the race.
    pub ahead: bool,
}

/// The Leech Coil's target: the nearest car ahead within `ahead_range`, else the nearest car
/// within `near_range` in any direction; only cars in sight count.
pub(crate) fn leech_target(
    me: Vec3,
    cars: &[Candidate],
    ahead_range: f32,
    near_range: f32,
    sight: impl Fn(Vec3) -> bool,
) -> Option<Entity> {
    let nearest = |range: f32, only_ahead: bool| {
        cars.iter()
            .filter(|c| (c.ahead || !only_ahead) && c.centre.distance(me) <= range)
            .filter(|c| sight(c.centre))
            .min_by(|a, b| a.centre.distance(me).total_cmp(&b.centre.distance(me)))
            .map(|c| c.car)
    };
    nearest(ahead_range, true).or_else(|| nearest(near_range, false))
}

/// Spawns a searching tether for this tick's [`UseItem`]s.
pub(crate) fn start_leeches(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    mut uses: MessageReader<UseItem>,
    cars: Query<&Transform, With<Racer>>,
    mut teleported: MessageWriter<Teleported>,
) {
    for u in uses.read().filter(|u| u.kind == ItemKind::LeechCoil) {
        let Ok(t) = cars.get(u.racer) else { continue };
        let entity = commands
            .spawn((
                Name::new("leech coil"),
                Projectile {
                    kind: ItemKind::LeechCoil,
                    owner: u.racer,
                },
                ItemTarget(None),
                ItemTimer(tuning.leech_coil.wait_secs),
                Tether::default(),
                Transform::from_translation(centre(t)),
                Interpolated,
                DespawnOnExit(RaceAttempt),
            ))
            .id();
        teleported.write(Teleported { entity });
    }
}

/// Racer cars as the tether systems see them.
type Cars<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform, &'static Charge), (With<Racer>, Without<Tether>)>;

/// Searching tethers ride on their owner, look for a target in sight and attach (a hit), or
/// fizzle when the wait runs out or a shield blocks them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn search_leeches(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    rapier: ReadRapierContext,
    standings: Option<Res<Standings>>,
    mut tethers: Query<(
        Entity,
        &Projectile,
        &mut ItemTarget,
        &mut ItemTimer,
        &mut Tether,
        &mut Transform,
    )>,
    cars: Cars,
    mut hits: Hits,
    mut candidates: Local<Vec<Candidate>>,
) {
    let Ok(ctx) = rapier.single() else { return };
    let l = &tuning.leech_coil;
    let rank = |e: Entity| {
        standings
            .as_ref()
            .and_then(|s| s.0.iter().position(|&x| x == e))
    };
    for (entity, projectile, mut target, mut timer, mut tether, mut t) in &mut tethers {
        let owner = projectile.owner;
        let (None, Ok((_, owner_t, _))) = (target.0, cars.get(owner)) else {
            continue;
        };
        let me = centre(owner_t);
        t.translation = me;
        if !run_down(&mut timer.0) {
            commands.entity(entity).despawn(); // nobody came in range
            continue;
        }
        let mine = rank(owner);
        candidates.clear();
        candidates.extend(
            cars.iter()
                .filter(|&(car, ..)| car != owner)
                .map(|(car, ct, _)| Candidate {
                    car,
                    centre: centre(ct),
                    ahead: matches!((rank(car), mine), (Some(a), Some(b)) if a < b),
                }),
        );
        let sight = |c: Vec3| in_sight(&ctx, me, c);
        let Some(victim) = leech_target(me, &candidates, l.ahead_range, l.near_range, sight) else {
            continue;
        };
        if !hits.land(victim, owner, ItemKind::LeechCoil, entity, Blow::default()) {
            commands.entity(entity).despawn(); // a shield blocked it
            continue;
        }
        let charge = cars.get(victim).map_or(0.0, |(.., c)| c.0);
        target.0 = Some(victim);
        timer.0 = l.secs;
        tether.per_tick = l.share * charge / secs_to_ticks(l.secs).max(1) as f32;
    }
}

/// Attached tethers ride on their owner, move charge and tow every tick, and break beyond the
/// range, without line of sight, when the target raises a shield, or when their time is up.
#[allow(clippy::too_many_arguments)]
pub(crate) fn drain_leeches(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    rapier: ReadRapierContext,
    mut tethers: Query<(
        Entity,
        &Projectile,
        &ItemTarget,
        &mut ItemTimer,
        &Tether,
        &mut Transform,
    )>,
    cars: Cars,
    mut hits: Hits,
    mut grants: MessageWriter<GrantCharge>,
) {
    let Ok(ctx) = rapier.single() else { return };
    let l = &tuning.leech_coil;
    for (entity, projectile, target, mut timer, tether, mut t) in &mut tethers {
        let (owner, Some(victim)) = (projectile.owner, target.0) else {
            continue;
        };
        let (Ok((_, owner_t, _)), Ok((_, victim_t, charge))) = (cars.get(owner), cars.get(victim))
        else {
            commands.entity(entity).despawn();
            continue;
        };
        let (me, them) = (centre(owner_t), centre(victim_t));
        t.translation = me;
        let held = them.distance(me) <= l.break_range && in_sight(&ctx, me, them);
        if !held || hits.shielded(victim) {
            commands.entity(entity).despawn();
            continue;
        }
        let amount = tether.per_tick.min(charge.0);
        if amount > 0.0 {
            hits.drain(victim, amount / charge.0);
            grants.write(GrantCharge { car: owner, amount });
        }
        let tow = Effect::TopSpeed {
            mul: l.tow,
            secs: TOW_REFRESH_S,
        };
        hits.effect(owner, tow, entity);
        if !run_down(&mut timer.0) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn car(i: u32, x: f32, z: f32, ahead: bool) -> Candidate {
        Candidate {
            car: Entity::from_raw_u32(i).expect("entity"),
            centre: Vec3::new(x, 0.0, z),
            ahead,
        }
    }

    #[test]
    fn prefers_the_nearest_car_ahead_then_any_car_nearby() {
        let all = |_| true;
        let cars = [
            car(1, 0.0, -35.0, true),
            car(2, 0.0, -25.0, true),
            car(3, 0.0, 5.0, false), // nearer, but behind
        ];
        let me = Vec3::ZERO;
        assert_eq!(leech_target(me, &cars, 40.0, 20.0, all), Some(cars[1].car));
        // Nobody ahead within 40 u: the nearest within 20 u in any direction.
        let cars = [
            car(1, 0.0, -45.0, true),
            car(3, 0.0, 15.0, false),
            car(4, 18.0, 0.0, false),
        ];
        assert_eq!(leech_target(me, &cars, 40.0, 20.0, all), Some(cars[1].car));
        // Out of sight does not count.
        let hidden = |c: Vec3| c.z != 15.0;
        assert_eq!(
            leech_target(me, &cars, 40.0, 20.0, hidden),
            Some(cars[2].car)
        );
        let lone = [car(5, 0.0, 30.0, false)];
        assert_eq!(leech_target(me, &lone, 40.0, 20.0, all), None);
    }
}
