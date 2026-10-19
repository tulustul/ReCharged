//! The cloud layer (outdoor): one inverted dome drawn after the atmosphere's sky with a
//! procedural cloud shader (`assets/shaders/render/clouds.wesl`): coverage from
//! `WeatherState.cloudiness`, drift with the wind, darker under rain, lit by lightning flashes.
//! Colours are display-referred (they bypass exposure), tuned per time of day in the environment.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

/// Dome radius (u): inside the cameras' far plane (2000 u), beyond every track.
const DOME_RADIUS_U: f32 = 1500.0;
/// Height of the cloud deck above the camera in shader units (sets the perspective of the layer).
const CLOUD_ALTITUDE: f32 = 600.0;
/// Noise scale of the cloud deck (1/u).
const CLOUD_SCALE: f32 = 1.0 / 900.0;
/// Wind (u/s) → cloud drift (shader units/s).
const DRIFT_PER_WIND: f32 = 2.5;

/// Cloud shader parameters.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct CloudMaterial {
    /// rgb: lit colour, a: coverage 0–1.
    #[uniform(0)]
    pub lit: Vec4,
    /// rgb: shade colour, a: rain darkening 0–1.
    #[uniform(1)]
    pub shade: Vec4,
    /// xyz: direction toward the sun, w: lightning flash 0–1.
    #[uniform(2)]
    pub sun: Vec4,
    /// xy: drift offset (shader units), z: noise scale, w: deck altitude.
    #[uniform(3)]
    pub deck: Vec4,
}

impl Material for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/render/clouds.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    /// Drawn before every other transparent: sorted by its centre (the world origin) it would
    /// come after farther ones (sky lightning, windows against the sky) and cover them.
    fn depth_bias(&self) -> f32 {
        -1.0e6
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Seen from inside.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Marker of the cloud dome entity.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct CloudDome;

pub(super) fn build(app: &mut App) {
    app.add_plugins(MaterialPlugin::<CloudMaterial>::default());
}

/// The dome (spawn it with a scope); its material is driven by [`update`].
pub(crate) fn dome(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<CloudMaterial>,
    centre: Vec3,
) -> impl Bundle {
    (
        Name::new("Cloud dome"),
        CloudDome,
        Mesh3d(meshes.add(Sphere::new(DOME_RADIUS_U).mesh().uv(48, 24))),
        MeshMaterial3d(materials.add(CloudMaterial {
            deck: Vec4::new(0.0, 0.0, CLOUD_SCALE, CLOUD_ALTITUDE),
            ..default()
        })),
        Transform::from_translation(centre),
        NotShadowCaster,
        NotShadowReceiver,
    )
}

/// What the cloud layer shows this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CloudLook {
    pub lit: [f32; 3],
    pub shade: [f32; 3],
    pub coverage: f32,
    pub rain: f32,
    pub toward_sun: Vec3,
    pub flash: f32,
    pub wind: Vec3,
}

/// Writes the look into the dome's material (drift integrates the wind over `dt`).
pub(crate) fn update(material: &mut CloudMaterial, look: &CloudLook, dt: f32) {
    let drift = Vec2::new(look.wind.x, look.wind.z) * DRIFT_PER_WIND * dt;
    material.lit = Vec4::new(look.lit[0], look.lit[1], look.lit[2], look.coverage);
    material.shade = Vec4::new(look.shade[0], look.shade[1], look.shade[2], look.rain);
    material.sun = look.toward_sun.extend(look.flash);
    material.deck.x += drift.x;
    material.deck.y += drift.y;
}
