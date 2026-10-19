//! Pipeline warm-up (ARCHITECTURE §9: Loading → Intro ≤ 3 s, no first-use compiles in a race).
//! Bevy specialises (and compiles) a render pipeline the first time a material/mesh/view
//! combination is drawn. Hidden cameras draw the combinations a race needs before it starts:
//! - in the menu, once per run ([`super::menu_warmup`]): what does not depend on the track —
//!   every car, effect, surface and common prop material — for every view variant;
//! - while Loading (here): a [`WarmupCamera`] — a `QualityCamera` like the game views, drawn under
//!   the loading screen — looks down on the whole track (the car visuals put one model per racer
//!   in front of it). It exists from the first frame of Loading but stays inactive until its view
//!   is final (quality, environment map, fog: all part of the pipeline keys), so it compiles while
//!   the other loaders still load. `"visuals"` completes once every other loader is done, the
//!   scene is complete (cars instanced) and the render world has neither queued nor created a
//!   pipeline for a few frames (or after a timeout).
//!
//! rc_vfx adds one never-culled instance of every effect material to whatever these cameras draw.
//! While a warm-up camera exists the sun's cascades reach the ground under it (prop shadow
//! pipelines); the Loading camera gets a shadowed point light at its cars when the track has
//! shadowed lamps (their cube-map pipelines). The split-screen UI overlay's sprite and
//! premultiplied-blit pipelines are warmed in the menu only (a replica cannot render during
//! Loading: the loading screen's 2D camera uses 4× MSAA on the same window, the overlay none);
//! the Loading camera, ordered before the loading screen's, also compiles the alpha-blended blit
//! of a second player view.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::camera::CameraOutputMode;
use bevy::camera::visibility::RenderLayers;
use bevy::light::ShadowFilteringMethod;
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use bevy::render::render_resource::{BlendState, PipelineCache, PipelineDescriptor};
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::shader::ShaderDefVal;
use rc_core::prelude::*;
use rc_track::Track;

use crate::cars::{CarModel, CarModelReady, WARMUP_DISTANCE_U};
use crate::quality::QualityCamera;

/// Frames a warm-up camera renders at least (its first frame queues the pipelines; with
/// synchronous compilation the next one already finds them built). Kept low: an occluded window
/// is throttled to 1 frame/s by the driver, and loading must not wait on frame counts.
const MIN_FRAMES: u32 = 3;
/// Consecutive frames with a complete scene, an empty pipeline queue and no new pipelines before
/// a warm-up counts as done (the render world reports a frame late).
const IDLE_FRAMES: u32 = 3;
/// Never hold the loading screen (or a menu warm-up stage) longer than this (s, wall clock).
const TIMEOUT_S: f32 = 20.0;
/// Camera height over the track as a multiple of its largest horizontal extent.
const HEIGHT_PER_EXTENT: f32 = 1.1;
/// Render layer of the overlay replica's sprite (drawn by no other camera).
const OVERLAY_LAYER: usize = 31;
/// Render order of the menu's warm-up cameras: after every window camera (menu 0, player views
/// 0–3, overlay 10). The first camera on the window replaces it, later ones blend over it, so a
/// warm-up camera ordered first changed the menu camera's blit pipeline — and Bevy compiles a new
/// blit pipeline blocking the frame, behind every pipeline already queued (a 1–2.5 s freeze).
pub(super) const MENU_WARMUP_ORDER: isize = 100;
/// Range of the warm-up point light (u): every warm-up car is inside it.
pub(super) const LAMP_RANGE_U: f32 = 12.0;
/// After GO and in the menu (its warm-up), frames slower than this are logged at debug level with
/// the pipelines they created.
const LONG_FRAME_S: f32 = 0.025;
/// Frames a warm-up view waits after its components are final before it draws: the render world
/// sees a new environment map's images a frame or two later, and a view drawn without them
/// compiles every variant without `ENVIRONMENT_MAP` (a second, useless set).
const SETTLE_FRAMES: u32 = 3;

/// The hidden camera that warms pipelines up during Loading. Car visuals attach one `CarModel`
/// per racer to it (in front of it, inside its view).
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct WarmupCamera;

