//! Landing hits (GDD §5.5) and the geometry every item shares.
//!
//! Every hit goes through [`Hits::land`]: a victim whose shield is up at that moment gets a
//! blocked `HitEvent` and nothing else (the effects system pops the shield next tick, so every
//! hit of the same tick is blocked too); otherwise the `HitEvent` and the blow's effect, impulse
//! and charge drain are sent.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::data::Tumble;

/// Height of a car's centre above its origin (the ground contact), along the car's up (u).
pub(crate) const CAR_CENTRE_U: f32 = 0.4;
/// Half a car's length (u): items spawn and drop clear of the bumpers.
pub(crate) const CAR_HALF_LENGTH_U: f32 = 1.0;

/// [`crate::ItemClock`] tick an item entity was spawned at (owner immunity).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Born(pub u64);

/// What a landed hit does to its victim.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Blow {
    /// A timed effect.
    pub effect: Option<Effect>,
    /// (linear, angular) impulse for rc_vehicle.
    pub impulse: Option<(Vec3, Vec3)>,
    /// Share of the victim's charge drained.
    pub drain: f32,
}

impl Blow {
    pub(crate) fn impulse(impulse: (Vec3, Vec3)) -> Self {
        Self {
            impulse: Some(impulse),
            ..default()
        }
    }
}

/// The outputs of hits (and the only writer of `ApplyEffect` in rc_items).
#[derive(SystemParam)]
pub(crate) struct Hits<'w, 's> {
    status: Query<'w, 's, &'static StatusEffects>,
    events: MessageWriter<'w, HitEvent>,
    effects: MessageWriter<'w, ApplyEffect>,
    impulses: MessageWriter<'w, ApplyImpulse>,
    drains: MessageWriter<'w, DrainCharge>,
}

impl Hits<'_, '_> {
    /// Lands `blow` on `victim` unless its shield blocks it; returns whether it landed.
    pub(crate) fn land(
        &mut self,
        victim: Entity,
        attacker: Entity,
        by: ItemKind,
        source: Entity,
        blow: Blow,
    ) -> bool {
        let blocked = self.status.get(victim).is_ok_and(StatusEffects::shielded);
        debug!("items: {by:?} by {attacker} hit {victim} (blocked: {blocked})");
        self.events.write(HitEvent {
            victim,
            attacker: Some(attacker),
            by,
            blocked,
        });
        if blocked {
            return false;
        }
        if let Some(effect) = blow.effect {
            self.effect(victim, effect, source);
        }
        if let Some((linear, angular)) = blow.impulse {
            self.impulses.write(ApplyImpulse {
                car: victim,
                linear,
                angular,
            });
        }
        if blow.drain > 0.0 {
            self.drain(victim, blow.drain);
        }
        true
    }

    /// A timed effect that is not a hit (Turbo Cell, Shield Bubble, the Leech Coil's tow).
    pub(crate) fn effect(&mut self, car: Entity, effect: Effect, source: Entity) {
        self.effects.write(ApplyEffect {
            car,
            effect,
            source: Some(source),
        });
    }

    /// Drains `fraction` of `car`'s charge.
    pub(crate) fn drain(&mut self, car: Entity, fraction: f32) {
        self.drains.write(DrainCharge { car, fraction });
    }

    /// The car's shield is up.
    pub(crate) fn shielded(&self, car: Entity) -> bool {
        self.status.get(car).is_ok_and(StatusEffects::shielded)
    }
}

/// Owner immunity: an item `born` at that tick ignores its owner for `immunity` ticks.
pub(crate) fn immune(car: Entity, owner: Entity, born: u64, now: u64, immunity: u64) -> bool {
    car == owner && now.saturating_sub(born) < immunity
}

/// Impulse of a tumble pushing along `dir` (only its horizontal part; straight up when it has
/// none): Δv `push` along it plus `up`, as the reference car's impulse, spun about the
/// horizontal axis across the push (the car rolls over in the push direction).
pub(crate) fn tumble(t: &Tumble, dir: Vec3) -> (Vec3, Vec3) {
    let flat = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
    let linear = (flat * t.push + Vec3::Y * t.up) * REFERENCE_CAR_MASS;
    let axis = if flat == Vec3::ZERO {
        Vec3::X
    } else {
        Vec3::Y.cross(flat)
    };
    (linear, axis * t.spin)
}

/// Centre of a car (distance checks use it).
pub(crate) fn centre(t: &Transform) -> Vec3 {
    t.translation + t.rotation * Vec3::Y * CAR_CENTRE_U
}

/// `v` without its vertical part, normalised (zero when vertical).
pub(crate) fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z).normalize_or_zero()
}

/// Scene queries against the world only: track pieces and props, never cars or sensors.
pub(crate) fn world_filter() -> QueryFilter<'static> {
    QueryFilter::new()
        .exclude_sensors()
        .groups(CollisionGroups::new(
            Group::from_bits_truncate(layers::PROJECTILE),
            Group::from_bits_truncate(layers::TRACK | layers::PROP),
        ))
}

/// Nothing of the world between `from` and `to`.
pub(crate) fn in_sight(ctx: &RapierContext, from: Vec3, to: Vec3) -> bool {
    let d = to - from;
    let len = d.length();
    len < 1e-3
        || ctx
            .cast_ray(from, d / len, len, true, world_filter())
            .is_none()
}

/// The ground below `from` within `reach`: (point, normal).
pub(crate) fn ground_below(ctx: &RapierContext, from: Vec3, reach: f32) -> Option<(Vec3, Vec3)> {
    let (_, hit) = ctx.cast_ray_and_get_normal(from, Vec3::NEG_Y, reach, true, world_filter())?;
    Some((hit.point, hit.normal))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_is_immune_only_for_the_window_and_only_to_own_items() {
        let (me, other) = (
            Entity::from_raw_u32(1).unwrap(),
            Entity::from_raw_u32(2).unwrap(),
        );
        assert!(immune(me, me, 100, 100, 60));
        assert!(immune(me, me, 100, 159, 60));
        assert!(!immune(me, me, 100, 160, 60));
        assert!(!immune(other, me, 100, 100, 60));
    }

    #[test]
    fn tumble_pushes_lifts_and_rolls_toward_the_push() {
        let t = Tumble {
            up: 8.0,
            push: 6.0,
            spin: 1.5,
        };
        let (linear, angular) = tumble(&t, Vec3::new(3.0, 5.0, 0.0));
        let dv = linear / REFERENCE_CAR_MASS;
        assert!((dv - Vec3::new(6.0, 8.0, 0.0)).length() < 1e-5, "{dv}");
        // Rolling toward +X: the top of the car moves +X, so ω × up points +X.
        assert!(angular.cross(Vec3::Y).x > 0.0, "{angular}");
        let (up_only, _) = tumble(&t, Vec3::Y);
        assert!((up_only / REFERENCE_CAR_MASS - Vec3::Y * 8.0).length() < 1e-5);
    }
}
