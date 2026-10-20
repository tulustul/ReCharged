//! The environment's point and spot lights (`EnvironmentDef::lights`) and the track editor's
//! hooks into the environment: [`EnvironmentDraft`] (unsaved edits, followed in a race) and
//! [`EnvPreview`] (a live gizmo drag of a light, the sun or the fill).
//!
//! The lights are spawned from a pure plan — (document index, light kind, pose), only those on at
//! the session's time, mirrored with the layout — the load as the changes from nothing, a changed
//! draft as the diff against what was built (`rc_track::placement_changes`: another kind
//! respawns, another pose moves). The sun and fill are not mirrored (§Irradiance bake).

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{PlacementChanges, placement_changes};

use super::{ActiveEnvironment, EnvLight, EnvironmentDef, lamp_lumens};
use crate::sky::{light_transform, sun_direction};

/// Spot inner cone relative to its outer angle.
const SPOT_INNER: f32 = 0.6;

/// Which `EnvironmentDef::lights` entry an environment light entity was spawned from (the index
/// before the `times` filter).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvLightSource(pub usize);

/// What the session uses instead of `maps/<id>/environment.ron` when it races track `id`: the
/// track editor's unsaved edits (written only by rc_editor; the game never has one). In a race,
/// a changed draft is copied into [`ActiveEnvironment`] (when it differs) and the environment
/// lights are diffed against it; the time look follows from there.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct EnvironmentDraft {
    /// Track it stands in for.
    pub id: TrackId,
    /// The environment.
    pub def: EnvironmentDef,
}

/// What a live track-editor drag shows before it is committed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvDrag {
    /// Environment light `i` (document index) at this pose.
    Light(usize, Transform),
    /// The key light from this direction (degrees).
    Sun {
        /// Above the horizon.
        elevation: f32,
        /// Clockwise from north.
        azimuth: f32,
    },
    /// The fill light from this direction (degrees).
    Fill {
        /// Above the horizon.
        elevation: f32,
        /// Clockwise from north.
        azimuth: f32,
    },
}

/// The pose a track-editor drag shows (written only by rc_editor; the game never has one). Kept
/// apart from [`EnvironmentDraft`] so a drag rebuilds nothing (the environment map is
/// regenerated only when the draft changes, on release); `None` puts everything back.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct EnvPreview(pub Option<EnvDrag>);

/// What makes two environment lights the same entity (a change of any of it respawns).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LightKind {
    /// Outer cone (degrees) of a spot; `None` = point light.
    spot_angle: Option<f32>,
    color: [f32; 3],
    lumens: f32,
    range: f32,
}

/// One planned light: (document index, kind, pose).
pub(crate) type PlannedLight = (usize, LightKind, Transform);

/// The environment lights on at `time`, mirrored (X negated) in mirror layouts.
pub(crate) fn plan(def: &EnvironmentDef, time: TimeOfDay, mirror: bool) -> Vec<PlannedLight> {
    let flip = |p: [f32; 3]| {
        let v = Vec3::from_array(p);
        if mirror { v.with_x(-v.x) } else { v }
    };
    (def.lights.iter().enumerate())
        .filter(|(_, l)| l.on_at(time))
        .map(|(i, l)| {
            let at = Transform::from_translation(flip(l.pos));
            let pose = match l.target {
                Some(target) => at.looking_at(flip(target), Vec3::Y),
                None => at,
            };
            let kind = LightKind {
                spot_angle: l.target.map(|_| l.angle),
                color: l.color,
                lumens: l.lumens,
                range: l.range,
            };
            (i, kind, pose)
        })
        .collect()
}

/// The environment lights the session has (sorted by source).
#[derive(Resource, Debug, Default)]
pub(super) struct BuiltLights(pub Vec<PlannedLight>);

