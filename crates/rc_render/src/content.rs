//! Content visuals (T13b): what the race world is made of — the track's meshes and room shell
//! ([`track_visuals`]), props ([`props`]), car models ([`cars`]) — and the capture tools
//! ([`capture`]). Design note: `docs/design/rc_render_content.md`.
//!
//! Registers the `"content"` loading key and completes it once the track meshes are built and
//! every car and prop scene of the session is loaded. Everything spawned here is presentation:
//! session-scoped track visuals, and children of visual proxies (racers, ghosts, dynamic props).
//!
//! [`track_visuals`]: crate::track_visuals
//! [`props`]: crate::props
//! [`cars`]: crate::cars
//! [`capture`]: crate::capture

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::asset::{LoadState, RenderAssetUsages};
use bevy::gltf::GltfLoaderSettings;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::MeshData;

use crate::lighting::ActiveEnvironment;

/// Loading-gate key of the content visuals.
pub const LOADING_KEY: &str = "content";

/// Track, prop and car visuals plus captures (added by `GraphicsPlugin`).
pub struct ContentVisualsPlugin;

impl Plugin for ContentVisualsPlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        // Reset when a session starts and when it ends: its scene handles must not keep the last
        // track's models loaded in the menu (Q21).
        let reset = |mut load: ResMut<ContentLoad>| *load = ContentLoad::default();
        app.init_resource::<ContentLoad>()
            .add_systems(OnEnter(AppState::Loading), reset)
            .add_systems(OnEnter(AppState::Unloading), reset)
            .add_systems(
                Update,
                report_loaded
                    .in_set(VisualSet)
                    .run_if(in_state(AppState::Loading)),
            );
        crate::track_visuals::build(app);
        crate::props::build(app);
        crate::cars::build(app);
        crate::capture::build(app);
    }
}

/// What the session's content still waits for (reset on entering `Loading`).
#[derive(Resource, Debug, Default)]
pub(crate) struct ContentLoad {
    /// Track meshes and the room shell are spawned.
    pub track: bool,
    /// Every prop has its model or fallback.
    pub props: bool,
    /// The session's car scenes are queued.
    pub cars: bool,
    /// Scenes to wait for: (handle, asset path, a failure fails the loading).
    pub scenes: Vec<(Handle<WorldAsset>, String, bool)>,
    /// `LoadingDone` or `LoadingFailed` was sent.
    pub reported: bool,
}

/// `Loading`: `LoadingDone("content")` once everything is built and loaded; a required scene
/// that fails (a broken car model) fails the loading with its path.
fn report_loaded(
    mut load: ResMut<ContentLoad>,
    server: Res<AssetServer>,
    mut done: MessageWriter<LoadingDone>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    if load.reported || !(load.track && load.props && load.cars) {
        return;
    }
    for (handle, path, required) in &load.scenes {
        match server.load_state(handle) {
            LoadState::Failed(e) if *required => {
                let error = format!("{path}: {e}");
                error!("content: {error}");
                failed.write(LoadingFailed {
                    key: LOADING_KEY,
                    error,
                });
                load.reported = true;
                return;
            }
            LoadState::Failed(_) => {}
            _ if !server.is_loaded_with_dependencies(handle) => return,
            _ => {}
        }
    }
    info!("content: track, {} scenes ready", load.scenes.len());
    load.reported = true;
    done.write(LoadingDone(LOADING_KEY));
}

/// Set once at startup by tools that read meshes, materials and textures on the CPU (the track
/// editor's light baker); the game keeps them on the GPU only.
static KEEP_CPU_COPIES: AtomicBool = AtomicBool::new(false);

/// Keep CPU copies of every mesh, material and texture rc_render loads or builds (call before the
/// app runs; costs the RSS that GPU-only assets save, Q21).
pub fn keep_cpu_copies() {
    KEEP_CPU_COPIES.store(true, Ordering::Relaxed);
}

/// Where rc_render's assets live: GPU only, unless [`keep_cpu_copies`] was called.
pub(crate) fn asset_usage() -> RenderAssetUsages {
    if KEEP_CPU_COPIES.load(Ordering::Relaxed) {
        RenderAssetUsages::default()
    } else {
        RenderAssetUsages::RENDER_WORLD
    }
}

/// Loads a glb's first scene with its meshes and textures kept only on the GPU: nothing in the
/// game reads model vertices or texels on the CPU (colliders come from the prop RON files), and
/// the main-world copies of every prop and car were most of the game's RSS (Q21). Every glb load
/// goes through here, so a file is never requested with two different settings.
pub(crate) fn load_scene(server: &AssetServer, path: &str) -> Handle<WorldAsset> {
    server
        .load_builder()
        .with_settings(|s: &mut GltfLoaderSettings| {
            s.load_meshes = asset_usage();
            s.load_materials = asset_usage();
        })
        .load(GltfAssetLabel::Scene(0).from_asset(path.to_owned()))
}

/// A Bevy mesh of rc_track-style buffers (world-unit UVs; tangents for the library's normal
/// maps). `None` for an empty buffer.
pub(crate) fn mesh_from_data(data: &MeshData) -> Option<Mesh> {
    if data.is_empty() {
        return None;
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, asset_usage())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, data.positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs.clone())
        .with_inserted_indices(Indices::U32(data.indices.clone()));
    if let Err(e) = mesh.generate_tangents() {
        warn_once!("content: no tangents for a mesh ({e}); its normal map is ignored");
    }
    Some(mesh)
}

/// The environment's time of day is Night (lamps on, headlights full).
pub(crate) fn is_night(env: Option<&ActiveEnvironment>) -> bool {
    env.is_some_and(|e| e.time == TimeOfDay::Night)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    fn quad() -> MeshData {
        MeshData {
            positions: vec![
                [0.0, 0.0, 0.0],
                [4.0, 0.0, 0.0],
                [4.0, 0.0, -4.0],
                [0.0, 0.0, -4.0],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            uvs: vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    #[test]
    fn mesh_conversion_keeps_buffers_and_adds_tangents() {
        let mesh = mesh_from_data(&quad()).expect("a mesh");
        assert_eq!(mesh.count_vertices(), 4);
        assert_eq!(mesh.indices().map(Indices::len), Some(6));
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("uvs");
        };
        assert_eq!(uvs[2], [4.0, 4.0], "UVs stay in world units");
        assert!(mesh.attribute(Mesh::ATTRIBUTE_TANGENT).is_some());
    }

    #[test]
    fn empty_buffers_make_no_mesh() {
        assert!(mesh_from_data(&MeshData::default()).is_none());
    }
}
