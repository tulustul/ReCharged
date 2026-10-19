//! Car models (ARCHITECTURE §5.9, `docs/design/art-cars.md`): the car glb as a child of every
//! racer and ghost proxy (and of the showroom's turntable via [`CarModel`]), recoloured with the
//! chosen paint, lamps at art-bible levels (brake idle 0.2× → 1× when braking, headlights 0.3× by
//! day), wheels from telemetry ([`wheels`]), a swaying antenna ([`antenna`]), Time Trial ghosts
//! translucent, respawn ghosting blinking (GDD §5.4) and status effects shown by the body's
//! shader ([`status`]).

mod antenna;
mod status;
mod wheels;

use std::collections::HashMap;

use bevy::gltf::GltfMaterialName;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use bevy::world_serialization::WorldInstanceReady;
use rc_core::prelude::*;

use crate::content::{ContentLoad, is_night};
use crate::lighting::{ActiveEnvironment, WarmupCamera};
use crate::materials::MaterialLibrary;
use status::CarBodyMaterial;

/// Brake lamps when not braking, relative to the glb's "on" strength (art bible §1.2).
const BRAKE_IDLE: f32 = 0.2;
/// Brake input above which the brake lamps light.
const BRAKE_ON_INPUT: f32 = 0.05;
/// Headlamps by day, relative to their "on" strength (full at Night).
const HEAD_DAY: f32 = 0.3;
/// Opacity of the Time Trial ghost.
const GHOST_ALPHA: f32 = 0.35;
/// Blink rate of a car while respawn ghosting (Hz).
const BLINK_HZ: f32 = 6.0;
/// Wheel node names in `VehicleTelemetry::wheels` order.
const WHEELS: [&str; 4] = ["wheel_fl", "wheel_fr", "wheel_rl", "wheel_rr"];
/// Pipeline warm-up: the session's car models sit this far in front of the warm-up camera, in a
/// row this wide (u), inside its view.
pub(crate) const WARMUP_DISTANCE_U: f32 = 14.0;
const WARMUP_SPREAD_U: f32 = 6.0;

/// A car model: the car's glb is instanced as children of this entity, painted with `paint`,
/// lamps at idle. rc_render adds it to racer and ghost proxies; the showroom puts it on its
/// turntable. Insert a different value to change the car or paint.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
#[require(Transform, Visibility)]
pub struct CarModel {
    /// Car (catalog id).
    pub car: CarId,
    /// Paint of that car.
    pub paint: PaintId,
}

/// Added once a [`CarModel`]'s glb is instanced and painted (removed while it re-instances).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CarModelReady;

/// The racer whose telemetry, controls and effects animate this model.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct DrivenBy(pub Entity);

/// Time Trial ghost look: translucent, no shadows.
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct GhostLook;

/// The model value that is instanced (re-inserting the same value does nothing).
#[derive(Component, Debug, Clone)]
struct Instanced(CarModel);

/// Animated parts of an instanced model.
#[derive(Component, Debug, Clone, Default)]
pub(crate) struct CarParts {
    /// Wheel nodes FL, FR, RL, RR with their rest poses.
    pub wheels: [Option<(Entity, Transform)>; 4],
    /// This instance's brake-lamp materials with their authored ("on") emission.
    brakes: Vec<(Handle<StandardMaterial>, LinearRgba)>,
    braking: bool,
    /// This instance's body (`paint`, `paint_accent`) materials.
    body: Vec<Handle<CarBodyMaterial>>,
}

/// Car scenes by car, kept for the app's lifetime (restarts and next races reuse them). Every
/// catalog car is loaded once the catalog exists (with all its paints), so asset counts do not
/// depend on which cars a session showed (the soak census) and the menu warm-up has every model.
#[derive(Resource, Debug, Default)]
pub(crate) struct CarScenes(HashMap<CarId, (Handle<WorldAsset>, String)>);

impl CarScenes {
    /// The car's scene, loading it on first use; `None` for a car the catalog lacks.
    fn get(
        &mut self,
        car: &CarId,
        catalog: &Catalog,
        server: &AssetServer,
    ) -> Option<&(Handle<WorldAsset>, String)> {
        if !self.0.contains_key(car) {
            let model = catalog.car(car)?.model.clone();
            let handle = crate::content::load_scene(server, &model);
            self.0.insert(car.clone(), (handle, model));
        }
        self.0.get(car)
    }
}

