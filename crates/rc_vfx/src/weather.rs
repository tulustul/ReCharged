//! Weather effects per game camera (ARCHITECTURE §5.8, §5.10): every `GameCamera` sees layer 0
//! plus its own [`player_layer`]; its rain or snow mesh (`assets/shaders/vfx/precip.wesl`) and
//! its sky lightning ([`crate::lightning`]) live on that layer only, so split-screen views never
//! see each other's precipitation. Snow vs rain follows `WeatherState::snowing` (not `kind`), density
//! `precipitation` × the particle setting. Splashes on wet ground: [`crate::splashes`].
//! Indoor tracks show weather only through their windows (rc_render): nothing here.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use rc_core::prelude::*;

use crate::fx::{Fx, FxRng};

/// The precipitation shader.
const PRECIP_SHADER: &str = "shaders/vfx/precip.wesl";
/// Drops in each camera's precipitation mesh (the `amount` uniform thins them out).
pub const MAX_DROPS: u32 = 14_000;
/// Rain fall speed at world scale (≈ 9 m/s × 4.44), u/s.
const RAIN_FALL_UPS: f32 = 40.0;
/// Snow fall speed, u/s.
const SNOW_FALL_UPS: f32 = 4.0;

/// Render layer of a player's private effects: precipitation, lightning, name tags, markers.
pub fn player_layer(slot: PlayerSlot) -> usize {
    1 + usize::from(slot.0)
}

/// Rain streaks / snowflakes around one camera (see `precip.wesl`).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct PrecipMaterial {
    /// xyz: fall velocity (u/s), w: precipitation clock (s).
    #[uniform(0)]
    pub motion: Vec4,
    /// x: box width, y: box height, z: streak length, w: streak width / flake size (u).
    #[uniform(1)]
    pub shape: Vec4,
    /// Colour (lighting folded in) and opacity.
    #[uniform(2)]
    pub color: LinearRgba,
    /// x: amount 0–1, y: 1 = snow, z: flake wobble (u).
    #[uniform(3)]
    pub params: Vec4,
}

impl Material for PrecipMaterial {
    fn vertex_shader() -> ShaderRef {
        PRECIP_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        PRECIP_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
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
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_1.at_shader_location(2),
        ])?];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// What a camera's precipitation looks like right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrecipLook {
    /// Snowflakes (else rain streaks).
    pub snow: bool,
    /// Fraction of the drops shown.
    pub amount: f32,
    /// Fall velocity with the wind, u/s.
    pub fall: Vec3,
    /// Colour with the scene's light level folded in, and opacity.
    pub color: LinearRgba,
    /// Box (width, height), streak length, width/size (u).
    pub shape: Vec4,
}

/// Brightness of unlit weather by time of day and cloud cover (night rain must not glow).
fn ambient(w: &WeatherState) -> f32 {
    let day = match w.time {
        TimeOfDay::Noon => 1.0,
        TimeOfDay::Morning | TimeOfDay::Sunset => 0.7,
        TimeOfDay::Night => 0.22,
    };
    day * (1.0 - 0.35 * w.cloudiness.clamp(0.0, 1.0))
}

/// Precipitation of one of `views` cameras for `w` at particle `density` (0.35–1); `None` when
/// nothing falls here (dry, or an indoor track whose weather is only outside).
pub fn precip_look(w: &WeatherState, density: f32, views: usize) -> Option<PrecipLook> {
    if w.outside_only || w.precipitation <= 0.01 {
        return None;
    }
    let split = match views {
        0 | 1 => 1.0,
        2 => 0.7,
        _ => 0.5,
    };
    let p = w.precipitation.clamp(0.0, 1.0);
    let amount = (p * density * split).clamp(0.0, 1.0);
    let light = ambient(w);
    let wind = Vec3::new(w.wind.x, 0.0, w.wind.z);
    Some(if w.snowing {
        PrecipLook {
            snow: true,
            // Flakes are small and slow: more of them read as the same intensity as rain.
            amount: (amount * 1.5).min(1.0),
            fall: Vec3::NEG_Y * SNOW_FALL_UPS + wind * 0.8,
            color: LinearRgba::new(0.95 * light, 0.97 * light, light, 0.85),
            shape: Vec4::new(24.0, 14.0, 0.0, 0.04),
        }
    } else {
        PrecipLook {
            snow: false,
            amount,
            fall: Vec3::NEG_Y * RAIN_FALL_UPS + wind,
            color: LinearRgba::new(0.62 * light, 0.68 * light, 0.76 * light, 0.26 + 0.2 * p),
            shape: Vec4::new(28.0, 18.0, 0.6 + 0.3 * p, 0.022),
        }
    })
}

