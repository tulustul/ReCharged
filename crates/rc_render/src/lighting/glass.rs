//! Window glass of indoor rooms (GDD §10.2 "outside weather"): an unlit pane showing the outside
//! light (`assets/shaders/render/window_glass.wesl`) — brightness and colour by time of day and
//! cloud cover, rain beads and running streaks while it rains outside, lightning flashes.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use rc_core::prelude::*;

use crate::weather_fx::{Look, LookSet};

/// Pane parameters.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct WindowMaterial {
    /// rgb: outside light (display-referred), a: rain on the glass 0–1.
    #[uniform(0)]
    pub sky: Vec4,
    /// x: lightning flash 0–1.
    #[uniform(1)]
    pub extra: Vec4,
}

impl Material for WindowMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/render/window_glass.wesl".into()
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
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Marker of window panes; `door` panes (open doors) show the outside light without rain.
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct WindowGlass {
    pub door: bool,
}

pub(super) fn build(app: &mut App) {
    app.add_plugins(MaterialPlugin::<WindowMaterial>::default())
        .add_systems(Update, drive_glass.in_set(VisualSet).after(LookSet));
}

fn drive_glass(
    look: Option<Res<Look>>,
    panes: Query<(&MeshMaterial3d<WindowMaterial>, &WindowGlass)>,
    mut materials: ResMut<Assets<WindowMaterial>>,
    mut done: Local<Vec<AssetId<WindowMaterial>>>,
) {
    let Some(look) = look else {
        return;
    };
    let extra = Vec4::new(look.clouds.flash, 0.0, 0.0, 0.0);
    // Panes share a few materials (windows, doors): update each once.
    done.clear();
    for (handle, glass) in &panes {
        if done.contains(&handle.id()) {
            continue;
        }
        done.push(handle.id());
        let rain = if glass.door { 0.0 } else { look.rain_on_glass };
        let sky = Vec4::new(look.window[0], look.window[1], look.window[2], rain);
        if materials
            .get(handle)
            .is_some_and(|m| m.sky != sky || m.extra != extra)
            && let Some(mut m) = materials.get_mut(handle)
        {
            m.sky = sky;
            m.extra = extra;
        }
    }
}
