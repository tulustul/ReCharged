//! [`WaterMaterial`]: the water ground areas (`SurfaceKind::Water`: puddles, wash bays), drawn by
//! `assets/shaders/render/water.wesl` in the transmissive pass, where the opaque scene behind them
//! is readable: the bed shows through, the surface mirrors what is on screen (ray-marched against
//! the depth prepass) and the environment map elsewhere, and foam rings the shore (the mesh's
//! UV_1, see `track_visuals/water.rs`).

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Water of the ground areas.
pub type WaterMaterial = ExtendedMaterial<StandardMaterial, WaterExtension>;

/// The water shader stage (no parameters of its own).
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct WaterExtension {}

impl MaterialExtension for WaterExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/render/water.wesl".into()
    }

    // Out of the depth prepass: the shader reads the depth of the bed under the surface.
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

pub(super) fn build(app: &mut App) {
    app.add_plugins(MaterialPlugin::<WaterMaterial>::default());
}

/// The one water material.
pub(super) fn water_material() -> WaterMaterial {
    WaterMaterial {
        base: StandardMaterial {
            // Tint of the bed seen through the water: dark, so reflections stand out.
            base_color: Color::srgb(0.5, 0.55, 0.58),
            perceptual_roughness: 0.04,
            // F0 = 0.16 × 0.79² ≈ 0.1 (still water is 0.02): puddles read as mirrors.
            reflectance: 0.79,
            // Only routes the material to the transmissive pass (the opaque scene copy); the
            // shader refracts the bed itself.
            specular_transmission: 1.0,
            ..default()
        },
        extension: WaterExtension {},
    }
}