/// The shadowed point light of a warm-up.
#[derive(Component, Debug, Clone, Copy)]
struct WarmupLamp;

/// Render pipelines waiting to compile (queued or compiling on Bevy's task threads) and created
/// so far, as the render world saw them at the end of its latest frame (one frame behind the main
/// world). Read by the warm-ups and by the binary, which waits for an idle queue before exiting.
#[derive(Resource, Debug, Clone, Default)]
pub struct PipelineQueue(Arc<[AtomicUsize; 2]>);

impl PipelineQueue {
    /// Pipelines queued or compiling.
    pub fn waiting(&self) -> usize {
        self.0[0].load(Ordering::Relaxed)
    }

    pub(super) fn created(&self) -> usize {
        self.0[1].load(Ordering::Relaxed)
    }
}

/// Progress of one warm-up (the Loading one, or one menu stage).
#[derive(Resource, Debug, Default)]
pub(super) struct Warmup {
    frames: u32,
    idle: u32,
    pub(super) elapsed: f32,
    created: usize,
}

/// The race start being watched for late pipeline compiles (from entering the race to GO).
#[derive(Resource, Debug, Default)]
struct StartWatch {
    created: usize,
    frames: u32,
    worst: f32,
}

pub(super) fn build(app: &mut App) {
    let queue = PipelineQueue::default();
    app.insert_resource(queue.clone())
        .add_systems(OnEnter(AppState::InRace), watch_start)
        .add_systems(OnEnter(RacePhase::Racing), report_start)
        .add_systems(OnEnter(AppState::Unloading), |mut commands: Commands| {
            commands.remove_resource::<StartWatch>();
        })
        .add_systems(
            Update,
            (
                time_start.run_if(resource_exists::<StartWatch>),
                log_long_frames
                    .run_if(in_state(RacePhase::Racing).or_else(in_state(AppState::Menu))),
                (activate_views, add_lamp).in_set(VisualSet),
            ),
        );
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(queue)
            .add_systems(Render, count_waiting.in_set(RenderSystems::Cleanup));
    }
}

/// Publishes the queue to the main world; names every new pipeline at debug level
/// (`RUST_LOG=rc_render=debug` shows what a hitch compiled).
fn count_waiting(cache: Res<PipelineCache>, queue: Res<PipelineQueue>, mut seen: Local<usize>) {
    queue.0[0].store(cache.waiting_pipelines().count(), Ordering::Relaxed);
    let created = cache.pipelines().count();
    queue.0[1].store(created, Ordering::Relaxed);
    for pipeline in cache.pipelines().skip(*seen) {
        debug!("visuals: new pipeline {}", describe(&pipeline.descriptor));
    }
    *seen = created;
}

/// Label and shaders of a pipeline, for the debug log.
fn describe(descriptor: &PipelineDescriptor) -> String {
    let path = |h: &Handle<Shader>| h.path().map_or_else(|| format!("{h:?}"), |p| p.to_string());
    match descriptor {
        PipelineDescriptor::RenderPipelineDescriptor(d) => format!(
            "{} (vertex {}, fragment {}; {}{})",
            d.label.as_deref().unwrap_or("?"),
            path(&d.vertex.shader),
            d.fragment
                .as_ref()
                .map_or_else(|| "-".into(), |f| path(&f.shader)),
            defs(&d.vertex.shader_defs),
            d.fragment.as_ref().map_or_else(String::new, |f| {
                // Material features (clear coat, normal map, alpha …) are fragment-only defs.
                let extra: Vec<ShaderDefVal> = f
                    .shader_defs
                    .iter()
                    .filter(|def| !d.vertex.shader_defs.contains(def))
                    .cloned()
                    .collect();
                if extra.is_empty() {
                    String::new()
                } else {
                    format!(" | fragment {}", defs(&extra))
                }
            })
        ),
        PipelineDescriptor::ComputePipelineDescriptor(d) => format!(
            "{} (compute {})",
            d.label.as_deref().unwrap_or("?"),
            path(&d.shader)
        ),
        PipelineDescriptor::MeshPipelineDescriptor(d) => {
            format!("{} (mesh)", d.label.as_deref().unwrap_or("?"))
        }
    }
}

