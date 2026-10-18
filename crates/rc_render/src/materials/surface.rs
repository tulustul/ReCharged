//! [`SurfaceMaterial`]: a `StandardMaterial` extended in world space
//! (`assets/shaders/render/surface.wesl`):
//! - **macro variation** — low-frequency noise modulates albedo and roughness so 2–4 u texture
//!   repeats do not read as a grid (art bible §3.1 "two-scale detail");
//! - **wetness** — darker, glossier surfaces and flat, mirror-like puddles on level ground
//!   (art bible §1.1: wet roughness × 0.35, albedo × 0.7);
//! - **snow cover** — white, rough cover on up-facing surfaces, broken up by noise.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Frequency of the macro variation noise (1/u): blotches of ≈ 12 u.
const VARIATION_FREQ: f32 = 1.0 / 12.0;
/// Roughness variation relative to the albedo variation strength.
const ROUGHNESS_VARIATION: f32 = 0.6;

/// A world-space surface material (environment ground, ribbons, walls).
pub type SurfaceMaterial = ExtendedMaterial<StandardMaterial, SurfaceExtension>;

/// Shader parameters of a [`SurfaceMaterial`].
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct SurfaceExtension {
    /// x: albedo variation strength, y: noise frequency (1/u), z: roughness variation strength,
    /// w: hue drift (patches turn drier: yellower, lighter).
    #[uniform(100)]
    pub variation: Vec4,
    /// x: wetness 0–1, y: puddle amount 0–1, z: snow cover 0–1, w: 1 = receives weather.
    #[uniform(101)]
    pub weather: Vec4,
}

impl SurfaceExtension {
    /// Variation `strength` and `hue` drift (0–1); `weather` = the surface reacts to wetness and
    /// snow.
    pub fn new(strength: f32, hue: f32, weather: bool) -> Self {
        Self {
            variation: Vec4::new(
                strength,
                VARIATION_FREQ,
                strength * ROUGHNESS_VARIATION,
                hue,
            ),
            weather: Vec4::new(0.0, 0.0, 0.0, if weather { 1.0 } else { 0.0 }),
        }
    }

    /// Sets the weather terms; returns `false` when nothing changed (no GPU re-upload needed).
    pub fn set_weather(&mut self, wetness: f32, puddles: f32, snow: f32) -> bool {
        let next = Vec4::new(wetness, puddles, snow, self.weather.w);
        let changed = next.distance_squared(self.weather) > 1e-8;
        self.weather = next;
        changed
    }
}

impl MaterialExtension for SurfaceExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/render/surface.wesl".into()
    }
}

pub(super) fn build(app: &mut App) {
    app.add_plugins(MaterialPlugin::<SurfaceMaterial>::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_updates_report_changes_only() {
        let mut ext = SurfaceExtension::new(0.2, 0.0, true);
        assert!(!ext.set_weather(0.0, 0.0, 0.0), "already dry");
        assert!(ext.set_weather(0.5, 0.3, 0.0));
        assert!(!ext.set_weather(0.5, 0.3, 0.0));
        assert_eq!(ext.weather.w, 1.0, "the opt-in flag survives updates");
        assert_eq!(SurfaceExtension::new(0.2, 0.0, false).weather.w, 0.0);
    }
}
