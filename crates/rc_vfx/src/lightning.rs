//! Sky lightning per game camera on `WeatherEvent::Lightning` (outdoor tracks): a bolt ribbon
//! (main channel + branches, midpoint displacement) far ahead of each camera, on its own layer,
//! flickering through a strike and two re-strikes. *Reduce flashing* shows a slow soft glow over
//! the horizon instead. The sky's flash itself is rc_render's.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use rc_core::prelude::*;

use bevy::mesh::MeshTag;

use crate::assets::VfxAssets;
use crate::blasts::bolt_path;
use crate::cars::FxVisual;
use crate::fx::{Fx, FxRng, sprite};
use crate::particles::{Facing, Look, Shape, pack_tag};
use crate::weather::player_layer;

/// The bolt shader (`assets/shaders/vfx/bolt.wesl`).
const BOLT_SHADER: &str = "shaders/vfx/bolt.wesl";
/// Sky bolts strike this far from the camera (u) and last this long (s).
const BOLT_DISTANCE_U: (f32, f32) = (150.0, 230.0);
const BOLT_LIFE_S: f32 = 0.5;
/// A bolt's top sits this far (radians) below the top edge of its camera's view, so the upper
/// part — above the houses, fences and trees that hide the lower part — and the crawlers along the
/// cloud base are in the picture; its height stays within this range (u).
const TOP_BELOW_VIEW_EDGE: f32 = 0.06;
const BOLT_HEIGHT_U: (f32, f32) = (45.0, 160.0);
/// Width of the main channel, of its downward branches and of the cloud crawlers (u).
const MAIN_WIDTH_U: f32 = 1.8;
const BRANCH_WIDTH_U: f32 = 0.8;
const CRAWLER_WIDTH_U: f32 = 0.9;
/// Horizontal reach of the crawlers that fork along the cloud base from the top of the bolt (u).
const CRAWLER_REACH_U: f32 = 55.0;
/// *Reduce flashing*: the soft glow swells and fades over this long (s).
const GLOW_LIFE_S: f32 = 1.0;

/// One sky bolt for one camera.
#[derive(Component, Debug)]
struct SkyBolt {
    material: Handle<BoltMaterial>,
    age: f32,
    peak: LinearRgba,
}

/// *Reduce flashing*: a soft glow over the horizon for one camera instead of a bolt.
#[derive(Component, Debug)]
struct SkyGlow {
    age: f32,
    peak: LinearRgba,
}

/// An unlit additive bolt that fades where it meets opaque geometry (soft particles).
pub(crate) type BoltMaterial = ExtendedMaterial<StandardMaterial, BoltExtension>;

/// The soft-fading fragment stage of [`BoltMaterial`].
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub(crate) struct BoltExtension {}

