//! Environments (GDD §10.1, art bible §3): which [`EnvironmentDef`] is active (the menu's
//! `showroom`, or the session track's `TrackInfo.environment`), its directional lights (key =
//! sun / window light / moon, fill, lightning flash), the outdoor atmosphere and cloud dome, the
//! cameras' look ([`camera`]), indoor rooms ([`room`]) and night headlights ([`headlights`]).
//! Also owns the `"visuals"` loading key: done once the session's textures are loaded and its
//! render pipelines are compiled ([`warmup`]).

mod camera;
mod env;
pub(crate) mod gi;
pub(crate) mod glass;
mod headlights;
mod menu_warmup;
mod room;
mod volumetric;
mod warmup;

use bevy::light::IrradianceVolume;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::materials::MaterialLibrary;
use crate::quality::shadow_tier;
use crate::sky::clouds::{self, CloudMaterial};
use crate::sky::{atmosphere, light_transform};
use crate::weather_fx::{LightLook, Look, LookSet};

pub use env::*;
pub use room::environment_radiance;
pub use warmup::PipelineQueue;
pub(crate) use warmup::WarmupCamera;

/// Environment id of the menu showroom ([`SHOWROOM_FILE`]).
pub const SHOWROOM_ENV: &str = "showroom";
/// Map whose neutral environment is used when a track's environment file is missing or broken.
pub const FALLBACK_MAP: &str = "test_oval";
/// First shadow cascade's far bound (u): the car and its surroundings get the sharpest texels.
const FIRST_CASCADE_U: f32 = 9.0;
/// Shadow distance under the Loading warm-up camera, × its height: the far corners of the track
/// are farther than the ground straight below.
const WARMUP_SHADOW_REACH: f32 = 2.0;

/// The environment being rendered: the menu showroom or the session's track environment.
/// Inserted in `OnEnter(Menu)` / `OnEnter(Loading)`, removed when they end.
#[derive(Resource, Debug, Clone)]
pub struct ActiveEnvironment {
    /// Environment id (file stem).
    pub id: String,
    /// Its definition.
    pub def: EnvironmentDef,
    /// Time of day (the race's; Noon in menus).
    pub time: TimeOfDay,
}

/// Role of one of the environment's directional lights.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvLight {
    /// Sun / window light / moon (casts shadows).
    Key,
    /// Moon / sky bounce, no shadows: a stand-in for indirect light, so it is off where a baked
    /// irradiance volume provides that (`gi.rs`).
    Fill,
    /// Lightning, no shadows.
    Flash,
}

/// The session's `"visuals"` key is still pending.
#[derive(Resource, Debug)]
struct VisualsPending;

pub(crate) fn build(app: &mut App) {
    app.init_resource::<gi::BakedLight>()
        .add_systems(OnEnter(AppState::Menu), enter_menu)
        .add_systems(OnEnter(AppState::Loading), enter_session)
        .add_systems(OnExit(AppState::Menu), clear_environment)
        .add_systems(OnEnter(AppState::Unloading), clear_environment)
        // A session left during Loading (quit, load error) drops its pending warm-up too.
        .add_systems(OnExit(AppState::Loading), |mut commands: Commands| {
            commands.remove_resource::<VisualsPending>();
            commands.remove_resource::<warmup::Warmup>();
        })
        .add_systems(
            Update,
            (
                (warmup::start, report_ready)
                    .chain()
                    .run_if(resource_exists::<VisualsPending>),
                (
                    apply_lights,
                    apply_shadows,
                    drive_clouds,
                    gi::follow_weather,
                )
                    .in_set(VisualSet)
                    .after(LookSet),
                (gi::spawn_volume, gi::switch_volume).chain(),
            ),
        );
    camera::build(app);
    glass::build(app);
    room::build(app);
    headlights::build(app);
    volumetric::build(app);
    warmup::build(app);
    menu_warmup::build(app);
}

fn enter_menu(mut commands: Commands, paths: Option<Res<UserPaths>>, spawner: EnvSpawner) {
    let root = paths
        .map(|p| p.assets_dir.clone())
        .unwrap_or_else(|| "assets".into());
    match EnvironmentDef::load_file(&root.join(SHOWROOM_FILE)) {
        Ok(def) => {
            let active = ActiveEnvironment {
                id: SHOWROOM_ENV.into(),
                def,
                time: TimeOfDay::Noon,
            };
            spawner.spawn(&mut commands, &active, DespawnOnExit(AppState::Menu));
            commands.insert_resource(active);
        }
        // The menu still works without its lighting file (React shows its 2D backdrop).
        Err(e) => error!("showroom environment: {e}"),
    }
}

