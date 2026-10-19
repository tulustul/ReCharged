//! Pipeline warm-up in the menu (Q9/B31: Loading → Intro ≤ 3 s). Once per run, while the player
//! is in the menus, hidden cameras far below the showroom draw what every race draws whatever its
//! track — every catalog car (and a Time Trial ghost), rc_vfx's effect instances, the four
//! surface-material variants, the common prop-material variants, window glass and the cloud deck,
//! under a shadowed point light — once for each view variant a race can use: 1, 2 and 3–4 player
//! views (the split-screen quality tiers), indoors and outdoors (Bevy's atmosphere is part of
//! every forward pipeline key). One stage per variant, the most common first, each as soon as the
//! previous one has compiled, so that Loading compiles only what its track adds (prop materials).
//!
//! Human runs compile on Bevy's task threads: the menu keeps its frame rate, provided nothing
//! makes the frame wait for those tasks — the stage cameras come after the menu camera and write
//! nothing to the window (else the menu camera's blit pipeline changes, which Bevy compiles
//! blocking the frame), the two replicas (split-screen overlay, second player view) switch their
//! blits on while nothing else is queued, and the warm-up starts once shader assets have stopped
//! arriving.
//! Automated runs (`AutomationConfig::is_automated`) compile synchronously, where each stage would
//! freeze the menu for seconds; they skip it and compile on demand (the Loading warm-up covers
//! their race). Leaving the menu abandons the warm-up for the run (the Loading warm-up compiles
//! whatever a race still needs).

use bevy::camera::CameraOutputMode;
use bevy::camera::visibility::RenderLayers;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use bevy::shader::Shader;
use rc_core::prelude::*;

use super::ActiveEnvironment;
use super::camera::ForceOutdoor;
use super::glass::WindowMaterial;
use super::warmup::{self, PipelineQueue, Warmup};
use crate::cars::{CarModel, CarModelReady, GhostLook};
use crate::materials::{
    MaterialDef, MaterialLibrary, SurfaceMaterial, TEXTURE_SETS, with_world_uvs,
};
use crate::quality::{QualityCamera, ViewsOverride};
use crate::sky::atmosphere;
use crate::sky::clouds::CloudMaterial;

/// Where the warm-up scene sits: far below the showroom, out of the menu camera's view (rc_vfx
/// puts its never-culled effect instances here too).
const AT: Vec3 = Vec3::new(0.0, -600.0, 0.0);
/// The stage camera, relative to [`AT`], and the point it looks at.
const CAMERA_FROM: Vec3 = Vec3::new(0.0, 6.0, 12.0);
const CAMERA_TO: Vec3 = Vec3::new(0.0, 0.0, 1.5);
/// Spacing of the cars (row at z = 0) and of the material samples (row at z = 3), u.
const CAR_STEP_U: f32 = 1.75;
const SAMPLE_STEP_U: f32 = 1.0;
/// Layers only the indoor / outdoor stage cameras draw besides the world (window glass is an
/// indoor material, the cloud deck an outdoor one).
const INDOOR_LAYER: usize = 28;
const OUTDOOR_LAYER: usize = 29;
/// The warm-up starts once no shader asset has arrived for this long (s): the render world takes
/// Bevy's shader cache to add an arriving shader, and compile tasks hold it while they build their
/// modules, so a shader arriving under a stage's compiles stalled the frame until all of them were
/// done (2.5 s during the splash).
const SHADERS_QUIET_S: f32 = 1.0;
/// The view variants in warm-up order: (player views, outdoor sky).
const STAGES: [(usize, bool); 6] = [
    (1, false),
    (1, true),
    (2, false),
    (2, true),
    (4, false),
    (4, true),
];

/// A stage camera of the menu warm-up.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct MenuWarmupCamera;

/// The root of the warm-up scene (cars, samples, lamp, atmosphere, overlay replica).
#[derive(Component, Debug, Clone, Copy)]
struct MenuWarmupScene;