pub(crate) fn build(app: &mut App) {
    status::build(app);
    app.init_resource::<CarScenes>()
        .add_observer(attach_to_proxies)
        .add_observer(warm_up_cars)
        .add_observer(instance_model)
        .add_observer(prepare_instance)
        .add_systems(
            Update,
            (
                preload_catalog_cars.run_if(resource_added::<Catalog>),
                preload_session_cars.run_if(in_state(AppState::Loading)),
                (
                    wheels::drive_wheels,
                    wheels::roll_ghost_wheels,
                    antenna::sway,
                    brake_lamps,
                    respawn_blink,
                    status::shade,
                )
                    .in_set(VisualSet),
            ),
        );
}

/// A racer's or ghost's proxy gets its car model as a child.
fn attach_to_proxies(
    add: On<Add<VisualOf>>,
    links: Query<&VisualOf>,
    sims: Query<(Option<&Racer>, Option<&GhostCar>)>,
    mut commands: Commands,
) {
    let proxy = add.entity;
    let Ok(sim) = links.get(proxy).map(|l| l.0) else {
        return;
    };
    match sims.get(sim) {
        Ok((Some(racer), _)) => {
            commands.spawn((
                Name::new("car model"),
                CarModel {
                    car: racer.car.clone(),
                    paint: racer.paint,
                },
                DrivenBy(sim),
                status::Flare::default(),
                ChildOf(proxy),
            ));
        }
        Ok((None, Some(ghost))) => {
            commands.spawn((
                Name::new("ghost car model"),
                CarModel {
                    car: ghost.car.clone(),
                    paint: ghost.paint,
                },
                GhostLook,
                wheels::Rolling::default(),
                ChildOf(proxy),
            ));
        }
        _ => {}
    }
}

/// The pipeline warm-up camera (T13a) gets every car model of the race (and the ghost) in front
/// of it, roofs toward it, so their pipelines compile during Loading.
fn warm_up_cars(
    add: On<Add<WarmupCamera>>,
    config: Option<Res<RaceConfig>>,
    mut commands: Commands,
) {
    let Some(config) = config else {
        return;
    };
    let racers = config.racers.iter().map(|r| (&r.car, r.paint, false));
    let ghost = config.ghost.iter().map(|g| (&g.car, g.paint, true));
    let models: Vec<_> = racers.chain(ghost).collect();
    let step = 2.0 * WARMUP_SPREAD_U / models.len().max(2) as f32;
    for (i, (car, paint, ghost)) in models.into_iter().enumerate() {
        let x = -WARMUP_SPREAD_U + step * i as f32;
        let mut model = commands.spawn((
            Name::new("warm-up car"),
            CarModel {
                car: car.clone(),
                paint,
            },
            Transform::from_xyz(x, 0.0, -WARMUP_DISTANCE_U)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ChildOf(add.entity),
        ));
        if ghost {
            model.insert(GhostLook);
        }
    }
}

/// Every car scene and paint of the catalog, once (see [`CarScenes`]).
fn preload_catalog_cars(
    catalog: Res<Catalog>,
    server: Res<AssetServer>,
    mut scenes: ResMut<CarScenes>,
    mut library: ResMut<MaterialLibrary>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for car in &catalog.cars {
        scenes.get(&car.id, &catalog, &server);
        for paint in &car.paints {
            library.car_paint(&mut materials, car, paint.id);
        }
    }
}

/// `Loading`: queue the scenes of every car in the race (and the ghost's) for the loading gate.
fn preload_session_cars(
    mut load: ResMut<ContentLoad>,
    mut scenes: ResMut<CarScenes>,
    config: Option<Res<RaceConfig>>,
    catalog: Option<Res<Catalog>>,
    server: Res<AssetServer>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    let (Some(config), Some(catalog)) = (config, catalog) else {
        return;
    };
    if load.cars || load.reported {
        return;
    }
    let ghost = config.ghost.as_ref().map(|g| &g.car);
    for car in config.racers.iter().map(|r| &r.car).chain(ghost) {
        let Some((handle, path)) = scenes.get(car, &catalog, &server).cloned() else {
            failed.write(LoadingFailed {
                key: crate::content::LOADING_KEY,
                error: format!("car `{car}` is not in the catalog"),
            });
            load.reported = true;
            return;
        };
        if !load.scenes.iter().any(|(h, ..)| *h == handle) {
            load.scenes.push((handle, path, true));
        }
    }
    load.cars = true;
}

