//! `SimSet::Effects`: the single writer of `StatusEffects` (GDD §5.5). Every tick: count the
//! timers down, pop the shields that blocked a hit last tick, then stack the new `ApplyEffect`s
//! (from items and race). Also advances [`ItemClock`].

use bevy::prelude::*;
use rc_core::prelude::*;

/// A timer below half a tick has run out: timers count whole ticks (an effect of `secs` is in
/// force for `secs_to_ticks(secs)` ticks).
pub(crate) const EXPIRED_S: f32 = 0.5 * SIM_DT;

/// Ticks since the attempt started (reset in `OnEnter(RacePhase::Setup)`, paused with the
/// simulation): the RNG tick and the age of item entities.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemClock(pub u64);

/// Counts `timer` down one tick; `true` while it still runs (it is set to 0 when it runs out).
pub(crate) fn run_down(timer: &mut f32) -> bool {
    *timer -= SIM_DT;
    if *timer > EXPIRED_S {
        true
    } else {
        *timer = 0.0;
        false
    }
}

fn left(remaining: f32, dt: f32) -> Option<f32> {
    let r = remaining - dt;
    (r > EXPIRED_S).then_some(r)
}

/// Counts every effect down by `dt` and drops the expired ones.
pub fn tick(e: &mut StatusEffects, dt: f32) {
    let mul = |m: Option<TimedMul>| {
        m.and_then(|m| {
            Some(TimedMul {
                remaining: left(m.remaining, dt)?,
                ..m
            })
        })
    };
    e.power_loss = mul(e.power_loss);
    e.grip = mul(e.grip);
    e.top_speed = mul(e.top_speed);
    e.boost = e.boost.and_then(|b| {
        Some(TimedBoost {
            remaining: left(b.remaining, dt)?,
            ..b
        })
    });
    e.shield = e.shield.and_then(|r| left(r, dt));
    e.ghost = e.ghost.and_then(|r| left(r, dt));
}

/// Same kind: the strongest magnitude (farthest from 1) stays, the duration is the longer one.
fn stack_mul(old: Option<TimedMul>, mul: f32, secs: f32) -> TimedMul {
    match old {
        Some(old) => TimedMul {
            mul: if (mul - 1.0).abs() > (old.mul - 1.0).abs() {
                mul
            } else {
                old.mul
            },
            remaining: old.remaining.max(secs),
        },
        None => TimedMul {
            mul,
            remaining: secs,
        },
    }
}

/// Adds `effect` by the stacking rules of GDD §5.5: one effect per kind; the same kind keeps
/// the strongest magnitude with duration = max(remaining, new); boosts never stack (the
/// strongest — top speed, then force — stays); different kinds coexist (the vehicle multiplies
/// them). Effects without a positive duration are ignored.
pub fn stack(e: &mut StatusEffects, effect: Effect) {
    let secs = effect.secs();
    if !(secs > 0.0 && secs.is_finite()) {
        return;
    }
    match effect {
        Effect::PowerLoss { mul, .. } => e.power_loss = Some(stack_mul(e.power_loss, mul, secs)),
        Effect::Grip { mul, .. } => e.grip = Some(stack_mul(e.grip, mul, secs)),
        Effect::TopSpeed { mul, .. } => e.top_speed = Some(stack_mul(e.top_speed, mul, secs)),
        Effect::Boost {
            top_speed,
            force,
            freeze_charge,
            ..
        } => {
            let new = TimedBoost {
                top_speed,
                force,
                freeze_charge,
                remaining: secs,
            };
            e.boost = Some(match e.boost {
                Some(old) => TimedBoost {
                    remaining: old.remaining.max(secs),
                    ..if (old.top_speed, old.force) >= (top_speed, force) {
                        old
                    } else {
                        new
                    }
                },
                None => new,
            });
        }
        Effect::Shield { .. } => e.shield = Some(e.shield.map_or(secs, |r| r.max(secs))),
        Effect::Ghost { .. } => e.ghost = Some(e.ghost.map_or(secs, |r| r.max(secs))),
    }
}

/// `SimSet::Effects`: clock, timers, shield pops, new effects — in that order, so a shield that
/// blocked hits last tick is gone before a new Shield Bubble of this tick goes up.
pub(crate) fn update_effects(
    mut clock: ResMut<ItemClock>,
    mut requests: MessageReader<ApplyEffect>,
    mut hits: MessageReader<HitEvent>,
    mut racers: Query<&mut StatusEffects>,
) {
    clock.0 += 1;
    for mut effects in &mut racers {
        if *effects != StatusEffects::default() {
            tick(&mut effects, SIM_DT);
        }
    }
    // Every hit that met the shield was blocked (HitEvent::blocked); the shield goes now.
    for hit in hits.read().filter(|h| h.blocked) {
        if let Ok(mut effects) = racers.get_mut(hit.victim) {
            effects.shield = None;
        }
    }
    for r in requests.read() {
        if let Ok(mut effects) = racers.get_mut(r.car) {
            stack(&mut effects, r.effect);
        }
    }
}

