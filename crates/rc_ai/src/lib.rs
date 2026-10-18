//! # rc_ai — AI drivers (L2)
//!
//! - **Responsibilities:** `CarControl` of `AiDriver` racers (pure pursuit on an offset racing
//!   line, speed profile × difficulty, overtaking, hazard avoidance, drift & boost, item use with
//!   reaction delays, perfect-start chance, stuck recovery), autopilot, `CatchUp`, and the steering
//!   assist (`SimSet::Assist`, `steer` only, for assisted local players).
//! - **Inputs:** `Track` (racing line, speed profile), `RaceProgress`, `Standings`,
//!   `VehicleTelemetry`, `ItemSlot`, `StatusEffects`, `Settings`, `RaceConfig`,
//!   `assets/data/ai.ron`.
//! - **Outputs:** `CarControl` (AI racers; `steer` of assisted humans), `CatchUp`.
//! - **Must never:** teleport or push cars, read input devices, or use non-keyed randomness.
//!
//! Status: skeleton (T01); implemented by T11.

use bevy::prelude::*;

/// AI drivers, autopilot and steering assist.
pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, _app: &mut App) {}
}
