//! # rc_vfx — visual effects (P)
//!
//! - **Responsibilities:** pooled particles (drift sparks, debris, boost flames, dust, splashes,
//!   explosions, EMP rings, confetti), skid marks, visuals of pickups/projectiles/hazards/golden
//!   battery, per-camera precipitation, lightning, 3D name tags and lock-on markers.
//! - **Inputs:** `VehicleTelemetry`, `Charge`, vehicle/item/race/weather messages, `Pickup` /
//!   `Projectile` / `Hazard` / `GoldenBattery`, `Visual`, `GameCamera`, `WeatherState`, `Settings`.
//! - **Outputs:** visual entities only.
//! - **Must never:** write simulation state; run gameplay visuals while paused.
//!
//! Status: skeleton (T01); implemented by T14.

use bevy::prelude::*;

/// Particles, skid marks and effect visuals.
pub struct VfxPlugin;

impl Plugin for VfxPlugin {
    fn build(&self, _app: &mut App) {}
}