/// The shader defs that are set (`NAME`, `NAME=value`), for the debug log.
fn defs(defs: &[ShaderDefVal]) -> String {
    let set = defs.iter().filter_map(|d| match d {
        ShaderDefVal::Bool(name, true) => Some(name.to_string()),
        ShaderDefVal::Bool(_, false) => None,
        ShaderDefVal::Int(name, v) => Some(format!("{name}={v}")),
        ShaderDefVal::UInt(name, v) => Some(format!("{name}={v}")),
    });
    set.collect::<Vec<_>>().join(" ")
}

fn watch_start(mut commands: Commands, queue: Res<PipelineQueue>) {
    commands.insert_resource(StartWatch {
        created: queue.created(),
        ..default()
    });
}

fn time_start(time: Res<Time<Real>>, mut watch: ResMut<StartWatch>) {
    watch.frames += 1;
    watch.worst = watch.worst.max(time.delta_secs());
}

/// GO of the session's first attempt: what the warm-up missed (pipelines compiled while the
/// Intro and countdown ran) and the worst frame — the hitch the warm-up exists to remove.
fn report_start(mut commands: Commands, watch: Option<Res<StartWatch>>, queue: Res<PipelineQueue>) {
    let Some(watch) = watch else {
        return;
    };
    info!(
        "visuals: race start: {} pipelines compiled after the warm-up, worst frame {:.0} ms ({} frames)",
        queue.created().saturating_sub(watch.created),
        watch.worst * 1000.0,
        watch.frames
    );
    commands.remove_resource::<StartWatch>();
}

/// Racing and in the menu: a slow frame and the pipelines created in it (debug level; Q20 hitch
/// hunting, and proof that the async menu warm-up keeps the menu smooth).
fn log_long_frames(time: Res<Time<Real>>, queue: Res<PipelineQueue>, mut last: Local<usize>) {
    let created = queue.created();
    let dt = time.delta_secs();
    if dt > LONG_FRAME_S && *last > 0 {
        debug!(
            "visuals: long frame {:.0} ms ({} new pipelines)",
            dt * 1000.0,
            created.saturating_sub(*last)
        );
    }
    *last = created;
}

/// `Loading`: the warm-up camera (inactive until its view is final, see [`activate_views`]) and
/// the warm-up UI node.
pub(super) fn start(
    mut commands: Commands,
    track: Option<Res<Track>>,
    cameras: Query<(), With<WarmupCamera>>,
) {
    let (Some(track), true) = (track, cameras.is_empty()) else {
        return;
    };
    let (min, max) = track.bounds();
    let extent = (max.x - min.x).max(max.z - min.z);
    let centre = Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5);
    let camera = commands
        .spawn((
            Name::new("Pipeline warm-up camera"),
            WarmupCamera,
            QualityCamera,
            Camera3d::default(),
            Camera {
                order: -1,
                is_active: false,
                ..default()
            },
            Transform::from_translation(centre + Vec3::Y * extent * HEIGHT_PER_EXTENT)
                .looking_at(centre, Vec3::NEG_Z),
            DespawnOnExit(AppState::Loading),
        ))
        .id();
    commands.spawn((ui_warmup_node(camera), DespawnOnExit(AppState::Loading)));
    commands.insert_resource(Warmup::default());
}

/// bevy_ui compiles its pipelines per target format: a 1-player race hosts the HUD on its HDR
/// game camera, while the loading screen's camera is LDR (and `--quick-race` skips the HDR
/// menu). A plain node, a gradient (Oklab, bevy-react's default) and a box shadow (the HUD's
/// glows) on the HDR warm-up view cover it.
fn ui_warmup_node(camera: Entity) -> impl Bundle {
    let px = Val::Px(2.0);
    (
        Name::new("UI warm-up"),
        Node {
            width: px,
            height: px,
            ..default()
        },
        BackgroundColor(Color::BLACK),
        BackgroundGradient::from(LinearGradient::to_right(vec![
            ColorStop::auto(Color::BLACK),
            ColorStop::auto(Color::WHITE),
        ])),
        BoxShadow::new(Color::BLACK, Val::ZERO, Val::ZERO, px, px),
        UiTargetCamera(camera),
    )
}

