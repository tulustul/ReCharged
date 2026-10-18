//! Environments (GDD §10.1, art bible §3): which [`EnvironmentDef`] is active (the menu's
//! `showroom`, or the session track's `TrackInfo.environment`), its directional lights (key =
//! sun / window light / moon, fill, lightning flash), the outdoor atmosphere and cloud dome, the
//! cameras' look ([`camera`]), indoor rooms ([`room`]) and night headlights ([`headlights`]).
//! Also owns the `"visuals"` loading key: done once the session's textures are loaded and its
//! render pipelines are compiled ([`warmup`]).

mod camera;
mod env;
pub(crate) mod glass;
mod headlights;
mod room;
mod volumetric;
mod warmup;

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
pub(crate) use warmup::WarmupCamera;

/// Environment of the menu showroom.
pub const SHOWROOM_ENV: &str = "showroom";
/// Neutral environment used when a track's environment file is missing or broken.
pub const FALLBACK_ENV: &str = "test";
/// First shadow cascade's far bound (u): the car and its surroundings get the sharpest texels.
const FIRST_CASCADE_U: f32 = 9.0;

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
pub(crate) enum EnvLight {
    /// Sun / window light / moon (casts shadows).
    Key,
    /// Moon / sky bounce, no shadows.
    Fill,
    /// Lightning, no shadows.
    Flash,
}

/// The session's `"visuals"` key is still pending.
#[derive(Resource, Debug)]
struct VisualsPending;

pub(crate) fn build(app: &mut App) {
    app.add_systems(OnEnter(AppState::Menu), enter_menu)
        .add_systems(OnEnter(AppState::Loading), enter_session)
        .add_systems(OnExit(AppState::Menu), clear_environment)
        .add_systems(OnEnter(AppState::Unloading), clear_environment)
        .add_systems(
            Update,
            (
                (warmup::start, report_ready)
                    .chain()
                    .run_if(resource_exists::<VisualsPending>),
                (apply_lights, apply_shadows, drive_clouds)
                    .in_set(VisualSet)
                    .after(LookSet),
            ),
        );
    camera::build(app);
    glass::build(app);
    room::build(app);
    headlights::build(app);
    volumetric::build(app);
    warmup::build(app);
}

fn enter_menu(mut commands: Commands, paths: Option<Res<UserPaths>>, spawner: EnvSpawner) {
    let root = paths
        .map(|p| p.assets_dir.clone())
        .unwrap_or_else(|| "assets".into());
    match EnvironmentDef::load(&root, SHOWROOM_ENV) {
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
/// [`FALLBACK_ENV`] (logged as an error); only when that fails too does the load fail.
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
    let loaded = EnvironmentDef::load(&root, &env_id).or_else(|error| {
        error!("visuals: {error}; using the '{FALLBACK_ENV}' environment");
        EnvironmentDef::load(&root, FALLBACK_ENV)
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
#[allow(clippy::too_many_arguments)]
fn report_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    time: Res<Time<Real>>,
    library: Option<Res<MaterialLibrary>>,
    track: Option<Res<rc_track::Track>>,
    queue: Res<warmup::PipelineQueue>,
    mut progress: Option<ResMut<warmup::Warmup>>,
    warmup_cameras: Query<(Has<EnvironmentMapLight>, Option<&Children>), With<WarmupCamera>>,
    cars: Query<Has<crate::cars::CarModelReady>, With<crate::cars::CarModel>>,
    mut done: MessageWriter<LoadingDone>,
) {
    if library.is_some_and(|l| l.textures_pending(&server) > 0) {
        return;
    }
    if !warmup::finished(
        progress.as_deref_mut(),
        &queue,
        time.delta_secs(),
        track.is_some(),
        warmup::scene_ready(&warmup_cameras, &cars),
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
            EnvLight::Fill => set_light(&mut light, &mut transform, look.fill.as_ref(), false),
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
    let distance = env.def.shadow_distance * tier.distance_scale;
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
