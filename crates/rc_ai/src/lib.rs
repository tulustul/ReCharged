//! # rc_ai — AI drivers (L2)
//!
//! - **Responsibilities:** `CarControl` of `AiDriver` racers — pure pursuit on a personal offset
//!   of the racing line, a per-car speed plan × difficulty, overtaking lanes, hazard and static
//!   prop avoidance, drift for charge and boost on straights, item use with reaction delays,
//!   perfect-start timing, stuck recovery, difficulty mistakes, shortcuts that are faster — the
//!   autopilot (an `AiDriver` on a `LocalPlayer` racer), `CatchUp`, and the steering assist
//!   (`SimSet::Assist`: `steer`, plus a gentle auto-brake that only lowers `throttle` / raises
//!   `brake`, for assisted local players).
//! - **Inputs:** `Track` (rc_track: racing line, frames, projection, jumps, walls), racer
//!   `Transform`s, `VehicleTelemetry`, `Charge`, `StatusEffects`, `ItemSlot`, `RaceProgress`,
//!   `Standings`, rc_items' `Hazard` / `Projectile` entities (`ItemTarget` = rocket lock or Goo
//!   carrier, `ItemRadius` = reach), the rapier world (line of sight for rockets and the Leech
//!   Coil), `CarRespawned`, `RaceConfig` (seed, catch-up, mode), `Catalog` (car stars, rival
//!   personalities), `WeatherState`, `Settings` (assist), `assets/data/ai.ron` ([`AiTuning`]).
//! - **Outputs:** `CarControl` of AI racers (and `steer` / the auto-brake of assisted humans),
//!   `CatchUp` of AI rivals, rc_ai's own [`TrackView`] / [`AiBrain`] / [`AssistState`]
//!   components, `LoadingDone("ai")` / `LoadingFailed`.
//! - **Must never:** teleport or push cars, read input devices, or use non-keyed randomness.
//!
//! Design note: `docs/design/rc_ai.md`.

mod assist;
mod brain;
mod catchup;
mod charge;
mod data;
mod driver;
mod items;
mod pace;
mod profile;
mod recovery;
mod scene;
mod shortcut;
mod steering;
mod traffic;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use assist::{AssistState, HANDS_OFF_SECS, MAX_AUTO_BRAKE};
pub use brain::{AiBrain, TrackView};
pub use data::{AI_FILE, AiTuning, LOADING_KEY};
pub use driver::AiClock;
pub use profile::{CarCaps, SpeedProfile};
pub use shortcut::{ShortcutChoice, ShortcutPlan};

/// AI drivers, autopilot and steering assist.
pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        app.init_resource::<AiClock>()
            .add_observer(brain::add_track_view)
            .add_observer(assist::add_assist_state)
            .add_systems(OnEnter(AppState::Loading), data::load_tuning)
            .add_systems(OnEnter(AppState::Unloading), data::unload_tuning)
            .add_systems(OnEnter(RacePhase::Setup), |mut clock: ResMut<AiClock>| {
                *clock = AiClock::default();
            })
            .add_systems(
                FixedUpdate,
                (
                    brain::observe_racers,
                    brain::ensure_brains,
                    catchup::update_catch_up,
                    driver::drive_ai,
                )
                    .chain()
                    .in_set(SimSet::Drivers),
            )
            .add_systems(FixedUpdate, assist::steering_assist.in_set(SimSet::Assist));
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            data::hot_reload.run_if(resource_exists::<data::AiTuningFile>),
        );
    }
}