/// A new or changed [`CarModel`] (re)instances its glb.
fn instance_model(
    insert: On<Insert<CarModel>>,
    models: Query<(&CarModel, Option<&Instanced>)>,
    catalog: Option<Res<Catalog>>,
    server: Res<AssetServer>,
    mut scenes: ResMut<CarScenes>,
    mut commands: Commands,
) {
    let Ok((model, instanced)) = models.get(insert.entity) else {
        return;
    };
    if instanced.is_some_and(|i| i.0 == *model) {
        return;
    }
    let Some((handle, _)) = catalog.and_then(|c| scenes.get(&model.car, &c, &server).cloned())
    else {
        error!("car model: `{}` is not in the catalog", model.car);
        return;
    };
    commands
        .entity(insert.entity)
        .remove::<(CarModelReady, CarParts, antenna::Antenna)>()
        .insert((Instanced(model.clone()), WorldAssetRoot(handle)));
}

/// Materials and parts of a freshly instanced car scene.
#[allow(clippy::too_many_arguments)]
fn prepare_instance(
    ready: On<WorldInstanceReady>,
    models: Query<(&CarModel, Has<GhostLook>)>,
    children: Query<&Children>,
    nodes: Query<(&Name, &Transform)>,
    meshes: Query<(&GltfMaterialName, &MeshMaterial3d<StandardMaterial>)>,
    catalog: Option<Res<Catalog>>,
    env: Option<Res<ActiveEnvironment>>,
    mut library: ResMut<MaterialLibrary>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bodies: ResMut<Assets<CarBodyMaterial>>,
    mut commands: Commands,
) {
    let root = ready.entity;
    let Ok((model, ghost)) = models.get(root) else {
        return;
    };
    let paint = catalog
        .as_ref()
        .and_then(|c| c.car(&model.car))
        .and_then(|info| library.car_paint(&mut materials, info, model.paint));
    if paint.is_none() {
        warn!(
            "car model {}: unknown paint {:?}; authored colours kept",
            model.car, model.paint
        );
    }
    let head = if is_night(env.as_deref()) {
        1.0
    } else {
        HEAD_DAY
    };
    let mut parts = CarParts::default();
    let mut made: HashMap<(AssetId<StandardMaterial>, &str), Handle<StandardMaterial>> =
        HashMap::new();
    let mut made_bodies: HashMap<(AssetId<StandardMaterial>, &str), Handle<CarBodyMaterial>> =
        HashMap::new();
    let mut sway = None;
    for entity in children.iter_descendants(root) {
        if let Ok((name, transform)) = nodes.get(entity) {
            if let Some(i) = WHEELS.iter().position(|w| name.as_str() == *w) {
                parts.wheels[i] = Some((entity, *transform));
            } else if name.as_str() == "antenna" {
                sway = Some(antenna::Antenna::new(entity, transform.rotation));
            }
        }
        let Ok((material_name, current)) = meshes.get(entity) else {
            continue;
        };
        let role: &str = material_name;
        let base = match (role, &paint) {
            ("paint", Some(p)) => p.base.clone(),
            ("paint_accent", Some(p)) => p.accent.clone(),
            _ => current.0.clone(),
        };
        if matches!(role, "paint" | "paint_accent") {
            // Per instance: the body shows this racer's status effects.
            let key = (base.id(), role);
            let handle = match made_bodies.get(&key) {
                Some(h) => h.clone(),
                None => {
                    let Some(mut m) = materials.get(&base).cloned() else {
                        continue;
                    };
                    if ghost {
                        ghostify(&mut m);
                    }
                    let h = bodies.add(CarBodyMaterial {
                        base: m,
                        extension: default(),
                    });
                    parts.body.push(h.clone());
                    made_bodies.insert(key, h.clone());
                    h
                }
            };
            let mut e = commands.entity(entity);
            e.remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d(handle));
            if ghost {
                e.insert(NotShadowCaster);
            }
            continue;
        }
        let level = match role {
            "light_head" => Some(head),
            "light_brake" => Some(BRAKE_IDLE),
            _ => None,
        };
        if level.is_none() && !ghost {
            if base != current.0 {
                commands.entity(entity).insert(MeshMaterial3d(base));
            }
            continue;
        }
        let key = (base.id(), role);
        let handle = match made.get(&key) {
            Some(h) => h.clone(),
            None => {
                let Some(mut m) = materials.get(&base).cloned() else {
                    continue;
                };
                let authored = m.emissive;
                if let Some(level) = level {
                    m.emissive = authored * level;
                }
                if ghost {
                    ghostify(&mut m);
                }
                let h = materials.add(m);
                if role == "light_brake" {
                    parts.brakes.push((h.clone(), authored));
                }
                made.insert(key, h.clone());
                h
            }
        };
        let mut e = commands.entity(entity);
        e.insert(MeshMaterial3d(handle));
        if ghost {
            e.insert(NotShadowCaster);
        }
    }
    if parts.wheels.iter().any(Option::is_none) {
        warn!(
            "car model {}: wheel nodes missing; wheels stay still",
            model.car
        );
    }
    let mut model = commands.entity(root);
    model.insert((parts, CarModelReady));
    if let Some(sway) = sway {
        model.insert(sway);
    }
}