/// `OnEnter(Loading)`: activates the track's environment, rebuilds the material library for it
/// and spawns its lights. A missing or broken environment file falls back to the neutral
/// one of [`FALLBACK_MAP`] (logged as an error); only when that fails too does the load fail.
#[allow(clippy::too_many_arguments)]
fn enter_session(
    mut commands: Commands,
    config: Option<Res<RaceConfig>>,
    catalog: Option<Res<Catalog>>,
    paths: Option<Res<UserPaths>>,
    mut library: Option<ResMut<MaterialLibrary>>,
    mut surfaces: ResMut<Assets<crate::materials::SurfaceMaterial>>,
    mut failed: MessageWriter<LoadingFailed>,
    spawner: EnvSpawner,
) {
    let root = paths
        .map(|p| p.assets_dir.clone())
        .unwrap_or_else(|| "assets".into());
    let track = config
        .as_ref()
        .map(|c| c.variant.track.clone())
        .unwrap_or_default();
    let env_id = catalog
        .as_ref()
        .and_then(|c| c.tracks.iter().find(|t| t.id == track))
        .map_or_else(|| "test".to_owned(), |t| t.environment.clone());
    let loaded = EnvironmentDef::load(&root, track.as_str()).or_else(|error| {
        error!("visuals: {error}; using the '{FALLBACK_MAP}' environment");
        EnvironmentDef::load(&root, FALLBACK_MAP)
    });
    let def = match loaded {
        Ok(def) => def,
        Err(error) => {
            error!("visuals: {error}");
            failed.write(LoadingFailed {
                key: crate::LOADING_KEY,
                error,
            });
            return;
        }
    };
    if let Some(library) = library.as_mut() {
        let missing =
            library.set_environment(Some(&env_id), &def.materials, &def.surfaces, &mut surfaces);
        for name in missing {
            warn!("environment {env_id}: unknown material '{name}'");
        }
    }
    let time = config.map_or(TimeOfDay::Noon, |c| c.weather.time);
    info!("visuals: environment {env_id} ({}), {time:?}", def.name);
    let active = ActiveEnvironment {
        id: env_id,
        def,
        time,
    };
    spawner.spawn(&mut commands, &active, DespawnOnExit(RaceSession));
    commands.insert_resource(active);
    commands.insert_resource(VisualsPending);
}

fn clear_environment(mut commands: Commands) {
    commands.remove_resource::<ActiveEnvironment>();
}

/// `"visuals"` is done when every texture of the library is on the CPU and the pipeline warm-up
/// is over.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn report_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    time: Res<Time<Real>>,
    (library, track, tasks): (
        Option<Res<MaterialLibrary>>,
        Option<Res<rc_track::Track>>,
        Option<Res<LoadingTasks>>,
    ),
    queue: Res<warmup::PipelineQueue>,
    mut progress: Option<ResMut<warmup::Warmup>>,
    warmup_cameras: Query<(Has<EnvironmentMapLight>, Option<&Children>), With<WarmupCamera>>,
    cars: Query<Has<crate::cars::CarModelReady>, With<crate::cars::CarModel>>,
    mut done: MessageWriter<LoadingDone>,
) {
    if library.is_some_and(|l| l.textures_pending(&server) > 0) {
        return;
    }
    // The scene is complete once every other loader is done (props, cars) and the warm-up cars
    // are instanced; until then the camera already compiles what has arrived.
    let others_done = tasks.is_some_and(|t| t.pending().all(|k| k == crate::LOADING_KEY));
    if !warmup::finished(
        progress.as_deref_mut(),
        &queue,
        time.delta_secs(),
        track.is_some(),
        others_done && warmup::scene_ready(&warmup_cameras, &cars),
    ) {
        return;
    }
    done.write(LoadingDone(crate::LOADING_KEY));
    commands.remove_resource::<VisualsPending>();
    commands.remove_resource::<warmup::Warmup>();
}

/// Spawns an environment's lights (and outdoors its sky) with a lifetime scope.
#[derive(bevy::ecs::system::SystemParam)]
struct EnvSpawner<'w> {
    media: ResMut<'w, Assets<ScatteringMedium>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    clouds: ResMut<'w, Assets<CloudMaterial>>,
}