/// Where the run's menu warm-up stands.
#[derive(Resource, Debug, Default)]
enum MenuWarmup {
    /// Waits for the first settled menu.
    #[default]
    Pending,
    /// Stage `stage` of [`STAGES`] is compiling.
    Running {
        stage: usize,
        progress: Warmup,
        /// Pipelines created and seconds spent before this stage / since the start.
        created: usize,
        elapsed: f32,
        /// Slowest frame of this stage, s (the menu stays smooth while it compiles).
        worst: f32,
    },
    /// Done or abandoned (never again this run).
    Over,
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<MenuWarmup>()
        .add_systems(
            Update,
            (start, advance)
                .chain()
                .in_set(VisualSet)
                .run_if(in_state(AppState::Menu)),
        )
        .add_systems(OnExit(AppState::Menu), abandon);
}

/// The first settled menu (showroom shown, texture sets loaded, no shader arriving for
/// [`SHADERS_QUIET_S`]): spawns the scene, the replicas and stage 1.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn start(
    mut commands: Commands,
    mut state: ResMut<MenuWarmup>,
    (ready, env, automation): (
        Option<Res<ShowroomReady>>,
        Option<Res<ActiveEnvironment>>,
        Option<Res<AutomationConfig>>,
    ),
    catalog: Option<Res<Catalog>>,
    server: Res<AssetServer>,
    queue: Res<PipelineQueue>,
    mut assets: SceneAssets,
    (time, mut shaders, mut quiet): (
        Res<Time<Real>>,
        MessageReader<AssetEvent<Shader>>,
        Local<f32>,
    ),
) {
    if !matches!(*state, MenuWarmup::Pending) {
        return;
    }
    if automation.is_some_and(|a| a.is_automated()) {
        info!("visuals: menu warm-up skipped (automated run: synchronous compilation)");
        *state = MenuWarmup::Over;
        return;
    }
    *quiet = if shaders.read().count() > 0 {
        0.0
    } else {
        *quiet + time.delta_secs()
    };
    if *quiet < SHADERS_QUIET_S || ready.is_none() || env.is_none() {
        return;
    }
    let Some(catalog) = catalog else {
        return;
    };
    if assets.library.textures_pending(&server) > 0 {
        return;
    }
    let root = assets.spawn_scene(&mut commands, &catalog);
    // Replicas of the split-screen overlay (premultiplied blit) and of a second player view (alpha
    // blended over the first): their 2D and UI pipelines compile with stage 1, their blits after
    // it (see `advance`). Both draw nothing (transparent).
    for (order, blend) in [
        (1, Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING)),
        (2, None),
    ] {
        let replica =
            warmup::spawn_overlay_replica(&mut commands, warmup::MENU_WARMUP_ORDER + order, blend);
        commands.entity(replica).insert(ChildOf(root));
    }
    spawn_stage_camera(&mut commands, root, STAGES[0]);
    info!(
        "visuals: menu warm-up started ({} view variants)",
        STAGES.len()
    );
    *state = MenuWarmup::Running {
        stage: 0,
        progress: Warmup::default(),
        created: queue.created(),
        elapsed: 0.0,
        worst: 0.0,
    };
}

/// Moves to the next stage once the current one has compiled; ends the warm-up after the last.
#[allow(clippy::too_many_arguments)]
fn advance(
    mut commands: Commands,
    mut state: ResMut<MenuWarmup>,
    time: Res<Time<Real>>,
    queue: Res<PipelineQueue>,
    roots: Query<Entity, With<MenuWarmupScene>>,
    cameras: Query<(Entity, &Camera), With<MenuWarmupCamera>>,
    mut replicas: Query<(&mut Camera, &warmup::ReplicaBlit), Without<MenuWarmupCamera>>,
    cars: Query<(&ChildOf, Has<CarModelReady>), With<CarModel>>,
) {
    let MenuWarmup::Running {
        stage,
        progress,
        created,
        elapsed,
        worst,
    } = &mut *state
    else {
        return;
    };
    *worst = worst.max(time.delta_secs());
    let (Ok(root), Ok((camera, view))) = (roots.single(), cameras.single()) else {
        return;
    };
    let ready = view.is_active
        && cars
            .iter()
            .filter(|(parent, _)| parent.parent() == root)
            .all(|(_, ready)| ready);
    if !warmup::finished(Some(progress), &queue, time.delta_secs(), true, ready) {
        return;
    }
    let (views, outdoor) = STAGES[*stage];
    *elapsed += progress.elapsed;
    info!(
        "visuals: menu warm-up stage {}/{} ({views} view(s), {}): {} pipelines in {:.1} s, \
         worst frame {:.0} ms",
        *stage + 1,
        STAGES.len(),
        if outdoor { "outdoor" } else { "indoor" },
        queue.created().saturating_sub(*created),
        progress.elapsed,
        *worst * 1000.0
    );
    *worst = 0.0;
    commands.entity(camera).despawn();
    if *stage == 0 {
        // The queue is idle and the next stage camera draws only a few frames from now: the
        // replicas' blits compile alone.
        for (mut replica, blit) in &mut replicas {
            replica.output_mode = blit.output();
        }
    }
    *created = queue.created();
    *stage += 1;
    let Some(&next) = STAGES.get(*stage) else {
        info!("visuals: menu warm-up done in {:.1} s", elapsed);
        commands.entity(root).despawn();
        *state = MenuWarmup::Over;
        return;
    };
    spawn_stage_camera(&mut commands, root, next);
    *progress = Warmup::default();
}

