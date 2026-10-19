//! Sky lightning per game camera on `WeatherEvent::Lightning` (outdoor tracks): a bolt ribbon
//! (main channel + branches, midpoint displacement) far ahead of each camera, on its own layer,
//! flickering through a strike and two re-strikes. *Reduce flashing* shows a slow soft glow over
//! the horizon instead. The sky's flash itself is rc_render's.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use rc_core::prelude::*;

use bevy::mesh::MeshTag;

use crate::assets::VfxAssets;
use crate::blasts::bolt_path;
use crate::cars::FxVisual;
use crate::fx::{Fx, sprite};
use crate::particles::{Facing, Look, Shape, pack_tag};
use crate::weather::player_layer;

/// Sky bolts strike this far from the camera (u) and last this long (s).
const BOLT_DISTANCE_U: (f32, f32) = (150.0, 230.0);
const BOLT_LIFE_S: f32 = 0.5;
/// *Reduce flashing*: the soft glow swells and fades over this long (s).
const GLOW_LIFE_S: f32 = 1.0;

/// One sky bolt for one camera.
#[derive(Component, Debug)]
struct SkyBolt {
    material: Handle<StandardMaterial>,
    age: f32,
    peak: LinearRgba,
}

/// *Reduce flashing*: a soft glow over the horizon for one camera instead of a bolt.
#[derive(Component, Debug)]
struct SkyGlow {
    age: f32,
    peak: LinearRgba,
}

pub(crate) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (sky_lightning, fade_bolts, fade_glows).in_set(crate::VfxSystems::Emit),
    );
}

/// Brightness of a sky bolt `t` s after it struck: a strike and two weaker re-strikes.
pub fn strike_brightness(t: f32) -> f32 {
    if !(0.0..BOLT_LIFE_S).contains(&t) {
        return 0.0;
    }
    [(0.0, 1.0), (0.11, 0.7), (0.24, 0.45)]
        .iter()
        .filter(|(start, _)| t >= *start)
        .map(|(start, amp)| amp * (-(t - start) * 16.0).exp())
        .sum::<f32>()
        .min(1.0)
}

/// A camera-facing ribbon mesh along bolt `paths` (`(points, width)`), seen from `eye`.
pub(crate) fn bolt_mesh(paths: &[(Vec<Vec3>, f32)], eye: Vec3) -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for (points, width) in paths {
        for w in points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let side = (b - a).cross(eye - a).normalize_or(Vec3::X) * (width * 0.5);
            let base = positions.len() as u32;
            positions.extend([a - side, a + side, b + side, b - side].map(|p| p.to_array()));
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    let normals = vec![[0.0, 0.0, 1.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// The additive material of a bolt at brightness `peak` (faded by `fade_bolts`).
pub(crate) fn bolt_material(peak: LinearRgba) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::from(peak),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

#[allow(clippy::too_many_arguments)]
fn sky_lightning(
    mut events: MessageReader<WeatherEvent>,
    weather: Option<Res<WeatherState>>,
    mut fx: ResMut<Fx>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<VfxAssets>,
    cameras: Query<(&GameCamera, &GlobalTransform)>,
) {
    for ev in events.read() {
        let WeatherEvent::Lightning { intensity } = *ev else {
            continue;
        };
        if weather.as_deref().is_none_or(|w| w.outside_only) {
            continue;
        }
        for (cam, at) in &cameras {
            let eye = at.translation();
            let fwd = (at.forward().as_vec3() * Vec3::new(1.0, 0.0, 1.0)).normalize_or(Vec3::NEG_Z);
            let dir = Quat::from_rotation_y(fx.rng.signed(0.6)) * fwd;
            let ground =
                eye + dir * fx.rng.range(BOLT_DISTANCE_U.0, BOLT_DISTANCE_U.1) - Vec3::Y * 6.0;
            let top = ground
                + Vec3::new(
                    fx.rng.signed(12.0),
                    fx.rng.range(70.0, 90.0),
                    fx.rng.signed(12.0),
                );
            let layer = RenderLayers::layer(player_layer(cam.slot));
            if fx.calm {
                // A soft glow low over the horizon (where the view sees it) instead of a bolt.
                let peak = LinearRgba::rgb(0.5, 0.6, 0.9) * intensity;
                let look = Look {
                    shape: Shape::Dot,
                    emissive: true,
                    color: [LinearRgba::BLACK; 2],
                    ..Look::DEFAULT
                };
                let at = Transform::from_translation(ground + Vec3::Y * 24.0)
                    .with_scale(Vec3::splat(70.0));
                commands.spawn((
                    Name::new("Sky glow"),
                    FxVisual,
                    SkyGlow { age: 0.0, peak },
                    sprite(&assets, &look, at),
                    layer,
                    DespawnOnExit(RaceSession),
                ));
                continue;
            }
            let (mesh, peak) = {
                let main = bolt_path(top, ground, 6, 0.2, &mut fx.rng);
                let mut paths = vec![(main.clone(), 1.1)];
                for _ in 0..3 {
                    let i = 6 + (fx.rng.unit() * (main.len() / 2) as f32) as usize;
                    let from = main[i.min(main.len() - 2)];
                    let to = from
                        + Vec3::new(
                            fx.rng.signed(25.0),
                            -fx.rng.range(12.0, 28.0),
                            fx.rng.signed(25.0),
                        );
                    paths.push((bolt_path(from, to, 4, 0.25, &mut fx.rng), 0.5));
                }
                (
                    bolt_mesh(&paths, eye),
                    LinearRgba::rgb(9.0, 10.0, 16.0) * intensity.max(0.4),
                )
            };
            let material = materials.add(bolt_material(peak));
            commands.spawn((
                Name::new("Sky lightning"),
                FxVisual,
                SkyBolt {
                    material: material.clone(),
                    age: 0.0,
                    peak,
                },
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::default(),
                layer,
                NoFrustumCulling,
                NotShadowCaster,
                DespawnOnExit(RaceSession),
            ));
        }
    }
}

fn fade_bolts(
    time: Res<Time>,
    mut commands: Commands,
    mut bolts: Query<(Entity, &mut SkyBolt)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut bolt) in &mut bolts {
        bolt.age += time.delta_secs();
        if bolt.age > BOLT_LIFE_S {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(mut m) = materials.get_mut(&bolt.material) {
            m.base_color = Color::from(bolt.peak * strike_brightness(bolt.age));
        }
    }
}

/// The calm glow swells and fades on a slow half sine.
fn fade_glows(
    time: Res<Time>,
    mut commands: Commands,
    mut glows: Query<(Entity, &mut SkyGlow, &mut MeshTag)>,
) {
    for (entity, mut glow, mut tag) in &mut glows {
        glow.age += time.delta_secs();
        let t = glow.age / GLOW_LIFE_S;
        if t >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let b = (t * std::f32::consts::PI).sin();
        tag.value = pack_tag(glow.peak * b, true, Shape::Dot, Facing::Camera);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lightning_strikes_flicker_then_fade() {
        assert!((strike_brightness(0.0) - 1.0).abs() < 1e-5);
        assert!(
            strike_brightness(0.1) < strike_brightness(0.11),
            "a re-strike brightens it"
        );
        assert!(strike_brightness(0.45) < 0.05);
        assert_eq!(strike_brightness(BOLT_LIFE_S + 0.1), 0.0);
        assert_eq!(player_layer(PlayerSlot(0)), 1);
        assert_eq!(player_layer(PlayerSlot(3)), 4);
    }
}
