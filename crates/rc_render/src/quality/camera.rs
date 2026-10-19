//! Camera decoration: rc_camera spawns bare `GameCamera` / `MenuCamera`s; this adds HDR, filmic
//! tonemapping, `Msaa::Off` and the per-quality post-processing, and re-applies it live whenever
//! the settings or the number of player views change. The split-screen `OverlayCamera` is never
//! touched.

use bevy::anti_alias::fxaa::Fxaa;
use bevy::anti_alias::smaa::Smaa;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::Hdr;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{ShadowFilteringMethod, VolumetricFog};
use bevy::pbr::{ContactShadows, ScreenSpaceAmbientOcclusion};
use bevy::post_process::bloom::Bloom;
use bevy::post_process::motion_blur::MotionBlur;
use bevy::prelude::*;
use bevy::render::camera::{MipBias, TemporalJitter};
use rc_core::prelude::*;

use super::{AaMode, ViewQuality, view_quality};

/// SSAO sample radius in view-space units (u): contact shadows under cars and in corners without
/// blackening whole spaces under overhangs (helix levels, tunnels).
const SSAO_RADIUS_U: f32 = 0.8;
/// SSAO thickness heuristic (u).
const SSAO_THICKNESS_U: f32 = 0.35;
/// Motion blur: shutter angle (fraction of a frame) and samples.
const MOTION_BLUR_SHUTTER: f32 = 0.5;
const MOTION_BLUR_SAMPLES: u32 = 2;
/// Volumetric fog raymarch steps.
const VOLUMETRIC_STEPS: u32 = 48;

/// A 3D view decorated by rc_render: HDR, tonemapping, quality post-processing and the active
/// environment's look. Added to every `GameCamera` and the `MenuCamera`; other 3D cameras of this
/// crate (captures) opt in by inserting it.
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct QualityCamera;

/// The quality last applied to a camera (re-applied only when it changes).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
struct Applied(ViewQuality);

/// Renders a `QualityCamera` as one of this many player views, whatever is on screen: the menu
/// warm-up compiles the split-screen variants with it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ViewsOverride(pub usize);

pub(super) fn build(app: &mut App) {
    app.add_observer(mark::<GameCamera>)
        .add_observer(mark::<MenuCamera>)
        .add_observer(decorate)
        .add_systems(Update, apply_quality.in_set(VisualSet));
}

fn mark<C: Component>(add: On<Add<C>>, mut commands: Commands) {
    commands.entity(add.entity).insert(QualityCamera);
}

fn decorate(add: On<Add<QualityCamera>>, mut commands: Commands) {
    commands
        .entity(add.entity)
        .insert((Hdr, Tonemapping::TonyMcMapface, Msaa::Off));
}

/// Number of player views: game cameras on screen (menus: 1). While Loading, before the game
/// cameras exist, the race's local players: the warm-up camera must compile the split-screen
/// variants the race will use.
pub(crate) fn views(game_cameras: usize, loading_players: Option<usize>) -> usize {
    match loading_players {
        Some(players) if game_cameras == 0 => players.clamp(1, 4),
        _ => game_cameras.max(1),
    }
}

/// Local players of the race being loaded (`None` outside `Loading`).
pub(crate) fn loading_players(
    state: Option<&State<AppState>>,
    config: Option<&RaceConfig>,
) -> Option<usize> {
    let loading = state.is_some_and(|s| *s.get() == AppState::Loading);
    let humans = |c: &RaceConfig| {
        c.racers
            .iter()
            .filter(|r| matches!(r.controller, Controller::Human { .. }))
            .count()
    };
    loading.then(|| config.map_or(1, humans))
}

#[allow(clippy::type_complexity)]
fn apply_quality(
    mut commands: Commands,
    settings: Option<Res<Settings>>,
    state: Option<Res<State<AppState>>>,
    config: Option<Res<RaceConfig>>,
    cameras: Query<(Entity, Option<&Applied>, Option<&ViewsOverride>), With<QualityCamera>>,
    game: Query<(), With<GameCamera>>,
) {
    let graphics = settings.map(|s| s.graphics.clone()).unwrap_or_default();
    let players = loading_players(state.as_deref(), config.as_deref());
    let on_screen = views(game.iter().count(), players);
    for (entity, applied, forced) in &cameras {
        let quality = view_quality(&graphics, forced.map_or(on_screen, |f| f.0));
        let old = applied.map(|a| a.0);
        if old == Some(quality) {
            continue;
        }
        let mut camera = commands.entity(entity);
        apply(&mut camera, old, quality);
        camera.insert(Applied(quality));
    }
}

