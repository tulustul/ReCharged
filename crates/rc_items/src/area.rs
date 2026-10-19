//! Area items (GDD §6.2). EMP Pulse: a wind-up entity rides on its user for 0.4 s, then a 12 u
//! pulse hits every other car it can see — no wall, ribbon or prop in between — (`PowerLoss(0)`
//! 2 s, steering still works, and half its charge). Storm Strike: every unfinished car ahead of
//! the user in `Standings` is struck, leader first, one every 0.25 s from 0.25 s after the use
//! (the `Used` event is the warning; `PowerLoss(0)` 1.5 s + a hop); a car under cover (a ribbon
//! or prop above it: an overpass, a table, the dollhouse) is spared and the next one struck.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::data::ItemTuning;
use crate::effects::run_down;
use crate::hits::{Blow, Hits, centre, exposed};
use crate::pickups::UseItem;

/// A Storm Strike bolt comes down from this high above a car (u): anything of the world in
/// between shelters it.
const STORM_DROP_U: f32 = 40.0;

/// An EMP winding up (with `Projectile { kind: EmpPulse }` and the wind-up in `ItemTimer`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Emp;

/// A Storm Strike working through its targets.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) struct Storm {
    pub owner: Entity,
    /// Cars still to strike, in order.
    pub targets: Vec<Entity>,
    /// Seconds until the next strike.
    pub wait: f32,
}

/// The cars a Storm Strike of `user` hits: everyone ahead in the standings who has not finished.
pub(crate) fn storm_targets(
    user: Entity,
    standings: &[Entity],
    finished: impl Fn(Entity) -> bool,
) -> Vec<Entity> {
    let Some(rank) = standings.iter().position(|&e| e == user) else {
        return Vec::new();
    };
    standings[..rank]
        .iter()
        .copied()
        .filter(|&e| !finished(e))
        .collect()
}

/// Starts the EMPs and Storm Strikes of this tick's [`UseItem`]s.
pub(crate) fn start_area_items(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    standings: Option<Res<Standings>>,
    mut uses: MessageReader<UseItem>,
    cars: Query<(&Transform, &RaceProgress), With<Racer>>,
    mut teleported: MessageWriter<Teleported>,
) {
    for u in uses.read() {
        match u.kind {
            ItemKind::EmpPulse => {
                let Ok((t, _)) = cars.get(u.racer) else {
                    continue;
                };
                let entity = commands
                    .spawn((
                        Name::new("emp pulse"),
                        Projectile {
                            kind: ItemKind::EmpPulse,
                            owner: u.racer,
                        },
                        Emp,
                        ItemTimer(tuning.emp_pulse.windup_secs),
                        ItemRadius(tuning.emp_pulse.radius),
                        Transform::from_translation(centre(t)),
                        Interpolated,
                        DespawnOnExit(RaceAttempt),
                    ))
                    .id();
                teleported.write(Teleported { entity });
            }
            ItemKind::StormStrike => {
                let order = standings.as_ref().map_or(&[][..], |s| &s.0[..]);
                let finished = |e| cars.get(e).is_ok_and(|(_, p)| p.finished);
                commands.spawn((
                    Name::new("storm strike"),
                    Storm {
                        owner: u.racer,
                        targets: storm_targets(u.racer, order, finished),
                        wait: tuning.storm_strike.interval_secs,
                    },
                    DespawnOnExit(RaceAttempt),
                ));
            }
            _ => {}
        }
    }
}

/// EMPs follow their user through the wind-up, then pulse.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn pulse_emps(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    rapier: ReadRapierContext,
    mut emps: Query<(Entity, &Projectile, &mut ItemTimer, &mut Transform), With<Emp>>,
    cars: Query<(Entity, &Transform), (With<Racer>, Without<Emp>)>,
    mut hits: Hits,
    mut explosions: MessageWriter<Explosion>,
) {
    let e = &tuning.emp_pulse;
    let ctx = rapier.single().ok();
    for (entity, p, mut timer, mut t) in &mut emps {
        if let Ok((_, owner)) = cars.get(p.owner) {
            t.translation = centre(owner);
        }
        if run_down(&mut timer.0) {
            continue;
        }
        let at = t.translation;
        explosions.write(Explosion {
            position: at,
            radius: e.radius,
            kind: ItemKind::EmpPulse,
        });
        for (car, c) in &cars {
            let hit = centre(c);
            if car != p.owner && hit.distance(at) <= e.radius && exposed(ctx.as_ref(), at, hit) {
                let blow = Blow {
                    effect: Some(Effect::PowerLoss {
                        mul: e.power,
                        secs: e.secs,
                    }),
                    impulse: None,
                    drain: e.drain,
                };
                hits.land(car, p.owner, ItemKind::EmpPulse, entity, blow);
            }
        }
        commands.entity(entity).despawn();
    }
}

/// Nothing of the world above the car centred at `at` (the sky is open for a bolt).
fn under_open_sky(ctx: Option<&RapierContext>, at: Vec3) -> bool {
    exposed(ctx, at + Vec3::Y * STORM_DROP_U, at)
}

/// Storm Strikes hit their next target every interval.
pub(crate) fn run_storms(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    rapier: ReadRapierContext,
    mut storms: Query<(Entity, &mut Storm)>,
    cars: Query<&Transform, With<Racer>>,
    mut hits: Hits,
) {
    let s = &tuning.storm_strike;
    let ctx = rapier.single().ok();
    for (entity, mut storm) in &mut storms {
        if storm.wait > 0.0 && run_down(&mut storm.wait) {
            continue;
        }
        let Some(&victim) = storm.targets.first() else {
            commands.entity(entity).despawn();
            continue;
        };
        storm.targets.remove(0);
        let open = cars
            .get(victim)
            .is_ok_and(|t| under_open_sky(ctx.as_ref(), centre(t)));
        if !open {
            continue; // gone or under cover: strike the next one now
        }
        storm.wait = s.interval_secs;
        let blow = Blow {
            effect: Some(Effect::PowerLoss {
                mul: s.power,
                secs: s.secs,
            }),
            impulse: Some((Vec3::Y * s.hop * REFERENCE_CAR_MASS, Vec3::ZERO)),
            drain: 0.0,
        };
        hits.land(victim, storm.owner, ItemKind::StormStrike, entity, blow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storm_strikes_only_unfinished_cars_ahead_leader_first() {
        let e = |i| Entity::from_raw_u32(i).expect("entity");
        let order = [e(1), e(2), e(3), e(4), e(5)];
        let none = |_| false;
        assert_eq!(storm_targets(e(4), &order, none), vec![e(1), e(2), e(3)]);
        assert_eq!(storm_targets(e(4), &order, |x| x == e(1)), vec![e(2), e(3)]);
        assert!(
            storm_targets(e(1), &order, none).is_empty(),
            "the leader hits nobody"
        );
        assert!(
            storm_targets(e(9), &order, none).is_empty(),
            "unranked: nobody"
        );
    }
}
