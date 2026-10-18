//! Prop visuals (ARCHITECTURE §4.4, §5.9): every `Prop` gets its glb (`PropDef::model`, an
//! environment artist's file in `assets/models/props/<env>/`) or, while that file does not exist
//! yet, tidy fallback meshes of its collider primitives ([`shapes`]) in library materials.
//! Knockable props wear the model on their visual proxy; static props get a session-scoped model
//! entity at their pose. Lamp props add real lights at Night (`lights` in `props/*.ron`, which
//! rc_track ignores) and dim their `lamp*` bulb materials by day. Decorative-only models without
//! colliders come from `assets/data/decor/<track>.ron` ([`decor`]).

mod decor;
mod lamps;
pub(crate) mod shapes;

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{PropDef, Track};

use crate::content::{ContentLoad, mesh_from_data};
use crate::lighting::ActiveEnvironment;
use crate::materials::{MaterialLibrary, SurfaceMaterial};
use lamps::PropLamps;

/// Library palette of fallback props: knockables cycle through it, static kinds pick one.
const PALETTE: [&str; 6] = [
    "plastic_red",
    "plastic_blue",
    "plastic_yellow",
    "plastic_green",
    "plastic_orange",
    "plastic_white",
];

/// Marks a `Prop` whose visuals exist.
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct PropDressed;

/// The root of a prop's visuals (glb scene or fallback meshes, lamps).
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct PropModel;

/// Prop scenes and fallback meshes by kind, kept for the app's lifetime.
#[derive(Resource, Debug, Default)]
struct PropAssets {
    scenes: HashMap<String, Handle<WorldAsset>>,
    fallbacks: HashMap<String, Vec<Handle<Mesh>>>,
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<PropAssets>()
        .init_resource::<PropLamps>()
        .add_observer(lamps::dim_bulbs)
        .add_systems(OnEnter(AppState::Loading), lamps::read_lamps)
        .add_systems(
            Update,
            decor::spawn_decor
                .in_set(VisualSet)
                .run_if(in_state(AppState::Loading)),
        )
        .add_systems(
            Update,
            dress_props
                .in_set(VisualSet)
                .run_if(in_state(AppState::Loading).or_else(in_state(AppState::InRace))),
        );
}

/// Material of a fallback prop: its drivable surface, else a palette colour (per instance for
/// knockables, per kind otherwise).
fn fallback_material(
    library: &MaterialLibrary,
    kind: &str,
    def: &PropDef,
    instance: usize,
) -> Handle<SurfaceMaterial> {
    if let Some(surface) = def.surface {
        return library.surface(surface);
    }
    let pick = if def.dynamic {
        instance
    } else {
        let mut h = DefaultHasher::new();
        kind.hash(&mut h);
        h.finish() as usize
    };
    library
        .named(PALETTE[pick % PALETTE.len()])
        .unwrap_or_else(|| library.surface(SurfaceKind::Asphalt))
}

/// Gives every undressed `Prop` its visuals once the track and the environment's materials exist.
#[allow(clippy::too_many_arguments)]
fn dress_props(
    mut commands: Commands,
    mut load: ResMut<ContentLoad>,
    track: Option<Res<Track>>,
    env: Option<Res<ActiveEnvironment>>,
    library: Res<MaterialLibrary>,
    lamp_lights: Res<PropLamps>,
    paths: Option<Res<UserPaths>>,
    server: Res<AssetServer>,
    mut assets: ResMut<PropAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    props: Query<(Entity, &Prop, &Transform, Option<&Visual>), Without<PropDressed>>,
    mut instance: Local<usize>,
) {
    let (Some(track), Some(env)) = (track, env) else {
        return;
    };
    if library.environment() != Some(env.id.as_str()) {
        return;
    }
    let practicals = env.def.practicals(env.time);
    let (mut waiting, mut dressed, mut lit) = (false, 0, 0);
    for (entity, prop, transform, visual) in &props {
        let Some(def) = track.prop_def(&prop.kind) else {
            commands.entity(entity).insert(PropDressed);
            continue;
        };
        let root = match (prop.dynamic, visual) {
            (true, Some(proxy)) => commands
                .spawn((
                    Name::new("prop model"),
                    PropModel,
                    Transform::IDENTITY,
                    Visibility::default(),
                    ChildOf(proxy.get()),
                ))
                .id(),
            (true, None) => {
                waiting = true;
                continue;
            }
            (false, _) => commands
                .spawn((
                    Name::new("prop model"),
                    PropModel,
                    *transform,
                    Visibility::default(),
                    DespawnOnExit(RaceSession),
                ))
                .id(),
        };
        commands.entity(entity).insert(PropDressed);
        dressed += 1;
        let has_model = paths
            .as_ref()
            .is_some_and(|p| p.assets_dir.join(&def.model).is_file());
        if has_model {
            let scene = assets
                .scenes
                .entry(prop.kind.clone())
                .or_insert_with(|| {
                    server.load(GltfAssetLabel::Scene(0).from_asset(def.model.clone()))
                })
                .clone();
            if !load.scenes.iter().any(|(h, ..)| *h == scene) {
                load.scenes.push((scene.clone(), def.model.clone(), false));
            }
            commands.entity(root).insert(WorldAssetRoot(scene));
        } else {
            let parts = assets
                .fallbacks
                .entry(prop.kind.clone())
                .or_insert_with(|| {
                    def.colliders
                        .iter()
                        .filter_map(|p| mesh_from_data(&shapes::primitive(p)))
                        .map(|m| meshes.add(m))
                        .collect()
                });
            *instance += 1;
            let material = fallback_material(&library, &prop.kind, def, *instance);
            for mesh in parts.iter() {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    ChildOf(root),
                ));
            }
        }
        if practicals > 0.0
            && let Some(lights) = lamp_lights.get(&prop.kind)
        {
            for light in lights {
                lamps::spawn_lamp(&mut commands, root, light, practicals);
                lit += 1;
            }
        }
    }
    if dressed > 0 {
        info!("props: {dressed} dressed, {lit} lamp lights");
    }
    if !waiting && !load.props {
        load.props = true;
    }
}