/// The blend a replica camera will write to the window with (see [`spawn_overlay_replica`]).
#[derive(Component, Debug, Clone, Copy)]
pub(super) struct ReplicaBlit(pub Option<BlendState>);

impl ReplicaBlit {
    /// Writing to the window with the wanted blend (`None`: the automatic one, alpha blending
    /// when it is not the window's first camera) compiles the replica's blit pipeline, blocking
    /// the frame behind every pipeline queued: switch to it only while the queue is idle.
    pub(super) fn output(&self) -> CameraOutputMode {
        CameraOutputMode::Write {
            blend_state: self.0,
            clear_color: ClearColorConfig::None,
        }
    }
}

/// A replica of rc_camera's split-screen UI overlay (`Camera2d`, LDR, `Msaa::Off`, cleared to
/// transparent) drawing one transparent sprite, so its sprite, UI and blit pipelines compile
/// before the race rather than at its start. It writes nothing to the window until its
/// [`ReplicaBlit::output`] is set. Returns the camera (the sprite is its child). Only where every
/// other camera on the window has `Msaa::Off` (the menu): mixed sample counts on one window are a
/// wgpu validation error.
pub(super) fn spawn_overlay_replica(
    commands: &mut Commands,
    order: isize,
    blend: Option<BlendState>,
) -> Entity {
    commands
        .spawn((
            Name::new("Overlay warm-up camera"),
            Camera2d,
            Camera {
                order,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                output_mode: CameraOutputMode::Skip,
                ..default()
            },
            ReplicaBlit(blend),
            Msaa::Off,
            RenderLayers::layer(OVERLAY_LAYER),
        ))
        .with_child((
            Sprite::from_color(Color::NONE, Vec2::ONE),
            RenderLayers::layer(OVERLAY_LAYER),
        ))
        .id()
}

/// A warm-up view is final once its quality, environment map (and, outdoors, atmosphere) and fog
/// are attached: drawing earlier would compile variants no race uses.
pub(super) fn view_final(env_map: bool, fog: bool, quality: bool) -> bool {
    env_map && fog && quality
}

/// Frames a warm-up view has been final (see [`SETTLE_FRAMES`]).
#[derive(Component, Debug, Default)]
struct Settling(u32);

/// Activates warm-up cameras (Loading and menu) once their view has been final for
/// [`SETTLE_FRAMES`].
#[allow(clippy::type_complexity)]
fn activate_views(
    mut commands: Commands,
    mut cameras: Query<
        (
            Entity,
            &mut Camera,
            Option<&mut Settling>,
            Has<EnvironmentMapLight>,
            Has<DistanceFog>,
            Has<ShadowFilteringMethod>,
        ),
        Or<(
            With<WarmupCamera>,
            With<super::menu_warmup::MenuWarmupCamera>,
        )>,
    >,
) {
    for (entity, mut camera, settling, env_map, fog, quality) in &mut cameras {
        if camera.is_active || !view_final(env_map, fog, quality) {
            continue;
        }
        match settling {
            None => {
                commands.entity(entity).insert(Settling::default());
            }
            Some(mut frames) if frames.0 + 1 >= SETTLE_FRAMES => {
                camera.is_active = true;
                frames.0 = SETTLE_FRAMES;
            }
            Some(mut frames) => frames.0 += 1,
        }
    }
}

/// The track has a shadowed point or spot light (lamps at night): a shadowed point light at the
/// warm-up cars compiles their cube-map shadow pipelines (otherwise compiled when a car first
/// drives into the lamp's range, B28).
#[allow(clippy::type_complexity)]
fn add_lamp(
    mut commands: Commands,
    cameras: Query<Entity, With<WarmupCamera>>,
    lamps: Query<(), (With<WarmupLamp>, Without<WarmupCamera>)>,
    points: Query<&PointLight, Without<WarmupLamp>>,
    spots: Query<&SpotLight>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let shadowed =
        points.iter().any(|l| l.shadow_maps_enabled) || spots.iter().any(|l| l.shadow_maps_enabled);
    if shadowed && lamps.is_empty() {
        commands.spawn((
            warmup_lamp(),
            Transform::from_xyz(0.0, 0.0, -WARMUP_DISTANCE_U + 2.0),
            ChildOf(camera),
        ));
    }
}

