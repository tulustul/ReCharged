//! # rc_weather — weather & time of day (L1)
//!
//! - **Responsibilities:** the weather plan from `RaceConfig::weather` and the track's indoor flag
//!   (indoor → outside-only weather), seeded Random/Dynamic presets (keyed RNG), wetness and snow
//!   accumulation/drying, wind gusts, lightning; reset in `OnEnter(RacePhase::Setup)`.
//! - **Inputs:** `RaceConfig`, `Catalog` (indoor flag), race distance (`RaceProgress`).
//! - **Outputs:** `WeatherState` (sole writer), `WeatherEvent`.
//! - **Must never:** render or play sounds, or use randomness other than `rc_core::rng`.
//!
//! Status: skeleton (T01); implemented by T15.

use bevy::prelude::*;

/// Weather simulation.
pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, _app: &mut App) {}
}
