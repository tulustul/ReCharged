//! # rc_items — pickups, weapons and status effects (L2)
//!
//! - **Responsibilities:** Volt Cubes from the track's item rows (attempt-scoped), the roulette
//!   and odds (`assets/data/items.ron`, band formula GDD §6.3, keyed RNG), item use, the ten items
//!   of GDD §6.2 (projectiles move kinematically with rapier shape casts; the Shock Orb follows
//!   the track), hazards; the sole writer of `StatusEffects` (stacking, shields, timers of GDD
//!   §5.5) and of `ItemSlot`.
//! - **Inputs:** `CarControl::{use_item, look_back}`, racer `Transform`s, `Charge`,
//!   `RaceProgress::finished`, `Standings`, `RaceConfig::{items, seed}`, `Track` (item rows, orb
//!   paths), rapier scene queries (walls, sight, ground) and `CollisionEvent`s (bumps),
//!   `ApplyEffect` (from items and race), `UserPaths`.
//! - **Outputs:** `Pickup` / `Projectile` / `Hazard` entities with `ItemTarget` / `ItemTimer` /
//!   `ItemRadius` (attempt-scoped, `Interpolated` + `Teleported` at spawn; a used-up dropped
//!   hazard gets `Spent` and fades out), `ItemSlot`,
//!   `StatusEffects`, `ItemEvent`, `HitEvent`, `Explosion`, `ApplyImpulse` (Δv ×
//!   `REFERENCE_CAR_MASS`; rc_vehicle scales by weight), `ApplyEffect` (own effects, consumed
//!   here), `DrainCharge`, `GrantCharge`, `LoadingDone("items")` / `LoadingFailed`.
//! - **Must never:** write car velocities or transforms (it sends `ApplyImpulse`), read input
//!   devices, or draw randomness outside `rc_core::rng`.
//!
//! Every system runs in `FixedUpdate`: `SimSet::Effects` (one system) and `SimSet::Items` (one
//! chain, which owns hit detection too — it reads the poses of the latest physics step, so no
//! rc_items system shares a set with rc_vehicle). Design note: `docs/design/rc_items.md`.

mod area;
mod data;
mod effects;
mod hazards;
mod hits;
mod leech;
mod odds;
mod orb;
mod pickups;
mod rocket;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use data::*;
pub use effects::{ItemClock, stack, tick};
pub use odds::{Band, Odds, OddsRow};

/// Items, projectiles, hazards and status effects.
pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        app.add_message::<pickups::UseItem>()
            .init_resource::<ItemClock>()
            .add_systems(OnEnter(AppState::Loading), data::load_tuning)
            .add_systems(OnEnter(AppState::Unloading), data::unload_tuning)
            .add_systems(OnEnter(RacePhase::Setup), effects::reset_clock)
            .add_systems(OnEnter(RaceAttempt), pickups::spawn_pickups)
            .add_systems(FixedUpdate, effects::update_effects.in_set(SimSet::Effects))
            .add_systems(
                FixedUpdate,
                (
                    pickups::roulette,
                    (pickups::take_pickups, pickups::use_items)
                        .chain()
                        .run_if(in_state(RacePhase::Racing)),
                    // Items already out go first, so a new item's clock starts the tick after
                    // its use (a 6 s fuse is exactly 360 ticks).
                    (
                        leech::search_leeches,
                        leech::drain_leeches,
                        area::pulse_emps,
                        area::run_storms,
                        rocket::home_rockets,
                        rocket::fly_rockets,
                        orb::roll_orbs,
                        hazards::zones,
                        hazards::goo_bombs,
                        hazards::shield_push,
                    )
                        .chain(),
                    (
                        pickups::self_items,
                        rocket::launch_rockets,
                        orb::launch_orbs,
                        leech::start_leeches,
                        area::start_area_items,
                        hazards::drop_hazards,
                        hazards::attach_goo,
                    )
                        .chain(),
                )
                    .chain()
                    .in_set(SimSet::Items)
                    .run_if(resource_exists::<ItemTuning>),
            );
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            data::hot_reload.run_if(resource_exists::<ItemTuning>),
        );
    }
}
