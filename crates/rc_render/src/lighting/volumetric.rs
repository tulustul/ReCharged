//! Volumetric fog (Ultra, single view — the quality mapping decides per camera): a fog volume
//! over the play area, lit by the key light (god rays through windows, sun shafts in fog), its
//! density from the clear-weather haze and `WeatherState.fog`.

use bevy::light::{FogVolume, VolumetricFog, VolumetricLight};
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use super::{ActiveEnvironment, EnvLight};
use crate::weather_fx::{Look, LookSet};

/// Fog density (per u) of clear air and of the densest fog preset.
const HAZE_DENSITY: f32 = 0.0012;
const FOG_DENSITY: f32 = 0.018;
/// Play area without a track (menus): half extents of the volume (u).
const MENU_HALF_EXTENTS: Vec3 = Vec3::new(40.0, 12.0, 40.0);

/// The environment's fog volume.
#[derive(Component, Debug, Clone, Copy)]
struct EnvFogVolume;

pub(super) fn build(app: &mut App) {
    app.add_systems(Update, drive_volumetrics.in_set(VisualSet).after(LookSet));
}

/// Fog density for the weather's fog amount 0–1.
pub(crate) fn fog_density(fog: f32) -> f32 {
    HAZE_DENSITY + (FOG_DENSITY - HAZE_DENSITY) * fog.clamp(0.0, 1.0).powf(1.5)
}

#[allow(clippy::too_many_arguments)]
fn drive_volumetrics(
    mut commands: Commands,
    look: Option<Res<Look>>,
    env: Option<Res<ActiveEnvironment>>,
    weather: Option<Res<WeatherState>>,
    track: Option<Res<Track>>,
    cameras: Query<(), With<VolumetricFog>>,
    keys: Query<(Entity, &EnvLight, Has<VolumetricLight>)>,
    mut volumes: Query<(Entity, &mut FogVolume, &mut Transform), With<EnvFogVolume>>,
) {
    let wanted = !cameras.is_empty() && look.is_some() && env.is_some();
    for (entity, role, has) in &keys {
        if *role != EnvLight::Key {
            continue;
        }
        if wanted && !has {
            commands.entity(entity).insert(VolumetricLight);
        } else if !wanted && has {
            commands.entity(entity).remove::<VolumetricLight>();
        }
    }
    let (Some(look), true) = (look, wanted) else {
        for (entity, ..) in &volumes {
            commands.entity(entity).despawn();
        }
        return;
    };
    let (centre, half) = match track.as_deref() {
        Some(t) => {
            let (min, max) = t.bounds();
            ((min + max) * 0.5, (max - min) * 0.5)
        }
        None => (Vec3::Y * MENU_HALF_EXTENTS.y, MENU_HALF_EXTENTS),
    };
    let fog = FogVolume {
        fog_color: look.fog_color,
        density_factor: fog_density(weather.map_or(0.0, |w| w.fog)),
        absorption: 0.2,
        scattering: 0.35,
        scattering_asymmetry: 0.6,
        ..default()
    };
    let transform = Transform::from_translation(centre).with_scale(half * 2.0);
    match volumes.iter_mut().next() {
        Some((_, mut volume, mut t)) => {
            volume.density_factor = fog.density_factor;
            volume.fog_color = fog.fog_color;
            t.set_if_neq(transform);
        }
        None => {
            commands.spawn((Name::new("Fog volume"), EnvFogVolume, fog, transform));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fog_density_rises_from_haze_to_the_fog_preset() {
        assert!((fog_density(0.0) - HAZE_DENSITY).abs() < 1e-7);
        assert!((fog_density(1.0) - FOG_DENSITY).abs() < 1e-7);
        assert!(fog_density(0.85) > fog_density(0.15) * 4.0);
        assert!(fog_density(2.0) <= FOG_DENSITY);
    }
}