/// `OnEnter(RacePhase::Setup)`: a new attempt starts at tick 0.
pub(crate) fn reset_clock(mut clock: ResMut<ItemClock>) {
    *clock = ItemClock::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn power(mul: f32, secs: f32) -> Effect {
        Effect::PowerLoss { mul, secs }
    }

    fn boost(top_speed: f32, force: f32, secs: f32, freeze_charge: bool) -> Effect {
        Effect::Boost {
            top_speed,
            force,
            secs,
            freeze_charge,
        }
    }

    #[test]
    fn same_kind_keeps_strongest_magnitude_and_longest_duration() {
        let mut e = StatusEffects::default();
        stack(&mut e, power(0.5, 3.0));
        stack(&mut e, power(0.0, 1.0)); // stronger, shorter
        assert_eq!(e.power_loss.map(|p| (p.mul, p.remaining)), Some((0.0, 3.0)));
        stack(&mut e, power(0.7, 5.0)); // weaker, longer
        assert_eq!(e.power_loss.map(|p| (p.mul, p.remaining)), Some((0.0, 5.0)));
        // Above-1 multipliers: the one farthest from 1 wins.
        stack(
            &mut e,
            Effect::TopSpeed {
                mul: 1.05,
                secs: 1.0,
            },
        );
        stack(
            &mut e,
            Effect::TopSpeed {
                mul: 1.08,
                secs: 0.5,
            },
        );
        assert_eq!(e.top_speed.map(|t| (t.mul, t.remaining)), Some((1.08, 1.0)));
    }

    #[test]
    fn different_kinds_coexist() {
        let mut e = StatusEffects::default();
        stack(&mut e, power(0.0, 2.0));
        stack(
            &mut e,
            Effect::Grip {
                mul: 0.2,
                secs: 1.2,
            },
        );
        stack(&mut e, Effect::Shield { secs: 10.0 });
        stack(&mut e, Effect::Ghost { secs: 1.5 });
        assert_eq!((e.power_mul(), e.grip_mul()), (0.0, 0.2));
        assert!(e.shielded() && e.ghosted());
    }

    #[test]
    fn boosts_never_stack_the_strongest_wins() {
        let mut e = StatusEffects::default();
        stack(&mut e, boost(1.30, 1.50, 1.0, false)); // perfect start
        stack(&mut e, boost(1.45, 1.80, 2.5, true)); // Turbo Cell
        let b = e.boost().expect("boost");
        assert_eq!((b.top_speed, b.force, b.freeze_charge), (1.45, 1.80, true));
        assert_eq!(b.remaining, 2.5);
        stack(&mut e, boost(1.30, 1.50, 3.0, false)); // weaker but longer: extends only
        let b = e.boost().expect("boost");
        assert_eq!(
            (b.top_speed, b.remaining, b.freeze_charge),
            (1.45, 3.0, true)
        );
    }

    #[test]
    fn timers_count_whole_ticks() {
        for (secs, ticks) in [(1.2, 72), (1.0, 60), (SIM_DT, 1), (0.25, 15)] {
            let mut e = StatusEffects::default();
            stack(&mut e, Effect::Grip { mul: 0.2, secs });
            let mut active = 0;
            while e.grip.is_some() {
                active += 1; // the vehicle sees it this tick
                tick(&mut e, SIM_DT);
                assert!(active < 1000);
            }
            assert_eq!(active, ticks, "{secs} s");
        }
    }

    #[test]
    fn shield_and_ghost_keep_the_longer_duration() {
        let mut e = StatusEffects::default();
        stack(&mut e, Effect::Shield { secs: 10.0 });
        for _ in 0..120 {
            tick(&mut e, SIM_DT);
        }
        stack(&mut e, Effect::Shield { secs: 5.0 });
        assert!((e.shield.expect("shield") - 8.0).abs() < 1e-3);
        stack(&mut e, Effect::Shield { secs: 9.0 });
        assert_eq!(e.shield, Some(9.0));
    }

    #[test]
    fn non_positive_durations_are_ignored() {
        let mut e = StatusEffects::default();
        stack(&mut e, power(0.0, 0.0));
        stack(&mut e, power(0.0, -1.0));
        stack(&mut e, power(0.0, f32::NAN));
        assert_eq!(e, StatusEffects::default());
    }

    #[test]
    fn run_down_reaches_zero_in_whole_ticks() {
        let mut t = 0.4;
        let mut ticks = 1;
        while run_down(&mut t) {
            ticks += 1;
        }
        assert_eq!((ticks, t), (24, 0.0));
    }
}