/// A short-range shadowed point light (perspective shadow views of everything near it).
pub(super) fn warmup_lamp() -> impl Bundle {
    (
        Name::new("Warm-up lamp"),
        WarmupLamp,
        PointLight {
            intensity: 0.0,
            range: LAMP_RANGE_U,
            shadow_maps_enabled: true,
            ..default()
        },
    )
}

/// Whether the warm-up camera's scene is complete: its environment map is attached and every
/// warm-up car is instanced (`true` before the camera exists; [`finished`] waits for it).
pub(super) fn scene_ready(
    cameras: &Query<(Has<EnvironmentMapLight>, Option<&Children>), With<WarmupCamera>>,
    cars: &Query<Has<CarModelReady>, With<CarModel>>,
) -> bool {
    cameras.iter().all(|(env_map, children)| {
        env_map
            && children
                .into_iter()
                .flatten()
                .all(|&c| cars.get(c).unwrap_or(true))
    })
}

/// Whether a warm-up is over this frame (also counts its frames): `ready` = the scene is complete
/// and nothing else is still loading into it.
pub(super) fn finished(
    warmup: Option<&mut Warmup>,
    queue: &PipelineQueue,
    dt: f32,
    has_track: bool,
    ready: bool,
) -> bool {
    let Some(warmup) = warmup else {
        // Nothing to warm up without a track (headless tools, tests).
        return !has_track;
    };
    warmup.frames += 1;
    warmup.elapsed += dt;
    let created = queue.created();
    let settled = ready && queue.waiting() == 0 && created == warmup.created;
    warmup.created = created;
    warmup.idle = if settled { warmup.idle + 1 } else { 0 };
    let done =
        (warmup.frames >= MIN_FRAMES && warmup.idle >= IDLE_FRAMES) || warmup.elapsed > TIMEOUT_S;
    if done {
        info!(
            "visuals: pipelines warm after {} frames ({:.1} s)",
            warmup.frames, warmup.elapsed
        );
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warmup_needs_a_complete_scene_and_settled_pipelines_or_times_out() {
        let queue = PipelineQueue::default();
        let mut world = World::new();
        world.insert_resource(Warmup::default());
        let step = |world: &mut World, waiting: usize, created: usize, ready: bool, dt: f32| {
            queue.0[0].store(waiting, Ordering::Relaxed);
            queue.0[1].store(created, Ordering::Relaxed);
            let mut warmup = world.get_resource_mut::<Warmup>();
            finished(warmup.as_deref_mut(), &queue, dt, true, ready)
        };
        for _ in 0..MIN_FRAMES {
            assert!(
                !step(&mut world, 3, 40, true, 0.016),
                "pipelines still compiling"
            );
        }
        for _ in 0..IDLE_FRAMES + 2 {
            assert!(
                !step(&mut world, 0, 40, false, 0.016),
                "cars not instanced yet, or another loader still loading"
            );
        }
        for _ in 0..IDLE_FRAMES - 1 {
            assert!(!step(&mut world, 0, 40, true, 0.016));
        }
        assert!(
            !step(&mut world, 0, 46, true, 0.016),
            "new variants (environment map)"
        );
        for _ in 0..IDLE_FRAMES - 1 {
            assert!(!step(&mut world, 0, 46, true, 0.016));
        }
        assert!(step(&mut world, 0, 46, true, 0.016), "settled long enough");

        world.insert_resource(Warmup::default());
        assert!(!step(&mut world, 9, 9, false, 1.0));
        assert!(step(&mut world, 9, 9, false, TIMEOUT_S), "times out");
        assert!(
            !finished(None, &queue, 0.016, true, true),
            "waits for the camera"
        );
        assert!(
            finished(None, &queue, 0.016, false, true),
            "no track: nothing to warm"
        );
    }

    #[test]
    fn a_warmup_view_draws_only_once_its_keys_are_final() {
        assert!(view_final(true, true, true));
        assert!(!view_final(false, true, true), "environment map pending");
        assert!(!view_final(true, false, true), "look pending");
        assert!(!view_final(true, true, false), "quality pending");
    }
}
