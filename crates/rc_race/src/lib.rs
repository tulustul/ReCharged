//! # rc_race — race rules (L2)
//!
//! - **Responsibilities:** spawning racers on the grid in `OnEnter(RaceAttempt)` (attempt-scoped,
//!   `Interpolated`), `GoldenBattery` and `GhostCar`; phases Intro → Countdown → Racing → Finished;
//!   perfect start / stall; progress, checkpoints, laps (in ticks), standings, wrong way, soft-lock
//!   guard; the sole decider of resets and respawns (`RespawnCar`); ghost recording (20 Hz);
//!   `RaceResults`; onboarding hints.
//! - **Inputs:** `RaceConfig`, `Track` (rc_track), `CarControl::reset_held`, `VehicleTelemetry`,
//!   the session lifecycle states.
//! - **Outputs:** racer entities (`Racer`, `LocalPlayer`, `AiDriver`), `RaceProgress`,
//!   `RaceStatus`, `Standings`, `RaceResults`, `RaceEvent`, `RespawnCar`, `ApplyEffect`,
//!   `Teleported`.
//! - **Must never:** move cars itself (it requests `RespawnCar`), read input devices, or spawn
//!   attempt entities in `OnEnter(RacePhase::Setup)`.
//!
//! Status: skeleton (T01; rc_game drives a placeholder phase sequence); implemented by T06.

use bevy::prelude::*;

/// Race rules and racer spawning.
pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, _app: &mut App) {}
}
