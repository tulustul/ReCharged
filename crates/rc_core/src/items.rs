//! Item contracts: kinds, status effects as data (GDD §5.5, §6), item entities and messages
//! (ARCHITECTURE §4.3–§4.5). The effects *system* (stacking, timers, shields) is rc_items'.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// An item a racer can hold (GDD §6.2). Order = columns of the odds table (GDD §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    /// Instant boost.
    TurboCell,
    /// Homing rocket.
    SparkRocket,
    /// Charge-stealing tether.
    LeechCoil,
    /// Track-following plasma ball.
    ShockOrb,
    /// Radius power-loss pulse.
    EmpPulse,
    /// Pass-it-on bomb.
    GooBomb,
    /// Slippery puddle dropped behind.
    OilSlick,
    /// Launch pad dropped behind.
    SpringPad,
    /// 10 s shield.
    ShieldBubble,
    /// Lightning on every car ahead (positions 6–8 only).
    StormStrike,
}

impl ItemKind {
    /// Every item in odds-table order.
    pub const ALL: [ItemKind; 10] = [
        Self::TurboCell,
        Self::SparkRocket,
        Self::LeechCoil,
        Self::ShockOrb,
        Self::EmpPulse,
        Self::GooBomb,
        Self::OilSlick,
        Self::SpringPad,
        Self::ShieldBubble,
        Self::StormStrike,
    ];
}

/// Kinds of hazard entities left on or attached to the track/cars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HazardKind {
    /// Oil puddle (grip loss + spin).
    OilSlick,
    /// Spring pad (launch).
    SpringPad,
    /// Goo Bomb stuck to a car.
    GooBomb,
}

/// A timed status effect (GDD §5.5). Tumbles are not effects: they are `ApplyImpulse`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Drive-force multiplier (EMP, Storm Strike: 0; stall).
    PowerLoss {
        /// Drive-force multiplier.
        mul: f32,
        /// Duration in seconds.
        secs: f32,
    },
    /// Grip multiplier (Oil Slick: 0.2).
    Grip {
        /// Grip multiplier.
        mul: f32,
        /// Duration in seconds.
        secs: f32,
    },
    /// Top-speed multiplier (Leech Coil tow: 1.08).
    TopSpeed {
        /// Top-speed multiplier.
        mul: f32,
        /// Duration in seconds.
        secs: f32,
    },
    /// Boost (Turbo Cell 1.45/1.80, perfect start 1.30/1.50); boosts never stack.
    Boost {
        /// Top-speed multiplier.
        top_speed: f32,
        /// Drive-force multiplier.
        force: f32,
        /// Duration in seconds.
        secs: f32,
        /// Charge is not drained while this boost is active (Turbo Cell).
        freeze_charge: bool,
    },
    /// Blocks hits (Shield Bubble).
    Shield {
        /// Duration in seconds.
        secs: f32,
    },
    /// No car–car collision, blinking (after a respawn).
    Ghost {
        /// Duration in seconds.
        secs: f32,
    },
}

impl Effect {
    /// Duration in seconds.
    pub fn secs(&self) -> f32 {
        match *self {
            Self::PowerLoss { secs, .. }
            | Self::Grip { secs, .. }
            | Self::TopSpeed { secs, .. }
            | Self::Boost { secs, .. }
            | Self::Shield { secs }
            | Self::Ghost { secs } => secs,
        }
    }
}

/// An active multiplier effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedMul {
    /// Multiplier.
    pub mul: f32,
    /// Seconds left.
    pub remaining: f32,
}

/// The active boost effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedBoost {
    /// Top-speed multiplier.
    pub top_speed: f32,
    /// Drive-force multiplier.
    pub force: f32,
    /// Charge drain frozen (Turbo Cell).
    pub freeze_charge: bool,
    /// Seconds left.
    pub remaining: f32,
}

/// Active effects of a racer, at most one per kind. Written only by rc_items (`SimSet::Effects`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct StatusEffects {
    /// Drive-force loss.
    pub power_loss: Option<TimedMul>,
    /// Grip change.
    pub grip: Option<TimedMul>,
    /// Top-speed change.
    pub top_speed: Option<TimedMul>,
    /// Boost.
    pub boost: Option<TimedBoost>,
    /// Shield: seconds left.
    pub shield: Option<f32>,
    /// Ghost: seconds left.
    pub ghost: Option<f32>,
}