impl MaterialExtension for BoltExtension {
    fn fragment_shader() -> ShaderRef {
        BOLT_SHADER.into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

pub(crate) fn build(app: &mut App) {
    app.add_plugins(MaterialPlugin::<BoltMaterial>::default());
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

/// Height (u) of a bolt whose ground point lies `distance` u from the camera and 6 u below its
/// eye, for a camera pitched `pitch` (radians, + up) with vertical field of view `fov`: the top
/// just under the view's upper edge.
pub(crate) fn bolt_height(distance: f32, pitch: f32, fov: f32) -> f32 {
    let top = (pitch + fov * 0.5 - TOP_BELOW_VIEW_EDGE).clamp(0.15, 1.0);
    (6.0 + distance * top.tan()).clamp(BOLT_HEIGHT_U.0, BOLT_HEIGHT_U.1)
}

/// The paths of a sky bolt `height` u from the cloud base down to `ground` (`(points, width)`):
/// the main channel, three branches forking downward from its upper half, and two crawlers forking
/// almost horizontally along the cloud base from its top, where buildings and trees do not hide
/// them.
pub(crate) fn sky_bolt_paths(ground: Vec3, height: f32, rng: &mut FxRng) -> Vec<(Vec<Vec3>, f32)> {
    let top = ground + Vec3::new(rng.signed(12.0), height, rng.signed(12.0));
    let main = bolt_path(top, ground, 6, 0.2, rng);
    let mut paths = vec![(main.clone(), MAIN_WIDTH_U)];
    for _ in 0..3 {
        let i = 6 + (rng.unit() * (main.len() / 2) as f32) as usize;
        let from = main[i.min(main.len() - 2)];
        let to = from + Vec3::new(rng.signed(25.0), -rng.range(12.0, 28.0), rng.signed(25.0));
        paths.push((bolt_path(from, to, 4, 0.25, rng), BRANCH_WIDTH_U));
    }
    for _ in 0..2 {
        let from = main[1 + (rng.unit() * 3.0) as usize];
        let heading = Vec3::new(rng.signed(1.0), 0.0, rng.signed(1.0)).normalize_or(Vec3::X);
        let to = from + heading * rng.range(0.5, 1.0) * CRAWLER_REACH_U + Vec3::Y * rng.signed(6.0);
        paths.push((bolt_path(from, to, 5, 0.25, rng), CRAWLER_WIDTH_U));
    }
    paths
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
pub(crate) fn bolt_material(peak: LinearRgba) -> BoltMaterial {
    BoltMaterial {
        base: StandardMaterial {
            base_color: Color::from(peak),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            fog_enabled: false,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: BoltExtension {},
    }
}

#[allow(clippy::too_many_arguments)]
fn sky_lightning(
    mut events: MessageReader<WeatherEvent>,
    weather: Option<Res<WeatherState>>,
    mut fx: ResMut<Fx>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BoltMaterial>>,
    assets: Res<VfxAssets>,
    cameras: Query<(&GameCamera, &GlobalTransform, &Projection)>,
) {
    for ev in events.read() {
        let WeatherEvent::Lightning { intensity } = *ev else {
            continue;
        };
        if weather.as_deref().is_none_or(|w| w.outside_only) {
            continue;
        }
        for (cam, at, projection) in &cameras {
            let eye = at.translation();
            let look = at.forward().as_vec3();
            let fwd = (look * Vec3::new(1.0, 0.0, 1.0)).normalize_or(Vec3::NEG_Z);
            let dir = Quat::from_rotation_y(fx.rng.signed(0.6)) * fwd;
            let distance = fx.rng.range(BOLT_DISTANCE_U.0, BOLT_DISTANCE_U.1);
            let ground = eye + dir * distance - Vec3::Y * 6.0;
            let fov = match projection {
                Projection::Perspective(p) => p.fov,
                _ => std::f32::consts::FRAC_PI_4,
            };
            let height = bolt_height(distance, look.y.clamp(-1.0, 1.0).asin(), fov);
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
            let paths = sky_bolt_paths(ground, height, &mut fx.rng);
            let (mesh, peak) = (
                bolt_mesh(&paths, eye),
                LinearRgba::rgb(9.0, 10.0, 16.0) * intensity.max(0.4),
            );
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
    mut materials: ResMut<Assets<BoltMaterial>>,
) {
    for (entity, mut bolt) in &mut bolts {
        bolt.age += time.delta_secs();
        if bolt.age > BOLT_LIFE_S {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(mut m) = materials.get_mut(&bolt.material) {
            m.base.base_color = Color::from(bolt.peak * strike_brightness(bolt.age));
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

    /// The top sits just under the view's upper edge: a chase camera pitched 10° down with a 59°
    /// vertical field of view sees up to ≈ 19° above the horizon.
    #[test]
    fn sky_bolts_top_out_just_under_the_view_edge() {
        let (pitch, fov) = ((-10f32).to_radians(), 58.7f32.to_radians());
        let h = bolt_height(190.0, pitch, fov);
        let elevation = ((h - 6.0) / 190.0).atan().to_degrees();
        assert!((14.0..19.3).contains(&elevation), "top at {elevation}°");
        assert_eq!(
            bolt_height(190.0, 1.4, fov),
            BOLT_HEIGHT_U.1,
            "looking up: capped"
        );
        assert_eq!(bolt_height(150.0, -1.4, fov), bolt_height(150.0, -1.0, fov));
    }

    #[test]
    fn sky_bolts_strike_the_ground_and_crawl_along_the_cloud_base() {
        let mut rng = FxRng::new(11);
        let ground = Vec3::new(10.0, -6.0, -180.0);
        for _ in 0..20 {
            let paths = sky_bolt_paths(ground, 70.0, &mut rng);
            assert_eq!(paths.len(), 6, "main, 3 branches, 2 crawlers");
            let (main, width) = &paths[0];
            assert_eq!(*width, MAIN_WIDTH_U);
            assert_eq!(
                main.last(),
                Some(&ground),
                "the main channel strikes the ground point"
            );
            assert!(
                (main[0].y - ground.y - 70.0).abs() < 1e-4,
                "its top at the given height"
            );
            for (crawler, _) in &paths[4..] {
                let (a, b) = (crawler[0], *crawler.last().expect("points"));
                let reach = Vec2::new(b.x - a.x, b.z - a.z).length();
                assert!(
                    reach > 0.4 * CRAWLER_REACH_U,
                    "crawlers go sideways: {reach}"
                );
                assert!((b.y - a.y).abs() < 10.0, "…along the cloud base");
                assert!(a.y - ground.y > 60.0, "…from the top");
            }
        }
    }

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