/// Inserts/removes the components that differ between `old` and `new`.
fn apply(camera: &mut EntityCommands, old: Option<ViewQuality>, new: ViewQuality) {
    // Anti-aliasing.
    camera.remove::<(Fxaa, Smaa, TemporalAntiAliasing)>();
    if old.is_some_and(|o| o.aa == AaMode::Taa) && new.aa != AaMode::Taa {
        // TAA's required components stay behind otherwise: a jittered, sharpened image.
        camera.remove::<(TemporalJitter, MipBias)>();
    }
    match new.aa {
        AaMode::Off => {}
        AaMode::Fxaa => {
            camera.insert(Fxaa::default());
        }
        AaMode::Smaa(preset) => {
            camera.insert(Smaa { preset });
        }
        AaMode::Taa => {
            camera.insert(TemporalAntiAliasing::default());
        }
    }
    // Gaussian everywhere: `Temporal` needs many frames of TAA to converge and shows stepped
    // shadow edges in stills (captures, the first frames after a cut).
    camera.insert(ShadowFilteringMethod::Gaussian);

    match new.ssao {
        Some(quality_level) => {
            camera.insert(ScreenSpaceAmbientOcclusion {
                quality_level,
                radius: SSAO_RADIUS_U,
                constant_object_thickness: SSAO_THICKNESS_U,
            });
        }
        None => {
            camera.remove::<ScreenSpaceAmbientOcclusion>();
        }
    }
    toggle(camera, new.bloom, Bloom::NATURAL);
    toggle(
        camera,
        new.motion_blur,
        MotionBlur {
            shutter_angle: MOTION_BLUR_SHUTTER,
            samples: MOTION_BLUR_SAMPLES,
        },
    );
    toggle(camera, new.contact_shadows, ContactShadows::default());
    toggle(
        camera,
        new.volumetric_fog,
        VolumetricFog {
            ambient_intensity: 0.0,
            step_count: VOLUMETRIC_STEPS,
            jitter: 0.5,
            ..default()
        },
    );

    // Drop prepasses this module needed before and no longer needs (never ones it did not add).
    if let Some(old) = old {
        let (depth, normal, motion) = prepasses(old);
        let (new_depth, new_normal, new_motion) = prepasses(new);
        if depth && !new_depth {
            camera.remove::<DepthPrepass>();
        }
        if normal && !new_normal {
            camera.remove::<NormalPrepass>();
        }
        if motion && !new_motion {
            camera.remove::<MotionVectorPrepass>();
        }
    }
}

fn toggle<C: Component>(camera: &mut EntityCommands, on: bool, component: C) {
    if on {
        camera.insert(component);
    } else {
        camera.remove::<C>();
    }
}

/// Prepasses (depth, normal, motion vectors) the effects of `q` require.
fn prepasses(q: ViewQuality) -> (bool, bool, bool) {
    let taa = q.aa == AaMode::Taa;
    (
        taa || q.ssao.is_some() || q.motion_blur || q.contact_shadows,
        q.ssao.is_some(),
        taa || q.motion_blur,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepasses_cover_every_effect_that_needs_them() {
        let base = view_quality(&GraphicsSettings::default(), 1);
        assert_eq!(prepasses(base), (true, true, true));
        let none = ViewQuality {
            aa: AaMode::Fxaa,
            ssao: None,
            bloom: true,
            motion_blur: false,
            contact_shadows: false,
            volumetric_fog: false,
        };
        assert_eq!(prepasses(none), (false, false, false));
        let blur = ViewQuality {
            motion_blur: true,
            ..none
        };
        assert_eq!(prepasses(blur), (true, false, true));
    }

    #[test]
    fn menus_count_as_one_view() {
        assert_eq!(views(0, None), 1);
        assert_eq!(views(3, None), 3);
        assert_eq!(views(0, Some(2)), 2, "warm-up of a 2-player race");
        assert_eq!(views(2, Some(2)), 2);
        assert_eq!(views(0, Some(0)), 1);
    }
}