impl EnvSpawner<'_> {
    fn spawn(
        mut self,
        commands: &mut Commands,
        env: &ActiveEnvironment,
        scope: impl Component + Clone,
    ) {
        for (role, name) in [
            (EnvLight::Key, "Key light"),
            (EnvLight::Fill, "Fill light"),
            (EnvLight::Flash, "Lightning light"),
        ] {
            commands.spawn((
                Name::new(name),
                role,
                DirectionalLight {
                    illuminance: 0.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                scope.clone(),
            ));
        }
        for light in env.def.lights.iter().filter(|l| l.on_at(env.time)) {
            let color = Color::srgb(light.color[0], light.color[1], light.color[2]);
            let pos = Vec3::from_array(light.pos);
            let intensity = lamp_lumens(light.lumens);
            match light.target {
                Some(target) => {
                    let outer = light.angle.to_radians();
                    commands.spawn((
                        Name::new("Environment spot"),
                        SpotLight {
                            color,
                            intensity,
                            range: light.range,
                            outer_angle: outer,
                            inner_angle: outer * 0.6,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_translation(pos)
                            .looking_at(Vec3::from_array(target), Vec3::Y),
                        scope.clone(),
                    ));
                }
                None => {
                    commands.spawn((
                        Name::new("Environment light"),
                        PointLight {
                            color,
                            intensity,
                            range: light.range,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_translation(pos),
                        scope.clone(),
                    ));
                }
            }
        }
        if !env.def.indoor {
            commands.spawn((
                Name::new("Atmosphere"),
                atmosphere(&mut self.media, env.def.sky.ground_albedo),
                scope.clone(),
            ));
            commands.spawn((
                clouds::dome(&mut self.meshes, &mut self.clouds, Vec3::ZERO),
                scope,
            ));
        }
    }
}

fn set_light(
    light: &mut DirectionalLight,
    transform: &mut Transform,
    look: Option<&LightLook>,
    shadows: bool,
) {
    let Some(look) = look else {
        light.illuminance = 0.0;
        light.shadow_maps_enabled = false;
        return;
    };
    light.illuminance = look.lux;
    light.color = look.color;
    light.shadow_maps_enabled = shadows && look.shadows && look.lux > 0.0;
    *transform = light_transform(look.toward);
}

fn apply_lights(
    look: Option<Res<Look>>,
    settings: Option<Res<Settings>>,
    game: Query<(), With<GameCamera>>,
    baked: Query<(), (With<gi::BakedGi>, With<IrradianceVolume>)>,
    mut lights: Query<(&EnvLight, &mut DirectionalLight, &mut Transform)>,
) {
    let Some(look) = look else {
        return;
    };
    let graphics = settings.map(|s| s.graphics.clone()).unwrap_or_default();
    let tier = shadow_tier(&graphics, game.iter().count().max(1));
    for (role, mut light, mut transform) in &mut lights {
        match role {
            EnvLight::Key => set_light(&mut light, &mut transform, Some(&look.key), tier.enabled),
            EnvLight::Fill => {
                // The baked volume carries the sky's bounce, with occlusion; the fill would light
                // everything again, through decks and under tables.
                let fill = look.fill.as_ref().filter(|_| baked.is_empty());
                set_light(&mut light, &mut transform, fill, false);
            }
            EnvLight::Flash => {
                let flash = (look.flash.lux > 0.0).then_some(&look.flash);
                set_light(&mut light, &mut transform, flash, false);
            }
        }
        // Contact shadows only from the key light (the screen-space pass is per light).
        light.contact_shadows_enabled = *role == EnvLight::Key && light.shadow_maps_enabled;
    }
}

/// Shadow-map size and the key light's cascades from the quality tier and the environment.
#[allow(clippy::too_many_arguments)]
fn apply_shadows(
    mut commands: Commands,
    env: Option<Res<ActiveEnvironment>>,
    settings: Option<Res<Settings>>,
    state: Option<Res<State<AppState>>>,
    config: Option<Res<RaceConfig>>,
    game: Query<(), With<GameCamera>>,
    keys: Query<(Entity, &EnvLight)>,
    added: Query<(), Added<EnvLight>>,
    map: Option<ResMut<DirectionalLightShadowMap>>,
    warmup: Query<&Transform, With<WarmupCamera>>,
    mut last: Local<Option<(crate::quality::ShadowTier, f32)>>,
) {
    let Some(env) = env else {
        return;
    };
    let graphics = settings.map(|s| s.graphics.clone()).unwrap_or_default();
    let players = crate::quality::loading_players(state.as_deref(), config.as_deref());
    let tier = shadow_tier(
        &graphics,
        crate::quality::views(game.iter().count(), players),
    );
    let distance = cascade_distance(
        env.def.shadow_distance * tier.distance_scale,
        warmup.iter().map(|t| t.translation.y).reduce(f32::max),
    );
    if *last == Some((tier, distance)) && added.is_empty() {
        return;
    }
    *last = Some((tier, distance));
    match map {
        Some(mut map) => map.size = tier.map_size,
        None => commands.insert_resource(DirectionalLightShadowMap {
            size: tier.map_size,
        }),
    }
    let config = CascadeShadowConfigBuilder {
        num_cascades: tier.cascades,
        minimum_distance: 0.1,
        maximum_distance: distance,
        first_cascade_far_bound: FIRST_CASCADE_U.min(distance * 0.5),
        overlap_proportion: 0.2,
    }
    .build();
    for (entity, role) in &keys {
        if *role == EnvLight::Key {
            commands.entity(entity).insert(config.clone());
        }
    }
}

/// The key light's shadow distance (u). While the Loading warm-up camera looks down from
/// `warmup_height`, the cascades reach past the ground under it, so every prop's shadow pipeline
/// compiles under the loading screen instead of when the race camera first sees it.
fn cascade_distance(distance: f32, warmup_height: Option<f32>) -> f32 {
    warmup_height.map_or(distance, |h| distance.max(h * WARMUP_SHADOW_REACH))
}

fn drive_clouds(
    time: Res<Time>,
    look: Option<Res<Look>>,
    domes: Query<&MeshMaterial3d<CloudMaterial>, With<clouds::CloudDome>>,
    mut materials: ResMut<Assets<CloudMaterial>>,
) {
    let Some(look) = look else {
        return;
    };
    for handle in &domes {
        if let Some(mut material) = materials.get_mut(handle) {
            clouds::update(&mut material, &look.clouds, time.delta_secs());
        }
    }
}