/// Leaving the menu ends the warm-up for this run (its entities go with the menu).
fn abandon(mut state: ResMut<MenuWarmup>) {
    if let MenuWarmup::Running { stage, .. } = *state {
        info!(
            "visuals: menu warm-up abandoned at stage {}/{}",
            stage + 1,
            STAGES.len()
        );
    }
    if !matches!(*state, MenuWarmup::Pending) {
        *state = MenuWarmup::Over;
    }
}

/// A hidden stage camera (inactive until its view is final, see `warmup::activate_views`).
fn spawn_stage_camera(commands: &mut Commands, root: Entity, (views, outdoor): (usize, bool)) {
    let layer = if outdoor { OUTDOOR_LAYER } else { INDOOR_LAYER };
    let mut camera = commands.spawn((
        Name::new(format!("Menu warm-up camera ({views} views)")),
        MenuWarmupCamera,
        QualityCamera,
        ViewsOverride(views),
        Camera3d::default(),
        // After every window camera, writing nothing to the window (see `MENU_WARMUP_ORDER`).
        Camera {
            order: warmup::MENU_WARMUP_ORDER,
            is_active: false,
            output_mode: CameraOutputMode::Skip,
            ..default()
        },
        RenderLayers::from_layers(&[0, layer]),
        Transform::from_translation(CAMERA_FROM).looking_at(CAMERA_TO, Vec3::Y),
        ChildOf(root),
    ));
    if outdoor {
        camera.insert(ForceOutdoor);
    }
}

/// What the warm-up scene is made of.
#[derive(bevy::ecs::system::SystemParam)]
struct SceneAssets<'w> {
    library: Res<'w, MaterialLibrary>,
    meshes: ResMut<'w, Assets<Mesh>>,
    images: ResMut<'w, Assets<Image>>,
    standard: ResMut<'w, Assets<StandardMaterial>>,
    surfaces: ResMut<'w, Assets<SurfaceMaterial>>,
    glass: ResMut<'w, Assets<WindowMaterial>>,
    clouds: ResMut<'w, Assets<CloudMaterial>>,
    media: ResMut<'w, Assets<ScatteringMedium>>,
}

