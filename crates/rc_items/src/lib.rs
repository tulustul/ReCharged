//! # rc_items — pickups, weapons and status effects (L2)
//!
//! - **Responsibilities:** pickup rows, roulette and odds (`assets/data/items.ron`, band formula
//!   GDD §6.3), item use, projectiles (kinematic + shape casts), hazards; the sole writer of
//!   `StatusEffects` (stacking, shield, immunity rules of GDD §5.5).
//! - **Inputs:** `CarControl::use_item` / `look_back`, `Standings`, `RaceProgress`, `Track`,
//!   `ApplyEffect` (from items and race).
//! - **Outputs:** `Pickup` / `Projectile` / `Hazard` entities (attempt-scoped), `ItemSlot`,
//!   `StatusEffects`, `ItemEvent`, `HitEvent`, `Explosion`, `ApplyImpulse`, `DrainCharge`,
//!   `GrantCharge`, `Teleported`.
//! - **Must never:** write car velocities or transforms (it sends `ApplyImpulse`).
//!
//! Status: skeleton (T01); implemented by T12.

use bevy::prelude::*;

/// Items, projectiles, hazards and status effects.
pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, _app: &mut App) {}
}
