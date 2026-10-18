//! The active environment's look on every `QualityCamera`: exposure, colour grading, distance
//! fog, ambient light, image-based lighting from the generated environment map (room or sky),
//! the visible sky (Bevy's atmosphere, outdoors) with the star field at night, bloom strength.
//!
//! Outdoor IBL deliberately avoids `AtmosphereEnvironmentMapLight`: in Bevy 0.20 its probe
//! bind-group preparation panics every frame here (`atmosphere_transforms.binding().unwrap()`),
//! so the sky's lighting comes from `sky::sky_radiance` through the same generated-map path as
//! indoor rooms.

use bevy::camera::Exposure;
use bevy::light::Skybox;
use bevy::pbr::{AtmosphereSettings, DistanceFog, FogFalloff};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::ColorGrading;
use rc_core::prelude::*;

use super::ActiveEnvironment;
use super::room::EnvMap;
use crate::quality::QualityCamera;
use crate::sky::star_cubemap;
use crate::weather_fx::{Look, LookSet};

/// Fog glow around the key light (`DistanceFog::directional_light_exponent`).
const FOG_GLOW_EXPONENT: f32 = 24.0;

pub(super) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (apply_environment, apply_look)
            .chain()
            .in_set(VisualSet)
            .after(LookSet),
    );
}

/// Which environment's static parts (sky, filtered IBL maps, stars) a camera carries.
#[derive(Component, Debug, Clone, PartialEq)]
struct CameraEnv {
    id: String,
    time: TimeOfDay,
    maps: Option<(AssetId<Image>, AssetId<Image>)>,
}

/// The one entity Bevy filters the environment map on (every frame); cameras share its filtered
/// maps instead of each filtering a copy (four player views did the work four times).
#[derive(Component, Debug)]
struct EnvProbe;

/// The star field, made once.
#[derive(Resource, Debug)]
struct StarMap(Handle<Image>);

/// Switches a camera's sky / IBL maps when the environment (or its room map) changes. The
/// environment map is filtered once, on the [`EnvProbe`].
#[allow(clippy::too_many_arguments)]
fn apply_environment(
    mut commands: Commands,
    env: Option<Res<ActiveEnvironment>>,
    env_map: Option<Res<EnvMap>>,
    stars: Option<Res<StarMap>>,
    mut images: ResMut<Assets<Image>>,
    probes: Query<
        (
            Entity,
            &GeneratedEnvironmentMapLight,
            Option<&EnvironmentMapLight>,
        ),
        With<EnvProbe>,
    >,
    cameras: Query<(Entity, Option<&CameraEnv>), With<QualityCamera>>,
) {
    let Some(env) = env else {
        return;
    };
    let map = env_map.filter(|m| m.env == env.id).map(|m| m.image.clone());
    // Point the probe at the current map (its filtered targets are refilled every frame).
    let probe = probes.single().ok();
    if let Some(map) = &map {
        let source = GeneratedEnvironmentMapLight {
            environment_map: map.clone(),
            ..default()
        };
        match probe {
            None => {
                commands.spawn((Name::new("Environment map probe"), EnvProbe, source));
            }
            Some((entity, current, _)) if current.environment_map != *map => {
                commands.entity(entity).insert(source);
            }
            Some(_) => {}
        }
    }
    // The filtered targets stay the same when the source changes (no frame without IBL).
    let filtered = probe.and_then(|(_, _, light)| light).cloned();
    let wanted = CameraEnv {
        id: env.id.clone(),
        time: env.time,
        maps: filtered
            .as_ref()
            .map(|l| (l.diffuse_map.id(), l.specular_map.id())),
    };
    for (entity, current) in &cameras {
        if current == Some(&wanted) {
            continue;
        }
        let mut camera = commands.entity(entity);
        camera.remove::<(AtmosphereSettings, EnvironmentMapLight, Skybox)>();
        if let Some(light) = &filtered {
            camera.insert(EnvironmentMapLight {
                intensity: 0.0,
                ..light.clone()
            });
        }
        if !env.def.indoor {
            camera.insert(AtmosphereSettings::default());
            if env.time == TimeOfDay::Night {
                let image = match &stars {
                    Some(s) => s.0.clone(),
                    None => {
                        let handle = images.add(star_cubemap());
                        commands.insert_resource(StarMap(handle.clone()));
                        handle
                    }
                };
                commands.entity(entity).insert(Skybox {
                    image: Some(image),
                    brightness: env.def.sky.stars,
                    ..default()
                });
            }
        }
        commands.entity(entity).insert(wanted.clone());
    }
}

type LookComponents<'a> = (
    Option<&'a mut Exposure>,
    Option<&'a mut ColorGrading>,
    Option<&'a mut DistanceFog>,
    Option<&'a mut AmbientLight>,
    Option<&'a mut EnvironmentMapLight>,
    Option<&'a mut Bloom>,
);

/// Per-frame look values (exposure, grading, fog, ambient, IBL and bloom intensity).
fn apply_look(
    mut commands: Commands,
    look: Option<Res<Look>>,
    mut cameras: Query<(Entity, LookComponents), With<QualityCamera>>,
) {
    let Some(look) = look else {
        return;
    };
    let fog = DistanceFog {
        color: look.fog_color,
        directional_light_color: look.fog_glow,
        directional_light_exponent: FOG_GLOW_EXPONENT,
        falloff: FogFalloff::from_visibility_squared(look.fog_visibility),
    };
    let ambient = AmbientLight {
        color: look.ambient_color,
        brightness: look.ambient,
        ..default()
    };
    for (entity, (exposure, grading, distance_fog, ambient_light, env_map, bloom)) in &mut cameras {
        let mut missing = commands.entity(entity);
        match exposure {
            Some(mut e) => e.ev100 = look.exposure,
            None => {
                missing.insert(Exposure {
                    ev100: look.exposure,
                });
            }
        }
        match grading {
            Some(mut g) => *g = look.grading.clone(),
            None => {
                missing.insert(look.grading.clone());
            }
        }
        match distance_fog {
            Some(mut f) => *f = fog.clone(),
            None => {
                missing.insert(fog.clone());
            }
        }
        match ambient_light {
            Some(mut a) => *a = ambient.clone(),
            None => {
                missing.insert(ambient.clone());
            }
        }
        if let Some(mut map) = env_map {
            map.intensity = look.env_light;
        }
        if let Some(mut bloom) = bloom {
            bloom.intensity = look.bloom;
        }
    }
}