impl SceneAssets<'_> {
    /// The scene root at [`AT`] with every car, the material samples and the lamp.
    fn spawn_scene(&mut self, commands: &mut Commands, catalog: &Catalog) -> Entity {
        let root = commands
            .spawn((
                Name::new("Menu warm-up scene"),
                MenuWarmupScene,
                Transform::from_translation(AT),
                Visibility::default(),
                DespawnOnExit(AppState::Menu),
            ))
            .id();
        let cars: Vec<(&CarInfo, bool)> = catalog
            .cars
            .iter()
            .map(|c| (c, false))
            .chain(catalog.cars.first().map(|c| (c, true)))
            .collect();
        let n = cars.len();
        for ((car, ghost), x) in cars.into_iter().zip(row(n, CAR_STEP_U)) {
            let Some(paint) = car.paints.first() else {
                continue;
            };
            let mut model = commands.spawn((
                Name::new("warm-up car"),
                CarModel {
                    car: car.id.clone(),
                    paint: paint.id,
                },
                Transform::from_xyz(x, 0.0, 0.0),
                ChildOf(root),
            ));
            if ghost {
                model.insert(GhostLook);
            }
        }
        self.spawn_samples(commands, root);
        commands.spawn((
            warmup::warmup_lamp(),
            Transform::from_xyz(0.0, 2.0, 1.5),
            ChildOf(root),
        ));
        // Only the outdoor stage cameras (`AtmosphereSettings`) use it.
        commands.spawn((
            Name::new("Warm-up atmosphere"),
            atmosphere(&mut self.media, [0.3, 0.3, 0.3]),
            ChildOf(root),
        ));
        root
    }

    /// One mesh per material variant the tracks' surfaces and props use (the vertex layout is
    /// part of the key: tangents only where library surfaces and normal-mapped glTF props have
    /// them): surfaces with or without a texture set (normal map) × clear coat; glTF prop
    /// materials plain, clear-coated, blended, masked, double-sided, and with tangents with and
    /// without a normal map; window glass (indoor views) and the cloud deck (outdoor views).
    fn spawn_samples(&mut self, commands: &mut Commands, root: Entity) {
        let plain = self.meshes.add(Cuboid::default());
        let tangents = self.meshes.add(with_world_uvs(Cuboid::default().into()));
        let shipped = TEXTURE_SETS.first().map(|(name, _)| (*name).to_owned());
        let surfaces = [
            (None, 0.0),
            (None, 1.0),
            (shipped.clone(), 0.0),
            (shipped, 1.0),
        ];
        let standard = [
            (&plain, StandardMaterial::default()),
            (
                &plain,
                StandardMaterial {
                    clearcoat: 1.0,
                    ..default()
                },
            ),
            (
                &plain,
                StandardMaterial {
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                },
            ),
            (
                &plain,
                StandardMaterial {
                    alpha_mode: AlphaMode::Mask(0.5),
                    ..default()
                },
            ),
            (
                &plain,
                StandardMaterial {
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                },
            ),
            (&tangents, StandardMaterial::default()),
            (
                &tangents,
                StandardMaterial {
                    normal_map_texture: Some(self.images.add(Image::default())),
                    ..default()
                },
            ),
        ];
        let count = surfaces.len() + standard.len() + 2;
        let mut at = row(count, SAMPLE_STEP_U).map(|x| {
            (
                Name::new("warm-up material"),
                Transform::from_xyz(x, 0.5, 3.0).with_scale(Vec3::splat(0.5)),
                ChildOf(root),
            )
        });
        let mut next = || at.next().expect("one position per sample");
        for (set, clearcoat) in surfaces {
            let def = MaterialDef {
                set,
                clearcoat,
                ..default()
            };
            let material = self.library.make(&def, &mut self.surfaces);
            commands.spawn((next(), Mesh3d(tangents.clone()), MeshMaterial3d(material)));
        }
        for (mesh, material) in standard {
            let material = self.standard.add(material);
            commands.spawn((next(), Mesh3d(mesh.clone()), MeshMaterial3d(material)));
        }
        commands.spawn((
            next(),
            Mesh3d(plain),
            MeshMaterial3d(self.glass.add(WindowMaterial::default())),
            RenderLayers::layer(INDOOR_LAYER),
        ));
        commands.spawn((
            next(),
            Mesh3d(self.meshes.add(Sphere::new(0.5).mesh().uv(16, 8))),
            MeshMaterial3d(self.clouds.add(CloudMaterial::default())),
            NotShadowCaster,
            NotShadowReceiver,
            RenderLayers::layer(OUTDOOR_LAYER),
        ));
    }
}

/// `n` positions `step` apart, centred on 0.
fn row(n: usize, step: f32) -> impl Iterator<Item = f32> {
    let first = -(n.saturating_sub(1) as f32) * step * 0.5;
    (0..n).map(move |i| first + i as f32 * step)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_cover_every_view_tier_indoors_and_out_most_common_first() {
        assert_eq!(STAGES[0], (1, false), "single player indoors first");
        for views in [1, 2, 4] {
            for outdoor in [false, true] {
                assert!(STAGES.contains(&(views, outdoor)), "{views} {outdoor}");
            }
        }
        let tiers: Vec<usize> = STAGES.iter().map(|s| s.0).collect();
        assert!(tiers.windows(2).all(|w| w[0] <= w[1]), "split tiers last");
    }

    #[test]
    fn rows_are_centred_and_evenly_spaced() {
        let r: Vec<f32> = row(3, 2.0).collect();
        assert_eq!(r, [-2.0, 0.0, 2.0]);
        assert_eq!(row(0, 1.0).count(), 0);
    }
}