/// The precipitation entity of one camera.
#[derive(Component, Debug)]
struct Precip {
    camera: Entity,
    material: Handle<PrecipMaterial>,
}

/// Shared precipitation mesh and the effects clock (stops while paused).
#[derive(Resource, Debug, Default)]
pub(crate) struct WeatherFx {
    pub(crate) mesh: Handle<Mesh>,
    clock: f32,
}

pub(crate) fn build(app: &mut App) {
    app.add_plugins(MaterialPlugin::<PrecipMaterial>::default())
        .init_resource::<WeatherFx>()
        .add_systems(Startup, build_mesh)
        .add_observer(decorate_camera)
        .add_systems(Update, update_precip.in_set(crate::VfxSystems::Emit));
}

fn build_mesh(mut meshes: ResMut<Assets<Mesh>>, mut state: ResMut<WeatherFx>) {
    state.mesh = meshes.add(precip_mesh(MAX_DROPS, &mut FxRng::new(0xD20F)));
}

/// `n` quads, each with one random seed position, corner UVs and (rank, size variation).
pub fn precip_mesh(n: u32, rng: &mut FxRng) -> Mesh {
    let mut seeds = Vec::with_capacity(n as usize * 4);
    let mut corners = Vec::with_capacity(n as usize * 4);
    let mut extra = Vec::with_capacity(n as usize * 4);
    let mut indices = Vec::with_capacity(n as usize * 6);
    for i in 0..n {
        let seed = [rng.unit(), rng.unit(), rng.unit()];
        // Ranks spread evenly, so `amount` thins the drops out uniformly.
        let rank = (i as f32 + rng.unit()) / n as f32;
        let var = rng.unit();
        for corner in [[-0.5, 0.0], [0.5, 0.0], [0.5, 1.0], [-0.5, 1.0]] {
            seeds.push(seed);
            corners.push(corner);
            extra.push([rank, var]);
        }
        let b = i * 4;
        indices.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, seeds)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, corners)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, extra)
    .with_inserted_indices(Indices::U32(indices))
}

/// Gives each game camera its private layer and its precipitation.
fn decorate_camera(
    add: On<Add<GameCamera>>,
    cameras: Query<&GameCamera>,
    state: Res<WeatherFx>,
    mut materials: ResMut<Assets<PrecipMaterial>>,
    mut commands: Commands,
) {
    let Ok(cam) = cameras.get(add.entity) else {
        return;
    };
    let layer = player_layer(cam.slot);
    commands
        .entity(add.entity)
        .insert(RenderLayers::from_layers(&[0, layer]));
    let material = materials.add(PrecipMaterial::default());
    commands.spawn((
        Name::new(format!("Precipitation P{}", cam.slot.0 + 1)),
        Precip {
            camera: add.entity,
            material: material.clone(),
        },
        Mesh3d(state.mesh.clone()),
        MeshMaterial3d(material),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(layer),
        NoFrustumCulling,
        NotShadowCaster,
        DespawnOnExit(RaceSession),
    ));
}