impl StatusEffects {
    /// Drive-force multiplier from power loss (1 when none).
    pub fn power_mul(&self) -> f32 {
        self.power_loss.map_or(1.0, |e| e.mul)
    }

    /// Grip multiplier (1 when none).
    pub fn grip_mul(&self) -> f32 {
        self.grip.map_or(1.0, |e| e.mul)
    }

    /// Top-speed multiplier from non-boost effects (1 when none).
    pub fn top_speed_mul(&self) -> f32 {
        self.top_speed.map_or(1.0, |e| e.mul)
    }

    /// The active boost, if any (the vehicle picks the strongest of this and the charge boost).
    pub fn boost(&self) -> Option<TimedBoost> {
        self.boost
    }

    /// A shield is up.
    pub fn shielded(&self) -> bool {
        self.shield.is_some()
    }

    /// Ghosted: no car–car collision.
    pub fn ghosted(&self) -> bool {
        self.ghost.is_some()
    }
}

/// The item a racer holds. Written only by rc_items.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct ItemSlot {
    /// Held item (granted when the roulette ends).
    pub item: Option<ItemKind>,
    /// Seconds of roulette left; the item is usable at 0.
    pub roulette: f32,
}

/// A Volt Cube pickup (attempt-scoped, spawned by rc_items).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pickup {
    /// Item row index along the lap.
    pub row: u8,
    /// Cube index within the row.
    pub slot: u8,
}

/// A flying item (attempt-scoped, also `Interpolated`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Projectile {
    /// What it is.
    pub kind: ItemKind,
    /// The racer who fired it.
    pub owner: Entity,
}

/// A hazard left behind or attached (attempt-scoped).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hazard {
    /// What it is.
    pub kind: HazardKind,
    /// The racer who dropped it.
    pub owner: Entity,
}

/// Request to apply a status effect (producers items, race; consumer the rc_items effects system).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ApplyEffect {
    /// Target car.
    pub car: Entity,
    /// The effect.
    pub effect: Effect,
    /// What caused it (projectile, hazard, racer), if anything.
    pub source: Option<Entity>,
}

/// Item lifecycle notifications for HUD, audio, VFX and AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemEventKind {
    /// Drove through a pickup; the roulette starts.
    Picked,
    /// The roulette ended with this item.
    Ready(ItemKind),
    /// The item was used.
    Used(ItemKind),
    /// A rocket locked onto `target`.
    LockOn {
        /// The locked car.
        target: Entity,
    },
    /// The lock was lost.
    LockLost,
}

/// An item happening of one racer (producer items).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemEvent {
    /// The racer the event belongs to.
    pub racer: Entity,
    /// What happened.
    pub kind: ItemEventKind,
}

/// A car was hit by an item (producer items).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitEvent {
    /// The car that was hit.
    pub victim: Entity,
    /// Who caused it, if known.
    pub attacker: Option<Entity>,
    /// The item.
    pub by: ItemKind,
    /// A shield blocked it.
    pub blocked: bool,
}

/// An explosion or pulse (producer items; consumers vfx, audio, camera).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct Explosion {
    /// Centre in world space.
    pub position: Vec3,
    /// Radius in u.
    pub radius: f32,
    /// The item that exploded.
    pub kind: ItemKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_effects_are_neutral() {
        let e = StatusEffects::default();
        assert_eq!(
            (e.power_mul(), e.grip_mul(), e.top_speed_mul()),
            (1.0, 1.0, 1.0)
        );
        assert!(e.boost().is_none() && !e.shielded() && !e.ghosted());
    }

    #[test]
    fn getters_report_active_effects() {
        let e = StatusEffects {
            power_loss: Some(TimedMul {
                mul: 0.0,
                remaining: 2.0,
            }),
            grip: Some(TimedMul {
                mul: 0.2,
                remaining: 1.2,
            }),
            shield: Some(10.0),
            ..default()
        };
        assert_eq!((e.power_mul(), e.grip_mul()), (0.0, 0.2));
        assert!(e.shielded() && !e.ghosted());
        assert_eq!(Effect::Ghost { secs: 1.5 }.secs(), 1.5);
    }
}
