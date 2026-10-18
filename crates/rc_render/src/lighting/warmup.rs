//! Pipeline warm-up while Loading: Bevy specialises (and compiles) render pipelines the first time
//! a material/mesh/view combination is drawn, which froze the first seconds of every race. Once
//! every other loader is done, a hidden [`WarmupCamera`] — a `QualityCamera` like the game views,
//! drawn under the loading screen — looks down on the whole track (the car visuals put one model
//! per racer in front of it), and `"visuals"` completes only when that scene is complete (cars
//! instanced, the generated environment map attached: both change the pipeline keys) and the
//! render world has neither queued nor created a pipeline for a few frames (or after a timeout).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::prelude::*;
use bevy::render::render_resource::PipelineCache;
use bevy::render::{Render, RenderApp, RenderSystems};
use rc_core::prelude::*;
use rc_track::Track;

use crate::cars::{CarModel, CarModelReady};
use crate::quality::QualityCamera;

/// Frames the warm-up camera renders at least (its first frame queues the pipelines; with
/// synchronous compilation the next one already finds them built). Kept low: an occluded window
/// is throttled to 1 frame/s by the driver, and loading must not wait on frame counts.
const MIN_FRAMES: u32 = 3;
/// Consecutive frames with a complete scene, an empty pipeline queue and no new pipelines before
/// the warm-up counts as done (the render world reports a frame late; a new environment map
/// reaches the view keys a frame after it is attached).
const IDLE_FRAMES: u32 = 3;
/// Never hold the loading screen longer than this for pipelines (s, wall clock).
const TIMEOUT_S: f32 = 20.0;
/// Camera height over the track as a multiple of its largest horizontal extent.
const HEIGHT_PER_EXTENT: f32 = 1.1;

/// The hidden camera that warms pipelines up during Loading. Car visuals attach one `CarModel`
/// per racer to it (in front of it, inside its view).
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct WarmupCamera;

/// Pipelines waiting to compile and pipelines created so far, written by the render world every
/// frame.
#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct PipelineQueue(Arc<[AtomicUsize; 2]>);

impl PipelineQueue {
    fn waiting(&self) -> usize {
        self.0[0].load(Ordering::Relaxed)
    }

    fn created(&self) -> usize {
        self.0[1].load(Ordering::Relaxed)
    }
}

/// Warm-up progress.
#[derive(Resource, Debug, Default)]
pub(super) struct Warmup {
    frames: u32,
    idle: u32,
    elapsed: f32,
    created: usize,
}

pub(super) fn build(app: &mut App) {
    let queue = PipelineQueue::default();
    app.insert_resource(queue.clone());
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(queue)
            .add_systems(Render, count_waiting.in_set(RenderSystems::Cleanup));
    }
}

fn count_waiting(cache: Res<PipelineCache>, queue: Res<PipelineQueue>) {
    queue.0[0].store(cache.waiting_pipelines().count(), Ordering::Relaxed);
    queue.0[1].store(cache.pipelines().count(), Ordering::Relaxed);
}

/// Spawns the warm-up camera once every loader but ours is done (the scene is complete).
pub(super) fn start(
    mut commands: Commands,
    tasks: Option<Res<LoadingTasks>>,
    track: Option<Res<Track>>,
    cameras: Query<(), With<WarmupCamera>>,
) {
    let others_done = tasks.is_some_and(|t| t.pending().all(|k| k == crate::LOADING_KEY));
    let (Some(track), true, true) = (track, others_done, cameras.is_empty()) else {
        return;
    };
    let (min, max) = track.bounds();
    let extent = (max.x - min.x).max(max.z - min.z);
    let centre = Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5);
    commands.spawn((
        Name::new("Pipeline warm-up camera"),
        WarmupCamera,
        QualityCamera,
        Camera3d::default(),
        Camera {
            order: -1,
            ..default()
        },
        Transform::from_translation(centre + Vec3::Y * extent * HEIGHT_PER_EXTENT)
            .looking_at(centre, Vec3::NEG_Z),
        DespawnOnExit(AppState::Loading),
    ));
    commands.insert_resource(Warmup::default());
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

/// Whether the warm-up is over this frame (also counts its frames).
pub(super) fn finished(
    warmup: Option<&mut Warmup>,
    queue: &PipelineQueue,
    dt: f32,
    has_track: bool,
    scene_ready: bool,
) -> bool {
    let Some(warmup) = warmup else {
        // Nothing to warm up without a track (headless tools, tests).
        return !has_track;
    };
    warmup.frames += 1;
    warmup.elapsed += dt;
    let created = queue.created();
    let settled = scene_ready && queue.waiting() == 0 && created == warmup.created;
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
                "cars not instanced yet"
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
}