#[allow(clippy::too_many_arguments)]
fn update_precip(
    time: Res<Time>,
    weather: Option<Res<WeatherState>>,
    fx: Res<Fx>,
    mut state: ResMut<WeatherFx>,
    mut commands: Commands,
    cameras: Query<(), With<GameCamera>>,
    mut precips: Query<(Entity, &Precip, &mut Visibility)>,
    mut materials: ResMut<Assets<PrecipMaterial>>,
) {
    state.clock += time.delta_secs();
    let look = weather
        .as_deref()
        .and_then(|w| precip_look(w, fx.density, fx.views));
    for (entity, precip, mut visibility) in &mut precips {
        if !cameras.contains(precip.camera) {
            commands.entity(entity).despawn();
            continue;
        }
        visibility.set_if_neq(if look.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let (Some(look), Some(mut m)) = (look, materials.get_mut(&precip.material)) else {
            continue;
        };
        m.motion = look.fall.extend(state.clock);
        m.shape = look.shape;
        m.color = look.color;
        m.params = Vec4::new(look.amount, if look.snow { 1.0 } else { 0.0 }, 0.35, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rain() -> WeatherState {
        WeatherState {
            kind: WeatherKind::Rain,
            precipitation: 0.6,
            wetness: 1.0,
            ..default()
        }
    }

    #[test]
    fn snow_or_rain_follows_snowing_not_kind() {
        let mut w = rain();
        assert!(!precip_look(&w, 1.0, 1).expect("rain").snow);
        // Snow fading out after the kind already turned Cloudy is still snow.
        w.kind = WeatherKind::Cloudy;
        w.snowing = true;
        let snow = precip_look(&w, 1.0, 1).expect("snow");
        assert!(snow.snow && snow.fall.y > -10.0, "{snow:?}");
    }

    #[test]
    fn density_follows_precipitation_quality_and_views() {
        let w = rain();
        let full = precip_look(&w, 1.0, 1).expect("rain").amount;
        assert!((full - 0.6).abs() < 1e-5);
        assert!(precip_look(&w, 0.35, 1).expect("rain").amount < full);
        assert!(
            precip_look(&w, 1.0, 4).expect("rain").amount < full,
            "split-screen is cheaper"
        );
        let dry = WeatherState {
            precipitation: 0.0,
            ..rain()
        };
        assert!(precip_look(&dry, 1.0, 1).is_none());
        let indoor = WeatherState {
            outside_only: true,
            ..rain()
        };
        assert!(
            precip_look(&indoor, 1.0, 1).is_none(),
            "indoors it rains only outside"
        );
    }

    #[test]
    fn wind_slants_the_rain_and_night_dims_it() {
        let w = WeatherState {
            wind: Vec3::new(14.0, 0.0, 0.0),
            ..rain()
        };
        let look = precip_look(&w, 1.0, 1).expect("rain");
        let slant = look.fall.x.atan2(-look.fall.y).to_degrees();
        assert!(
            (15.0..25.0).contains(&slant),
            "storm wind slants rain ≈ 20°: {slant}"
        );
        let night = WeatherState {
            time: TimeOfDay::Night,
            ..rain()
        };
        let dim = precip_look(&night, 1.0, 1).expect("rain").color;
        assert!(dim.red < look.color.red * 0.5);
    }

    #[test]
    fn precipitation_mesh_ranks_cover_the_range() {
        let mesh = precip_mesh(100, &mut FxRng::new(1));
        assert_eq!(mesh.count_vertices(), 400);
        assert_eq!(mesh.indices().map(|i| i.len()), Some(600));
    }

    #[test]
    fn each_camera_sees_only_its_own_precipitation() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<PrecipMaterial>()
            .init_resource::<WeatherFx>()
            .add_observer(decorate_camera);
        let cams: Vec<Entity> = (0..2)
            .map(|slot| {
                app.world_mut()
                    .spawn(GameCamera {
                        slot: PlayerSlot(slot),
                    })
                    .id()
            })
            .collect();
        app.update();
        let world = app.world_mut();
        let layers =
            |e: Entity, world: &World| world.get::<RenderLayers>(e).cloned().expect("layers");
        let mut precips = world.query::<(&Precip, &RenderLayers)>();
        let found: Vec<(Entity, RenderLayers)> = precips
            .iter(world)
            .map(|(p, l)| (p.camera, l.clone()))
            .collect();
        assert_eq!(found.len(), 2, "one precipitation mesh per camera");
        for (camera, precip_layers) in &found {
            for &other in &cams {
                let sees = layers(other, world).intersects(precip_layers);
                assert_eq!(
                    sees,
                    other == *camera,
                    "camera {other} vs precipitation of {camera}"
                );
            }
        }
        assert!(
            layers(cams[0], world).intersects(&RenderLayers::layer(0)),
            "and the world"
        );
    }
}