/// Spawns planned lights with a lifetime scope.
pub(super) fn spawn(
    commands: &mut Commands,
    lights: &[PlannedLight],
    scope: impl Component + Clone,
) {
    for (source, kind, pose) in lights {
        let color = Color::srgb(kind.color[0], kind.color[1], kind.color[2]);
        let intensity = lamp_lumens(kind.lumens);
        let mut light = commands.spawn((EnvLightSource(*source), *pose, scope.clone()));
        match kind.spot_angle {
            Some(angle) => {
                let outer = angle.to_radians();
                light.insert((
                    Name::new("Environment spot"),
                    SpotLight {
                        color,
                        intensity,
                        range: kind.range,
                        outer_angle: outer,
                        inner_angle: outer * SPOT_INNER,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                ));
            }
            None => {
                light.insert((
                    Name::new("Environment light"),
                    PointLight {
                        color,
                        intensity,
                        range: kind.range,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                ));
            }
        }
    }
}

/// `InRace`, when the editor's draft changed: the session's environment becomes the draft (only
/// when it differs, so an unrelated draft write regenerates nothing) and its lights follow.
pub(super) fn follow_draft(
    mut commands: Commands,
    draft: Res<EnvironmentDraft>,
    config: Option<Res<RaceConfig>>,
    env: Option<ResMut<ActiveEnvironment>>,
    mut built: ResMut<BuiltLights>,
    mut lights: Query<(Entity, &EnvLightSource, &mut Transform)>,
) {
    let (Some(config), Some(mut env)) = (config, env) else {
        return;
    };
    if config.variant.track != draft.id || env.def == draft.def {
        return;
    }
    let lit = plan(&draft.def, env.time, config.variant.mirror);
    let changes: PlacementChanges<LightKind> = placement_changes(&built.0, &lit);
    for (entity, source, mut transform) in &mut lights {
        if changes.despawn.contains(&source.0) {
            commands.entity(entity).despawn();
        } else if let Some((_, pose)) = changes.moved.iter().find(|(i, _)| *i == source.0) {
            *transform = *pose;
        }
    }
    spawn(&mut commands, &changes.spawn, DespawnOnExit(RaceSession));
    changes.apply(&mut built.0);
    env.def = draft.def.clone();
    info!(
        "visuals: environment edited in place (lights: {} despawned, {} moved, {} spawned)",
        changes.despawn.len(),
        changes.moved.len(),
        changes.spawn.len()
    );
}

/// The editor's [`EnvPreview`] changed: the previewed light goes to the preview pose, the one
/// previewed before back to where it was.
pub(super) fn preview_light(
    preview: Res<EnvPreview>,
    mut lights: Query<(Entity, &EnvLightSource, &mut Transform)>,
    mut moved: Local<Option<(Entity, Transform)>>,
) {
    let target = match preview.0 {
        Some(EnvDrag::Light(i, pose)) => Some((i, pose)),
        _ => None,
    };
    if let Some((entity, original)) = *moved {
        match lights.get_mut(entity) {
            Ok((_, source, mut transform)) if target.is_none_or(|(i, _)| source.0 != i) => {
                *transform = original;
                *moved = None;
            }
            Ok(_) => {}
            Err(_) => *moved = None, // despawned by a session reload
        }
    }
    let Some((i, pose)) = target else {
        return;
    };
    for (entity, source, mut transform) in &mut lights {
        if source.0 == i {
            moved.get_or_insert((entity, *transform));
            *transform = pose;
        }
    }
}

/// After `apply_lights` (which poses the key and fill from the look every frame): a previewed
/// sun or fill direction.
pub(super) fn preview_sky(
    preview: Option<Res<EnvPreview>>,
    mut lights: Query<(&EnvLight, &mut Transform)>,
) {
    let (role, elevation, azimuth) = match preview.and_then(|p| p.0) {
        Some(EnvDrag::Sun { elevation, azimuth }) => (EnvLight::Key, elevation, azimuth),
        Some(EnvDrag::Fill { elevation, azimuth }) => (EnvLight::Fill, elevation, azimuth),
        _ => return,
    };
    for (r, mut transform) in &mut lights {
        if *r == role {
            *transform = light_transform(sun_direction(elevation, azimuth));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::lighting::EnvLightDef;

    fn toy_town() -> EnvironmentDef {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        EnvironmentDef::load(&assets, "toy_town").expect("toy_town environment")
    }

    fn spot(pos: [f32; 3], target: [f32; 3]) -> EnvLightDef {
        EnvLightDef {
            pos,
            target: Some(target),
            angle: 30.0,
            color: [1.0; 3],
            lumens: 1000.0,
            range: 20.0,
            times: vec![],
        }
    }

    /// A spot spawned in a mirror layout stands at −x and aims at its mirrored target.
    #[test]
    fn mirror_layouts_mirror_the_environment_lights() {
        let mut def = toy_town();
        def.lights = vec![spot([-26.0, 15.0, -11.0], [-40.0, 5.0, -25.0])];
        let mut world = World::new();
        world
            .run_system_once(move |mut commands: Commands| {
                let lights = plan(&def, TimeOfDay::Noon, true);
                spawn(&mut commands, &lights, DespawnOnExit(RaceSession));
            })
            .expect("spawns");
        let mut spots = world.query::<(&SpotLight, &EnvLightSource, &Transform)>();
        let (_, source, t) = spots.single(&world).expect("one spot");
        assert_eq!(source.0, 0);
        assert_eq!(t.translation, Vec3::new(26.0, 15.0, -11.0));
        let aim = (Vec3::new(40.0, 5.0, -25.0) - t.translation).normalize();
        assert!(t.forward().dot(aim) > 0.9999, "aims at the mirrored target");
    }

    /// Light indices are document indices: the `times` filter leaves gaps, never shifts them.
    #[test]
    fn planned_lights_keep_their_document_index() {
        let def = toy_town();
        let noon: Vec<usize> = plan(&def, TimeOfDay::Noon, false)
            .iter()
            .map(|l| l.0)
            .collect();
        assert!(
            noon.is_empty(),
            "Toy Town's spots are on at sunset and night only"
        );
        let sunset: Vec<usize> = (plan(&def, TimeOfDay::Sunset, false).iter())
            .map(|l| l.0)
            .collect();
        assert_eq!(sunset, [0, 1], "the night-only point light is left out");
        let night = plan(&def, TimeOfDay::Night, false);
        assert_eq!(night.iter().map(|l| l.0).collect::<Vec<_>>(), [0, 1, 2]);
    }

    /// Edits applied incrementally (diffed against what was built) end where a fresh plan of the
    /// final environment does; a load is the changes from nothing.
    #[test]
    fn incremental_light_plans_equal_fresh_ones() {
        let mut def = toy_town();
        let time = TimeOfDay::Night;
        let mut built = Vec::new();
        placement_changes(&[], &plan(&def, time, false)).apply(&mut built);
        let edits: [fn(&mut EnvironmentDef); 7] = [
            |d| d.lights[0].pos[0] += 2.0,                  // moved
            |d| d.lights[1].target = Some([0.0, 0.0, 0.0]), // aimed
            |d| d.lights[2].color = [0.2, 0.4, 1.0],        // another kind: respawned
            |d| {
                d.lights.remove(0); // later sources shift down
            },
            |d| d.lights.push(spot([1.0, 9.0, 1.0], [1.0, 4.0, 1.0])),
            |d| d.lights[0].times = vec![TimeOfDay::Noon], // off at this time
            |d| d.lights[0].times.clear(),                 // on again
        ];
        for edit in edits {
            edit(&mut def);
            placement_changes(&built, &plan(&def, time, false)).apply(&mut built);
        }
        assert_eq!(built, plan(&def, time, false));
    }
}
