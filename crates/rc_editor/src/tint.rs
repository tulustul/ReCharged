//! Hover and selection tint: every mesh of the hovered and of the selected prop or decor model
//! wears its own clone of its material (`StandardMaterial`, or `SurfaceMaterial` for fallback
//! props) with an emissive boost; un-hovering or deselecting puts the original back and drops the
//! clone. A material swapped under a tint (a model's glb arriving after it was selected, the lamp
//! dimming on `WorldInstanceReady`) is re-tinted from the new one.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use rc_core::prelude::*;
use rc_render::{DecorSource, SurfaceMaterial};
use rc_track::PropSource;
use rc_ui::editor::EditorItem;

use crate::view::Hover;
use crate::{Editor, Mode};

/// Added light of a tinted mesh (emissive is not scaled by the exposure: a fraction of white).
const HOVER: LinearRgba = LinearRgba::rgb(0.02, 0.08, 0.12);
const SELECTED: LinearRgba = LinearRgba::rgb(0.12, 0.09, 0.0);

/// A tinted mesh: its own material and the clone it wears.
#[derive(Debug)]
enum Swap {
    Standard(Handle<StandardMaterial>, Handle<StandardMaterial>),
    Surface(Handle<SurfaceMaterial>, Handle<SurfaceMaterial>),
}

#[derive(Resource, Debug, Default)]
pub(crate) struct Tint {
    meshes: HashMap<Entity, (bool, Swap)>,
    /// What is tinted: (hovered, selected).
    shown: (Option<EditorItem>, Option<EditorItem>),
    /// Models were instanced or spawned since: look again.
    dirty: bool,
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Tint>()
        .add_observer(|_: On<WorldInstanceReady>, mut tint: ResMut<Tint>| tint.dirty = true)
        .add_systems(
            Update,
            tint.after(crate::view::pointer)
                .run_if(in_state(AppState::InRace)),
        );
}

/// Model roots spawned since the last frame.
type NewModels = Or<(Added<PropSource>, Added<DecorSource>)>;

/// What [`tint`] reads besides the editor.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Models<'w, 's> {
    roots: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static PropSource>,
            Option<&'static DecorSource>,
        ),
    >,
    added: Query<'w, 's, (), NewModels>,
    children: Query<'w, 's, &'static Children>,
    standard: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>>,
    surface: Query<'w, 's, &'static MeshMaterial3d<SurfaceMaterial>>,
}

fn tint(
    editor: Res<Editor>,
    hover: Res<Hover>,
    models: Models,
    mut tint: ResMut<Tint>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut surface: ResMut<Assets<SurfaceMaterial>>,
    mut commands: Commands,
) {
    let editing = editor.mode == Mode::Editing;
    let shown = if editing {
        (hover.0, editor.selection)
    } else {
        (None, None)
    };
    if shown == tint.shown && !tint.dirty && models.added.is_empty() {
        return;
    }
    tint.shown = shown;
    tint.dirty = false;
    // Every mesh entity of the hovered and selected models (selected wins).
    let mut want: HashMap<Entity, bool> = HashMap::new();
    for (selected, item) in [(false, shown.0), (true, shown.1)] {
        let Some(item) = item else {
            continue;
        };
        for (root, prop, decor) in &models.roots {
            let hit = match item {
                EditorItem::Prop { index } => prop.is_some_and(|p| p.0 == index as usize),
                EditorItem::Decor { index } => decor.is_some_and(|d| d.0 == index as usize),
                _ => false,
            };
            if hit {
                for e in std::iter::once(root).chain(models.children.iter_descendants(root)) {
                    want.insert(e, selected);
                }
            }
        }
    }
    // Put back what should no longer be tinted.
    let gone: Vec<Entity> = (tint.meshes.keys())
        .filter(|e| !want.contains_key(*e))
        .copied()
        .collect();
    for e in gone {
        let Some((_, swap)) = tint.meshes.remove(&e) else {
            continue;
        };
        if let Some(original) = worn(&models, e, &swap) {
            original.wear(&mut commands, e);
        } // else swapped by someone else meanwhile (or despawned): theirs stays
    }
    for (e, selected) in want {
        let boost = if selected { SELECTED } else { HOVER };
        // The material under the tint: kept from our swap while it is still worn, else what the
        // mesh wears now.
        let base = match tint.meshes.get(&e) {
            Some((was, swap)) => match worn(&models, e, swap) {
                Some(_) if *was == selected => continue,
                Some(original) => Some(original),
                None => None,
            },
            None => None,
        };
        let base = base.or_else(|| {
            (models
                .standard
                .get(e)
                .map(|m| Base::Standard(m.0.clone()))
                .ok())
            .or_else(|| {
                models
                    .surface
                    .get(e)
                    .map(|m| Base::Surface(m.0.clone()))
                    .ok()
            })
        });
        let swap = match base {
            Some(Base::Standard(original)) => {
                let Some(mut copy) = standard.get(&original).cloned() else {
                    continue;
                };
                copy.emissive += boost;
                let clone = standard.add(copy);
                commands.entity(e).insert(MeshMaterial3d(clone.clone()));
                Swap::Standard(original, clone)
            }
            Some(Base::Surface(original)) => {
                let Some(mut copy) = surface.get(&original).cloned() else {
                    continue;
                };
                copy.base.emissive += boost;
                let clone = surface.add(copy);
                commands.entity(e).insert(MeshMaterial3d(clone.clone()));
                Swap::Surface(original, clone)
            }
            None => continue,
        };
        tint.meshes.insert(e, (selected, swap));
    }
}

/// A mesh's own material.
enum Base {
    Standard(Handle<StandardMaterial>),
    Surface(Handle<SurfaceMaterial>),
}

impl Base {
    fn wear(self, commands: &mut Commands, e: Entity) {
        match self {
            Base::Standard(h) => commands.entity(e).insert(MeshMaterial3d(h)),
            Base::Surface(h) => commands.entity(e).insert(MeshMaterial3d(h)),
        };
    }
}

/// The original material when the mesh still wears the clone of `swap`.
fn worn(models: &Models, e: Entity, swap: &Swap) -> Option<Base> {
    match swap {
        Swap::Standard(original, clone) => {
            (models.standard.get(e).ok()?.0 == *clone).then(|| Base::Standard(original.clone()))
        }
        Swap::Surface(original, clone) => {
            (models.surface.get(e).ok()?.0 == *clone).then(|| Base::Surface(original.clone()))
        }
    }
}