/// The Time Trial ghost version of a material: translucent and single-sided, emission kept.
fn ghostify(m: &mut StandardMaterial) {
    m.base_color = m.base_color.with_alpha(GHOST_ALPHA);
    m.alpha_mode = AlphaMode::Blend;
    m.double_sided = false;
    m.cull_mode = Some(Face::Back);
}

/// `VisualSet`: brake lamps light while the racer brakes.
fn brake_lamps(
    mut models: Query<(&DrivenBy, &mut CarParts)>,
    controls: Query<&CarControl>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (driven, mut parts) in &mut models {
        let braking = controls
            .get(driven.0)
            .is_ok_and(|c| c.brake > BRAKE_ON_INPUT);
        if braking == parts.braking {
            continue;
        }
        parts.braking = braking;
        let level = if braking { 1.0 } else { BRAKE_IDLE };
        for (handle, authored) in &parts.brakes {
            if let Some(mut m) = materials.get_mut(handle) {
                m.emissive = *authored * level;
            }
        }
    }
}

/// `VisualSet`: a respawn-ghosted racer blinks (GDD §5.4).
fn respawn_blink(
    mut models: Query<(&DrivenBy, &mut Visibility), With<CarModel>>,
    effects: Query<&StatusEffects>,
) {
    for (driven, mut visibility) in &mut models {
        let left = effects.get(driven.0).ok().and_then(|e| e.ghost);
        let shown = left.is_none_or(blink_shown);
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

/// Blink phase with `left` seconds of ghosting remaining (ends visible).
fn blink_shown(left: f32) -> bool {
    (left * BLINK_HZ * 2.0).floor() as i64 % 2 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blinking_alternates_and_ends_visible() {
        let step = 1.0 / (BLINK_HZ * 2.0);
        assert!(blink_shown(0.0) && blink_shown(step * 0.5));
        assert!(!blink_shown(step * 1.5));
        assert!(blink_shown(step * 2.5));
    }

    #[test]
    fn ghost_materials_are_translucent_and_keep_their_glow() {
        let mut m = StandardMaterial {
            base_color: Color::srgb(0.8, 0.1, 0.1),
            emissive: LinearRgba::rgb(6.0, 5.0, 4.0),
            double_sided: true,
            cull_mode: None,
            ..default()
        };
        ghostify(&mut m);
        assert!((m.base_color.alpha() - GHOST_ALPHA).abs() < 1e-6);
        assert_eq!(m.alpha_mode, AlphaMode::Blend);
        assert_eq!(m.emissive, LinearRgba::rgb(6.0, 5.0, 4.0));
        assert_eq!(m.cull_mode, Some(Face::Back));
    }
}
